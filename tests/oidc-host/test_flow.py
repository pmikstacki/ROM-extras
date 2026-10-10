import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('flow', Path(__file__).parents[2] / 'examples/oidc-host/flow.py')
flow = importlib.util.module_from_spec(spec)
spec.loader.exec_module(flow)

class HostFlowTests(unittest.TestCase):
    def test_callback_is_exact_and_one_use(self):
        state = flow.LoginState('retained', 'nonce', 'verifier', 100)
        callback = 'https://127.0.0.1:55476/callback'
        for bad in [callback+'?state=wrong&code=a', callback+'?state=retained&state=retained&code=a', callback+'?state=retained&code=a&code=b', 'https://other.invalid/callback?state=retained&code=a']:
            with self.assertRaises(flow.Rejected):
                state.consume(bad, callback, 99)
        self.assertEqual(state.consume(callback+'?state=retained&code=approved', callback, 99), 'approved')
        with self.assertRaises(flow.Rejected):
            state.consume(callback+'?state=retained&code=approved', callback, 99)

    def test_expiry_and_error_are_rejected(self):
        callback = 'https://127.0.0.1:55476/callback'
        for suffix, now in [('?state=s&code=a', 100), ('?state=s&error=denied', 99), ('?state=s&code=', 99)]:
            with self.assertRaises(flow.Rejected):
                flow.LoginState('s', 'n', 'v', 100).consume(callback+suffix, callback, now)

    def test_controlled_host_does_not_resolve_dns_or_external_addresses(self):
        for base in ['https://issuer.example', 'https://192.0.2.1:443']:
            with self.assertRaises(flow.Rejected):
                flow.Host(base, '/unused', initialize=False)

    def test_urls_cannot_change_origin_or_resolve_userinfo(self):
        host = flow.Host('https://127.0.0.1:55475', '/unused', initialize=False)
        for bad in ['http://127.0.0.1:55475/x', 'https://other.invalid/x', 'https://user@127.0.0.1:55475/x', 'https://127.0.0.1:55475/x#fragment', 'https://127.0.0.1:55475/x\\evil']:
            with self.assertRaises(flow.Rejected):
                host.approve(bad)
        self.assertEqual(host.approve('https://127.0.0.1:55475/realms/rom-fixture'), '/realms/rom-fixture')

if __name__ == '__main__':
    unittest.main()
