# MSSQL deadlock and restart qualification

Inspection date: 2026-10-09. Source review only. No tests, service operations, or maintained edits occurred. Task baseline: `1f11301760b1578a3850df6dc4396f7c1418a430`.

## Existing driver constraints

Inspected `crates/rom-mssql/src/{connection.rs,transaction.rs,config.rs}` and downstream `tests/public-consumer/tests/mssql_executor.rs`. `Connection::run` maps server statement errors to OwnerError::Unavailable. Timeout/transport uncertainty retires the native client and returns Unknown. Transaction errors poison commit. Shared finish attempts full rollback before preserving the statement error.

`cleanup` uses XACT_STATE and @@TRANCOUNT to establish no remaining transaction. `Transaction::execute` permits prepared bounded native persistence, but forbids session changes, transaction changes, DDL and rowsets. Preserve this boundary. Do not add policy callbacks or expose native messages merely to implement a test.

Deadlines cap connect/I/O at 60 seconds and require a positive shorter native lock timeout. The existing fixture's shorter timeout is not automatically adequate for deterministic deadlock qualification.

## Native cycle without a production hook

Provision a disposable auxiliary row S and control row C using the existing fixture setup. Use a separate raw Tiberius fixture connection A. The maintained driver connection B retains its default NORMAL deadlock priority.

1. On A, set XACT_ABORT ON and DEADLOCK_PRIORITY HIGH before its explicit transaction.
2. A updates S and retains its lock.
3. B begins through maintained code and locks C through shared ownership validation.
4. B stages a fixed prepared sentinel write, then issues a prepared UPDATE on S. This blocks on A.
5. After fixture coordination confirms the wait, A attempts UPDATE on C. This creates the C/S cycle.
6. Observe SQL Server choosing B as victim. Require native 1205 evidence, not only a finite driver error.
7. Let A finish and explicitly rollback its fixture-only writes. Verify B's sentinel and owner changes did not persist.

The intentional reversed lock order belongs only to fault qualification. Production protected transactions continue locking C first. A can use raw fixed SQL because it is an infrastructure adversary, not a domain action callback.

Different DEADLOCK_PRIORITY values determine the victim. SQL Server rolls back that victim and returns 1205. HIGH is greater than NORMAL, so this topology selects B without changing B's session inside Transaction::execute. Equal priorities would allow rollback cost to decide. See [DEADLOCK_PRIORITY](https://learn.microsoft.com/en-us/sql/t-sql/statements/set-deadlock-priority-transact-sql?view=sql-server-ver17) and [1205](https://learn.microsoft.com/en-us/sql/relational-databases/errors-events/mssqlserver-1205-database-engine-error?view=sql-server-ver17).

Synchronize with bounded fixture channels and native blocking observations. A sleep alone does not prove B has entered the second lock wait. An observer can inspect the isolated sessions' blocking requests before closing the cycle. No raw SQL text, credentials or private session details should enter published evidence.

## 1205 versus 1222

Current public OwnerError intentionally loses the native error code. Unavailable alone cannot prove 1205. Neither an elapsed duration nor a rollback result distinguishes a deadlock from lock timeout.

Use fixture-side native observation instead of expanding production errors solely for testing. A short-lived isolated Extended Events session can observe `xml_deadlock_report` and numeric `error_reported` codes. Resolve the actual event/action schema through native metadata before setup. Match the victim to B and the resource cycle to the unique C/S fixture objects. Reject unrelated events. Publish only sanitized booleans/counts: victim matched, 1205 count, 1222 count, resource cycle matched.

SQL Server's deadlock guide documents `xml_deadlock_report`, victim/process/resource lists, and the default system_health capture. A bounded dedicated session avoids mixing unrelated historical reports. Existing system_health is an alternative when fixture permissions prohibit a dedicated session, but bounded event filtering and exact attribution remain necessary. See [deadlock guide](https://learn.microsoft.com/en-us/sql/relational-databases/sql-server-deadlocks-guide?view=sql-server-ver17).

