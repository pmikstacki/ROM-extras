# SQL Server native ownership mapping

Inspection date: 2026-10-09. Evidence: official documentation and source review only. No build, backend mutation, or new native qualification occurred.

## Recommendation

Implement the native ownership mapping in a maintained `rom-mssql` module after its real-server acceptance passes. Keep `rom-sql-core` responsible for shared arbitration. Keep downstream tests responsible for independent public consumption. Do not leave the substantive SQL Server driver only in a fixture.

This advances the requested SQL Server family without pretending to implement full ROM Storage. Public ROM main remains `d7ef529040eec60dc869034c2d33130219db85fe`. Its incremental Work preparation/read/accounting ports are absent. Native ownership needs no private Work policy or full-ledger serialization. See [public Work ledger](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/reaction_work/ledger.rs) and [Storage](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/persistence.rs).

Inspected extras files are `crates/rom-sql-core/src/{owner.rs,owner_transaction.rs,executor.rs}` and `tests/public-consumer/tests/mssql_executor.rs`. The shared trait already consumes transactions. Its finish helper preserves a semantic error only after confirmed rollback. Unknown rollback becomes Unknown. Its `with_owner` requires the same lock for prepared protected writes.

The existing SQL Server fixture uses a worker-owned current-thread Tokio runtime and Tiberius client. It sets `LOCK_TIMEOUT 5000` and `XACT_ABORT ON`. Its self-signed certificate bypass belongs only to that synthetic loopback fixture. The task reports server version 17.0.5005.3; this research did not independently query it.

## Exact transaction mapping

