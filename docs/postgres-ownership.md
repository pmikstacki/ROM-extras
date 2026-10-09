# Maintained PostgreSQL ownership

`rom-postgres` implements `rom-sql-core::OwnerTransaction` through bounded native PostgreSQL operations.
The shared core owns claim, takeover, release and stale-owner arbitration.
This is not ROM Storage, Resource bundles, journal publication or incremental Work.
The public ROM pin remains d7ef529040eec60dc869034c2d33130219db85fe; required incremental Work ports remain unpublished.

## Host configuration

Create a `Connection` on a `rom_sql_core::Executor` worker.
Supply native `Config`, validated `Deadlines`, a host TLS connector and an exact `ControlTable` identity.
`Connection::connect` forces TLS Require. The host controls certificate identity, trust, client certificates and SDK tracing.
Only one explicit TCP host is admitted; multiple ports, hostaddr overrides, Unix sockets and empty hosts are rejected.
`connect_loopback` is a separate explicit plaintext route for one literal loopback IP.
There is no default remote endpoint, automatic provision, retry, reconnect or ownership release.
The native plaintext fixture does not establish a production TLS profile.

`Deadlines::new(connect, io, statement, lock)` requires positive connect/I/O limits through 60 seconds.
Native lock and statement limits use whole milliseconds: lock < statement < I/O.
Each admitted transaction uses READ COMMITTED and local statement, lock, idle-in-transaction and synchronous-commit settings.
The idle limit equals the statement limit. Native timeouts are distinct from the outer whole-operation I/O deadline.
The host bounds the complete sequence of statements, parameter sizes and CPU work between operations.
The executor's waiting deadline does not cancel admitted native work.
SDK and host TLS futures must remain cooperative; no cumulative callback deadline is claimed.

Calls from an active Tokio runtime fail before blocking.
Transport uncertainty retires the client, aborts its owned task and drives task destruction outside an active host runtime.
A retired handle rejects further native use. A native commit can still have succeeded before response loss.
Drop can run after async host handoff without dropping a blocking runtime inside that host runtime.
Public errors and adapter Debug contain no SQL, native messages, endpoints or credentials.
The host must prevent native SDK tracing from disclosing SQL or payloads.

Provision a logged singleton table separately:

```sql
CREATE TABLE public.rom_extras_owner (
    singleton_key INT NOT NULL PRIMARY KEY CHECK (singleton_key = 1),
    format_version INT NOT NULL CHECK (format_version = 1),
    store_identity BYTEA NOT NULL CHECK (octet_length(store_identity) = 32),
    generation BIGINT NOT NULL CHECK (generation >= 0),
    owner_token BYTEA NULL,
    CHECK (owner_token IS NULL OR
        (octet_length(owner_token) = 32 AND owner_token <> decode(repeat('00', 32), 'hex'))),
    CHECK (generation > 0 OR owner_token IS NULL)
);
-- Bind the host's exact store identity as $1 during explicit provisioning:
INSERT INTO public.rom_extras_owner VALUES (1, 1, $1, 0, NULL);
```

Do not add triggers, replace tables or change the transaction/session inside trusted driver persistence.
The portable schema/table validator admits 1..=63 ASCII letters, digits or underscores; the first byte is a letter or underscore.
PostgreSQL identifiers are quoted without changing case. MSSQL retains its own quoting through the same validator.
The exact 32-byte identity is checked under FOR UPDATE before protected writes.
The query returns at most two rows and guards BYTEA materialization with octet_length/CASE.
Missing provisioning, unknown format, malformed metadata and exhausted generation reject without repair.
Ownership generations use i64 range and remain separate from full-u64 Resource revisions and exact Resource identifiers.

## Protected persistence and recovery

Use the shared public `claim_owner`, `takeover_owner`, `release_owner` and `with_owner` operations.
`Transaction::execute` belongs to trusted persistence drivers, never Resource action authors.
Fixed SQL is bounded to 16KiB and 64 parameters. The driver must bound parameter bytes and native work.
Do not run DDL, change session/transaction state, emit rowsets or invoke external effects.
Prepare canonical application data before taking locks when ROM's public preparation port becomes available.

A caught execution error poisons the transaction. It cannot re-lock, write or commit staged data.
Drop attempts bounded complete rollback. Unconfirmed commit/rollback returns Unknown and retires the native client.
A successful rollback consumes the native terminal response before another operation is admitted.
Do not interpret Unknown or a closed connection as proof that COMMIT failed.
Inspect authoritative state through a fresh connection before a separately authorized takeover or replay decision.
A state inspection does not mint acknowledged Ownership. The adapter never retries a protected write automatically.
The host supplies fresh unpredictable owner tokens; constant tokens belong only to qualification fixtures.

## Executed qualification

Configure the protected native PostgreSQL fixture, then run:

```sh
./scripts/check-postgres-owner
```

The gate is required by `check-all`; missing configuration is an error.
The [independent consumer](../tests/postgres-owner-consumer/src/main.rs) uses only maintained public APIs and separate fixture provisioning.
The normalized Cargo archives execute the same native consumer with an independent locked graph.
Both preserve uniquely named logged tables and private process logs; the owned native process has a 40-second deadline.

The native profile is PostgreSQL 18.6, literal loopback 55439 and explicit NoTls, with fsync/synchronous_commit enabled.
Tests check competing claims, exact release/reclaim and stale surviving connections.
A unique control-table query must show an actual native lock wait before ordered takeover proceeds.
Statement errors, native statement cancellation, lock timeout and Drop roll back staged writes.
Caught errors and rejected SQL/parameter limits cannot commit a prepared prefix.
Independently malformed metadata covers short/oversized binary values, zero tokens, negative/exhausted generations, identity and format.
Missing and duplicate control rows are distinct rejected profiles.

The real response-loss profile uses an opaque one-client relay and an acknowledged delivery barrier before COMMIT.
It discards every later server byte. A fresh committed read must observe value17 before the caller returns Unknown.
The relay must observe native client closure before its own cleanup; the driver handle remains alive during this observation.
Removing the transport-task await caused this qualification to fail. The restored driver passed.
This does not claim parsed success tokens, lost owner-acquisition acknowledgement or durable receipt replay.

A controlled startup blackhole confirms the connect deadline; it is not a real successful database service.
TLS Require with NoTls, incorrect native authentication, multiple endpoints and remote plaintext admission are rejected.
Active-runtime construction fails and async handoff Drop is exercised.
Production certificate chains, same-volume server restart, power loss, deadlock victims and distributed profiles remain unqualified.
The existing [historical SQL-core consumer](sql-ownership.md) and its synthetic acknowledgement evidence remain preserved.
See [primary-source decisions](research/postgres-maintained-owner-2026-10-09.md) and [verification status](verification/postgres-maintained-owner-2026-10-09.json).

The affected PostgreSQL and MSSQL gates passed for local sources and normalized Cargo archives.
The full local verifier passed on 2026-10-09. All 1319 frozen source files retained identical membership and SHA-256 hashes.
Only this result summary, the verification record and plan status changed after the verifier.
