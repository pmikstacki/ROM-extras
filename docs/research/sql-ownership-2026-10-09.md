# Shared SQL ownership and control metadata

Inspection date: 2026-10-09. Evidence category: source review only. No builds, fixture mutations, service operations, or qualification tests occurred during this research.

## Implementable increment

Implement an extras-owned control-row transaction protocol first. PostgreSQL is the first native driver. Keep the protocol independent of Work policy and complete-ledger serialization.

The existing `crates/rom-sql-core/src/executor.rs` owns one connection on one worker. Its queue is bounded, but closure payload bytes are not. Its `execute`, `submit`, `close`, and `shutdown` methods do not implement database ownership. Waiting cancellation does not cancel admitted SQL. Driver errors are nested inside executor results. These are inspected source facts, not new test results.

Create a bounded protocol module and a PostgreSQL implementation using the existing locked driver graph. A successful native control/fence conformance suite is substantive groundwork. It does not complete `SqlStore<D>: Storage` or any provider family.

## Public ROM boundary

Authoritative source is ROM commit `d7ef529040eec60dc869034c2d33130219db85fe`. Inspect these exact files:

- `crates/rom/src/persistence.rs`: public `Storage`, `Bundle`, `Receipt`, and `Row`.
- `crates/rom/src/storage_ownership.rs`: process-local `StorageOwnership` and opaque `StorageOwner`.
- `crates/rom/src/reaction_work.rs`: exported Work types and `WorkLedger`.
- `crates/rom/src/reaction_work/ledger.rs`: public `enqueue`, `apply`, and full `records`; private storage/accounting helpers.
- `crates/rom/src/storage_state.rs` and its operator/maintenance modules: existing whole-state boundary.

[Public Storage source](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/persistence.rs), [ownership source](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/storage_ownership.rs), [Work ledger source](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/reaction_work/ledger.rs).

The public API does not export incremental Work readers, editors, accounting, or prepared deltas. Atomic `commit(Bundle)` includes reactions and completed Work. `reaction_update`, `control_work`, and Work maintenance also need those transitions. Implementing their complete bounded native persistence would require copying private policy or serializing the complete ledger. Neither workaround is approved.

The missing incremental Work boundary blocks full shared Storage implementation. It does not block ownership, transaction arbitration, immutable format validation, or bounded control metadata. The process-local owner guard has no public native-release callback. Native ownership must have a separately documented adapter lifetime. Do not claim automatic release when the final Runtime handle disappears while the adapter remains alive.

## Proposed control row

Use one immutable singleton key per logical store. Provision this row under explicit schema initialization. Keep it separate from Work records.

| Field | Proposed invariant |
| --- | --- |
| store identity | Fixed-size binary identity; immutable after creation |
| format | Exact supported format and family marker |
| epoch | Nonnegative signed 64-bit integer; checked increment; never reset or wrap |
| owner token | Optional fixed-size binary identity, compared byte-for-byte |
| control revision | Checked monotonic revision for bounded metadata CAS |
| last operation identity | Fixed-size identity for bounded immediate outcome reconciliation |
| control bytes | Strict bounded, versioned metadata; no complete Work ledger |

A signed 64-bit epoch is an extras protocol choice, not a restriction on ROM's public u64 revisions. Reject exhaustion before writes. Reject invalid sizes, negative counters, unknown format, and inconsistent null-owner state before mutation.

Control metadata can initially hold an extras protocol revision and a synthetic conformance counter. Do not invent defaults for ROM retry epochs, retention floors, or Work accounting. Later canonical ROM metadata must use published preparation helpers when available.

## Portable transaction seam

Proposed typed operations are `inspect_control`, `claim_idle`, `release_exact`, `takeover_exact`, and `update_control_exact`. All inputs have fixed or declared maximum sizes. Return owned bounded snapshots, never database guards.

A protected transaction obtains an exclusive arbitration lock on the singleton control row. It validates format, store identity, owner token, and epoch inside that transaction. It holds arbitration until commit or confirmed rollback. All protected writes use that same native transaction.

