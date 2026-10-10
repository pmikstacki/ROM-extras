"""Actual isolated Keycloak protocol qualification; all credentials remain private."""
import importlib.util
import json
import os
from pathlib import Path
import secrets
import subprocess
import sys
import time
from urllib.parse import quote

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('flow', ROOT/'examples/oidc-host/flow.py')
flow = importlib.util.module_from_spec(spec)
spec.loader.exec_module(flow)
BASE = 'https://127.0.0.1:55475'
ISSUER = BASE+'/realms/rom-fixture'
CALLBACK = 'https://127.0.0.1:55476/callback'


def private_json(path, value):
    with os.fdopen(os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600), 'w') as f:
        json.dump(value, f)


def qualify(fixture, consumer, run):
    credentials_path = fixture/'credentials.json'
    if credentials_path.stat().st_mode & 0o077:
        raise flow.Rejected()
    credentials = json.loads(credentials_path.read_text())
    ca = str(fixture/'tls/ca.crt')
    host = lambda: flow.Host(BASE, ca)
    original = flow.login(host(), ISSUER, CALLBACK, credentials)
    assert flow.login(host(), ISSUER, CALLBACK, credentials, wrong_pkce=True)['wrong_pkce_rejected']
    admin = host()
    # Password grant is confined to fixture administration. Human proof above uses code+PKCE.
    status, _, raw = admin.request(BASE+'/realms/master/protocol/openid-connect/token', {
        'grant_type':'password','client_id':'admin-cli','username':credentials['admin_username'],'password':credentials['admin_password']})
    if status != 200:
        raise flow.Rejected()
    bearer = json.loads(raw)['access_token']
    status, _, raw = admin.request(BASE+'/admin/realms/rom-fixture', bearer=bearer)
    if status != 200:
        raise flow.Rejected()
    realm = json.loads(raw)
    components_url = BASE+'/admin/realms/rom-fixture/components'
    status, _, raw = admin.request(components_url+'?type=org.keycloak.keys.KeyProvider', bearer=bearer)
    if status != 200:
        raise flow.Rejected()
    components = json.loads(raw)
    signature = [c for c in components if c['providerId']=='rsa-generated' and c.get('config',{}).get('algorithm',['RS256'])==['RS256'] and c.get('config',{}).get('enabled',['true'])==['true']]
    priority = max(int(c['config'].get('priority',['0'])[0]) for c in signature)+1
    document = {'name':'rom-native-'+secrets.token_hex(8),'providerId':'rsa-generated','providerType':'org.keycloak.keys.KeyProvider','parentId':realm['id'],'config':{'priority':[str(priority)],'enabled':['true'],'active':['true'],'algorithm':['RS256'],'keySize':['2048']}}
    status, location, _ = admin.request(components_url, bearer=bearer, document=document)
    if status != 201 or not location:
        raise flow.Rejected()
    admin.approve(location)
    rotated = flow.login(host(), ISSUER, CALLBACK, credentials)
    # Disable the former keys without deleting their configuration or private data.
    # The preserved native database retains all historical key components.
    for component in signature:
        component['config']['enabled']=['false']
        component['config']['active']=['false']
        status, _, _ = admin.request(components_url+'/'+quote(component['id'],safe=''), bearer=bearer, document=component, method='PUT')
        if status not in (200,204):
            raise flow.Rejected()
    _, current_jwks = host().document(ISSUER+'/protocol/openid-connect/certs')
    container = 'rom-extras-keycloak-oidc-20261010'
    expected_image = 'quay.io/keycloak/keycloak@sha256:d79bc4bf1c54e802735ef91926b5c003de1fbb50b1a93382611972277219c9ad'
    image = subprocess.run(['docker','inspect','--format','{{.Config.Image}}',container],capture_output=True,text=True,check=True,timeout=10).stdout.strip()
    if image != expected_image:
        raise flow.Rejected()
    with (run/'restart.log').open('x') as output:
        subprocess.run(['docker','restart','--time','20',container],stdout=output,stderr=output,check=True,timeout=45)
    ready = False
    deadline = time.monotonic()+45
    for _ in range(30):
        if time.monotonic() >= deadline:
            break
        try:
            host().document(ISSUER+'/.well-known/openid-configuration')
            ready = True
            break
        except flow.Rejected:
            time.sleep(1)
    if not ready:
        raise flow.Rejected()
    rotated = flow.login(host(), ISSUER, CALLBACK, credentials)
    before = sorted(json.loads(current_jwks)['keys'],key=lambda key:key['kid'])
    after = sorted(json.loads(rotated['jwks'])['keys'],key=lambda key:key['kid'])
    if before != after:
        raise flow.Rejected()
    rotated['revoked'] = original
    rotated['previous_jwks'] = original['jwks']
    private_json(run/'original.json', original)
    private_json(run/'rotated.json', rotated)
    native = run/'stores'
    native.mkdir(mode=0o700)
    with (run/'consumer.log').open('x') as output:
        result = subprocess.run([str(consumer),str(run/'rotated.json'),str(native)], stdout=output, stderr=output, timeout=60)
    if result.returncode:
        raise flow.Rejected()
    private_json(run/'result.json', {'actual_service':'Keycloak 26.8.0','transport':'trusted TLS loopback','human_grant':'authorization_code','pkce':'S256','code_replay_rejected':True,'wrong_pkce_rejected':True,'native_rotation':True,'native_restart_with_persistent_keys':True,'former_signature_keys_disabled_without_deletion':True,'consumer_public_api':True,'credentials_in_public_output':False,'completed_unix':int(time.time())})
    print('Actual Keycloak code/PKCE, native rotation and independent SQLite/redb consumer passed.')


if __name__ == '__main__':
    os.umask(0o077)
    try:
        qualify(Path(sys.argv[1]).resolve(),Path(sys.argv[2]).resolve(),Path(sys.argv[3]).resolve())
    except Exception:
        # Never print raw HTTP responses, forms, code, cookies or bearer errors.
        print('Native OIDC qualification failed; inspect private retained evidence.',file=sys.stderr)
        sys.exit(1)
