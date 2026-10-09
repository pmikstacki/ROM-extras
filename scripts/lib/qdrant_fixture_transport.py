"""Reuse the existing redirect/proxy refusal policy for native fixture clients."""
from pathlib import Path
import sys
sys.path.insert(0,str(Path(__file__).resolve().parents[2]/'tests/projection-probes'))
from probe_transport import client
__all__=['client']
