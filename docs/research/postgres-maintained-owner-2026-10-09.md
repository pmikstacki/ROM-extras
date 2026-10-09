# Maintained PostgreSQL ownership driver

Inspection date: 2026-10-09. Source review only. No builds, backend operations, or maintained edits occurred.

## Current evidence and proposed implementation

Inspected `tests/sql-owner-consumer/src/{driver.rs,cases.rs,main.rs}` and its independent Cargo.lock. The native qualification driver uses synchronous postgres::Transaction. It sets connect_timeout=3s, statement_timeout=3s and lock_timeout=1.5s. It directly reads BYTEA values, exposes the inner transaction, and implements lose_ack after successful SDK commit. That last case is synthetic acknowledgement loss, not suppressed wire response evidence.

The consumer lock resolves postgres 0.19.14 and tokio-postgres 0.7.18. The async version is not 0.7.17. Registry checksums are `33ad20e0aa0b24f5a394eab4f78c781d248982b22b25cecc7e3aa46a681605bd` and `a528f7d280f6d5b9cd149635c8705b0dd049754bc67d81d31fa25169a93809d3` respectively.

Implement maintained `rom-postgres` using the existing shared OwnerTransaction. Use worker-owned tokio-postgres with a synchronous bounded facade. Keep setup/provision in fixtures. Move native behavior into the module; downstream tests consume it independently. Full ROM Storage remains blocked by the public incremental Work preparation/accounting ports at ROM d7ef529. Do not copy private policy or serialize the complete ledger.

## Dependency and transport choice