A read of the epoch outside the transaction is insufficient. Checking the epoch and then writing after releasing its lock permits an old writer to cross takeover. The lock order is control row first, then bounded subordinate keys in deterministic order. Do not invoke policies, codecs, network callbacks, or arbitrary host closures while holding database locks.

- `claim_idle`: while locked, require no owner, increment epoch, and install the caller's unique token atomically. A conflicting active owner returns Busy without writes.
- `release_exact`: require exact store identity, epoch, and token. Clear owner without decrementing epoch. A stale release cannot clear the next owner.
- `takeover_exact`: trusted explicit recovery operation; require the expected old epoch/token and increment epoch. This is not automatic failure detection.
- `update_control_exact`: require current owner and expected control revision. Update bounded control metadata and operation identity together. A stale owner or revision changes nothing.
- `inspect_control`: bounded coherent snapshot for diagnosis/reconciliation. Inspection never grants ownership.

Each mutating transaction must first inspect format under its row lock. Do not use an UPDATE to lock before validating an unsupported format. Normal open performs no automatic DDL, migration, reset, or takeover. An absent table is Uninitialized, not permission to overwrite an arbitrary database.

`takeover_exact` fences future database writes by the old owner. It cannot retract earlier commits or external effects. An in-flight protected transaction can commit before takeover, because takeover waits for its lock. After acknowledged takeover, no old-token transaction may commit a protected write.

No owner lease or automatic expiry is required for this first increment. A crashed owner can leave a durable Busy row. Explicit trusted takeover restores availability. Automatic leases would require separate clock, renewal, expiry, and delayed-writer tests. Never equate a TCP disconnection with proof that every old session stopped.

## PostgreSQL mapping

