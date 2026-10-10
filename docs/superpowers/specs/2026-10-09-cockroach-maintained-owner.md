# Maintained CockroachDB ownership and shared PGwire boundary

Advance the approved full ROM-extras goal without redefining ownership as full Storage.
Preserve all public rom-postgres constructors, parameter paths, transaction behavior and historical evidence.
Keep the public ROM pin; do not copy unpublished Work or Storage policy.
Root owns maintained edits. A final reviewer owns only a named ignored review note.

## Architecture

Add rom-pgwire as the shared native protocol boundary for tokio-postgres0.7.18.
It owns endpoint validation, TLS Require versus explicit literal-loopback NoTls, current-thread runtime and retained transport task.
Expose finite errors with SQLSTATE only; never expose SQL, credentials or native error strings.
Expose bounded complete batch, execute and metadata query operations, availability and retirement.
Do not expose Client, runtime, transport task or arbitrary host initialization/policy callbacks.
Complete connect/I/O futures remain bounded through shared OperationDeadlines. Refuse blocking calls under active Tokio.
Abort and await owned transport destruction outside active host runtime; async-handoff Drop destroys the private runtime safely.
Keep provider-owned transaction/session SQL, native profile admission and ownership generation logic out of the transport.

Add a pure rom-sql-core owner-row validator for already-decoded format/identity/generation/token/length primitives.
Keep SDK wire decoding in providers. Preserve exact32-byte identity/token and signed native generation range.
This is shared ownership validation, not a copied ROM Resource, Work or authorization policy.
Use it in PostgreSQL/Cockroach; remove remaining duplicated validation in maintained MySQL/MSSQL where native mappings permit.
Do not force common transport on different protocols.

Add separate rom-cockroach Connection, Transaction, ControlTable and Deadlines over shared PGwire and OwnerTransaction.
PostgreSQL still starts READ COMMITTED with its existing native profile.
Cockroach admits the verified26.3 family and explicitly starts SERIALIZABLE.
Set and verify enable_durable_locking_for_serializable=true and optimizer_use_lock_op_for_serializable=true for its own session.
Set and verify provider-owned statement, row-lock and idle limits with explicit millisecond units.
No global/cluster mutation, implicit reconnect, provision, retry, default external endpoint or liveness takeover.
Do not treat synchronous_commit as Cockroach durability evidence.

## Ownership and failures

Retain the exclusive singleton lock through complete commit/rollback acknowledgement.
Host provisions immutable indexed non-null control identity/generation in one native column family.
Project identity/token with OCTET_LENGTH and CASE; use LIMIT2 and exact row/column bounds.
Decode actual native integer/BYTES wire types without panic or string/float coercion.
Malformed format, identity, token, generation or provisioning rejects without repair.
Every caught prepared rejection poisons the transaction. Confirm full rollback before reuse.
40001 is not permission to replay only one statement or a host FnOnce callback.
Initially use explicit host retry only after confirmed rollback and fresh ownership reads; no automatic retry policy.
40003 and lost commit acknowledgements remain Unknown. Conservative commit failures retire the native transport.
Do not mint ownership from inspection after Unknown.
Use existing SQL16KiB/64parameter persistence limits; host must bound payload, total statements and CPU work between calls.
This trusted persistence port forbids DDL, session control, rowsets and external effects.

## Qualification

TDD includes a new independent public consumer and behavioral negative controls.
Qualify exact transitions, competing claims, stale surviving clients, ordered locking, timeouts and caught-error prefix rollback.
Use independent malformed-field resets, missing/duplicate rows, exact identity and generation exhaustion.
Verify native40001 with rollback and fresh reread; keep confirmed rejection separate from ambiguous commit.
Use a controlled opaque native relay for actual committed response loss, whole-I/O bounds and peer closure before cleanup.
Verify constructor blackhole and async-handoff Drop.
Run source and normalized archive consumers independently of workspace feature unification.
Run current PostgreSQL/MySQL/MSSQL affected gates after shared refactors, then the full local verifier on frozen sources.
Record dependency features, advisories, licenses, actual Rust version and preserved lock identities/checksums.
A required maintained same-image/same-volume restart profile must preserve acknowledged owner guards without minting replacements.
Do not infer range splits, lease transfers, quorum loss, multi-region behavior or power-loss support from a single node.
The first local native qualification remains explicitly limited; broader distributed qualification stays in the full goal.

## Source evidence before implementation

Primary review: docs/research/cockroach-maintained-owner-2026-10-09.md.
Native session preflight on exact retained26.3.2 fixture confirmed defaults off, session enabling, milliseconds and SERIALIZABLE.
This preflight is not adapter, ownership, retry or distributed qualification.
