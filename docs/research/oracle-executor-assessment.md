# Oracle executor assessment

Inspection date: 2026-10-08. Evidence category: source review only.

This assessment proposes an Oracle connection fixture for the shared SQL executor. It does not establish a complete ROM Storage adapter. No client archive or container was downloaded. No Oracle license was accepted. No database or native build was executed.

## Driver candidate

The inspected driver is `oracle 0.6.3`, tag `v0.6.3`, commit `4593547ff4e8803dd9739cbbb67191442e4a7782`. Its manifest declares Rust `1.60.0` and `UPL-1.0/Apache-2.0`. It pins `odpic-sys =0.1.1`, with ODPI-C `5.4.1`. This is the latest version exposed by the inspected documentation. That observation does not establish a maintenance response time or security audit. [Tagged manifest](https://github.com/kubo/rust-oracle/blob/v0.6.3/Cargo.toml), [published documentation](https://docs.rs/oracle/0.6.3/oracle/).

The driver is synchronous and uses native OCI through ODPI-C. Compilation needs a C compiler. Runtime needs Oracle Client 11.2 or later. The tagged README does not require the Instant Client SDK package. Prefer the Basic runtime package for the first fixture. Do not introduce optional `chrono` or unstable AQ features. Native Oracle Net controls transport security; this is not the existing PostgreSQL or MySQL Rust TLS graph. [Tagged README](https://github.com/kubo/rust-oracle/blob/v0.6.3/README.md), [ODPI-C installation](https://odpi-c.readthedocs.io/en/stable/user_guide/installation.html).

The existing `rom-sql-core::Executor<C>` creates, uses, and destroys its connection on one worker. Its connection need not implement `Send`. This fits synchronous OCI without adding a Tokio runtime inside each operation. Keep statements, rows, LOBs, and connection references within that worker. Queue capacity bounds operation count, not captured payload bytes. Initialization still needs a connect deadline. See [executor source](../../crates/rom-sql-core/src/executor.rs) and [approved Task 4](../superpowers/plans/2026-10-08-rom-extras.md).

## Transactions and failure boundaries

Autocommit defaults to false. `commit()` and `rollback()` directly call native operations and return their errors. `Connection::drop` clears an object cache; it does not return rollback evidence. Native handle lifetime also depends on retained references. [Tagged connection implementation](https://github.com/kubo/rust-oracle/blob/v0.6.3/src/connection.rs).

ODPI-C 5.4.1 close logic attempts rollback for a live, owned session with an outstanding transaction. A rollback error marks the session dead. External handles do not receive that ownership cleanup. Thus panic retirement can trigger cleanup, but its result is not a confirmed rollback receipt. Use an ordinary, worker-owned connection and verify panic rollback through a fresh connection. [Exact native close implementation](https://github.com/oracle/odpi/blob/v5.4.1/src/dpiConn.c#L133).

An OCI commit error can leave its outcome unknown. Do not replay a mutation merely because the connection failed. Oracle documents Transaction Guard and LTXID outcome inspection for this problem. This fixture does not yet configure or qualify that mechanism. Reconcile an application operation identifier through a fresh connection before any proposed replay. [Oracle Transaction Guard technical brief](https://www.oracle.com/docs/tech/database/transaction-guard-technical-brief.pdf).

`break_execution()` exists, but invoking it is not commit-outcome evidence. Call timeout applies per round trip. Cleanup can consume another timeout period. The driver distinguishes recoverable cleanup (`DPI-1067`) from an unusable connection (`ORA-3114`). Neither provides a total operation deadline. An executor ticket timeout does not cancel admitted database work. [Driver timeout and break APIs](https://github.com/kubo/rust-oracle/blob/v0.6.3/src/connection.rs#L995), [executor contract](../../crates/rom-sql-core/src/executor.rs).

`ORA-00001` differs from PostgreSQL transaction abortion. Oracle's implicit statement savepoint rolls back the failed duplicate insert; earlier statements can remain pending. Explicitly roll back the complete test transaction after the expected duplicate error. Keep schema DDL outside transaction tests because DDL has implicit commit behavior. [Oracle 26ai transaction control](https://docs.oracle.com/en/database/oracle/oracle-database/26/lnpls/transaction-processing-and-control.html).

Use bound `RAW` values for binary operation identities. Oracle Net does not perform character conversion for RAW. Test embedded zero bytes and distinct byte sequences. Test empty strings separately because Oracle treats zero-length character values as null. Bound integer fixtures should also verify the driver's numeric conversions and Oracle NUMBER precision. These tests are prerequisites, not a complete ROM codec mapping. [Oracle 26ai data types](https://docs.oracle.com/en/database/oracle/oracle-database/26/sqlrf/Data-Types.html).

## Official artifacts and license gates

Oracle currently advertises Database 26ai Free and `container-registry.oracle.com/database/free:latest`. The registry distinguishes Full and Lite images. Prefer Full for the first profile to avoid feature exclusions. The public registry detail could not be reliably read during this inspection. No immutable Linux x86-64 image tag, digest, anonymous pull result, or authentication requirement was verified. Do not substitute an ARM image identifier or a mutable `latest` tag for an executed artifact pin. [Official getting started](https://www.oracle.com/database/free/get-started/), [official registry](https://container-registry.oracle.com/ords/ocr/ba/database/free).

Oracle's Docker source shows listener port `1521`, service `FREEPDB1`, `ORACLE_PWD`, and persistent `/opt/oracle/oradata`. Its sample-built `oracle/database:23.26.0-free` is not an inspected registry digest. The repository separates UPL-licensed scripts from database binaries. It explicitly requires acceptance of the binary download license. A successful anonymous pull would not establish acceptance authority. [Oracle container source and license section](https://github.com/oracle/docker-images/blob/main/OracleDatabase/SingleInstance/README.md).

Current Linux x86-64 Instant Client Basic is `23.26.3.0.0`. Oracle lists archive `instantclient-basic-linux.x64-23.26.3.0.0.zip`, 136,114,689 bytes, SHA-256 `fce485361332927f8328ea4b68d98968c6b0ea124e62470de8c9d0cef880130e`. Requirements include glibc 2.28 or later and the OS `libaio` package. Library discovery requires installation paths or a configured loader search path before process startup. These are published values, not a verified downloaded archive. [Official downloads and installation instructions](https://www.oracle.com/database/technologies/instant-client/linux-x86-64-downloads.html).

Oracle's Instant Client FAQ describes free use and redistribution under its license. The referenced OTN distribution terms impose conditions on unmodified redistribution and acceptance. They also restrict publication of benchmark results. Do not bundle native libraries in ROM-extras on that basis alone. Verify the exact archive's notices and governing terms first. Preserve the client outside the repository unless redistribution is separately approved. [Official FAQ](https://www.oracle.com/database/technologies/faq-instant-client.html), [OTN distribution terms](https://www.oracle.com/downloads/licenses/distribution-license.html).

Nix compatibility is unverified. A normal Nix shell does not establish that a foreign Oracle ELF library can find its loader and dependencies. Inspect the archive's ELF dependencies before choosing a separate FHS environment or other loader arrangement. Record that arrangement in native consumer evidence. Do not patch or redistribute Oracle binaries without checking their actual terms.

No interactive acceptance flow was traversed. If the chosen download requires a click-through or organizational license acceptance, retain that exact pending step. Generic approval of ROM-extras development does not prove that an authorized person accepted those terms.

## Proposed isolated fixture and acceptance

Oracle's Linux Free guide specifies minimum RAM of 1 GB, recommended RAM of 2 GB, and minimum disk space of 10 GB. Its swap recommendation is 2 GB or twice RAM, whichever is less. These are installation requirements, not a verified container limit. Free's ceilings of two CPUs, 2 GB database RAM, and 12 GB user data are separate. [Linux requirements](https://docs.oracle.com/en/database/oracle/oracle-database/26/xeinl/requirements.html), [Free product limits](https://www.oracle.com/database/free/).

Propose two CPUs, a measured container memory allowance above the database RAM requirement, and at least 10 GB available disk plus image overhead. Determine the actual memory limit during qualification; do not describe an arbitrary 2 GB container cap as supported. Bind only `127.0.0.1:55458` to listener `1521`. Keep administration ports unpublished. Keep persistent data and credentials in ignored, private paths. Use a separate fixture user with bounded table privileges. Obtain the precise registry configuration before constructing its launch command.

After artifact and license checks, reuse the public executor in a packaged consumer. Preserve the generic executor and published ROM boundary. Run these real-server scenarios:

1. Connect on the worker. Bind and read RAW identifiers and NUMBER values. Record client and server versions without credentials.
2. Commit two related writes. Reject a duplicate with native code 1. Explicitly roll back and verify earlier pending writes disappear.
3. Panic after an uncommitted insert. Drain retirement and verify absence through a fresh connection.
4. Time out the caller while admitted work continues. Drain the worker and inspect the actual committed outcome.
5. Reject excess queued work with the existing bounded admission contract. Keep fixture payload sizes bounded independently.
6. Shut down the executor. Restart the same database with the same persistent volume. Reconnect and verify acknowledged commits remain.

Actual commit acknowledgement loss needs an independent network fault profile. These scenarios do not establish it. They also do not establish ROM state/event atomicity, stale-revision checks, owner fencing, durable receipts, or complete Storage conformance.

Before adoption, resolve the dependency graph and run license, advisory, and minimum-Rust checks. Include ODPI-C and OCI native components in the audit scope. Rust advisories alone cannot establish native client safety. Record Oracle update/support availability and actual redistribution notices. All these gates remain open.
