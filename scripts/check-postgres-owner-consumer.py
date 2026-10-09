#!/usr/bin/env python3
"""Bound one owned maintained PostgreSQL qualification, with retained private evidence."""
from pathlib import Path
import subprocess
import sys
import tempfile
from lib.fixture_processes import install_interrupt_handlers, wait_owned
install_interrupt_handlers()
root = Path('.superpowers/postgres-owner-runs')
root.mkdir(exist_ok=True)
run = Path(tempfile.mkdtemp(prefix='run-', dir=root)).resolve()
run.chmod(0o700)
log = run / 'native.log'
with log.open('x') as output:
    log.chmod(0o600)
    code = wait_owned([str(Path(sys.argv[1]).resolve())], timeout=40, stdout=output, stderr=output)
assert code == 0, f'Maintained PostgreSQL native qualification failed; retained {log}'
print('Maintained PostgreSQL native ownership, response loss and admission passed; evidence retained:', run)
