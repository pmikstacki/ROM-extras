"""Native version/mapping/auth probe; this is not a ROM storage/search adapter."""
import json
import pathlib
import ssl
import subprocess
import time
import urllib.error
import urllib.request
import uuid

from probe_transport import client

root = pathlib.Path('/root/ROM-extras/.superpowers/opensearch-fixture')
clients = {}
for name in ['projection-writer', 'projection-reader', 'admin']:
    context = ssl.create_default_context(cafile=str(root / 'tls/ca.pem'))
    context.load_cert_chain(root / f'tls/{name}.pem', root / f'client-private/{name}.key')
    clients[name] = client(context)


def call(method, path, value=None, actor='projection-writer'):
    body = value if isinstance(value, bytes) else (None if value is None else json.dumps(value, allow_nan=False).encode())
    content_type = 'application/json'
    if isinstance(value, bytes):
        body = value
        content_type = 'application/x-ndjson'
    request = urllib.request.Request('https://127.0.0.1:55460' + path, data=body, method=method,
                                     headers={'Content-Type': content_type})
    with clients[actor].open(request, timeout=5) as response:
        if isinstance(value, bytes):
            assert response.status == 200
        raw = response.read(1048577)
        assert len(raw) <= 1048576
        return json.loads(raw)


def rejected(method, path, value, actor, status):
    try:
        call(method, path, value, actor)
    except urllib.error.HTTPError as error:
        assert error.code == status
    else:
        raise AssertionError('expected native rejection')


def write(name, identity, version, live=True, title='fixture title'):
    body = {'identity': str(identity), 'revision': version, 'rom_live': live}
    if live:
        body['title'] = title
    return call('PUT', f'/{name}/_doc/{identity}?version={version}&version_type=external&refresh=wait_for', body)


def run():
    info = call('GET', '/')
    assert info['version']['number'] == '3.9.0'
    assert info['version']['build_hash'] == '4ee42a94e87f66fbf1e62a9871b1b87f91e02472'
    name = 'rom_extras_fence_' + uuid.uuid4().hex
    mapping = {'dynamic': False, 'properties': {'identity': {'type': 'keyword'}, 'revision': {'type': 'long'},
                                               'rom_live': {'type': 'boolean'}, 'title': {'type': 'text'}}}
    created = call('PUT', '/' + name, {'settings': {'number_of_shards': 1, 'number_of_replicas': 0,
                                                   'index.translog.durability': 'request'}, 'mappings': mapping})
    assert created['acknowledged'] is True
    write(name, 1, 3)
    rejected('PUT', f'/{name}/_doc/1?version=2&version_type=external',
             {'identity': '1', 'revision': 2, 'rom_live': True, 'title': 'stale'}, 'projection-writer', 409)
    rejected('PUT', f'/{name}/_doc/1?version=3&version_type=external',
             {'identity': '1', 'revision': 3, 'rom_live': True, 'title': 'conflict'}, 'projection-writer', 409)
    stored = call('GET', f'/{name}/_doc/1', actor='projection-reader')
    assert stored['_version'] == 3 and stored['_source']['title'] == 'fixture title'
    write(name, 2, 4, live=False)
    rejected('PUT', f'/{name}/_doc/2?version=3&version_type=external',
             {'identity': '2', 'revision': 3, 'rom_live': True, 'title': 'late'}, 'projection-writer', 409)
    tombstone = call('GET', f'/{name}/_doc/2', actor='projection-reader')
    assert tombstone['_version'] == 4 and tombstone['_source']['rom_live'] is False
    assert 'title' not in tombstone['_source']
    write(name, 3, 2**63-1)
    rejected('PUT', f'/{name}/_doc/3?version={2**63}&version_type=external',
             {'identity': '3', 'rom_live': True}, 'projection-writer', 400)
    ndjson = b'\n'.join(json.dumps(row).encode() for row in [
        {'index': {'_id': '4', 'version': 1, 'version_type': 'external'}},
        {'identity': '4', 'revision': 1, 'rom_live': True, 'title': 'bulk'},
        {'index': {'_id': '5', 'version': 1, 'version_type': 'external'}},
        {'identity': '5', 'revision': 'invalid-number', 'rom_live': True, 'title': 'rejected'}]) + b'\n'
    bulk = call('POST', f'/{name}/_bulk?refresh=wait_for', ndjson)
    assert bulk['errors'] is True
    assert [item['index']['status'] for item in bulk['items']] == [201, 400]
    assert call('GET', f'/{name}/_doc/4', actor='projection-reader')['_source']['revision'] == 1
    rejected('GET', f'/{name}/_doc/5', None, 'projection-reader', 404)
    rejected('PUT', f'/{name}/_doc/reader-write', {'rom_live': True}, 'projection-reader', 403)
    rejected('PUT', '/foreign_' + uuid.uuid4().hex, {}, 'projection-writer', 403)
    hits = call('POST', f'/{name}/_search', {'size': 8, '_source': ['identity', 'revision'],
                                         'query': {'bool': {'filter': [{'term': {'rom_live': True}}]}}},
                actor='projection-reader')['hits']['hits']
    assert {hit['_id'] for hit in hits} == {'1', '3', '4'}
    print('PASS verified mTLS writer/reader scopes; external version stale/equal fences; signed revision limit; persistent tombstone; HTTP200 Bulk partial failure inspected; live query excludes tombstone')
    subprocess.run(['timeout', '--kill-after=2s', '30s', 'docker', 'restart', '--time', '10',
                    'rom-extras-opensearch-20261008'], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    deadline = time.monotonic() + 35
    while True:
        try:
            assert call('GET', '/')['version']['number'] == '3.9.0'
            break
        except (urllib.error.URLError, ConnectionError):
            if time.monotonic() >= deadline:
                raise
            time.sleep(0.1)
    for identity, expected in [(1, 3), (2, 4), (3, 2**63-1), (4, 1)]:
        assert call('GET', f'/{name}/_doc/{identity}', actor='projection-reader')['_version'] == expected
    persisted = call('GET', f'/{name}/_doc/2', actor='projection-reader')['_source']
    assert persisted['rom_live'] is False and 'title' not in persisted
    rejected('PUT', f'/{name}/_doc/2?version=3&version_type=external', {'rom_live': True}, 'projection-writer', 409)
    (root / 'probe-index.json').write_text(json.dumps(name))
    print('PASS persistent service restart: documents, i64::MAX and tombstone revision fence retained')


if __name__ == '__main__':
    run()
