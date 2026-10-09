"""Native Collector evidence and separate controlled HTTP failure evidence; no provisioning."""
import base64, collections, hashlib, http.client, http.server, json, os, pathlib, ssl, subprocess, threading, time, uuid
from lib.fixture_processes import install_interrupt_handlers, wait_owned, defer_interrupts
install_interrupt_handlers()
root=pathlib.Path(__file__).resolve().parents[1]
fixture=pathlib.Path(os.environ['ROM_EXTRAS_OTEL_NATIVE_FIXTURE']).resolve()
metadata=json.loads((fixture/'fixture.json').read_text())
name=os.environ['ROM_EXTRAS_OTEL_NATIVE_CONTAINER']
assert name==metadata['container'] and name.startswith('rom-extras-collector-native-')
for field,key in [('Id','container_id'),('Image','image_id')]:
    assert subprocess.check_output(['docker','inspect','--format','{{.'+field+'}}',name],text=True).strip()==metadata[key]
assert subprocess.check_output(['docker','inspect','--format','{{.State.Running}}',name],text=True).strip()=='true'
version=subprocess.check_output(['docker','exec',name,'/otelcol-contrib','--version'],text=True,timeout=5).strip()
assert version=='otelcol-contrib version '+metadata['native_version']
run=root/'.superpowers/opentelemetry-native-runs'/uuid.uuid4().hex
run.mkdir(parents=True,mode=0o700)
credentials=json.loads((fixture/'credentials.json').read_text())
auth='Basic '+base64.b64encode((credentials['username']+':'+credentials['password']).encode()).decode()
ca=fixture/'tls/ca.pem'
context=ssl.create_default_context(cafile=str(ca))
base={**{key:value for key,value in os.environ.items() if not key.startswith('OTEL_')},'ROM_EXTRAS_OTEL_CA':str(ca),'ROM_EXTRAS_OTEL_AUTHORIZATION':auth}
base.pop('ROM_EXTRAS_OTEL_CONNECT_ADDRESS',None)
command=['cargo','run','--manifest-path',os.environ.get('ROM_EXTRAS_OTEL_PACKAGED_MANIFEST',str(root/'tests/opentelemetry-native-consumer/Cargo.toml')),'--locked','--offline','--target-dir',str(root/('target/opentelemetry-native-packaged-consumer' if 'ROM_EXTRAS_OTEL_PACKAGED_MANIFEST' in os.environ else 'target/opentelemetry-native-consumer'))]
results=[]

def consume(case,endpoint=None,address=None):
    env={**base,'ROM_EXTRAS_OTEL_CASE':case,'ROM_EXTRAS_OTEL_ENDPOINT':endpoint or metadata['endpoint'],'ROM_EXTRAS_OTEL_RUNTIME_PATH':str(run/case/'runtime')}
    if address:env['ROM_EXTRAS_OTEL_CONNECT_ADDRESS']=address
    if case=='env_guard':env['OTEL_EXPORTER_OTLP_HEADERS']='authorization=private-poison-canary'
    log=run/(case+'.log');start=time.monotonic()
    with log.open('wb') as output:
        code=wait_owned(command,cwd=root,env=env,stdout=output,stderr=subprocess.STDOUT,timeout=45)
    if case=='env_guard':
        assert code!=0 and 'SDK environment overrides are forbidden' in log.read_text()
        assert 'private-poison-canary' not in log.read_text()
    else:assert code==0, 'consumer rejected case '+case+'; see private preserved log'
    results.append({'case':case,'seconds':round(time.monotonic()-start,3),'exit_code':code,'log':str(log)})

def exports():
    values={}
    for signal_name in ['metrics','traces']:
        path=fixture/'exports'/(signal_name+'.jsonl')
        size=path.stat().st_size if path.exists() else 0
        assert size<=16*1024*1024, 'preserve this fixture and choose a fresh one'
        values[signal_name]=path.read_bytes() if path.exists() else b''
    return values

def settle():time.sleep(.3)

