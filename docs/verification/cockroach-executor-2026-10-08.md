# CockroachDB shared executor verification

Date: 2026-10-08. ROM-extras baseline 15190e5bcaaff54ab96e66bc9509a42f171296bf plus recorded acceptance changes.
Public ROM revision remains d7ef529040eec60dc869034c2d33130219db85fe; rom-sql-core production sources are unchanged.
The executed [official image and binary](cockroach-fixture-2026-10-08.json) identify the actual local backend.
This is executor acceptance, not complete CockroachDB ROM Storage support.

## Shared boundary and actual scenarios

PostgreSQL and CockroachDB use the same connection-owning Executor and synchronous postgres driver.
The existing PostgreSQL deadline and panic assertions were extracted into tests/common/pgwire, with its original test names and settings preserved.
The common NoTls helper now rejects hostaddr overrides. Cockroach additionally requires the exact fixed fixture endpoint and database.
Both lockfiles are byte-identical to the baseline. The new consumer feature adds no dependency version or resolved package.
The prior 440-package consumer audit remains an audit of those unchanged dependency identities; it is not a new server vulnerability assessment.

| Scenario | Actual acceptance |
| --- | --- |
| Current-thread Tokio caller | Worker-owned synchronous client returns typed BIGINT 17; exact backend version and SERIALIZABLE session |
| Caller deadline | Zero wait returns Unknown; admitted transaction later confirms commit; fresh connection retrieves 41 |
| Worker panic | Connection retires; fresh connection sees no uncommitted 9 row |
| Duplicate 23505 | Explicit rollback removes preceding insert; fresh transaction stores false |
| Controlled serialization rejection | Two independent workers establish stale read 0; actual 40001 is required; explicit rollback then a complete fresh transaction reads 1 and commits 2 |
| Binary identity | A,a,precomposed/decomposed Unicode and one/two zero-byte keys remain six distinct BYTEA primary keys with false values |
| Restart | Confirmed false row survives graceful exited 0 stop and restart of the same disk store |

The controlled retry case performs exactly one explicit retry after actual 40001; it is not a production retry-policy implementation.
The test fails if no serialization rejection occurs, the fresh read is stale, or the final value differs.
It does not replay external effects or classify40003 as retryable.

## Executed commands and retained failures

The initial five cases passed against the live server. [Initial log](cockroach-initial-2026-10-08.log).
The first seven-case run confirmed restart retention but Docker forced SIGKILL after 20 seconds.
The next cold DDL hit SQLSTATE 57014 under its then-three-second statement timeout; five remaining cases failed on poisoned fixture mutex.
All six failures remain in [the failed seven-case log](cockroach-seven-cases-2026-10-08.log).

Process inspection showed Cockroach was already PID1; no entrypoint correction was made.
The revised restart requires stop completion with exited 0, with a finite 60 second grace and 70 second outer stop command limit.
Cockroach-only statement timeout became 10 seconds for this local startup/DDL profile; PostgreSQL retains 3 seconds.
The revised seven-case gate and warnings-denied Clippy exited 0 in [the graceful gate log](cockroach-graceful-gate-2026-10-08.log).
The extracted PostgreSQL three-case gate and Clippy also exited 0 in [its affected gate](cockroach-postgres-shared-2026-10-08.log).

Independent review identified a DSN mismatch hazard and a subsequent hostaddr bypass.
Fixed-port and hostaddr negative scenarios compile and fail 101 before provider access for their intended validation reason.
See [wrong-port rejection](cockroach-wrong-port-2026-10-08.log) and [hostaddr rejection](cockroach-hostaddr-rejected-2026-10-08.log).
The review confirmed no remaining actionable findings by source inspection; it did not rerun tests.

An initial full verifier was explicitly terminated 143 when the hostaddr finding arrived.
Its [interrupted log](cockroach-full-verifier-interrupted-2026-10-08.log) and source freeze are preserved; it is not a successful verifier.
The next complete attempt exited 101 because the terminated restart had left the fixture stopped with exit0.
Its [stopped-fixture failure log](cockroach-full-verifier-stopped-fixture-2026-10-08.log) is retained.
The same container/store was started and [SQL readiness confirmed](cockroach-restored-ready-confirmed-2026-10-08.log) before another attempt.
The replacement complete `./scripts/check-all` verifier exited 0 on the configured SQL, messaging, Azure and RustFS fixtures.
All 116 final frozen runtime inputs had identical paths and SHA-256 hashes after the run.
The PostgreSQL gate passed three cases; the Cockroach gate passed seven cases, no failures or ignored tests, with warnings-denied Clippy.
The full final S3 gate also passed seven cases.
See [unmodified full log](cockroach-full-verifier-2026-10-08.log) and [source-input hashes](cockroach-source-inputs-2026-10-08.json).

## Limits

The required gate is now part of check-all and does not skip missing Cockroach configuration.
The insecure single-node development profile has no authentication, TLS, replication, quorum, failover or production sizing claim.
Actual 40003 response loss, cancellation race, crash/power-loss durability and replicated ownership fencing remain unverified.
Complete ROM Storage state/event/receipt atomicity, incremental work integration, end-to-end I/O bounds, migrations and packaged release remain pending.
The complete ROM-extras goal remains active.

## Later storage-worker verification adjustment

The storage-worker full verifier exposed a graceful-stop failure at the existing 60-second grace limit.
The stopped container had exit 137 without OOM; native logs still showed SQL and descriptor-lease draining.
The current restart fixture permits 300 seconds of grace and 315 seconds for the command, following vendor termination guidance.
It still requires exited 0 and verifies the same persistent disk store. This is not a driver request deadline change.
The storage-worker verification record retains both attempts and the exact revised source fence.
