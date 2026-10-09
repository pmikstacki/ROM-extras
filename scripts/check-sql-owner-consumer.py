#!/usr/bin/env python3
"""Bound only the qualification processes we start; preserve logs and native tables."""
from pathlib import Path
import sys
import tempfile
from lib.fixture_processes import install_interrupt_handlers, wait_owned

install_interrupt_handlers()
binary = str(Path(sys.argv[1]).resolve())
root = Path('.superpowers/sql-owner-runs')
root.mkdir(exist_ok=True)
run = Path(tempfile.mkdtemp(prefix='run-', dir=root)).resolve()
for name, args, budget in [('native', [], 30), ('setup-failure', ['--race-setup-failure'], 5)]:
    log = run / (name + '.log')
    with log.open('x') as output:
        log.chmod(0o600)
        code = wait_owned([binary, *args], timeout=budget, stdout=output, stderr=output)
    assert code == 0, f'Ownership qualification failed: {name}; retained log {log}'
print('Native ownership: eight case groups and bounded failed setup passed; evidence retained:', run)