before=exports();consume('env_guard');settle();assert exports()==before
before=exports();consume('happy');settle();after=exports()
received={}
for signal_name in ['metrics','traces']:
    assert after[signal_name].startswith(before[signal_name])
    delta=after[signal_name][len(before[signal_name]):]
    assert delta and len(delta)<=1048576
    for canary in ['private-telemetry-fixture-resource-actor-error',credentials['password'],auth]:assert canary.encode() not in delta
    received[signal_name]=[json.loads(line) for line in delta.splitlines()]
    (run/(signal_name+'.jsonl')).write_bytes(delta)
metrics=[m for doc in received['metrics'] for resource in doc['resourceMetrics'] for scope in resource['scopeMetrics'] for m in scope['metrics']]
by_name={m['name']:m for m in metrics}
assert len(by_name)==9 # status-failure counter has no recorded samples.
assert by_name['rom.host.operations']['unit']=='{operation}'
counts={tuple(sorted((a['key'],a['value']['stringValue']) for a in p['attributes'])):int(p['asInt']) for p in by_name['rom.host.operations']['sum']['dataPoints']}
expected={('action','success'):6,('action','rejected'):2,('action','denied'):2,('work','unknown'):1,('work','not_committed'):1,('work','accepted'):1,('recovery','abandoned'):1}
assert counts=={(('operation',op),('outcome',outcome)):count for (op,outcome),count in expected.items()}
assert by_name['rom.host.operations']['sum']['isMonotonic'] is True
assert by_name['rom.host.operation.duration']['unit']=='s'
assert sum(int(p['count']) for p in by_name['rom.host.operation.duration']['histogram']['dataPoints'])==14
for p in by_name['rom.runtime.intake']['gauge']['dataPoints']:
    state=p['attributes'][0]['value']['stringValue'];assert int(p['asInt'])==int(state=='stopped')
assert int(by_name['rom.runtime.ready']['gauge']['dataPoints'][0]['asInt'])==0
spans=[span for doc in received['traces'] for resource in doc['resourceSpans'] for scope in resource['scopeSpans'] for span in scope['spans']]
assert len(spans)==14
for span in spans:
    assert span['traceId']=='01'*16 and span['parentSpanId']=='02'*8
    assert len(span['attributes'])==2 and not span.get('events') and not span.get('links')
    assert span['name'] in ['rom.host.action','rom.host.work','rom.host.recovery']
    assert int(span['endTimeUnixNano'])>=int(span['startTimeUnixNano'])
assert collections.Counter((s['name'],next(a['value']['stringValue'] for a in s['attributes'] if a['key']=='outcome')) for s in spans)==collections.Counter({('rom.host.'+op,outcome):count for (op,outcome),count in expected.items()})
for case in ['missing_auth','wrong_auth','untrusted_ca','wrong_identity','refused']:
    before=exports()
    endpoint='https://wrong.fixture.test:4318' if case=='wrong_identity' else 'https://10.246.72.3:4318' if case=='refused' else None
    consume(case,endpoint);settle();assert exports()==before
# An actual owned Collector stop must not rewrite any durable Runtime result.
before=exports()
try:
    with defer_interrupts():subprocess.run(['docker','stop','--time','3',name],check=True,stdout=subprocess.DEVNULL,timeout=8)
    consume('collector_stopped');assert exports()==before
finally:
    with defer_interrupts():subprocess.run(['docker','start',name],check=True,stdout=subprocess.DEVNULL,timeout=8)
ready=False
for _ in range(40):
    try:
        connection=http.client.HTTPSConnection('10.246.72.2',4318,context=context,timeout=.5)
        connection.request('POST','/v1/metrics',b'',{'Content-Type':'application/x-protobuf'})
        response=connection.getresponse();assert response.status==401;response.read(4096);connection.close();ready=True;break
    except (OSError,http.client.HTTPException):time.sleep(.1)
assert ready
assert exports()==before
def valid_padded_request(size):
    # Valid unknown LEN field: a parser accepts the boundary request; rejection cannot be attributed to malformed protobuf.
    def varint(value):
        data=bytearray()
        while value>=128:data.append((value&127)|128);value>>=7
        data.append(value);return bytes(data)
    payload=b'\0'*(size-5)
    body=varint((2047<<3)|2)+varint(len(payload))+payload
    assert len(body)==size
    return body
