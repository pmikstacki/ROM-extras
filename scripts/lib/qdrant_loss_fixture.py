"""Actual TLS forwarding; discard one completed native PUT response per armed operation."""
import http.server, socket, ssl, threading, urllib.request, json, hmac
from .qdrant_fixture_transport import client
class LossFixture:
 def __init__(self,root,endpoint,control_key=None):
  owner=self
  context=ssl.create_default_context(cafile=str(root/'tls/ca.pem'))
  opener=client(context)
  self.armed=False;self.forwarded=0;self.lost=0;self.mode=None;self.authored=0;self.skip=0
  self.version_value="1.19.3";self.version_other_calls=0
  self.native_requests=0
  self.query_mode=None;self.queries=0;self.native_queries=0
  self.query_started=threading.Event();self.query_release=threading.Event();self.query_release.set()
  class Handler(http.server.BaseHTTPRequestHandler):
   def log_message(self,*_args): pass
   def do_GET(self): self.dispatch()
   def do_POST(self): self.dispatch()
   def do_PUT(self): self.dispatch()
   def do_PATCH(self): self.dispatch()
   def dispatch(self):
    if self.path.startswith('/fixture/'):
     assert control_key is not None and hmac.compare_digest(self.headers.get('api-key',''),control_key)
     length=int(self.headers.get('Content-Length','0'));assert 0<=length<=1024
     value=json.loads(self.rfile.read(length)) if length else {}
     if self.path=='/fixture/config' and self.command=='POST':
      owner.query_release.set();owner.query_started.clear();owner.query_mode=value['mode']
      if owner.query_mode=='hold': owner.query_release.clear()
     elif self.path=='/fixture/release' and self.command=='POST': owner.query_release.set()
     else: assert self.path=='/fixture/status' and self.command=='GET'
     raw=json.dumps({'native_requests':owner.native_requests,'queries':owner.queries,'native_queries':owner.native_queries,'started':owner.query_started.is_set()}).encode()
     self.send_response(200);self.send_header('Content-Length',str(len(raw)));self.end_headers();self.wfile.write(raw);return
    if owner.mode=='version':
     if self.command!='GET' or self.path.split('?')[0]!='/': owner.version_other_calls+=1
     raw=json.dumps({'version':owner.version_value}).encode()
     self.send_response(200);self.send_header('Content-Length',str(len(raw)));self.end_headers();self.wfile.write(raw);return
    query=self.command=='POST' and self.path.split('?')[0].endswith('/points/query')
    if query: owner.queries+=1
    mode=owner.mode if owner.mode is not None else owner.query_mode if query else None
    if mode is not None and mode!='hold':
     owner.authored+=1
     if mode=='delay':
      import time
      time.sleep(6)
     self.send_response(429 if mode=='429' else 302 if mode=='redirect' else 200)
     if mode=='oversized': self.send_header('Content-Length','1048577')
     else: self.send_header('Content-Length','1' if mode=='malformed' else '0')
     if mode=='redirect': self.send_header('Location','https://example.invalid/')
     try:
      self.end_headers()
      if mode=='malformed': self.wfile.write(b'{')
     except (BrokenPipeError,ConnectionResetError,ssl.SSLError): pass
     return
    length=int(self.headers.get('Content-Length','0'));assert 0<=length<=1048576
    body=self.rfile.read(length) if length else None
    request=urllib.request.Request(endpoint.rstrip('/')+self.path,data=body,headers={'api-key':self.headers['api-key'],'Content-Type':'application/json'},method=self.command)
    owner.native_requests+=1
    with opener.open(request,timeout=5) as response:
     raw=response.read(1048577);assert len(raw)<=1048576;status=response.status
    if query:
     owner.native_queries+=1
     if mode=='hold':
      owner.query_started.set();assert owner.query_release.wait(timeout=5),'Unreleased native query response'
    if self.command=='PUT' and owner.armed:
     assert json.loads(raw)['result']['status']=='completed'
     owner.forwarded+=1
     if owner.skip:
      owner.skip-=1
     else:
      owner.armed=False;owner.lost+=1
      self.connection.shutdown(socket.SHUT_RDWR);self.connection.close();return
    self.send_response(status);self.send_header('Content-Length',str(len(raw)));self.end_headers();self.wfile.write(raw)
  self.server=http.server.ThreadingHTTPServer(('127.0.0.1',0),Handler)
  tls=ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER);tls.load_cert_chain(root/'tls/server.pem',root/'tls/server.key')
  self.server.socket=tls.wrap_socket(self.server.socket,server_side=True)
  self.thread=threading.Thread(target=self.server.serve_forever,daemon=True);self.thread.start()
  self.endpoint=f'https://127.0.0.1:{self.server.server_port}/'
 def close(self):
  self.query_release.set()
  self.server.shutdown();self.server.server_close();self.thread.join(timeout=5);assert not self.thread.is_alive()
