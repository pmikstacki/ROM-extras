#!/usr/bin/env python3
"""Bound native Oracle qualification and restart only the exact retained authorized fixture."""
from pathlib import Path
import json
import subprocess
import sys
import tempfile
import time
from lib.fixture_processes import install_interrupt_handlers, wait_owned, close_owned_group

install_interrupt_handlers()
binary = str(Path(sys.argv[1]).resolve())
root = Path('.superpowers/oracle-owner-runs')
root.mkdir(exist_ok=True)
run = Path(tempfile.mkdtemp(prefix='run-', dir=root)).resolve()
run.chmod(0o700)
log = run / 'native.log'
with log.open('x') as output:
    log.chmod(0o600)
    code = wait_owned([binary], timeout=90, stdout=output, stderr=output)
assert code == 0, f'Maintained Oracle native qualification failed; retained {log}'
container = 'rom-extras-oracle-20261008'
container_id = 'ef3b4ccfe9485930aa300059b74af11e345990c11fb1e357b66e3f954ebc6f9e'
image_id = 'cdf2f86bedfa41904dfd7dbf27defe90d46a2fb8b34d85ad1c279f2bda839420'
volume = 'rom-extras-oracle-20261008'

def identity():
    data = json.loads(subprocess.run(['docker', 'inspect', container_id], check=True, capture_output=True, timeout=5).stdout)[0]
    assert data['Id'] == container_id and data['Image'] == image_id
    assert data['Name'].lstrip('/') == container and data['Config']['Labels']['rom-extras.fixture'] == 'oracle'
    mounts = [(m['Type'], m.get('Name'), m['Source'], m['Destination'], m['RW']) for m in data['Mounts'] if m['Destination'] == '/opt/oracle/oradata']
    assert len(mounts) == 1 and mounts[0][0:2] == ('volume', volume) and mounts[0][4]
    assert data['HostConfig']['PortBindings']['1521/tcp'] == [{'HostIp': '127.0.0.1', 'HostPort': '55458'}]
    return {'id': data['Id'], 'image': data['Image'], 'data_mount': mounts[0], 'started_at': data['State']['StartedAt'], 'status': data['State']['Status'], 'exit_code': data['State']['ExitCode'], 'oom_killed': data['State']['OOMKilled']}

def native_identity():
    command = "$ORACLE_HOME/bin/sqlplus -s '/ as sysdba' <<'SQL'\nwhenever sqlerror exit failure\nset heading off feedback off pagesize 0 linesize 200\nSELECT TO_CHAR(d.dbid)||'|'||TO_CHAR(i.startup_time,'YYYYMMDDHH24MISS')||'|'||(SELECT open_mode FROM v$pdbs WHERE name='FREEPDB1') FROM v$database d CROSS JOIN v$instance i;\nexit\nSQL"
    result = subprocess.run(['docker', 'exec', container_id, '/bin/bash', '-c', command], capture_output=True, timeout=10)
    if result.returncode:
        return None
    lines = [line.strip() for line in result.stdout.decode().splitlines() if '|' in line]
    if len(lines) != 1:
        return None
    fields = lines[0].split('|')
    if len(fields) != 3 or not fields[0].isdigit() or len(fields[1]) != 14 or not fields[1].isdigit() or fields[2] != 'READ WRITE':
        return None
    return {'database_id': fields[0], 'instance_started': fields[1], 'pdb_mode': fields[2]}

before = identity()
assert before['status'] == 'running' and not before['oom_killed']
native_before = native_identity()
assert native_before is not None
restart_log = run / 'restart.log'
with restart_log.open('x') as output:
    restart_log.chmod(0o600)
    child = subprocess.Popen([binary, '--restart', str(run)], start_new_session=True, stdout=output, stderr=output)
    try:
        deadline = time.monotonic() + 30
        while not (run / 'prepared').exists():
            assert child.poll() is None, f'Recovery preparation failed; retained {restart_log}'
            assert time.monotonic() < deadline, f'Recovery preparation deadline; retained {restart_log}'
            time.sleep(.05)
        assert (run / 'prepared').read_text() == 'ready\n' and child.poll() is None
        since = str(int(time.time()))
        subprocess.run(['timeout', '--kill-after=2s', '100s', 'docker', 'stop', '--time', '90', container_id], check=True, timeout=103, stdout=subprocess.DEVNULL, stderr=output)
        stopped = identity()
        assert stopped['status'] == 'exited' and stopped['exit_code'] == 143 and not stopped['oom_killed']
        logs = subprocess.run(['docker', 'logs', '--since', since, container_id], check=True, capture_output=True, timeout=10)
        text = logs.stdout + logs.stderr
        assert b'ORACLE instance shut down.' in text, 'Fresh native shutdown witness required'
        shutdown = run / 'shutdown.log'
        shutdown.write_bytes(text)
        shutdown.chmod(0o600)
        subprocess.run(['docker', 'start', container_id], check=True, timeout=30, stdout=subprocess.DEVNULL, stderr=output)
        deadline = time.monotonic() + 120
        while True:
            native_after = native_identity()
            if native_after is not None:
                break
            assert time.monotonic() < deadline, 'Native READ WRITE readiness deadline'
            time.sleep(.25)
        after = identity()
        assert {k: before[k] for k in ['id', 'image', 'data_mount']} == {k: after[k] for k in ['id', 'image', 'data_mount']}
        assert before['started_at'] != after['started_at'] and after['status'] == 'running'
        assert native_before['database_id'] == native_after['database_id']
        assert native_before['instance_started'] != native_after['instance_started']
        signal = run / 'restarted.tmp'
        signal.write_text('resume\n')
        signal.chmod(0o600)
        signal.replace(run / 'restarted')
        code = child.wait(timeout=90)
        assert code == 0 and (run / 'verified').read_text() == 'generation2 retained; generation1 stale; generation3 acknowledged; value42\n', f'Native Oracle recovery failed; retained {restart_log}'
        evidence = run / 'restart-identity.json'
        evidence.write_text(json.dumps({'before': before, 'stopped': stopped, 'after': after, 'native_before': native_before, 'native_after': native_after, 'profile': 'Graceful native shutdown after acknowledged generation2 and closed clients'}, indent=2) + '\n')
        evidence.chmod(0o600)
    finally:
        close_owned_group(child)
        state = identity()
        if state['status'] == 'exited':
            subprocess.run(['docker', 'start', container_id], check=True, timeout=30, stdout=subprocess.DEVNULL, stderr=output)
print('Maintained Oracle: native ownership groups and same-volume graceful recovery passed; evidence retained:', run)