Ring-buffer targets can drop or truncate events and process asynchronously. Missing evidence is an inconclusive test, not proof that no deadlock occurred. Poll with a fixed deadline and bounded result bytes. Remove only the newly created fixture session afterward. Do not print whole event XML; it can include SQL text and application values. See [Extended Events targets](https://learn.microsoft.com/en-us/sql/relational-databases/extended-events/targets-for-extended-events-in-sql-server?view=sql-server-ver17).

For an independent raw-client control experiment, Tiberius Error::code() exposes the numeric server code. That experiment can verify observation plumbing, but cannot substitute for proving the maintained driver's victim and rollback behavior. See [Tiberius 0.13 error source](https://github.com/tiberius-rs/tiberius/blob/v0.13.0/src/error.rs).

The monitor initially checks at approximately five-second intervals, and adjusts frequency. A five-second lock timeout can expire before the cycle is detected. Use a longer bounded fixture Deadlines profile, for example lock 45 seconds and I/O 55 seconds, within the maintained API constraints. This is a proposed test profile, not a timing guarantee. Require 1205 and explicitly reject 1222. Do not increase production defaults or hide timeout failures with retries. See [deadlock detection](https://learn.microsoft.com/en-us/sql/relational-databases/sql-server-deadlocks-guide?view=sql-server-ver17) and [LOCK_TIMEOUT](https://learn.microsoft.com/en-us/sql/t-sql/statements/set-lock-timeout-transact-sql?view=sql-server-ver17).

## Rollback and fence revalidation

XACT_ABORT ON is an additional guard. It does not eliminate explicit cleanup or distinguish lost acknowledgements. Compilation errors and RAISERROR have different handling. See [XACT_ABORT](https://learn.microsoft.com/en-us/sql/t-sql/statements/set-xact-abort-transact-sql?view=sql-server-ver17).

After the maintained victim returns Unavailable, require the sentinel to be absent. Require cleanup to establish XACT_STATE=0 and @@TRANCOUNT=0 through the existing driver behavior. Confirm a fresh transaction works; do not reuse the dead transaction.

Then complete acknowledged takeover through another maintained connection. Retry the prepared operation explicitly with the old Ownership. Require Stale and no sentinel write. Repeat with the newly acknowledged owner and require one committed result. This proves full native rollback followed by current fence revalidation. No automatic production retry or Work policy is needed.

Tiberius request cancellation does not establish transaction rollback. Dropping its future leaves server work potentially active and can desynchronize the client. Preserve the existing timeout retirement path. See [Tiberius 0.13 Client cancellation and transaction methods](https://github.com/tiberius-rs/tiberius/blob/v0.13.0/src/client.rs).

## Same-disk restart acceptance

Before restart, acknowledge a maintained claim/takeover and one protected sentinel write. Retain the exact trusted owner state and immutable store identity privately. Drain/shutdown admitted executor jobs before the clean restart scenario.

Restart only the explicitly selected fixture container on its existing storage. Do not recreate the database, initialize a new volume, delete tables, or claim a fresh generation during reopen. Confirm persistent host volume/bind identity and the same database files. SQL Server container storage requires preserved mounted directories or the same container filesystem; deleting a container without persistent storage loses its files. See [SQL Server container persistence](https://learn.microsoft.com/en-us/sql/linux/containers/configure?view=sql-server-ver17).

Use readiness checks against the real fixture database. Reopen with the maintained driver and inspect exact owner state. Claim while active must remain Busy. Exact-owner protected writes can proceed. Take over from the inspected state, require a larger generation, then reject the old state on a fresh connection.

Check that database delayed durability is DISABLED for this acceptance. Fully durable acknowledgement requires log persistence before response; delayed durability changes this guarantee. Record the effective setting without changing unrelated databases. See [transaction durability](https://learn.microsoft.com/en-us/sql/relational-databases/logs/control-transaction-durability?view=sql-server-ver17).

A graceful same-container restart establishes the tested reopen profile. It does not establish sudden power-loss, disk corruption, failover, or actual lost COMMIT response behavior. Those remain separate profiles.

## Acceptance record

Record source revision, dirty changes, lockfile, compiler, native server version, bounded deadline profile, command and exit result. Record native1205 attribution separately from the public finite outcome. Record pre/post generation equality across restart and strict advancement after takeover. Preserve historical fixtures and logs. Full ROM Storage and incremental Work remain outside this native ownership gate.

## Implemented qualification decisions

The two-session fixture uses HIGH aggressor priority and NORMAL maintained-driver priority.
Native Extended Events match error1205 and the observed victim session. The graph must include the exact C/S key-lock objects.
Foreign-session selectors must return zero. Protected XML evidence is bounded to1MiB and retained before capture stops.
No production diagnostic port or native error disclosure was added.

A requested graceful Docker restart escalated to SIGKILL after30 seconds in one actual run.
[Docker restart semantics](https://docs.docker.com/reference/cli/docker/container/restart/) explain that timeout escalation.
The qualified repeatable profile now uses explicit [Docker kill](https://docs.docker.com/reference/cli/docker/container/kill/) followed by start.
It requires exit137, noOOM, unchanged container/image/data-volume identity and a changed start timestamp.
The consumer closes native clients before kill and retains acknowledged Ownership values in memory.
The profile qualifies SIGKILL after acknowledged persistence, not power loss or an interruption during COMMIT.
The orchestrator acts on the verified container ID. Bounded cleanup starts only that same preserved instance if it was left stopped.
