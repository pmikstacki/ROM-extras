import signal
import base64,email.policy,hashlib,http.client,json,os,pathlib,socket,ssl,subprocess,time,uuid
from email.parser import BytesParser
root=pathlib.Path(__file__).resolve().parents[1]
fixture=pathlib.Path(os.environ['ROM_EXTRAS_SMTP_NATIVE_FIXTURE']).resolve()
name=os.environ['ROM_EXTRAS_SMTP_NATIVE_CONTAINER']
assert name.startswith('rom-extras-mailpit-native-')
run_id=uuid.uuid4().hex
run=root/'.superpowers/smtp-native-consumer-runs'/run_id;run.mkdir(parents=True,mode=0o700)
def close_owned_group(child):
    for sig in [signal.SIGTERM,signal.SIGKILL]:
        try: os.killpg(child.pid,sig)
        except ProcessLookupError: pass
        try: child.wait(timeout=5 if sig==signal.SIGTERM else 1)
        except subprocess.TimeoutExpired: continue
    if child.poll() is None: raise RuntimeError('owned process did not close')
stage='prepare explicit host environment' 
try:
    setup=json.loads((fixture/'setup.json').read_text());credentials=json.loads((fixture/'credentials.json').read_text())
    assert subprocess.check_output(['docker','inspect','--format','{{.Image}}',name],text=True).strip()==setup['image']
    address=subprocess.check_output(['docker','inspect','--format','{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}',name],text=True).strip()
    assert subprocess.check_output(['docker','inspect','--format','{{.Id}}',name],text=True).strip()==setup['id']
    subprocess.run(['openssl','x509','-in',str(fixture/'tls/ca.pem'),'-outform','DER','-out',str(run/'ca.der')],check=True,stdout=subprocess.DEVNULL,stderr=subprocess.PIPE)
    context=ssl.create_default_context(cafile=str(fixture/'tls/ca.pem'))
    auth='Basic '+base64.b64encode((credentials['username']+':'+credentials['api_password']).encode()).decode()
    class PinnedHTTPS(http.client.HTTPSConnection):
        def connect(self):self.sock=context.wrap_socket(socket.create_connection((address,8025),timeout=2),server_hostname='mailpit.fixture.test')
    def request(path):
        c=PinnedHTTPS('mailpit.fixture.test',8025,timeout=2)
        try:
            c.request('GET',path,headers={'Host':'localhost:55470','Authorization':auth});r=c.getresponse();data=r.read(1048577)
            assert r.status==200 and len(data)<=1048576
            return data
        finally:c.close()
    def messages():return json.loads(request('/api/v1/messages?limit=1000'))
    environment={**os.environ,'ROM_EXTRAS_SMTP_ADDRESS':address+':1025','ROM_EXTRAS_SMTP_USERNAME':credentials['username'],'ROM_EXTRAS_SMTP_PASSWORD':credentials['smtp_password'],'ROM_EXTRAS_SMTP_CA_DER':str(run/'ca.der'),'ROM_EXTRAS_SMTP_RUN_ID':run_id}
    manifest=os.environ.get('ROM_EXTRAS_SMTP_PACKAGED_MANIFEST','tests/smtp-public-consumer/Cargo.toml')
    target='target/smtp-packaged-consumer' if 'ROM_EXTRAS_SMTP_PACKAGED_MANIFEST' in os.environ else 'target/smtp-public-consumer'
    def consumer(phase,label):
        with (run/(label+'.log')).open('w') as log:
            child=subprocess.Popen(['cargo','run','--locked','--offline','--manifest-path',manifest,'--target-dir',target,'--',phase],env=environment,stdout=log,stderr=subprocess.STDOUT,start_new_session=True,cwd=root)
            try:
                assert child.wait(timeout=60)==0, 'public consumer failed'
            finally:
                close_owned_group(child)
    before=messages()['total'];stage='native DATA acceptance';consumer('accept','before-restart')
    first=messages();assert first['total']==before+1
    selected=[m for m in first['messages'] if m['Subject']=='native-'+run_id];assert len(selected)==1
    raw=request('/api/v1/message/'+selected[0]['ID']+'/raw');(run/'before-restart.eml').write_bytes(raw)
    message=BytesParser(policy=email.policy.default).parsebytes(raw)
    assert message['Message-ID']=='<'+hashlib.sha256(('native-'+run_id).encode()).hexdigest()+'@host.example.invalid>'
    assert message['MIME-Version']=='1.0'
    assert message.get_content().replace('\r\n','\n')=='Treść natywna\n.\n'
    stage='native authentication and TLS refusals';consumer('negative','negative');assert messages()['total']==before+1
    stage='restart owned native fixture'
    subprocess.run(['docker','restart','--time','5',name],check=True,stdout=subprocess.DEVNULL,stderr=subprocess.PIPE,timeout=30)
    address=subprocess.check_output(['docker','inspect','--format','{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}',name],text=True).strip()
    environment['ROM_EXTRAS_SMTP_ADDRESS']=address+':1025'
    deadline=time.monotonic()+10
    while True:
        try:
            after=messages();assert after['total']==before+1;break
        except (OSError,http.client.HTTPException):
            if time.monotonic()>=deadline:raise RuntimeError()
            time.sleep(0.2)
    assert any(m['ID']==selected[0]['ID'] for m in after['messages'])
    stage='explicit same-identity resubmission';consumer('accept','after-restart')
    after=messages();assert after['total']==before+2
    duplicate=[m for m in after['messages'] if m['Subject']=='native-'+run_id];assert len(duplicate)==2
    ids=[]
    for i,m in enumerate(duplicate):
        raw=request('/api/v1/message/'+m['ID']+'/raw');(run/('duplicate-'+str(i)+'.eml')).write_bytes(raw)
        parsed=BytesParser(policy=email.policy.default).parsebytes(raw);ids.append(str(parsed['Message-ID']))
        assert parsed.get_content().replace('\r\n','\n')=='Treść natywna\n.\n'
    assert ids[0]==ids[1]
    stage='native post-storage acknowledgment loss and public Runtime recovery'
    proxy_record=run/'suppressed-acknowledgments.jsonl'
    proxy_env={**os.environ,'ROM_EXTRAS_SMTP_NATIVE_IP':address,'ROM_EXTRAS_SMTP_NATIVE_FIXTURE':str(fixture),'ROM_EXTRAS_SMTP_PROXY_RECORD':str(proxy_record)}
    with (run/'proxy.log').open('w') as proxy_log:
        proxy=subprocess.Popen(['node','tests/smtp-public-consumer/ack-proxy.mjs'],env=proxy_env,stdout=subprocess.PIPE,stderr=proxy_log,text=True,start_new_session=True,cwd=root)
        try:
            import select
            assert select.select([proxy.stdout],[],[],10)[0], 'proxy startup deadline'
            port=json.loads(proxy.stdout.readline())['port']
            environment['ROM_EXTRAS_SMTP_ADDRESS']='127.0.0.1:'+str(port)
            environment['ROM_EXTRAS_SMTP_RUNTIME_PATH']=str(run/'runtime')
            consumer('runtime','runtime')
        finally:
            close_owned_group(proxy)
    observed=[json.loads(line) for line in proxy_record.read_text().splitlines()]
    assert len(observed)==2 and all(row['native_data_acknowledgment_observed'] and not row['acknowledgment_forwarded'] for row in observed)
    final=messages();assert final['total']==before+6
    for backend in ['sqlite','redb']:
        subject='runtime-'+run_id+'-'+backend
        assert len([row for row in observed if row['subject']==subject])==1
        pair=[m for m in final['messages'] if m['Subject']==subject];assert len(pair)==2
        parsed=[]
        for i,m in enumerate(pair):
            raw=request('/api/v1/message/'+m['ID']+'/raw');(run/(backend+'-'+str(i)+'.eml')).write_bytes(raw)
            mail=BytesParser(policy=email.policy.default).parsebytes(raw)
            assert mail.get_content().replace('\r\n','\n')=='Treść natywna\n.\n'
            parsed.append(mail)
        assert parsed[0]['Message-ID']==parsed[1]['Message-ID']
        assert parsed[0]['Date']==parsed[1]['Date']
        assert not any(m['Subject'] in ['denied-source-'+subject,'denied-service-'+subject] for m in final['messages'])
    record={'scope':'Independent public Rust SMTP consumer, native Mailpit DATA/TLS/AUTH, persistence/restart and explicit duplicate risk, native post-storage acknowledgment loss, SQLite/redb public Runtime recovery and authority checks; no internet delivery','run_id':run_id,'consumer_manifest':manifest,'before_count':before,'final_count':final['total'],'runtime_backends':['sqlite','redb'],'native_post_storage_acknowledgments_suppressed':2,'denied_notifications_submitted':0,'same_Message_ID_duplicate_count':2,'original_native_message_survived_restart':True,'native_image':setup['image'],'native_tag':setup['tag'],'log_directory':str(run),'outcome':'passed'}
    (run/'evidence.json').write_text(json.dumps(record,indent=2)+'\n');print(json.dumps(record))
except Exception as e:
    print('Native consumer check failed at '+stage+'; category '+type(e).__name__+'; logs and database retained at '+str(run))
    raise SystemExit(1)
