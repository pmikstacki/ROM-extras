# rom-projection-core

Bounded durable host projection intents and checkpoints using public ROM identities and an extras-owned redb file.

Implemented: private local Linux creation, exclusive ownership, bounded metadata, page preparation, exact observation reconciliation, and atomic completion.
Restart admission uses read-only preflight and a validated private repair copy for unclean accepted formats.
Unknown commit outcomes retire the engine and retain a bounded transition token for reconciliation after reopening.

This crate does not yet implement the projection worker, OpenSearch/Qdrant transport, ROM export policy, authorized queries, or generation switching.
Actual filesystem tests and controlled native I/O faults do not establish arbitrary hardware or distributed durability.

See [the checkpoint guide](https://github.com/pmikstacki/ROM-extras/blob/codex/initial-extras/docs/projection-checkpoints.md) for the qualified scope and remaining work.
