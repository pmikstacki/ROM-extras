"""Both required gates refuse inherited diagnostic-only mode before any backend operation."""
import os
import subprocess
for gate in ['scripts/check-qdrant-target','scripts/check-all']:
 result=subprocess.run(['bash',gate],env={**os.environ,'ROM_EXTRAS_QDRANT_TARGET_REPLAY_LOSS_PROBE':'1'},capture_output=True,text=True,timeout=3)
 assert result.returncode==2, f'{gate} accepted diagnostic-only mode: {result.returncode}'
 assert 'Required qualification refuses' in result.stderr
print('Required Qdrant and complete gates refuse diagnostic-only mode')
