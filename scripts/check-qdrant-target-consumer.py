"""Isolated native fixture; retain every collection, private config and owned-process log."""
import base64, hashlib, hmac, json, os, ssl, sys, tempfile, time, uuid
from pathlib import Path
import subprocess, urllib.request, urllib.error
from lib.qdrant_loss_fixture import LossFixture
from lib.qdrant_fixture_transport import client
from lib.fixture_processes import install_interrupt_handlers, wait_owned
install_interrupt_handlers()
root=Path(os.environ.get('ROM_EXTRAS_QDRANT_TARGET_FIXTURE','.superpowers/qdrant-target-native-port55471-20261009')).resolve()
container='rom-extras-qdrant-target-native-port55471-20261009'
mount=subprocess.run(['docker','inspect','--format','{{range .Mounts}}{{if eq .Destination "/qdrant/config/production.yaml"}}{{.Source}}{{end}}{{end}}',container],capture_output=True,text=True,timeout=5,check=True).stdout.strip()
assert Path(mount).resolve()==root/'fixture.yaml','Selected root does not belong to the owned native container'
admin=json.loads((root/'credentials.json').read_text())['admin']
endpoint='https://127.0.0.1:55471/'
run=Path(tempfile.mkdtemp(prefix='run-',dir=root)).resolve(); run.chmod(0o700)
def encode(value):
 return base64.urlsafe_b64encode(json.dumps(value,separators=(',',':')).encode()).rstrip(b'=')
def token(name,access='prw'):
 data=encode({'alg':'HS256','typ':'JWT'})+b'.'+encode({'exp':int(time.time())+3600,'access':[{'collection':name,'access':access}]})
 return (data+b'.'+base64.urlsafe_b64encode(hmac.new(admin.encode(),data,hashlib.sha256).digest()).rstrip(b'=')).decode()
rows=[]
for metric in ['dot','euclid','manhattan']:
 name='rom_extras_target_'+metric+'_'+uuid.uuid4().hex
 rows.append({'physical':name,'nonce':uuid.uuid4().hex,'writer':token(name),'reader':token(name,'r')})
config={'endpoint':endpoint,'ca':str(root/'tls/ca.pem'),'admin':admin,'generations':rows}
config_path=run/'host.json';config_path.write_text(json.dumps(config));config_path.chmod(0o600)
binary=str(Path(sys.argv[1]).resolve())
# No secret data is printed by the definition command.
result=subprocess.run([binary,'--definitions',str(config_path)],capture_output=True,timeout=10,check=True)
definitions=json.loads(result.stdout)
context=ssl.create_default_context(cafile=config['ca'])
opener=client(context)
def call(method,path,value,key):
 request=urllib.request.Request(endpoint+path,data=json.dumps(value).encode(),headers={'api-key':key,'Content-Type':'application/json'},method=method)
 try:
  with opener.open(request,timeout=5) as response:
   data=response.read(1048577);assert len(data)<=1048576;return response.status,json.loads(data)
 except urllib.error.HTTPError as error:
  return error.code,None
for row,definition in zip(rows,definitions):
 name=row['physical'];assert call('PUT','collections/'+name,definition,admin)[0]==200
 # Actual service enforcement, not merely claims decoded by a client.
 for method,path,value in [('PUT','collections/forbidden_'+uuid.uuid4().hex,definition),('DELETE','collections/'+name,{}),('PATCH','collections/'+name,{'metadata':{'bad':True}}),('PUT','collections/'+name+'/vectors/extra',{'dense':{'size':3,'distance':'Dot'}}),('PUT','collections/'+name+'/index',{'field_name':'extra','field_schema':'keyword'}),('POST','collections/'+rows[(rows.index(row)+1)%3]['physical']+'/points',{'ids':[]})]:
  status,_=call(method,path,value,row['writer']);assert status in [401,403],f'Expected native scoped denial for {method} {path}, got {status}'
