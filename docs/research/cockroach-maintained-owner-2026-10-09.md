# Maintained CockroachDB ownership

Inspection date: 2026-10-09. Primary-source review only. No maintained edits, backend actions or builds occurred.

## Module decision

Use a separate `rom-cockroach` provider with shared OwnerTransaction arbitration. Reuse the PGwire transport implementation, not PostgreSQL's transaction/session semantics. Preserve existing rom-postgres public constructors and READ COMMITTED behavior.

Inspected `crates/rom-postgres/src/{connection.rs,transaction.rs,config.rs}`, shared owner/core modules, `tests/common/pgwire/connection.rs` and existing native Cockroach executor fixture. rom-postgres begin explicitly sets READ COMMITTED and synchronous_commit; neither identifies the desired Cockroach contract. An alias to that driver would hide substantive differences.

Factor a small transport-only module/crate if both maintained providers need identical connect/whole-I/O/retirement logic. Keep TLS, Tokio task ownership and SQLSTATE access there. Keep transaction initialization, row typing, native retry classification and provider checks in provider modules. Do not move transport into rom-sql-core or offer arbitrary host initialization SQL/policy callbacks to achieve reuse.

Public ROMd7ef incremental Work preparation/accounting APIs remain absent. Native ownership can advance without those APIs; full Storage cannot be claimed.

## Locking and strict ownership contract