# Exact native protocol authentication and request-size behavior.
for authorization,body,status in [(None,b'',401),('Basic aW52YWxpZDppbnZhbGlk',b'',401),(auth,valid_padded_request(65536),200),(auth,valid_padded_request(65537),400)]:
    c=http.client.HTTPSConnection('10.246.72.2',4318,context=context,timeout=3)
    try:
        headers={'Content-Type':'application/x-protobuf'}
        if authorization:headers['Authorization']=authorization
        c.request('POST','/v1/metrics',body,headers);response=c.getresponse();assert response.status==status;assert len(response.read(4097))<=4096
    finally:c.close()
# Controlled HTTP faults are separate from actual Collector qualification.
class FaultServer(http.server.ThreadingHTTPServer):
    daemon_threads=True
    block_on_close=False
class Handler(http.server.BaseHTTPRequestHandler):
    def log_message(self,*args):pass
    def do_POST(self):
        self.connection.settimeout(3)
        size=int(self.headers.get('Content-Length','0'));assert size<=65536
        data=self.rfile.read(size);assert len(data)==size
        with self.server.count_lock:self.server.requests+=1
        if self.headers.get('Authorization')!=auth:self.send_error(401);return
        case=self.server.case
        if case=='held':time.sleep(3)
        status=307 if case=='redirect' else 429 if case=='rate_limited' else 503 if case=='server_error' else 200
        body=b'\x0a\x02\x08\x01' if case=='partial_success' else b'\xff' if case=='malformed_success' else b''
        if case=='oversized_response':body=b'\0'*(4*1024*1024+1)
        try:
            self.send_response(status);self.send_header('Content-Type','application/x-protobuf');self.send_header('Content-Length',str(len(body)))
            if case=='redirect':self.send_header('Location',metadata['endpoint']+self.path)
            if case=='rate_limited':self.send_header('Retry-After','50')
            self.end_headers();self.wfile.write(body)
        except (BrokenPipeError,ConnectionResetError,ssl.SSLError):pass
for case in ['client_limit','redirect','rate_limited','server_error','held','oversized_response','partial_success','malformed_success']:
    server=FaultServer(('127.0.0.1',0),Handler);server.case=case;server.requests=0;server.count_lock=threading.Lock()
    tls=ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER);tls.load_cert_chain(fixture/'tls/server.pem',fixture/'tls/server.key');server.socket=tls.wrap_socket(server.socket,server_side=True)
    thread=threading.Thread(target=server.serve_forever,daemon=True);thread.start()
    before=exports()
    try:
        consume(case,'https://collector.fixture.test:'+str(server.server_port),'127.0.0.1:'+str(server.server_port))
        assert server.requests==(0 if case=='client_limit' else 3), 'one attempt per explicit export: metric flush, trace flush, metric shutdown'
        settle();assert exports()==before
    finally:server.shutdown();server.server_close();thread.join(timeout=3);assert not thread.is_alive()
    results[-1]['controlled_requests']=server.requests
record={'scope':'Actual Collector TLS/auth and OTLP metrics/traces for public SQLite/redb host observations; separate controlled HTTP fault cases, no committed diagnostic bridge','fixture':metadata,'actual_binary_version':version,'configuration_sha256':hashlib.sha256((fixture/'config.yaml').read_bytes()).hexdigest(),'run':str(run),'native_metric_count':len(metrics),'native_span_count':len(spans),'cases':results,'native_protocol_statuses':{'missing_auth':401,'wrong_auth':401,'accepted_valid_body_65536':200,'oversized_valid_body_65537':400},'controlled_success_is_not_acceptance':['partial_success','malformed_success'],'sha256':{signal_name:hashlib.sha256((run/(signal_name+'.jsonl')).read_bytes()).hexdigest() for signal_name in received}}
(run/'verification.json').write_text(json.dumps(record,indent=2)+'\n')
print('Native Collector: exact metrics, 14 parent-linked spans, safe canaries; 16 bounded native/controlled cases passed')
print(str(run/'verification.json'))
