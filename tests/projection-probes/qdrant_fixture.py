import json
import pathlib
import ssl
import urllib.error
import urllib.request

from probe_transport import client

root = pathlib.Path('/root/ROM-extras/.superpowers/qdrant-fixture')
keys = json.loads((root / 'credentials.json').read_text())
context = ssl.create_default_context(cafile=str(root / 'tls/ca.pem'))
opener = client(context)

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

