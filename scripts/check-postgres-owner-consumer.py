#!/usr/bin/env python3
"""Bound maintained qualification and crash only the exact retained PostgreSQL fixture."""
from pathlib import Path
import json
import subprocess
import sys
import tempfile
import time
from lib.fixture_processes import install_interrupt_handlers, wait_owned, close_owned_group

install_interrupt_handlers()
binary = str(Path(sys.argv[1]).resolve())
root = Path('.superpowers/postgres-owner-runs')
root.mkdir(exist_ok=True)
run = Path(tempfile.mkdtemp(prefix='run-', dir=root)).resolve()
run.chmod(0o700)
log = run / 'native.log'
with log.open('x') as output:
    log.chmod(0o600)
    code = wait_owned([binary], timeout=40, stdout=output, stderr=output)
assert code == 0, f'Maintained PostgreSQL native qualification failed; retained {log}'

# Exact already-authorized persistent fixture, not an arbitrary host or fresh database.
container = 'rom-extras-postgres-20261008'
container_id = '4b8e4f950e2f312868eccffc4918225b4a0bf082172b713c3cd33c81b2d14843'
image_id = 'cc72d79b4ea60df4943193bbeac2a0c84f4b6bd8c536680f6d4b14a94b272d41'
volume = '9ab5935cb153721c476cc2c306a7150d96115c08c31f5e0ae5487ac253734f31'

def identity():
    result = subprocess.run(['docker', 'inspect', container_id], check=True, capture_output=True, timeout=5)
    data = json.loads(result.stdout)[0]
    assert data['Id'] == container_id and data['Image'] == image_id
    assert data['Name'].lstrip('/') == container
    mounts = [(m['Type'], m.get('Name'), m['Source'], m['Destination'], m['RW']) for m in data['Mounts'] if m['Destination'] == '/var/lib/postgresql']
    assert len(mounts) == 1 and mounts[0][0:2] == ('volume', volume) and mounts[0][4]
    assert data['HostConfig']['PortBindings']['5432/tcp'] == [{'HostIp': '127.0.0.1', 'HostPort': '55439'}]
    return {'id': data['Id'], 'image': data['Image'], 'data_mount': mounts[0], 'started_at': data['State']['StartedAt'], 'status': data['State']['Status'], 'exit_code': data['State']['ExitCode'], 'oom_killed': data['State']['OOMKilled']}

before = identity()
assert before['status'] == 'running'
exe = subprocess.run(['docker', 'exec', '--user', 'postgres', container_id, 'readlink', '/proc/1/exe'], check=True, capture_output=True, timeout=5)
assert exe.stdout.strip() == b'/usr/local/bin/postgres'
restart_log = run / 'restart.log'
with restart_log.open('x') as output:
    restart_log.chmod(0o600)
    child = subprocess.Popen([binary, '--restart', str(run)], start_new_session=True, stdout=output, stderr=output)
    try:
        deadline = time.monotonic() + 15
        while not (run / 'prepared').exists():
            assert child.poll() is None, f'Recovery ended before acknowledged preparation; retained {restart_log}'
            assert time.monotonic() < deadline, f'Recovery preparation deadline; retained {restart_log}'
            time.sleep(.02)
        assert (run / 'prepared').read_text() == 'ready\n'
        assert child.poll() is None
        subprocess.run(['docker', 'kill', '--signal', 'KILL', container_id], check=True, timeout=5, stdout=subprocess.DEVNULL)
        killed = identity()
        assert killed['status'] == 'exited' and killed['exit_code'] == 137 and not killed['oom_killed']
        subprocess.run(['docker', 'start', container_id], check=True, timeout=15, stdout=subprocess.DEVNULL)
        after = identity()
        assert {k: before[k] for k in ['id', 'image', 'data_mount']} == {k: after[k] for k in ['id', 'image', 'data_mount']}
        assert after['status'] == 'running' and before['started_at'] != after['started_at']
        signal = run / 'restarted.tmp'
        signal.write_text('resume\n')
        signal.chmod(0o600)
        signal.replace(run / 'restarted')
        code = child.wait(timeout=75)
        assert code == 0, f'Native PostgreSQL recovery failed; retained {restart_log}'
        # Selected private log window; retain finite observations, never native statements/credentials.
        logs = subprocess.run(['docker', 'logs', '--since', after['started_at'], '--tail', '120', container_id], check=True, capture_output=True, timeout=5)
        window = logs.stdout + logs.stderr
        assert len(window) <= 262144, 'native startup log window exceeded bound'
        messages = {name: needle in window for name, needle in [('interrupted_shutdown', b'database system was not properly shut down; automatic recovery in progress'), ('wal_redo_started', b'redo starts at'), ('wal_redo_finished', b'redo done at'), ('native_ready', b'database system is ready to accept connections')]}
        assert all(messages.values()), f'Native crash-start observations missing; retained {restart_log}'
        evidence = run / 'restart-identity.json'
        evidence.write_text(json.dumps({'before': before, 'killed': killed, 'after': after, 'profile': 'SIGKILL after acknowledged generation2 and closed clients', 'startup_observations': messages, 'startup_window_bytes': len(window)}, indent=2) + '\n')
        evidence.chmod(0o600)
    finally:
        close_owned_group(child)
        state = identity()
        if state['status'] == 'exited':
            assert state['id'] == before['id'] and state['data_mount'] == before['data_mount']
            subprocess.run(['docker', 'start', container_id], check=True, timeout=15, stdout=subprocess.DEVNULL, stderr=output)
print('Maintained PostgreSQL: eight native groups, admission and same-volume SIGKILL recovery passed; evidence retained:', run)
