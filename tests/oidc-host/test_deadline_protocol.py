"""Real controlled TLS readers; no fault is attributed to the Keycloak service."""
import importlib.util
import os
from pathlib import Path
import socket
import ssl
import threading
import time
import unittest

spec = importlib.util.spec_from_file_location('flow', Path(__file__).parents[2]/'examples/oidc-host/flow.py')
flow = importlib.util.module_from_spec(spec)
spec.loader.exec_module(flow)

@unittest.skipUnless(os.environ.get('ROM_EXTRAS_KEYCLOAK_FIXTURE'), 'Explicit controlled fixture CA required')
class AbsoluteDeadlineProtocolTests(unittest.TestCase):
    def check_trickle(self, chunk_header):
        fixture = Path(os.environ['ROM_EXTRAS_KEYCLOAK_FIXTURE'])
        context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        context.load_cert_chain(fixture/'tls/server.crt', fixture/'tls/server.key')
        listener = socket.socket()
        listener.bind(('127.0.0.1',0))
        listener.listen(1)
        listener.settimeout(2)
        port = listener.getsockname()[1]
        stop = threading.Event()
        def serve():
            try:
                conn, _ = listener.accept()
                with context.wrap_socket(conn, server_side=True) as tls:
                    tls.settimeout(2)
                    tls.recv(16384)
                    if chunk_header:
                        tls.sendall(b'HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n')
                        prefix, suffix = b'0;', b'\r\n\r\n'
                    else:
                        prefix, suffix = b'HTTP/1.1 200 OK\r\nX-Fixture: ', b'\r\nContent-Length: 0\r\n\r\n'
                    tls.sendall(prefix)
                    for _ in range(32):
                        if stop.wait(.02):
                            return
                        tls.sendall(b'x')
                    tls.sendall(suffix)
            except OSError:
                pass  # expected when the host cancels this controlled connection
            finally:
                listener.close()
        worker = threading.Thread(target=serve,daemon=True)
        worker.start()
        try:
            base = f'https://127.0.0.1:{port}'
            host = flow.Host(base,str(fixture/'tls/ca.crt'))
            host.deadline = time.monotonic()+.15
            begin = time.monotonic()
            with self.assertRaises(flow.Rejected):
                host.request(base+'/controlled')
            self.assertLess(time.monotonic()-begin,.4, 'trickled protocol fields exceeded absolute deadline')
        finally:
            stop.set()
            listener.close()
            worker.join(3)
            self.assertFalse(worker.is_alive())
    def test_trickled_response_headers_are_cancelled(self):
        self.check_trickle(False)
    def test_trickled_chunk_headers_are_cancelled(self):
        self.check_trickle(True)

if __name__ == '__main__':
    unittest.main()
