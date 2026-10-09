#!/usr/bin/env python3
"""Owned Nginx qualification and actual storage process exit/recovery; retain all evidence."""
import http.client
import json
import os
from pathlib import Path
import ssl
import subprocess
import sys
import tempfile
import time
from urllib.parse import urlsplit
from lib.fixture_processes import wait_owned, install_interrupt_handlers, defer_interrupts

install_interrupt_handlers()
fixture = Path(os.environ['ROM_EXTRAS_IMPORT_NATIVE_FIXTURE']).resolve()
container = os.environ['ROM_EXTRAS_IMPORT_NATIVE_CONTAINER']
binary = str(Path(sys.argv[1]).resolve())
manifest = json.loads((fixture / 'fixture.json').read_text())
assert container == manifest['container']
assert (fixture / 'credentials.json').stat().st_mode & 0o777 == 0o600
credentials = fixture / 'credentials.json'
token = json.loads(credentials.read_text())['token']
ca = fixture / 'ca.pem'
root = Path('.superpowers/import-native-runs')
root.mkdir(exist_ok=True)
run = Path(tempfile.mkdtemp(prefix='run-', dir=root)).resolve()
record = {'fixture': str(fixture), 'cases': [], 'restored': False}
stopped = False

def docker(*args):
    return subprocess.check_output(['docker', *args], text=True, stderr=subprocess.STDOUT, timeout=15).strip()

def identity():
    actual = docker('inspect', '--format', '{{.Id}} {{.Image}} {{.State.Status}}', container).split()
    assert actual[0] == manifest['container_id']
    assert actual[1].removeprefix('sha256:') == manifest['image_id'].removeprefix('sha256:')
    assert actual[2] == 'running'
    assert docker('exec', container, 'nginx', '-v') == manifest['nginx_version']

def head():
    parsed = urlsplit(manifest['endpoint'])
    assert parsed.scheme == 'https' and parsed.query == '' and parsed.username is None
    connection = http.client.HTTPSConnection(parsed.hostname, parsed.port, timeout=2, context=ssl.create_default_context(cafile=str(ca)))
    try:
        connection.request('HEAD', parsed.path, headers={'Authorization': 'Bearer ' + token})
        response = connection.getresponse()
        assert response.status == 200
        etag = response.getheader('ETag')
        assert etag and len(etag) <= 256
        return etag
    finally:
        connection.close()

def ready():
    deadline = time.monotonic() + 15
    while True:
        try:
            return head()
        except (OSError, http.client.HTTPException):
            if time.monotonic() >= deadline:
                raise
            time.sleep(.05)

def cli(name, args, expected=0):
    log = run / (name + '.log')
    with log.open('x') as output:
        log.chmod(0o600)
        result = wait_owned([binary, *args], timeout=30, stdout=output, stderr=subprocess.STDOUT)
    assert result == expected, f'Host consumer failed: {name}; retained log {log}'
    record['cases'].append({'name': name, 'exit_code': result, 'log': str(log)})

try:
    identity()
    etag = ready()
    assert (fixture / 'data/input.json').read_bytes() == b'21'
    common = [manifest['endpoint'], str(ca), str(credentials), etag]
    cli('native-read', ['--native-read', *common])
    cli('prepare-original', ['--native-prepare', *common, str(run)])
    for backend in ['sqlite', 'redb']:
        cli(backend + '-before-commit', ['--native-attempt', str(run), backend, 'before'], 70)
        cli(backend + '-after-commit-unknown', ['--native-attempt', str(run), backend, 'after'], 71)
    with defer_interrupts():
        stopped = True
        docker('stop', '--time', '5', container)
    for backend in ['sqlite', 'redb']:
        cli(backend + '-resume-service-stopped', ['--native-resume', str(run), backend])
    with defer_interrupts():
        docker('start', container)
        identity()
        assert ready() == etag
        stopped = False
        record['restored'] = True
    cli('native-read-after-restart', ['--native-read', *common])
    print('Native Nginx and SQLite/redb committed Unknown/process recovery: passed; source stayed stopped during exact retry')
finally:
    with defer_interrupts():
        if stopped:
            docker('start', container)
            identity()
            ready()
            record['restored'] = True
        (run / 'verification.json').write_text(json.dumps(record, indent=2) + '\n')
print('Evidence retained:', run)
