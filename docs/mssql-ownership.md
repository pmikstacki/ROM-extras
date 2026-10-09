# Native SQL Server ownership

`rom-mssql` implements the existing `rom-sql-core::OwnerTransaction` contract on an encrypted TDS connection.
The shared SQL core owns claim, takeover, release and stale-owner arbitration.
This increment does not implement ROM Storage, Resource bundles, journal publication or incremental Work.
The public ROM pin still lacks the required incremental Work ports.

## Configure the host

Create the connection on a `rom_sql_core::Executor` worker.
Supply a Tiberius `Config`, explicit `Deadlines`, and a `ControlTable` with the exact 32-byte store identity.
The adapter forces encryption. Certificate validation and endpoint selection belong to the host.
It never enables `trust_cert`, creates tables, retries operations or reconnects automatically.
The independent fixture explicitly trusts a synthetic certificate on fixed loopback; this does not qualify a production certificate chain.

`Deadlines::new(connect, io, lock)` admits positive connect and I/O deadlines up to 60 seconds.
The lock deadline must use whole milliseconds and be shorter than I/O.
Each native operation, including response consumption, has the whole I/O deadline.
The host bounds the complete sequence of protected statements. The executor's waiting timeout does not cancel accepted work.
Calls from an active Tokio runtime return `Invalid` before blocking.
Drop closes the native client and safely retires its runtime, including a handle returned to an async host.

The host must provision a logged singleton control table separately:

```sql
CREATE TABLE dbo.rom_extras_owner (
    singleton_key INT NOT NULL PRIMARY KEY CHECK (singleton_key = 1),
    format_version INT NOT NULL CHECK (format_version = 1),
    store_identity VARBINARY(32) NOT NULL CHECK (DATALENGTH(store_identity) = 32),
    generation BIGINT NOT NULL CHECK (generation >= 0),
    owner_token VARBINARY(32) NULL,
    CHECK (owner_token IS NULL OR
        (DATALENGTH(owner_token) = 32 AND owner_token <>
         0x0000000000000000000000000000000000000000000000000000000000000000)),
    CHECK (generation > 0 OR owner_token IS NULL)
);
-- Bind the host's exact store identity as @P1 during explicit provisioning:
INSERT dbo.rom_extras_owner VALUES (1, 1, @P1, 0, NULL);
```

Do not add triggers to this table. Unknown formats require a separately reviewed migration.
Identifiers accept at most 63 ASCII letters, digits or underscores; the first byte must be a letter or underscore.
The adapter checks exact format, identity, generation and token on every locking read.
Binary length guards prevent oversized control values from entering result buffers.
A native generation is limited to i64::MAX. It is not a Resource revision and never changes a Resource identifier.

## Use the shared core

The [independent consumer](../tests/mssql-owner-consumer/src/main.rs) is an executable integration example.
Its [fixture](../tests/mssql-owner-consumer/src/fixture.rs) separates explicit schema provisioning from maintained adapter operations.
Its [cases](../tests/mssql-owner-consumer/src/cases.rs) call `claim_owner`, `takeover_owner`, `release_owner` and `with_owner` through public API.

`Connection::begin` sets persistent session bounds through SQLBatch, then opens native READ COMMITTED.
`lock_owner` uses `UPDLOCK,HOLDLOCK`. Every protected transaction retains that lock until its native end.
A competing takeover waits for an already locked old transaction. A surviving old connection is then rejected by its exact fence.
Takeover requires the host's explicit approval and exact observed state; the driver does not infer owner death.

Inside `with_owner`, trusted persistence drivers can use `Transaction::execute` with fixed SQL and bound parameters.
SQL text is limited to 16 KiB and parameters to 64. The trusted driver must bound parameter sizes and statement work.
Do not expose this extension to Resource action authors. Do not change transaction/session state, invoke external effects or emit rowsets.
A native execution error prevents subsequent writes, locking and commit even when the driver catches the error.

Rollback uses SQLBatch and confirms both `XACT_STATE()=0` and `@@TRANCOUNT=0` after consuming the complete response.
This includes a transaction the server already aborted. A statement error rolls back previously prepared writes.
Session settings and cleanup must not use `sp_executesql`: its scope can restore settings and reject changed transaction counts.
Commit errors return `Unknown`. An I/O timeout or incomplete response permanently retires the client.
Do not interpret a closed client as proof that a preceding COMMIT failed.
Inspect authoritative state through a fresh connection before a host-authorized recovery decision.

The adapter returns only finite `OwnerError` values. Connection, transaction and control-table Debug output is redacted.
It does not install SDK tracing subscribers. The host must prevent native SDK SQL, payload and server-error tracing from reaching public logs.
Native release is explicit. Closing the worker does not clear the persisted owner or integrate with ROM's opaque StorageOwner lifetime.

## Run qualification

Configure the retained local SQL Server fixture and protected password, then run:

```sh
./scripts/check-mssql-owner
```

The gate requires native SQL Server and never skips missing credentials.
Seven case groups run through direct public imports and normalized `rom-mssql`/`rom-sql-core` Cargo archives.
Each owned consumer process has a 40-second deadline. Unique tables and process logs remain available as evidence.
The fixture uses SQL Server 2025 17.0.5005.3, encrypted TDS and `delayed_durability=0`.

Cases cover competing claims, stale surviving connections, explicit release/reclaim and observed native lock ordering.
They verify rollback after a native error and the rejection of an attempted re-lock after a caught error.
Lock timeout is distinct from outer I/O timeout; confirmed cleanup permits a later explicit operation.
Malformed metadata includes short, oversized and zero binary tokens, identity mismatch, unsupported format and generation exhaustion.
Both VARBINARY(32) and malformed VARBINARY(MAX) fixture shapes are tested.
Drop rolls back prepared writes. I/O timeout retires the native client and a fresh read observes unchanged data.
A caller waiting timeout permits an admitted native transaction to commit later.
Active Tokio invocation and cross-context Drop are also exercised.

Deadlock victim recovery, actual lost COMMIT wire responses and SQL Server restart remain unqualified in this ownership increment.
Earlier SQL executor restart evidence remains separate. Production trust chains, distributed profiles and full ROM Storage also remain open.
See [official-source research](research/mssql-ownership-2026-10-09.md) and [the implementation plan](superpowers/plans/2026-10-09-mssql-ownership.md).

Affected native and packaged checks passed. The full local verifier passed on 1,289 unchanged source files.
See [the verification record](verification/mssql-ownership-2026-10-09.json) for logs, preserved failed attempts and isolated OpenSearch fixture evidence.