Both inspected package manifests declare Rust 1.85 and MIT OR Apache-2.0. tokio-postgres defaults to the runtime feature, which enables Tokio net/time convenience connection methods. Its TLS implementation is supplied externally. Binary owner tokens and BIGINT generations need no JSON/UUID/chrono integration features. See [postgres 0.19.14 package source](https://docs.rs/crate/postgres/0.19.14/source/Cargo.toml) and [tokio-postgres 0.7.18 package source](https://docs.rs/crate/tokio-postgres/0.7.18/source/Cargo.toml). Manifest inspection was local against the exact locked package; this task did not execute dependency/advisory/native audits.

Synchronous postgres delegates to async tokio-postgres internally. Its private `src/connection.rs::poll_block_on` drives transport and the request on an internal runtime without a whole-operation timeout. The public connect_timeout applies to each socket connection attempt. Multiple addresses and DNS can exceed one such interval. TCP user timeout addresses unacknowledged transmitted bytes, not arbitrary application response waits. See [postgres Config](https://docs.rs/postgres/0.19.14/postgres/config/struct.Config.html) and [exact synchronous connection source](https://docs.rs/crate/postgres/0.19.14/source/src/connection.rs).

An outer ticket wait timeout does not interrupt this synchronous call. Server statement_timeout likewise cannot bound a response blackhole. Keeping synchronous postgres would need a separate bounded transport mechanism; it is less direct than owning the async connection future.

Async tokio-postgres returns Client plus Connection. The Connection future performs actual transport and must be driven. Requests begin when first polled; concurrent polling can pipeline them. Keep this driver strictly sequential. See [tokio-postgres 0.7.18 behavior](https://docs.rs/tokio-postgres/0.7.18/tokio_postgres/).

## Runtime and whole-I/O deadline

Create the runtime, client and retained transport task on the Executor worker. Reject facade calls from an already-entered Tokio runtime, matching the existing MSSQL boundary. A bounded blocking bridge belongs to the host.

Wrap the complete connect future in a connect deadline, including DNS, TLS and authentication. Retain connect_timeout as a secondary per-attempt bound. Wrap each complete operation, including response decoding/draining, in a whole-I/O deadline. Add lock_timeout and statement_timeout as server-side limits with lock shorter than statement, and statement shorter than I/O.

Keep the transport JoinHandle. On I/O timeout or transport uncertainty, mark the connection unavailable, drop all client handles, abort transport, and drive its termination. Do not detach its socket-owning task. Tokio abort is asynchronous until the task is polled; awaiting the handle confirms destruction. A bounded teardown needs an explicit fallback that destroys the owned runtime/transport without leaving live work. See [Tokio JoinHandle abort](https://docs.rs/tokio/latest/tokio/task/struct.JoinHandle.html#method.abort).

Check transport termination before admitting each new transaction. Returning a driver error does not automatically retire the shared Executor. The wrapper must reject all subsequent jobs after native retirement.

This bounds each native operation, not arbitrary host CPU work between calls. A cumulative transaction deadline is an additional option. Document which bound is implemented. Never claim a per-statement deadline bounds a transaction containing unlimited statements.

## Session and transaction mapping

Use explicit READ COMMITTED begin on a clean session. Set owned session safety bounds during connect and transaction-local limits during begin. SET LOCAL ends with the transaction; session SET persists. Include an idle-in-transaction safety limit to avoid indefinitely held locks after host misuse. Do not promise a whole transaction bound from idle timeout. PostgreSQL18 also offers transaction_timeout, but compatibility outside18 requires explicit version handling. See [PostgreSQL18 timeouts](https://www.postgresql.org/docs/18/runtime-config-client.html).

Use fixed qualified control table identifiers, validated before interpolation. Bind every value. Normal open never creates, migrates or resets unknown format. A missing table/row maps to Uninitialized; format mismatch rejects without writes.

Proposed locking projection:

```sql
SELECT format,
       CASE WHEN octet_length(store_id)=32 THEN store_id ELSE NULL END,
       generation,
       CASE WHEN octet_length(token)=32 THEN token ELSE NULL END,
       octet_length(token)
FROM fixed_schema.fixed_table
WHERE singleton
LIMIT 2 FOR UPDATE;
```

Use a compatible BIGINT cast for length when required by the decoder. Require exactly one row and fixed column count. Validate types with try_get, never panic on malformed schema. A malformed token length must not become idle because CASE produced NULL. Keep separate nullable length evidence.

This bounds returned identities before application decoding. It does not guarantee bounded backend work for a hostile database/query plan. Require the extras-owned singleton schema and primary key. Query byte allocation must remain bounded before collecting arbitrary results.

FOR UPDATE holds the row until transaction end. A waiting READ COMMITTED locking read returns the updated row. Every protected write and takeover must lock this same row before checking the fence. Do not use SKIP LOCKED or an unprotected preliminary SELECT. See [PostgreSQL18 row locks](https://www.postgresql.org/docs/18/explicit-locking.html).

write_owner requires the retained lock, supported format and exact immutable store identity. Bind generation as i64 and token as BYTEA. Require exactly one affected row. Validate shared OwnerState/OwnerToken constraints. Poison the transaction on any prepared operation error, including caught errors. Never expose the raw Client or Transaction for host bypass.

Prepared execute follows the maintained MSSQL seam: bounded fixed SQL, bounded parameter count/bytes and driver-only persistence. It prohibits transaction/session changes, DDL, external effects and application policy evaluation under locks.

## Commit, rollback and Drop

Consume the transaction at commit. Report success only after the complete native operation succeeds. Commit timeout, socket failure or unresolved response maps to Unknown and retires the client/transport. A cancellation request does not reverse an earlier commit. Do not automatically retry COMMIT.

tokio-postgres Transaction Drop queues an implicit rollback through the client. Explicit rollback returns its errors to the caller. A queued Drop rollback is not synchronous confirmation that PostgreSQL completed it. See [exact transaction source](https://docs.rs/crate/tokio-postgres/0.7.18/source/src/transaction.rs).

The maintained synchronous facade must attempt bounded explicit rollback on Drop while healthy. Unknown cleanup retires transport. Shared finish only preserves Busy/Stale/Invalid after confirmed rollback. Do not claim Drop establishes a result to the original caller.

PostgreSQL SQLSTATE errors such as lock timeout, undefined table and transaction-aborted state can be classified privately. They must not leak native messages or credentials. An authoritative SQL error plus completed rollback differs from transport uncertainty. A fresh connection can inspect exact durable state after Unknown; absence while the old request can still finish is not proof of non-commit.

## TLS host boundary

Prefer generic host-supplied MakeTlsConnect over a hardwired TLS backend. Its resulting stream/connection future must satisfy the bounds needed for the retained task. Explicit generic bounds are compile-tested through the public constructor, not left as documentation suggestions.

Do not override host TLS settings, trust roots, SNI, hostname checks, host/hostaddr pairing, client certificates or SSL mode. The SDK connects arbitrary streams and supplies external TLS integration. NoTls is a deliberate loopback fixture choice, not a default for a production network profile. See [TLS extension boundary](https://docs.rs/tokio-postgres/0.7.18/tokio_postgres/#ssltls-support) and [versioned Config](https://docs.rs/tokio-postgres/0.7.18/tokio_postgres/config/struct.Config.html).

A generic seam avoids immediately adopting a new TLS connector graph. It does not prove a secure real TLS profile. Native-tls/OpenSSL connector adoption later needs exact version, MSRV/license/advisory/native-build assessment and positive/negative certificate tests. A supplied already-connected Client alone is insufficient unless the facade also owns its transport lifecycle and deadlines.

## Minimum credible conformance

Preserve current consumer cases: idle claim race, ordered takeover, surviving stale connection, lock timeout, prepared write rollback, wrong identity/format and generation exhaustion. Re-run them against maintained public module methods.

Add malformed oversized native BYTEA with CASE bounding; caught prepared error cannot commit; bounded I/O blackhole; retirement prevents later writes; cleanup on Drop; drained executor shutdown; same-disk native restart; host TLS constructor compile fixture. Synthetic lose_ack remains explicitly synthetic until real response-loss proxy evidence exists.

An independent archive consumer must build against packaged extras using only public APIs and exact public ROMd7ef dependency. Keep NoTls/TLS graph features separate and compile without workspace feature unification. Archive smoke proves packaging, not real backend behavior.

Native ownership qualification is narrower than full Storage, Work controls, journal ordering or native TLS qualification. Record executed source, lockfile, compiler, command and result when root runs those gates. This report supplies source recommendations only.

## Selected implementation and portability

The maintained constructor forces SslMode::Require instead of preserving a downgrade setting.
This is an explicit safety rule; the host still owns its TLS connector and certificate policy.
The separate literal-loopback constructor is the only NoTls route. Both constructors reject multiple hosts/ports and hostaddr overrides.
Normal TLS construction also rejects Unix sockets and empty TCP hosts. It does not qualify production trust chains.

Portable control identifiers and positive connect/I/O validation are shared with MSSQL through rom-sql-core.
Identifiers use at most 63 ASCII bytes, excluding quote delimiters; each driver retains its own quoting and native statement/lock bounds.
PostgreSQL's standard build uses a63-byte limit and case-sensitive quoted names.
See [PostgreSQL18 identifiers](https://www.postgresql.org/docs/18/sql-syntax-lexical.html#SQL-SYNTAX-IDENTIFIERS).
SQL Server supports delimited identifiers; the preexisting conservative63-byte policy remains unchanged.
See [SQL Server identifiers](https://learn.microsoft.com/en-us/sql/relational-databases/databases/database-identifiers?view=sql-server-ver17).

The retained current-thread transport task is aborted and driven to destruction after uncertainty.
A native response-loss consumer requires its relay to observe client closure before the relay performs cleanup.
Removing the task await caused this real native qualification to fail; the restored driver passed.
The explicit async Drop path cancels the owned runtime without blocking inside another Tokio runtime.

The native profile is PostgreSQL 18.6, explicit plaintext loopback 55439, fsync=on and synchronous_commit=on.
A prepared write completes before an acknowledged response-suppression barrier; unchanged native COMMIT follows.
A fresh committed read observes value17 before the caller's whole-I/O timeout returns Unknown.
The relay never records credential or request bytes. Its sanitized counters and peer-closure observation remain separate from native state proof.
This is actual protected-write response loss, unlike the retained historical post-SDK synthetic acknowledgement fault.

A raw connection-startup blackhole bounds the constructor at100 ms without authentication bytes or a fake success response.
Native statement cancellation and lock timeout instead produce confirmed rollback and permit later explicit use.
Connect/query timeouts require cooperative SDK/TLS futures. The host must supply a nonblocking TLS connector and bound its own CPU work.
A hostname can involve SDK DNS/address resolution; the executed profile uses a literal loopback IP.
No cumulative deadline for arbitrary host callback work, full Storage, owner-acquisition response loss or production TLS qualification is claimed.
