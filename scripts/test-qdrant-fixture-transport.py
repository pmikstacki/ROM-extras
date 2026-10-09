"""Two controlled loopback receivers: redirect denial must keep credentials off the destination."""
import http.server,ssl,threading,urllib.request,urllib.error
from lib.qdrant_fixture_transport import client
hits=[]
class Receiver(http.server.BaseHTTPRequestHandler):
 def log_message(self,*_args): pass
 def do_GET(self):
  hits.append(self.headers.get('api-key'));self.send_response(200);self.end_headers()
destination=http.server.ThreadingHTTPServer(('127.0.0.1',0),Receiver)
class Redirect(http.server.BaseHTTPRequestHandler):
 def log_message(self,*_args): pass
 def do_GET(self):
  self.send_response(302);self.send_header('Location',f'http://127.0.0.1:{destination.server_port}/');self.end_headers()
source=http.server.ThreadingHTTPServer(('127.0.0.1',0),Redirect)
threads=[threading.Thread(target=s.serve_forever,daemon=True) for s in [source,destination]]
for thread in threads:thread.start()
try:
 request=urllib.request.Request(f'http://127.0.0.1:{source.server_port}/',headers={'api-key':'synthetic-never-forward'})
 try:
  with client(ssl.create_default_context()).open(request,timeout=2) as response:
   raise AssertionError(f'Redirect was followed, destination status {response.status}')
 except urllib.error.HTTPError as error: assert error.code==302
 assert hits==[],f'Redirect destination received {len(hits)} request(s)'
 print('Fixture redirects rejected; destination received zero requests')
finally:
 for server in [source,destination]:server.shutdown();server.server_close()
 for thread in threads:thread.join(timeout=3);assert not thread.is_alive()