Use a worker-owned synchronous facade around Tiberius async operations. Tiberius 0.13.0 exports `begin_transaction_with_isolation`, `commit_transaction`, and `rollback_transaction`. These send TDS Transaction Manager requests. A successful begin records the transaction descriptor for subsequent requests. See [versioned Client source](https://github.com/tiberius-rs/tiberius/blob/v0.13.0/src/client.rs).

Open an explicit READ COMMITTED transaction on a clean connection. Set bounded connection, I/O, and lock deadlines. Do not allow nested transactions or arbitrary host batches on this connection. Require an existing provisioned singleton row and fixed fully qualified table name.

Proposed locking read:

```sql
SELECT format_version, store_identity, generation, owner_token
FROM dbo.rom_extras_owner WITH (UPDLOCK, HOLDLOCK)
WHERE singleton_key = @P1;
```

`UPDLOCK` holds update locks through transaction completion. `HOLDLOCK` gives SERIALIZABLE semantics for the referenced table. These hints protect arbitration even when ordinary reads use versioning. `ROWLOCK` can be added as a granularity preference, but is not a guarantee against index-key locks or escalation. Do not use NOLOCK or READPAST. See [SQL Server 2025 table hints](https://learn.microsoft.com/en-us/sql/t-sql/queries/hints-transact-sql-table?view=sql-server-ver17).

`lock_owner` requires exactly one row. It validates the immutable format and store identity before any write. It decodes the generation and token through the existing shared constructors. Zero rows means Uninitialized. Malformed state is Invalid. Future format means UnsupportedFormat. No normal-open or lock path provisions or repairs the row.

`write_owner` updates that same singleton inside the retained transaction. Bind generation as BIGINT and token as binary bytes. Require exactly one affected row, or one bounded `OUTPUT inserted.generation` result. Drain the entire TDS response before the next operation. The table is extras-owned and should have no host triggers. Triggers and extra result sets require separate support rather than assumptions about first-result ordering.

`commit` consumes the facade and reports success only after the completed native response. Map commit transport failure, deadline, or missing terminal acknowledgement to Unknown. A returned SQL error can narrow classification only when authoritative evidence proves complete rollback.

`rollback` consumes the facade and establishes terminal transaction cleanup. A bounded batch `IF XACT_STATE() <> 0 ROLLBACK TRANSACTION; SELECT XACT_STATE();` is a possible verified cleanup path. Require one result equal to zero after fully draining the response. This handles a transaction SQL Server already aborted. It does not prove that a preceding uncertain COMMIT failed.

## Binary identity and counters

Current shared `OwnerToken` is exactly 32 nonzero bytes. `OwnerState` accepts generation 0 through i64::MAX; generation zero must be idle. Preserve these constraints in SQL checks and host decoding. BIGINT supports this signed range. See [SQL Server integer types](https://learn.microsoft.com/en-us/sql/t-sql/data-types/int-bigint-smallint-and-tinyint-transact-sql?view=sql-server-ver17).

A proposed StoreIdentity should use an explicitly chosen fixed byte width. Do not silently infer its width from an unpublished type. SQL token/store columns can use VARBINARY with DATALENGTH checks. Exact length must also be validated before binding. BINARY casts can pad or truncate inputs, so successful conversion is not validation. Avoid character collations and UNIQUEIDENTIFIER byte-order conversion for opaque identity. See [binary and varbinary](https://learn.microsoft.com/en-us/sql/t-sql/data-types/binary-and-varbinary-transact-sql?view=sql-server-ver17).

Test zero bytes, high-bit bytes, malformed length, null idle owner, zero generation, negative persisted generation, and i64::MAX exhaustion. Never log token bytes, DSNs, native messages, or passwords.

## Timeout and error handling

`LOCK_TIMEOUT` is a session setting in milliseconds. Its default is unlimited; zero rejects waiting immediately. It does not replace a connect deadline or full operation deadline. A lock wait raises error 1222. Do not treat statement cancellation as complete rollback. See [LOCK_TIMEOUT](https://learn.microsoft.com/en-us/sql/t-sql/statements/set-lock-timeout-transact-sql?view=sql-server-ver17) and [locking guide](https://learn.microsoft.com/en-us/sql/relational-databases/sql-server-transaction-locking-and-row-versioning-guide?view=sql-server-ver17).

`XACT_ABORT ON` requests whole-transaction abort for runtime errors. Compilation errors are excluded. THROW honors this setting; RAISERROR does not. It is an additional guard, not a reason to omit explicit cleanup. See [XACT_ABORT](https://learn.microsoft.com/en-us/sql/t-sql/statements/set-xact-abort-transact-sql?view=sql-server-ver17).

`XACT_STATE` distinguishes no transaction (0), committable (1), and uncommittable (-1). State -1 needs complete rollback; a savepoint rollback is insufficient. See [XACT_STATE](https://learn.microsoft.com/en-us/sql/t-sql/functions/xact-state-transact-sql?view=sql-server-ver17).

SQL Server deadlock error 1205 identifies a victim whose transaction was rolled back. A bounded retry must restart the whole operation and reread ownership. It must not rerun only the last statement. See [1205](https://learn.microsoft.com/en-us/sql/relational-databases/errors-events/mssqlserver-1205-database-engine-error?view=sql-server-ver17).

Tiberius exposes `Error::code()` for server errors. Classify numeric codes, not message text. Keep finite shared error outcomes. No generic native-error string belongs in public diagnostics. See [versioned error source](https://github.com/tiberius-rs/tiberius/blob/v0.13.0/src/error.rs).

## Drop, cancellation and retirement

Tiberius 0.13.0 does not provide the shared synchronous transaction facade or its rollback-on-Drop contract. Implement that facade on the dedicated worker. Drop must attempt bounded complete rollback for an active healthy transaction. If cleanup is uncertain, discard the client and mark the connection unavailable. Drop cannot communicate confirmed rollback to the original caller.

The inspected Client documentation warns that dropped requests or result streams can leave the connection out of sync. It recommends discarding the client rather than reusing it. Dropping a future does not tell SQL Server to stop working. `cancel_query` sends Attention and drains its acknowledgement, but successful batch cancellation does not establish whole-transaction rollback or reverse a completed commit. Prefer retirement after timeout for the first maintained mapping. See [Client cancellation and transaction source](https://github.com/tiberius-rs/tiberius/blob/v0.13.0/src/client.rs).

Do not attempt to reuse a client after uncertain partial writes, incomplete responses, or rollback deadlines. A worker connection wrapper can hold `Option<Client>` and reject all subsequent jobs after taking/discarding the client. Existing Executor only retires automatically on an unwinding panic. Returning OwnerError alone does not retire its native connection.

A cancelled caller ticket is different: accepted worker SQL continues. Preserve current executor behavior. After uncertain commit, use a fresh connection for inspection and explicit recovery. A read showing absence while the original request can still finish is not proof of non-commit.

## Application-lock alternative

`sp_getapplock` supports Transaction and Session ownership. Transaction mode requires an active transaction and releases at its end. Session mode has a different lifetime. Its return codes include timeout, cancellation, and deadlock; a deadlock return requires explicit transaction handling. See [sp_getapplock](https://learn.microsoft.com/en-us/sql/relational-databases/system-stored-procedures/sp-getapplock-transact-sql?view=sql-server-ver17).

It can support optional admission, but cannot replace the persistent generation check in every protected transaction. A surviving second connection can outlive loss of the admission session. The row-lock mapping is simpler and directly matches the shared OwnerTransaction contract.

## Real-server acceptance before publishing support

- Race two idle claims on separate native connections; require one acknowledged owner and one Busy result.
- Hold a protected old transaction; prove takeover cannot acknowledge until that transaction ends.
- After takeover, retain an old second connection and reject both its protected write and release.
- Lose only the first acquisition connection; repeat takeover and surviving-connection fencing.
- Reject unsupported format and malformed binary state without writes; verify row bytes after reopen.
- Force a lock timeout; confirm complete rollback and unchanged prepared writes.
- Force a statement error after a prepared write; check cleanup for XACT_ABORT ON and intentionally OFF negative profile.
- Induce 1205 separately; verify complete rollback and full retry with fresh fence validation.
- Exercise delayed caller acknowledgement, actual lost commit response, Drop cleanup and native client retirement as distinct cases.
- Restart the same disk-backed SQL Server; preserve acknowledged generation and fence the old token.

These cases qualify native ownership only. They must consume the maintained module, not duplicate its implementation in fixtures. Keep schema provisioning separate. Full Work, Resource bundle, journal, registration, and maintenance support remain later gates.

The inspected registry package records VCS revision `7fd75bb7070fe6ea41374698abf78928bb983bda` with dirty=true. Therefore the package source is not represented here as a clean reproducible Git checkout. Versioned official source URLs and locked package evidence are separate. No fresh license, MSRV, dependency, or advisory audit was executed in this task.

## Implemented decisions and separate executed evidence

The maintained increment follows the row-lock design. Native SQLBatch sets session bounds and confirms rollback.
Actual SQL Server rejected rollback through sp_executesql with error266. RPC-scoped settings also failed to persist the lock deadline.
These observations explain the SQLBatch choice; they do not change the shared arbitration contract.
The adapter permanently retires timed-out native clients. It does not attempt Attention or implicit retry.

The ordering oracle joins [sys.dm_exec_requests](https://learn.microsoft.com/en-us/sql/relational-databases/system-dynamic-management-objects/sys-dm-exec-requests-transact-sql?view=sql-server-ver17)
with [sys.dm_exec_sql_text](https://learn.microsoft.com/en-us/sql/relational-databases/system-dynamic-management-objects/sys-dm-exec-sql-text-transact-sql?view=sql-server-ver17).
It matches the unique native locking query and rejects a separate actual native waiter.
These DMV permissions belong to the qualification fixture, not the runtime application role.

Thirteen registry dependencies entered the workspace; no previously locked package version changed.
Their declared licenses and MSRVs were inspected. All cached archive checksums and source files match Cargo's locked registry packages.
Tiberius declares Rust1.88 and MIT/Apache-2.0. Its packaged dirty VCS marker remains distinct from source archive integrity.
Rust1.99 compilation is required by the gate. Redistribution notices still need release review.
The independent consumer audit found zero advisories and warnings in 99 packages at RustSec commit7eebec69c352c7191b1f13eb95dd510eeca5d1de.
See the maintained guide and verification record for actual native qualification and exclusions.
