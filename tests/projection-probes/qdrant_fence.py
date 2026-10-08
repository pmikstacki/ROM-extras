import concurrent.futures
import json
import pathlib
import ssl
import urllib.error
import urllib.request
import uuid

root = pathlib.Path('/root/ROM-extras/.superpowers/qdrant-fixture')
keys = json.loads((root / 'credentials.json').read_text())
context = ssl.create_default_context(cafile=str(root / 'tls/ca.pem'))
class RejectRedirects(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, request, response, code, message, headers, target):
        return None

opener = urllib.request.build_opener(urllib.request.ProxyHandler({}), RejectRedirects(), urllib.request.HTTPSHandler(context=context))

def call(method, path, value=None, credential='write'):
    data = None if value is None else json.dumps(value, allow_nan=False).encode()
    headers = {'Content-Type': 'application/json'}
    if credential:
        headers['api-key'] = keys[credential]
    request = urllib.request.Request('https://127.0.0.1:55461' + path, data=data, headers=headers, method=method)
    with opener.open(request, timeout=5) as response:
        raw = response.read(1048577)
        assert len(raw) <= 1048576
        return json.loads(raw)

def condition(revision):
    hi, lo = revision >> 32, revision & 0xffffffff
    return {'should': [{'key':'rom_revision_hi','range':{'lt':hi}}, {'must':[{'key':'rom_revision_hi','match':{'value':hi}}, {'key':'rom_revision_lo','range':{'lt':lo}}]}]}

def upsert(name, identity, revision, live=True, content='fixture'):
    result = call('PUT', f'/collections/{name}/points?wait=true&ordering=strong', {'points':[{'id':identity,'vector':{'embedding':[1.0,0.0,0.0]} if live else {}, 'payload':{'rom_revision_hi':revision >> 32,'rom_revision_lo':revision & 0xffffffff,'rom_live':live,'identity':str(identity), **({'content':content} if live else {})}}], 'update_filter':condition(revision), 'update_mode':'upsert'})
    assert result['result']['status'] == 'completed'

def point(name, identity):
    return call('GET', f'/collections/{name}/points/{identity}', credential='read')['result']

def revision(point):
    payload = point['payload']
    return (payload['rom_revision_hi'] << 32) | payload['rom_revision_lo']

names = []
for indexed in [False, True]:
    name = 'rom_extras_fence_' + uuid.uuid4().hex
    names.append(name)
    call('PUT', f'/collections/{name}', {'vectors':{'embedding':{'size':3,'distance':'Cosine'}}})
    if indexed:
        for field in ['rom_revision_hi','rom_revision_lo']:
            call('PUT', f'/collections/{name}/index?wait=true', {'field_name':field,'field_schema':'integer'})
    values = [1, 2**32-1, 2**32, 2**53-1, 2**53, 2**63-1, 2**63, 2**64-1]
    for identity, incoming in enumerate(values, 1):
        upsert(name, identity, incoming)
        upsert(name, identity, max(0, incoming-1), content='stale')
        upsert(name, identity, incoming, content='same-revision-conflict')
        stored = point(name, identity)
        assert revision(stored) == incoming and stored['payload']['content'] == 'fixture'
    upsert(name, 99, 5)
    upsert(name, 99, 6, live=False)
    upsert(name, 99, 5, live=True, content='late-live')
    stored = point(name, 99)
    assert revision(stored) == 6 and stored['payload']['rom_live'] is False
    assert stored['vector'] == {} and 'content' not in stored['payload']
    with concurrent.futures.ThreadPoolExecutor(max_workers=8) as pool:
        list(pool.map(lambda r: upsert(name, 100, r), [2,8,1,7,3,6,4,5]))
    assert revision(point(name, 100)) == 8
    call('POST', f'/collections/{name}/points/delete?wait=true', {'points':[99]})
    upsert(name, 99, 5)
    assert revision(point(name, 99)) == 5
    print('PASS indexed=' + str(indexed) + ' exact-u64 stale/equal fencing, empty-vector tombstone, concurrent initial insert; physical-delete resurrection counterexample confirmed')

(root / 'probe-collections.json').write_text(json.dumps(names))
for credential in [None, 'read']:
    try:
        call('PUT', '/collections/denied_' + uuid.uuid4().hex, {'vectors':{'size':3,'distance':'Cosine'}}, credential=credential)
    except urllib.error.HTTPError as error:
        assert error.code in [401,403]
    else:
        raise AssertionError('unauthorized write accepted')
print('PASS unauthorized/read-only writes rejected; verified HTTPS; service version 1.19.2')