Use an explicit transaction with a bounded `SELECT ... WHERE singleton_key = $1 FOR UPDATE`, followed by format and fence checks. PostgreSQL holds row locks until transaction end. At READ COMMITTED, a waiting locking read returns the updated row. SERIALIZABLE can instead reject conflicting changes; do not silently reuse stale results. See [PostgreSQL 18 locking](https://www.postgresql.org/docs/18/explicit-locking.html).

Use parameterized conditional updates and `RETURNING`, requiring exactly one returned row. PostgreSQL treats zero updated rows as a successful SQL statement, so the adapter must map it to protocol conflict. This avoids assuming portable affected-row counting for no-op writes. See [PostgreSQL 18 UPDATE](https://www.postgresql.org/docs/18/sql-update.html).

Schema creation belongs to explicit provision, not normal open. A unique singleton primary key arbitrates concurrent initial inserts. PostgreSQL `ON CONFLICT` is one driver-specific provisioning alternative. Do not port its syntax as a generic SQL assumption. See [PostgreSQL 18 INSERT](https://www.postgresql.org/docs/18/sql-insert.html).

Set driver deadlines and transaction-local lock/statement timeouts deliberately. A lock timeout is not a universal commit outcome classifier. Require normal logged tables and the qualified durability settings. Disabling fsync, synchronous_commit, or using unlogged tables changes durable acceptance. See [client timeouts](https://www.postgresql.org/docs/18/runtime-config-client.html) and [non-durable settings](https://www.postgresql.org/docs/18/non-durability.html).

A session advisory lock is an optional additional PostgreSQL admission guard. It must not replace the persisted fence. Session locks survive transaction rollback and differ from transaction-scoped locks. Shared row arbitration avoids requiring equivalent advisory locks from the other five backends. See [advisory locks](https://www.postgresql.org/docs/18/explicit-locking.html#ADVISORY-LOCKS).

## Other native mappings to qualify

| Backend | Mapping candidate and required distinction |
| --- | --- |
| MSSQL | Explicit transaction; singleton read with appropriate update/hold locking or conditional write arbitration. Verify behavior under actual isolation/RCSI profile; do not assume plain SELECT locks the row. |
| MySQL | InnoDB explicit transaction with `SELECT FOR UPDATE`; autocommit must not end the lock before subsequent writes. |
| MariaDB | Same protocol, independently qualified native profile; `FOR UPDATE` requires a transaction/autocommit disabled for InnoDB. |
| CockroachDB | Serializable transaction; conditional mutation and complete transaction retry on confirmed 40001. Correctness must not depend only on a non-durable lock surviving topology changes. |
| Oracle | Explicit transaction with `SELECT FOR UPDATE`; parameterized RAW identities and exact result checks. Statement failure does not establish whole-transaction rollback. |

Primary mappings: [SQL Server isolation](https://learn.microsoft.com/en-us/sql/t-sql/statements/set-transaction-isolation-level-transact-sql?view=sql-server-ver17), [MySQL 8.4 locking reads](https://dev.mysql.com/doc/refman/8.4/en/innodb-locking-reads.html), [MariaDB FOR UPDATE](https://mariadb.com/docs/server/reference/sql-statements/data-manipulation/selecting-data/for-update), [CockroachDB v26.3 retry errors](https://www.cockroachlabs.com/docs/v26.3/transaction-retry-error-reference), [Oracle 26 SELECT](https://docs.oracle.com/en/database/oracle/oracle-database/26/sqlrf/SELECT.html).

These are mapping candidates, not executed protocol compatibility. The first implementation should expose only the confirmed PostgreSQL mapping. Keep generic protocol types ready for the other drivers without asserting six implemented ownership providers.

## Outcome and recovery contract

Distinguish InvalidInput, UnsupportedFormat, Uninitialized, Busy, StaleOwner, Conflict, ConfirmedRollback, Unknown, and Unavailable. A retryable transaction abort is not a lost commit acknowledgement. A timeout waiting for an executor ticket means the admitted job can still finish.

After commit acknowledgement loss, inspect through a fresh bounded connection. Matching operation identity plus resulting revision supports committed reconciliation. Absence only establishes non-commit after the original transaction is known terminal; it cannot prove an in-flight transaction will never commit.

A single last-operation marker is only sufficient while later writes are gated during unresolved recovery. If later independent mutations can overwrite it, use durable bounded per-operation receipts with a defined retention horizon. Do not advertise arbitrary historic exactly-once replay from one overwritten marker.

An unknown acquire must not grant a usable owner token. Reconcile first. An unknown release must not authorize takeover through inference. Retire or quarantine unhealthy native sessions. The existing executor does not itself classify driver commit errors or retire a connection on an ordinary returned database error.

## Concrete qualification cases

Reuse `tests/public-consumer/tests/postgres_executor.rs`, `cockroach_executor.rs`, `mssql_executor.rs`, `mysql_executor.rs`, and `oracle_executor.rs`. MySQL and MariaDB are separate profiles of the mysql fixture. Shared PGwire cases are in `tests/common/pgwire/cases.rs`. These files already distinguish executor checks from Storage conformance.

1. Provision, claim, update, release, and reopen the same native disk store; verify exact identity and monotonic revisions.
2. Race two independent connections claiming an idle row; require one acknowledged owner and one Busy result.
3. Hold an old protected transaction, begin takeover, and prove takeover cannot acknowledge before that transaction ends.
4. After takeover, keep a second old connection alive; reject its writes and release without changes.
5. Disconnect only the original acquisition connection; retain the old pooled connection and repeat stale-fence assertions.
6. Race takeover against a new protected transaction; every successful write belongs entirely before or after the fence transition.
7. Inject statement failure after tentative control mutation; explicitly rollback the whole transaction and verify unchanged bytes.
8. Delay acknowledgement beyond the ticket deadline; establish Unknown, then reconcile the real committed operation.
9. Drop commit acknowledgement separately; distinguish actual wire fault from a test barrier after SDK success.
10. Reject future format, malformed bytes, epoch exhaustion, wrong store identity, and oversized control bytes without maintained-row writes.
11. Restart the actual server on the same disk; verify acknowledged control metadata and stale-token rejection.
12. Bound lock waits, transaction retries, queue admission, and captured bytes. Confirm failed admission performs no SQL.

For every backend, reuse the same protocol assertions with native transaction mappings. Existing executor qualification does not prove these new ownership invariants. Full Storage conformance remains blocked by the public incremental Work boundary described above.
