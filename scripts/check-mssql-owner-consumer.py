#!/usr/bin/env python3
"""Bound owned native profiles; restart only the exact retained SQL Server fixture."""
from pathlib import Path
import json
import subprocess
import sys
import tempfile
import time
from lib.fixture_processes import install_interrupt_handlers, wait_owned, close_owned_group

install_interrupt_handlers()
binary = str(Path(sys.argv[1]).resolve())
root = Path('.superpowers/mssql-owner-runs')
root.mkdir(exist_ok=True)
run = Path(tempfile.mkdtemp(prefix='run-', dir=root)).resolve()
log = run / 'native.log'
with log.open('x') as output:
    log.chmod(0o600)
    code = wait_owned([binary], timeout=40, stdout=output, stderr=output)
assert code == 0, f'SQL Server native qualification failed; retained log {log}'

# This named Developer fixture is already authorized and retained. Never accept an arbitrary host.
container = 'rom-extras-mssql-20261008'
def identity(target=container):
    result = subprocess.run(['docker', 'inspect', target], check=True, capture_output=True, timeout=5)
    data = json.loads(result.stdout)[0]
    mounts = [(m['Type'], m.get('Name'), m['Source'], m['Destination']) for m in data['Mounts'] if m['Destination'] == '/var/opt/mssql']
    assert len(mounts) == 1 and mounts[0][0:2] == ('volume', 'rom-extras-mssql-20261008')
    binding = data['HostConfig']['PortBindings']['1433/tcp']
    assert binding == [{'HostIp': '127.0.0.1', 'HostPort': '55440'}]
    return {'id': data['Id'], 'image': data['Image'], 'data_mount': mounts[0], 'started_at': data['State']['StartedAt'], 'status': data['State']['Status'], 'exit_code': data['State']['ExitCode'], 'oom_killed': data['State']['OOMKilled']}

before = identity()
assert before['status'] == 'running'
restart_log = run / 'restart.log'
with restart_log.open('x') as output:
    restart_log.chmod(0o600)
    child = subprocess.Popen([binary, '--restart', str(run)], start_new_session=True, stdout=output, stderr=output)
    try:
        deadline = time.monotonic() + 15
        while not (run / 'prepared').exists():
            assert child.poll() is None, f'Restart consumer ended before preparation; retained {restart_log}'
            assert time.monotonic() < deadline, f'Restart preparation deadline; retained {restart_log}'
            time.sleep(.02)
        assert (run / 'prepared').read_text() == 'ready\n'
        assert child.poll() is None
        subprocess.run(['docker', 'kill', '--signal', 'KILL', before['id']], check=True, timeout=5, stdout=subprocess.DEVNULL)
        killed = identity(before['id'])
        assert killed['status'] == 'exited' and killed['exit_code'] == 137 and not killed['oom_killed']
        subprocess.run(['docker', 'start', before['id']], check=True, timeout=15, stdout=subprocess.DEVNULL)
        after = identity(before['id'])
        assert {k: before[k] for k in ['id', 'image', 'data_mount']} == {k: after[k] for k in ['id', 'image', 'data_mount']}
        assert before['started_at'] != after['started_at']
        evidence = run / 'restart-identity.json'
        evidence.write_text(json.dumps({'before': before, 'killed': killed, 'after': after, 'profile': 'explicit SIGKILL after acknowledged owner transition and closed clients'}, indent=2) + '\n')
        evidence.chmod(0o600)
        signal = run / 'restarted.tmp'
        signal.write_text('resume\n')
        signal.chmod(0o600)
        signal.replace(run / 'restarted')
        code = child.wait(timeout=80)
        assert code == 0, f'Restart ownership qualification failed; retained {restart_log}'
    finally:
        close_owned_group(child)
        # Recover only our verified original instance if a failed/interrupting start left it stopped.
        state = identity(before['id'])
        if state['status'] == 'exited':
            assert state['id'] == before['id'] and state['data_mount'] == before['data_mount']
            subprocess.run(['docker', 'start', before['id']], check=True, timeout=15, stdout=subprocess.DEVNULL, stderr=output)
print('SQL Server ownership: eleven native groups and same-volume SIGKILL recovery passed; evidence retained:', run)
