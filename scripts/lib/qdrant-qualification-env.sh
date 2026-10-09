# A diagnostic loss probe is a separate invocation, never a successful required acceptance gate.
if [[ "${ROM_EXTRAS_QDRANT_TARGET_REPLAY_LOSS_PROBE:-0}" == 1 ]]; then
    echo 'Required qualification refuses diagnostic-only Qdrant replay mode.' >&2
    exit 2
fi
