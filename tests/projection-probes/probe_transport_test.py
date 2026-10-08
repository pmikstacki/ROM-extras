"""Controlled redirect regression; no backend credentials are used."""
import http.server
import ssl
import threading
import urllib.error
import urllib.request

from probe_transport import client


class Redirect(http.server.BaseHTTPRequestHandler):
    calls = []

    def answer(self):
        self.calls.append(self.path)
        if self.path == '/target':
            self.send_response(200)
        else:
            self.send_response(int(self.path.rsplit('/', 1)[1]))
            self.send_header('Location', '/target')
        self.end_headers()

    do_GET = answer
    do_POST = answer

    def log_message(self, *args):
        pass


server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Redirect)
thread = threading.Thread(target=server.serve_forever, daemon=True)
thread.start()
try:
    transport = client(ssl.create_default_context())
    for status, method in [(301, 'GET'), (302, 'GET'), (303, 'POST'), (307, 'POST'), (308, 'POST')]:
        path = f'/redirect/{status}'
        request = urllib.request.Request(f'http://127.0.0.1:{server.server_port}' + path,
                                         method=method, data=b'x' if method == 'POST' else None)
        before = len(Redirect.calls)
        try:
            transport.open(request, timeout=3)
        except urllib.error.HTTPError as error:
            assert error.code == status
        else:
            raise AssertionError('redirect followed')
        assert Redirect.calls[before:] == [path]
    print('PASS all 301/302/303/307/308 redirects rejected; zero target calls; no private credential used')
finally:
    server.shutdown()
    server.server_close()
    thread.join(timeout=3)
    assert not thread.is_alive()
