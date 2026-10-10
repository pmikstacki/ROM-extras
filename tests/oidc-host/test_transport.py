"""Injected transport boundaries; actual service evidence comes from qualify.py."""
import importlib.util
from pathlib import Path
import unittest
from unittest.mock import patch
import time

spec = importlib.util.spec_from_file_location('flow', Path(__file__).parents[2]/'examples/oidc-host/flow.py')
flow = importlib.util.module_from_spec(spec)
spec.loader.exec_module(flow)

class Response:
    def __init__(self, data=b'', status=200, length=None):
        self.data, self.status, self.length = data, status, length
    def getheader(self, name):
        return self.length if name == 'Content-Length' else None
    def getheaders(self):
        return []
    def read1(self, n):
        data, self.data = self.data[:n], self.data[n:]
        return data

class Connection:
    sock = None
    def __init__(self, response):
        self.response, self.calls, self.closed = response, 0, False
    def request(self, *args):
        self.calls += 1
    def getresponse(self):
        return self.response
    def close(self):
        self.closed = True

class BoundedTransportTests(unittest.TestCase):
    def host(self):
        return flow.Host('https://127.0.0.1:55475', '/unused', initialize=False)
    def test_body_limits_close_connection(self):
        for response in [Response(b'12345'), Response(length='5'), Response(length='invalid')]:
            conn = Connection(response)
            with patch.object(flow.http.client, 'HTTPSConnection', return_value=conn):
                with self.assertRaises(flow.Rejected):
                    self.host().request('https://127.0.0.1:55475/x', limit=4)
            self.assertTrue(conn.closed)
    def test_429_has_no_automatic_retry(self):
        conn = Connection(Response(b'limited', status=429))
        with patch.object(flow.http.client, 'HTTPSConnection', return_value=conn):
            status, _, _ = self.host().request('https://127.0.0.1:55475/x')
        self.assertEqual(status, 429)
        self.assertEqual(conn.calls, 1)
        self.assertTrue(conn.closed)
    def test_timeout_errors_do_not_echo_response_or_secret(self):
        conn = Connection(Response())
        with patch.object(flow.http.client,'HTTPSConnection',return_value=conn), patch.object(conn,'getresponse',side_effect=TimeoutError('fixture-private-marker')):
            with self.assertRaises(flow.Rejected) as error:
                self.host().request('https://127.0.0.1:55475/x')
        self.assertNotIn('fixture-private-marker', str(error.exception))
        self.assertTrue(conn.closed)
    def test_deadline_and_request_budget_stop_before_io(self):
        for expired in [True, False]:
            host = self.host()
            if expired:
                host.deadline = time.monotonic()-1
            else:
                host.remaining = 0
            with patch.object(flow.http.client,'HTTPSConnection') as connection:
                with self.assertRaises(flow.Rejected):
                    host.request('https://127.0.0.1:55475/x')
                connection.assert_not_called()

if __name__ == '__main__':
    unittest.main()
