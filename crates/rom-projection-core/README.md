# rom-projection-core

Bounded durable host projection intents and checkpoints using public ROM identities and an extras-owned redb file.

Implemented: private local Linux creation, exclusive ownership, bounded metadata, page preparation, exact observation reconciliation, and atomic completion.
Restart admission uses read-only preflight and a validated private repair copy for unclean accepted formats.
Unknown commit outcomes retire the engine and retain a bounded transition token for reconciliation after reopening.

The typed storage worker owns one native thread, bounds its command queue, and joins through checkpoint destruction.
Asynchronous responses do not cancel admitted operations when dropped. Startup and shutdown require a blocking host context.

Pending-history reconstruction validates bounded authorized-batch metadata, clamps overrun and compares the complete immutable intent without checkpoint publication.
Immutable selected-field documents derive canonical content identities and durable profiles, with bounded exact numeric admission and finite vectors.
Actual public SQLite/redb feeds and native provider round-trips remain unqualified.

This crate does not yet implement page/provider orchestration, OpenSearch/Qdrant transport, ROM export policy, authorized queries, or generation switching.
Actual filesystem tests and controlled native I/O faults do not establish arbitrary hardware or distributed durability.

See [the checkpoint guide](https://github.com/pmikstacki/ROM-extras/blob/codex/initial-extras/docs/projection-checkpoints.md) for the qualified scope and remaining work.
