# rom-projection-core

Bounded durable host projection intents and checkpoints using public ROM identities and an extras-owned redb file.

Implemented: private local Linux creation, exclusive ownership, bounded metadata, page preparation, exact observation reconciliation, and atomic completion.
Restart admission uses read-only preflight and a validated private repair copy for unclean accepted formats.
Unknown commit outcomes retire the engine and retain a bounded transition token for reconciliation after reopening.

The typed storage worker owns one native thread, bounds its command queue, and joins through checkpoint destruction.
Asynchronous responses do not cancel admitted operations when dropped. Standalone startup and shutdown require a blocking host context.
StorageLifecycle supervises one managed owner, with async startup responses, nonblocking owner Drop and retained shutdown_async completion.
Create and finally join the lifecycle host from a blocking embedding context. Native I/O has no hard termination deadline.

Pending-history reconstruction validates bounded authorized-batch metadata, clamps overrun and compares the complete immutable intent without checkpoint publication.
Immutable selected-field documents derive canonical content identities and durable profiles, with bounded exact numeric admission and finite vectors.
RuntimeHistory uses public authorized SQLite/redb journals. Native tests cover restart, current row/field revocation and filtered cursor progress.
Provider transport qualification is separate from the source contract.

The core Worker composes durable preparation, target observation and exact pending-history recovery.
Target adapters and history sources remain trusted seams; synthetic tests do not qualify native integration.

This crate does not yet implement native provider transport, OpenSearch/Qdrant transport, ROM export policy, authorized queries, or generation switching.
Actual filesystem tests and controlled native I/O faults do not establish arbitrary hardware or distributed durability.

See [the checkpoint guide](https://github.com/pmikstacki/ROM-extras/blob/codex/initial-extras/docs/projection-checkpoints.md) for the qualified scope and remaining work.


Typed text search adds a separate trusted SearchTarget and an asynchronous host SearchPolicy.
Search fixes the caller, Resource kind and provider generation. TextQuery bounds results to 64 and candidates to 256.
The core mints ApprovedTextQuery only after current scope approval. It rechecks policy after backend completion and before publication.
Candidates carry original keys and indexed revisions. Public Runtime::read_projected supplies every returned current value.
Missing, denied, tombstoned, stale and hidden-query-field candidates are omitted. Foreign kinds, duplicate keys and candidate overruns are rejected.
Debug and errors omit text and keys. One candidate page can yield fewer permitted results than requested.
Native SQLite/redb tests use a controlled candidate target; this does not qualify OpenSearch or Qdrant search transport.
The host policy is trusted and must perform bounded current checks. There is no atomic multi-row authorization snapshot or continuous revocation fence.
