#!/usr/bin/env python3
"""Run only owned native qualification processes with a whole-process deadline."""
from pathlib import Path
import sys
import tempfile
from lib.fixture_processes import install_interrupt_handlers, wait_owned

install_interrupt_handlers()
binary = str(Path(sys.argv[1]).resolve())
root = Path('.superpowers/mssql-owner-runs')
root.mkdir(exist_ok=True)
run = Path(tempfile.mkdtemp(prefix='run-', dir=root)).resolve()
log = run / 'native.log'
with log.open('x') as output:
    log.chmod(0o600)
    code = wait_owned([binary], timeout=40, stdout=output, stderr=output)
assert code == 0, f'SQL Server ownership failed; retained log {log}'
print('SQL Server ownership: seven native groups passed; evidence retained:', run)
