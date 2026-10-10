# Maintained MySQL and MariaDB ownership

`rom-mysql` implements the shared `rom-sql-core::OwnerTransaction` with one protocol driver and explicit vendor profiles.
The shared core owns claim, takeover, release and stale-owner arbitration.
This is not full ROM Storage, journal publication or incremental Work.
The public ROM pin remains d7ef529040eec60dc869034c2d33130219db85fe; required incremental Work ports remain unpublished.

## Host configuration

Construct `Config::new(profile, host, port, user, password, database)` with host-owned credentials and an explicit endpoint.
Select `Profile::MySql` or `Profile::MariaDb`. The driver verifies the native vendor and durable InnoDB flush setting.
`Connection::connect` requires verified `SslOpts`. Invalid certificate acceptance, skipped domain validation and hostname overrides are rejected.
Host trust roots and client identity remain explicit. The SDK refuses a TLS-required connection when the server lacks TLS capability.
`connect_loopback` is a separate plaintext fixture route for one literal loopback IP.
The current native evidence does not qualify production TLS.

`Deadlines::new(connect, io, lock)` admits positive connect/I/O limits through60 seconds.
The lock limit uses whole seconds and must be shorter than I/O.
Each native transaction uses READ COMMITTED and the selected InnoDB lock timeout.
One absolute I/O deadline covers each complete operation, including response parsing and draining.
The host must bound total statements and CPU work between calls; no cumulative transaction deadline is claimed.

A private capacity1 SQL Executor owns the unpooled native client and its runtime.
Blocking operations refuse active Tokio callers. Use a host-owned bounded blocking bridge.
Connection Drop joins its worker, including after async host handoff.
Uncertain operations retire the native connection and owned runtime; subsequent operations fail.
SDK Drop runs outside an active Tokio context to prevent detached socket cleanup.
There is no automatic reconnect, retry, provision, ownership release or default remote endpoint.
Public errors and adapter Debug contain no credentials, endpoint, SQL or native error messages.
The host must keep SDK tracing and native query logs private.

## Provision and protect persistence

Provision an indexed InnoDB singleton separately:

```sql
CREATE TABLE rom_extras_owner (
    singleton_key INT NOT NULL PRIMARY KEY CHECK(singleton_key=1),
    format_version INT NOT NULL CHECK(format_version=1),
    store_identity BINARY(32) NOT NULL,
    generation BIGINT NOT NULL CHECK(generation>=0),
    owner_token VARBINARY(32) NULL,
    CHECK(owner_token IS NULL OR OCTET_LENGTH(owner_token)=32),
    CHECK(generation>0 OR owner_token IS NULL)
) ENGINE=InnoDB;
-- Bind the exact host store identity during explicit provisioning:
INSERT INTO rom_extras_owner VALUES(1,1,?,0,NULL);
```

Supply `ControlTable::new(database, table, exact_identity)`.
Portable names use1..=63 ASCII letters, digits or underscores; the first byte is a letter or underscore.
The adapter quotes native identifiers and preserves identity bytes.
FOR UPDATE returns at most2 rows; CASE/length projections prevent oversized identity/token materialization.
The driver checks format, exact identity, nonnegative integer generation, token invariants and InnoDB before writes.
Malformed, missing, duplicate, unsupported or exhausted state rejects without repair.
Do not replace the schema or add triggers during application use.
All protected tables and statements must preserve the same native transaction and InnoDB guarantees.

Use `claim_owner`, `takeover_owner`, `release_owner` and `with_owner` from the shared core.
`Transaction::execute(sql, Vec<Value>)` is trusted persistence, not an application action API.
Limits are16KiB fixed SQL,64 parameters and60KiB parameter budget including scalar/prefix reserves.
The native packet cap is64KiB; the prepared statement cache is32.
Never supply DDL, session/transaction control, rowsets, infile operations or external effects.
Unexpected rowsets cause Unknown and retirement. Every caught statement error poisons the transaction.
Reuse is permitted only after acknowledged complete rollback.
A commit failure is Unknown; it can follow a successful native commit. Never automatically retry that write.
Ownership generations use the i64 range and remain separate from full-u64 Resource revisions and exact Resource identifiers.

## Qualification and limits

Run `./scripts/check-mysql-owner` with both explicit private loopback DSNs and the configured Python executable.
The independent consumer is [tests/mysql-owner-consumer](../tests/mysql-owner-consumer/src/main.rs).
The gate also consumes normalized Cargo archives without workspace feature unification.
The runner checks exact retained containers, images, persistent volumes and ports55452/55453.
A separate private root observer reads lock diagnostics; application privileges remain unchanged.

MySQL8.4.11 and MariaDB11.8.9 passed independently.
Cases cover exact transitions, surviving stale clients, competing claims, observed lock ordering and native lock timeout.
They also cover prefix rollback, caught-error poison, Drop, malformed metadata, engine rejection and input limits.
Real COMMIT acknowledgement loss and byte-paced responses both produce Unknown within the absolute I/O limit.
Independent native reads confirm commit before Unknown; relay observations confirm peer closure before cleanup.
Constructor blackhole, unexpected rowset and async-handoff Drop cover further transport boundaries.
These are controlled native protocol qualifications, not mock server success.

Production TLS, native maintained restart recovery, deadlock victims and uncertain owner-acquisition acknowledgements remain unqualified.
Malicious infile injection and additional packet/result fault profiles remain unqualified.
The SDK advertises LOCAL_FILES but has no configured handler; this is refusal, not disabled protocol capability.
Historical executor/restart evidence remains separate and unchanged.
See [the source research](research/mysql-maintained-owner-2026-10-09.md) and [verification record](verification/mysql-maintained-owner-2026-10-09.json).

The full local verifier passed on1344 unchanged maintained source files. The initial shared-helper Clippy failure and correction remain in evidence.

## Native process recovery increment

Both retained MySQL8.4.11 and MariaDB11.8.9 fixtures now pass direct and normalized archive recovery consumers.
Each consumer retains the actual acknowledged generation2 guards in memory while the native process is killed.
An acknowledged-only profile closes clients before SIGKILL. A separate incomplete profile keeps an independently witnessed uncommitted write open.
The runner verifies exact container, image, data volume, process executable, exit137/noOOM and changed start time.
Native Uptime must reset; a no-op restart signal fails this witness.
After startup, exact generation2/data17 survive and the uncommitted probe remains0.
Old guards fail, current guards write42, and release/reclaim advances to generation3, observed through a fresh connection.
Writes and owner transitions are never retried. Fixture connection readiness polls alone are bounded and explicit.

The source-only final review found no Critical/Important issue in this isolated fixture profile.
Two Minor suggestions remain: use immutable container IDs for lifecycle commands, and independently assert exact native version literals.
Current checks pin image/container identity and compare observed versions across recovery.
The root and both consumer dependency audits found zero known vulnerabilities; the existing root paste warning remains unsuppressed.
Source/archive152-package registry graphs match, with no added dependencies.
The full local verifier passed on 1430 unchanged frozen source files.
Its first attempt stopped at an OpenSearch fixture capacity block; the retained second attempt passed after a documented fixture-only change.
See [the executed verification record](verification/mysql-native-recovery-2026-10-10.json).
This profile does not prove OS/storage power loss, torn writes, production TLS, deadlock handling or full ROM Storage.
See [primary research and limits](research/mysql-native-recovery-2026-10-10.md).
