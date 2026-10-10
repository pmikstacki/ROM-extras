#!/usr/bin/env python3
"""Run both retained native vendors with separate private observer credentials and owned deadlines."""
from pathlib import Path
import json
import os
import subprocess
import sys
import tempfile
from lib.mysql_native_recovery import recover
from lib.fixture_processes import install_interrupt_handlers, wait_owned
install_interrupt_handlers()
binary = str(Path(sys.argv[1]).resolve())
root = Path('.superpowers/mysql-owner-runs')
root.mkdir(exist_ok=True)
run = Path(tempfile.mkdtemp(prefix='run-', dir=root)).resolve()
run.chmod(0o700)
for profile, port in [('mysql', '55452'), ('mariadb', '55453')]:
    container = f'rom-extras-{profile}-20261008'
    data = json.loads(subprocess.run(['docker', 'inspect', container], check=True, capture_output=True, timeout=5).stdout)[0]
    expected = {'mysql': ('c21bf1c3c8a853430a1973d3e53c9671f7e35a8798207b4cb65b3083206a6696','8d132912e9d3c985237567595ec2aeae04936044eb1f9336e8e4da2326238b84'), 'mariadb': ('607e2bb0d13bf7e8a0bc92c0f1465e1d240c6cabf9d2f7c79255573e87314e06','53ef799caed285438d88678b529b4ff24406d4788f47ab8a74bb2212c707899a')}
    assert (data['Id'], data['Image']) == expected[profile]
    assert data['State']['Running'] and data['Config']['Labels']['rom-extras.fixture'] == profile
    assert data['HostConfig']['PortBindings']['3306/tcp'] == [{'HostIp': '127.0.0.1', 'HostPort': port}]
    mount = [m for m in data['Mounts'] if m['Destination'] == '/var/lib/mysql']
    assert len(mount) == 1 and mount[0]['RW'] and mount[0]['Type'] == 'volume' and mount[0]['Name'] == container
    identity = {'container': data['Id'], 'image': data['Image'], 'mount': mount, 'profile': profile}
    (run / f'{profile}-identity.json').write_text(json.dumps(identity, indent=2)+'\n')
    private = Path(f'.superpowers/{profile}.env')
    assert private.stat().st_mode & 0o077 == 0
    entries = dict(line.split('=', 1) for line in private.read_text().splitlines() if '=' in line)
    env = os.environ.copy()
    env['ROM_EXTRAS_MYSQL_PROFILE'] = profile
    env['ROM_EXTRAS_MYSQL_MONITOR_PASSWORD'] = entries[profile.upper()+'_ROOT_PASSWORD']
    log = run / f'{profile}-native.log'
    with log.open('x') as output:
        log.chmod(0o600)
        code = wait_owned([binary], timeout=40, env=env, stdout=output, stderr=output)
    assert code == 0, f'{profile} maintained qualification failed; retained {log}'
    def identity():
        current = json.loads(subprocess.run(['docker', 'inspect', container], check=True, capture_output=True, timeout=5).stdout)[0]
        assert (current['Id'], current['Image']) == expected[profile]
        assert current['Name'].lstrip('/') == container
        assert current['HostConfig']['PortBindings']['3306/tcp'] == [{'HostIp': '127.0.0.1', 'HostPort': port}]
        current_mount = [m for m in current['Mounts'] if m['Destination'] == '/var/lib/mysql']
        assert current_mount == mount
        assert current['HostConfig']['RestartPolicy']['Name'] in ('no', '')
        return {'id': current['Id'], 'image': current['Image'], 'data_mount': current_mount, 'started_at': current['State']['StartedAt'], 'pid': current['State']['Pid'], 'status': current['State']['Status'], 'exit_code': current['State']['ExitCode'], 'oom_killed': current['State']['OOMKilled']}
    recover(binary, run / f'{profile}-noop', env, identity, container, noop=True)
    recover(binary, run / f'{profile}-acknowledged', env, identity, container)
    recover(binary, run / f'{profile}-incomplete', env, identity, container, incomplete=True)
    print(profile, 'maintained qualification, no-op negative and both native recovery profiles passed; retained', run)
