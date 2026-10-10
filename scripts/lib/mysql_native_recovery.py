"""Owned isolated MySQL/MariaDB process-death qualification; no production lifecycle API."""
import json
import subprocess
import time
from .fixture_processes import close_owned_group


def recover(binary, directory, env, identity, container, incomplete=False, noop=False):
    directory.mkdir(mode=0o700)
    before = identity()
    assert before['status'] == 'running'
    exe = subprocess.run(['docker', 'exec', '--user', 'mysql', container, 'readlink', '/proc/1/exe'], check=True, capture_output=True, timeout=5)
    assert exe.stdout.strip().rsplit(b'/', 1)[-1] in (b'mysqld', b'mariadbd')
    log = directory / 'native.log'
    with log.open('x') as output:
        log.chmod(0o600)
        mode = '--restart-incomplete' if incomplete else '--restart'
        child = subprocess.Popen([binary, mode, str(directory)], env=env, start_new_session=True, stdout=output, stderr=output)
        try:
            end = time.monotonic() + 20
            while not (directory / 'prepared').exists():
                assert child.poll() is None, f'Preparation failed; retained {log}'
                assert time.monotonic() < end, f'Preparation deadline; retained {log}'
                time.sleep(.02)
            assert (directory / 'prepared').read_text() == 'ready\n'
            assert child.poll() is None
            killed = None
            if not noop:
                subprocess.run(['docker', 'kill', '--signal', 'KILL', container], check=True, timeout=5, stdout=subprocess.DEVNULL)
                killed = identity()
                assert killed['status'] == 'exited' and killed['exit_code'] == 137 and not killed['oom_killed']
                subprocess.run(['docker', 'start', container], check=True, timeout=15, stdout=subprocess.DEVNULL)
            after = identity()
            keys = ('id', 'image', 'data_mount')
            assert {k: before[k] for k in keys} == {k: after[k] for k in keys}
            assert after['status'] == 'running'
            if not noop:
                assert before['started_at'] != after['started_at']
            signal = directory / 'restarted.tmp'
            signal.write_text('resume\n'); signal.chmod(0o600); signal.replace(directory / 'restarted')
            code = child.wait(timeout=75)
            if noop:
                assert code == 101 and 'native uptime did not reset' in log.read_text(), f'No-op did not fail intended native witness; retained {log}'
            else:
                assert code == 0, f'Native recovery failed; retained {log}'
            evidence = directory / 'identity.json'
            evidence.write_text(json.dumps({'before': before, 'killed': killed, 'after': after, 'incomplete': incomplete, 'noop': noop, 'exit': code}, indent=2)+'\n')
            evidence.chmod(0o600)
        finally:
            close_owned_group(child)
            state = identity()
            if state['status'] == 'exited':
                assert state['id'] == before['id'] and state['data_mount'] == before['data_mount']
                subprocess.run(['docker', 'start', container], check=True, timeout=15, stdout=subprocess.DEVNULL, stderr=output)