for row in rows:
 status,_=call('PUT','collections/'+row['physical']+'/points?wait=true',{'points':[]},row['reader'])
 assert status in [401,403],'Native read-only credential unexpectedly allowed writes'
proxy=LossFixture(root,endpoint,admin)
try:
 lossy={**config,'endpoint':proxy.endpoint}
 loss_path=run/'loss-host.json';loss_path.write_text(json.dumps(lossy));loss_path.chmod(0o600)
 # Authored protocol refusals are recorded separately from actual native-service writes.
 for mode in ['429','oversized','redirect','delay','cancel']:
  proxy.mode='delay' if mode=='cancel' else mode
  with (run/('authored-'+mode+'.log')).open('x') as output:
   code=wait_owned([binary,'--refusal',str(loss_path),mode],timeout=10,stdout=output,stderr=output)
  assert code==0,f'Authored refusal {mode} failed; retained {run}'
 assert proxy.forwarded==0 and proxy.lost==0
 proxy.mode=None
 if os.environ.get('ROM_EXTRAS_QDRANT_TARGET_REPLAY_LOSS_PROBE')!='1':
  for backend in ['sqlite','redb']:
   database=run/('vector-'+backend);database.mkdir(mode=0o700)
   with (run/('vector-'+backend+'.log')).open('x') as output:
    code=wait_owned([binary,'--vector',str(loss_path),str(database),backend],timeout=70,stdout=output,stderr=output)
   assert code==0,f'Native vector {backend} failed; retained {run}'
  assert proxy.native_queries>0

 loss_probe=os.environ.get('ROM_EXTRAS_QDRANT_TARGET_REPLAY_LOSS_PROBE')=='1'
 for backend in (['sqlite'] if loss_probe else ['sqlite','redb']):
  database=run/backend;database.mkdir(mode=0o700)
  proxy.armed=True;proxy.skip=1
  for phase,cfg,expected in [('write',loss_path,70),('replay',config_path,71 if loss_probe else 0)]:
   with (run/(backend+'-'+phase+'.log')).open('x') as output:
    code=wait_owned([binary,'--recovery-'+phase,str(cfg),str(database),backend],timeout=30,stdout=output,stderr=output)
   assert code==expected,f'Recovery {backend}/{phase} failed: {code}; retained {run}'
   if phase=='write':
    with (run/(backend+'-native-restart.log')).open('x') as output:
     code=wait_owned(['docker','restart','--time','5',container],timeout=20,stdout=output,stderr=output)
    assert code==0,'Owned Qdrant fixture restart failed'
    deadline=time.monotonic()+15
    while True:
     try:
      status,_=call('GET','collections/'+rows[0]['physical'],{},rows[0]['writer'])
      if status==200: break
     except (urllib.error.URLError,TimeoutError,ConnectionError): pass
     assert time.monotonic()<deadline,'Owned Qdrant fixture not ready after restart'
     time.sleep(.1)
    if loss_probe:
     assert call('POST','collections/'+rows[0]['physical']+'/points/delete?wait=true&ordering=strong',{'filter':{'must':[{'key':'rom_id','match':{'value':backend+'-a'}}]}},admin)[0]==200

 assert proxy.forwarded==(2 if loss_probe else 4) and proxy.lost==(1 if loss_probe else 2)
finally:
 proxy.close()
if loss_probe:
 print('Missing native state was refused before replay; retained evidence:',run)
 sys.exit(0)
log=run/'native.log'
with log.open('x') as output:
 log.chmod(0o600)
 code=wait_owned([binary,'--native',str(config_path)],timeout=60,stdout=output,stderr=output)
assert code==0,f'Native consumer failed; retained log {log}'
secrets=[admin,*[row[key] for row in rows for key in ['writer','reader']]]
for log in run.glob('*.log'):
 text=log.read_text()
 assert not any(secret in text for secret in secrets),'Credential appeared in an owned fixture log'
print('Native Qdrant target and separately authored refusals passed; retained evidence:',run)
