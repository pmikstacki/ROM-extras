"""Actual TLS forwarding; discard one completed native PUT response per armed operation."""
import http.server, socket, ssl, threading, urllib.request
from .qdrant_fixture_transport import client
class LossFixture:
 def __init__(self,root,endpoint):
  owner=self
  context=ssl.create_default_context(cafile=str(root/'tls/ca.pem'))
  opener=client(context)
  self.armed=False;self.forwarded=0;self.lost=0;self.mode=None;self.authored=0;self.skip=0
  class Handler(http.server.BaseHTTPRequestHandler):
   def log_message(self,*_args): pass
   def do_GET(self): self.dispatch()
   def do_POST(self): self.dispatch()
   def do_PUT(self): self.dispatch()
   def dispatch(self):
    if owner.mode is not None:
     owner.authored+=1
     mode=owner.mode
     if mode=='delay':
      import time
      time.sleep(6)
     self.send_response(429 if mode=='429' else 302 if mode=='redirect' else 200)
     if mode=='oversized': self.send_header('Content-Length','1048577')
     else: self.send_header('Content-Length','0')
     if mode=='redirect': self.send_header('Location','https://example.invalid/')
     try: self.end_headers()
     except (BrokenPipeError,ConnectionResetError,ssl.SSLError): pass
     return
    length=int(self.headers.get('Content-Length','0'));assert 0<=length<=1048576
    body=self.rfile.read(length) if length else None
    request=urllib.request.Request(endpoint.rstrip('/')+self.path,data=body,headers={'api-key':self.headers['api-key'],'Content-Type':'application/json'},method=self.command)
    with opener.open(request,timeout=5) as response:
     raw=response.read(1048577);assert len(raw)<=1048576;status=response.status
    if self.command=='PUT' and owner.armed:
     import json
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
  self.server.shutdown();self.server.server_close();self.thread.join(timeout=5);assert not self.thread.is_alive()
