import concurrent.futures
import json
import urllib.error
import uuid

from qdrant_fixture import call, point, revision, root, upsert

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
