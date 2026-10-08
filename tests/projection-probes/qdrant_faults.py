"""Actual TLS fault qualification for the isolated Qdrant fixture, not a ROM adapter."""
import concurrent.futures
import http.client
import http.server
import json
import socket
import ssl
import subprocess
import threading
import time
import urllib.error
import urllib.request
import uuid

from qdrant_fixture import call, condition, context, keys, opener, point, revision, root, upsert


class FaultProxy:
    def __init__(self, name, mode):
        self.path = f'/collections/{name}/points?wait=true&ordering=strong'
        self.mode = mode
        self.arrived = threading.Event()
        self.release = threading.Event()
        self.completed = threading.Event()
        self.forwarded = 0
        self.dropped = 0
        self.error = False
        proxy = self

        class Handler(http.server.BaseHTTPRequestHandler):
            def do_PUT(self):
                try:
                    self.connection.settimeout(5)
                    if self.path != proxy.path or self.headers.get('api-key') != keys['write']:
                        self.send_error(403)
                        return
                    size = int(self.headers.get('Content-Length', '0'))
                    if not 0 < size <= 1048576:
                        self.send_error(413)
                        return
                    body = self.rfile.read(size)
                    if len(body) != size:
                        raise ValueError('incomplete request')
                    value = json.loads(body)
                    proxy.arrived.set()
                    if proxy.mode == 'hold' and not proxy.release.wait(5):
                        raise TimeoutError('fixture release deadline')
                    result = call('PUT', proxy.path, value)
                    assert result['result']['status'] == 'completed'
                    proxy.forwarded += 1
                    proxy.completed.set()
                    if proxy.mode == 'drop':
                        proxy.dropped += 1
                        self.close_connection = True
                        self.connection.shutdown(socket.SHUT_RDWR)
                        self.connection.close()
                        return
                    encoded = json.dumps(result).encode()
                    self.send_response(200)
                    self.send_header('Content-Type', 'application/json')
                    self.send_header('Content-Length', str(len(encoded)))
                    self.end_headers()
                    self.wfile.write(encoded)
                except (BrokenPipeError, ConnectionResetError, ssl.SSLError):
                    # The held client deliberately times out before forwarding.
                    self.close_connection = True
                except Exception:
                    proxy.error = True
                    self.close_connection = True

            def log_message(self, *args):
                pass

        self.server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Handler)
        tls = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        tls.minimum_version = ssl.TLSVersion.TLSv1_2
        tls.load_cert_chain(root / 'tls/server.pem', root / 'tls/server.key')
        self.server.socket = tls.wrap_socket(self.server.socket, server_side=True)
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()

    def submit(self, identity, incoming, live=True, timeout=3):
        payload = {'identity': str(identity), 'rom_revision_hi': incoming >> 32,
                   'rom_revision_lo': incoming & 0xffffffff, 'rom_live': live}
        if live:
            payload['content'] = 'fault-fixture'
        body = {'points': [{'id': identity, 'vector': {'embedding': [0.0, 1.0, 0.0]} if live else {},
                            'payload': payload}], 'update_filter': condition(incoming), 'update_mode': 'upsert'}
        request = urllib.request.Request(f'https://127.0.0.1:{self.server.server_port}' + self.path,
                                         data=json.dumps(body).encode(), method='PUT',
                                         headers={'Content-Type': 'application/json', 'api-key': keys['write']})
        try:
            with opener.open(request, timeout=timeout) as response:
                response.read(1048577)
        except (TimeoutError, urllib.error.URLError, http.client.RemoteDisconnected):
            return 'unknown'
        return 'acknowledged'

    def close(self):
        self.release.set()
        self.server.shutdown()
        self.server.server_close()
        self.thread.join(timeout=3)
        assert not self.thread.is_alive() and not self.error


def assert_tombstone(name, identity, expected):
    stored = point(name, identity)
    assert revision(stored) == expected
    assert stored['payload']['rom_live'] is False
    assert 'content' not in stored['payload'] and stored['vector'] == {}


def run():
    info = call('GET', '/', credential='read')
    assert info['version'] == '1.19.2'
    assert info['commit'] == '016542aa5deb6c66380bb137badf73d54f742bde'
    names = []
    for indexed in [False, True]:
        name = 'rom_extras_fault_' + uuid.uuid4().hex
        names.append(name)
        call('PUT', f'/collections/{name}', {'vectors': {'embedding': {'size': 3, 'distance': 'Cosine'}}})
        if indexed:
            for field in ['rom_revision_hi', 'rom_revision_lo']:
                call('PUT', f'/collections/{name}/index?wait=true', {'field_name': field, 'field_schema': 'integer'})
        for identity, tombstone in [(1, False), (2, True)]:
            proxy = FaultProxy(name, 'hold')
            try:
                with concurrent.futures.ThreadPoolExecutor(max_workers=1) as pool:
                    old = pool.submit(proxy.submit, identity, 2, True, 0.2)
                    assert proxy.arrived.wait(3)
                    assert old.result(timeout=3) == 'unknown'
                    assert proxy.forwarded == 0
                    upsert(name, identity, 3, live=not tombstone)
                    proxy.release.set()
                    assert proxy.completed.wait(3)
                    assert proxy.forwarded == 1
                if tombstone:
                    assert_tombstone(name, identity, 3)
                else:
                    stored = point(name, identity)
                    assert revision(stored) == 3 and stored['payload']['content'] == 'fixture'
                    assert stored['vector'] == {'embedding': [1.0, 0.0, 0.0]}
            finally:
                proxy.close()
        proxy = FaultProxy(name, 'drop')
        try:
            assert proxy.submit(3, 2) == 'unknown'
            assert proxy.completed.wait(3)
            assert proxy.forwarded == 1 and proxy.dropped == 1
            accepted = point(name, 3)
            assert revision(accepted) == 2
            assert accepted['vector'] == {'embedding': [0.0, 1.0, 0.0]}
            upsert(name, 3, 3)
            upsert(name, 3, 2, content='replay')
            assert revision(point(name, 3)) == 3
        finally:
            proxy.close()
        upsert(name, 4, 2**64-1, live=False)
        upsert(name, 4, 2**64-2)
        assert_tombstone(name, 4, 2**64-1)
        print(f'PASS indexed={indexed}: delayed request after client timeout cannot replace newer live/tombstone; actual response drop reconciled; stale replay rejected')

    subprocess.run(['timeout', '--kill-after=2s', '30s', 'docker', 'restart', '--time', '10',
                    'rom-extras-qdrant-20261008'], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    deadline = time.monotonic() + 15
    while True:
        try:
            assert call('GET', '/', credential='read')['version'] == '1.19.2'
            break
        except (urllib.error.URLError, ConnectionError):
            if time.monotonic() >= deadline:
                raise
            time.sleep(0.1)
    for name in names:
        assert_tombstone(name, 2, 3)
        assert_tombstone(name, 4, 2**64-1)
        assert revision(point(name, 3)) == 3
        upsert(name, 4, 2**64-2)
        assert_tombstone(name, 4, 2**64-1)
    (root / 'fault-collections.json').write_text(json.dumps(names))
    print('PASS restart: indexed/unindexed tombstones including u64::MAX remain fenced; lost-response/newer state persists')


if __name__ == '__main__':
    run()