Cockroach26.3 supports SERIALIZABLE and READ COMMITTED, with SERIALIZABLE default. Its default SERIALIZABLE FOR UPDATE locks are best-effort in-memory locks. Range events can drop locks, changing intended access ordering. Serializable correctness still holds, but the default is insufficient evidence for a strict retained-lock OwnerTransaction contract. See [v26.3 FOR UPDATE](https://www.cockroachlabs.com/docs/v26.3/select-for-update).

The documented `enable_durable_locking_for_serializable=true` option replicates locks through Raft across lease transfers. `optimizer_use_lock_op_for_serializable=true` selects the newer Lock operator. Both default off in the inspected documentation. Set and verify the bounded provider-owned session profile explicitly; fail closed if required settings are unsupported. Do not silently fall back to default best-effort locking. See [v26.3 session variables](https://www.cockroachlabs.com/docs/v26.3/set-vars).

Use explicit SERIALIZABLE begin, exclusive singleton FOR UPDATE, exact immutable format/store identity checks and current generation/token checks. No SKIP LOCKED. Use a single column family with non-null control identity and generation; documented completely-null-family locking limitations should not be reachable. All protected writes and takeover use the same control row and native transaction.

A conditional control-row update can strengthen arbitration with a persisted write intent. It still requires unsupported-format inspection before writes and explicit serializable error handling. It is an alternative, not permission to relax shared correctness. Qualify the actual native profile rather than infer distributed behavior from PostgreSQL tests.

## Retry and ambiguity

A server 40001 is a transaction retry error. Retry the complete native operation with fresh reads/fence validation after established rollback. Do not replay only the final statement or blindly rerun a host FnOnce callback. Bound retry count, total elapsed time and backoff. Initial implementation may return a finite rejection and require explicit host retry rather than provide transparent callback replay. See [v26.3 retry reference](https://www.cockroachlabs.com/docs/v26.3/transaction-retry-error-reference).

40003 is explicitly ambiguous: neither commit nor failure can be assumed. A transport-lost COMMIT also remains Unknown. A later ROLLBACK cannot retract a committed transaction. 23505 and ordinary statement errors require abort/rollback and a new transaction before retry. Multi-roundtrip transactions cannot assume automatic retry based on a small result-buffer size. See [v26.3 transactions](https://www.cockroachlabs.com/docs/v26.3/transactions).

Current generic OwnerError has no Retryable variant. Preserve finite public errors; private SQLSTATE evidence can establish confirmed 40001 handling in native tests. Do not claim all Unavailable errors are safe to retry. A separate retry helper must accept only replayable prepared operations; no copied Work policy.

Commit should remain conservative Unknown unless its precise native result proves rollback. If shared finish cannot represent a confirmed retryable COMMIT abort, document that limitation rather than map ambiguity to a harmless conflict.

## Session bounds and durability

The current26.3 table documents statement_timeout, lock_timeout and idle_in_transaction_session_timeout. lock_timeout applies per row-level lock, including reads that wait on conflicting locks. Idle timeout terminates idle transaction sessions. `synchronous_commit` is a PostgreSQL compatibility no-op, so SHOW on does not establish durability. Use explicit duration units and verify effective settings on the actual session. See [v26.3 SET](https://www.cockroachlabs.com/docs/v26.3/set-vars).

Do not assume PostgreSQL numeric millisecond syntax or SET LOCAL behavior without native tests. The maintained profile can set provider-owned session values before BEGIN and reapply/verify them on reconnect. A prepared persistence extension must forbid changing those session values.

Absolute connect and native whole-response deadlines remain supplied by the owned Tokio facade. Server timeouts cannot bound a blackholed response. Poison transactions on caught prepared errors; retire client and transport after uncertain cleanup/commit. Use explicit bounded complete ROLLBACK for semantic rejections. See [v26.3 ROLLBACK](https://www.cockroachlabs.com/docs/v26.3/rollback-transaction).

## Binary projections and SDK decoding

Cockroach BYTES is the binary type; BYTEA is an alias. Preserve exactly32-byte store identities/tokens. Use OCTET_LENGTH/CASE projections and separate nullable token-length evidence before collection, with LIMIT2 and fixed columns. Validate malformed length instead of interpreting CASE NULL as idle. See [v26.3 BYTES](https://www.cockroachlabs.com/docs/v26.3/bytes) and [functions/operators](https://www.cockroachlabs.com/docs/v26.3/functions-and-operators).

Explicitly cast selected format to INT4 and generation/length to INT8 where the SDK decoders require those wire types. Cockroach default integer typing must not be assumed identical to a PostgreSQL INTEGER schema. Use try_get and reject schema mismatch without panic. Native tests must check the actual wire column types and affected-row result.

Use locked tokio-postgres0.7.18. Requests only begin when their futures are polled; Client is separate from the transport-driving Connection future. Keep that task retained and sequentially process requests. Transaction Drop queues rollback rather than confirming it synchronously. The facade must perform explicit bounded cleanup or retire transport. See [exact SDK behavior](https://docs.rs/tokio-postgres/0.7.18/tokio_postgres/) and [transaction source](https://docs.rs/crate/tokio-postgres/0.7.18/source/src/transaction.rs).

## TLS and version profile

Keep the host-supplied TLS connector and force encrypted connection for remote profiles. Encryption alone does not establish hostname or CA verification; those remain explicit host configuration and independent TLS tests. NoTls belongs only to the labelled literal-loopback insecure fixture. Never infer a secure cluster from its PostgreSQL-compatible port. See [Cockroach TLS](https://www.cockroachlabs.com/docs/v26.3/security-reference/transport-layer-security).

Require actual Cockroach server identity and tested26.3 release range. Do not treat a PostgreSQL version string compatibility presentation as sufficient. Record actual native version and required session settings through bounded trusted observations. Do not mutate cluster settings merely to make an unsupported profile pass.

## Qualification and distributed limits

Run existing owner protocol assertions through maintained public constructors: competing claim, release, ordered takeover, old second connection, lock timeout, malformed control bytes, exhaustion, immutable-format rejection and caught-error rollback. Add confirmed40001 followed by whole-operation reread; Unknown commit and actual wire-response loss remain separate cases.

For native restart, acknowledge an owner transition/sentinel, close clients without releasing durable ownership, restart the exact same disk-backed fixture and compare immutable identity, generation/token and sentinel. Use Cockroach-native cluster/node/session observations, not PostgreSQL postmaster functions. Query readiness against the actual database before reusing ownership. Do not reprovision or create a new store on missing data.

The single-node local executor fixture does not qualify lease transfers, range splits, replication quorum loss, multi-region ordering or distributed ambiguous recovery. Source-documented durable-lock settings are prerequisites, not executed proofs of those failures. A later multi-node profile must cover them independently.

Record exact source/lock/compiler/native version and command/result when implementation is tested. This report recommends bounded native groundwork only, not a completed Cockroach Storage adapter or distributed durability certification.

## Root native preflight

The current retained native fixture is26.3.2, SQL port55457 mapped to container26259, database rom_extras.
A bounded CLI session confirmed both durable-lock/operator defaults off and successful own-session enabling.
It confirmed statement_timeout3000ms, lock_timeout500ms, idle timeout3000ms and explicit SERIALIZABLE.
No cluster settings or persisted application data changed.
The initial probe used an incorrect historical default port; that failed before connection and its log is retained.
Exact native source: .superpowers/cockroach-native-profile-preflight-attempt2-2026-10-09.log.
This is native setting evidence only; ownership, retry, recovery and distributed support remain unimplemented/unqualified.

## Implementation research and controlled native observations

The exact [tokio-postgres RowStream contract](https://docs.rs/tokio-postgres/0.7.18/tokio_postgres/struct.RowStream.html) exposes affected rows after exhaustion.
Its prepared execute implementation discards DataRow messages; the shared persistence primitive instead rejects rowsets while consuming valid complete responses.
Retained row/column/decoded-byte budgets do not cap one SDK frame allocation. No hostile-server allocation qualification is claimed.

The [official lock observability RFC](https://raw.githubusercontent.com/cockroachdb/cockroach/master/docs/RFCS/20220104_cluster_locks.md) distinguishes replicated lock holders and waiters.
Its historical schema is guidance, not the26.3 executable contract. The retained26.3.2 fixture confirmed actual columns before qualification.
Exact current-table witnesses observe one replicated exclusive SELECT lock and one waiter before any protected write.
Disabling durable locking makes that witness fail; the source is restored afterward.

The [current session-variable documentation](https://www.cockroachlabs.com/docs/v26.3/set-vars) explains default-off allow_unsafe_internals.
Native26.3.2 force_retry first failed42501 even for root; only named diagnostic sessions now explicitly enable this variable.
The production provider never enables it. Native40001 rollback and fresh ownership reread pass in this controlled profile.
The [official developer builtin source](https://raw.githubusercontent.com/cockroachdb/cockroach/master/pkg/sql/sem/builtins/builtins.go) defines force_error as a testing function.
Native developer injection proves40003 classification and malformed SQLSTATE disposal; it does not simulate an actual distributed ambiguous commit.
A native view emitting40003 reproduced Unavailable during ownership read before the classifier fix; restored behavior is Unknown with retirement.
Malformed native error codes can contain arbitrary text; only five uppercase/digit bytes are retained after validation.

The [official26.3 licensing FAQ](https://www.cockroachlabs.com/docs/v26.3/licensing-faqs) distinguishes developer single-node access from licensed deployment.
The maintained MIT adapter does not determine server license eligibility or enable production telemetry.
Full Storage, production TLS and multi-node failure qualification remain separate outstanding work.
