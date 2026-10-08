# ROM-extras SQL driver assessment

Date: 2026-10-08. Status: source review only. No dependencies were installed. No database integration tests were executed.

## Inspected candidates

Versions below identify inspected manifests. They are not a claim about the latest release or verified ROM compatibility.

| Provider | Candidate and inspected version | Declared license | Declared `rust-version` | Execution model |
| --- | --- | --- | --- | --- |
| PostgreSQL | `postgres` 0.19.14 | MIT OR Apache-2.0 | 1.85 | Synchronous facade over `tokio-postgres` |
| PostgreSQL/CockroachDB alternative | `tokio-postgres` 0.7.18 | MIT OR Apache-2.0 | 1.85 | Asynchronous |
| Microsoft SQL Server | `tiberius` 0.13.0 | MIT/Apache-2.0 | 1.88 | Asynchronous; runtime independent |
| MySQL/MariaDB | `mysql` 28.0.3 | MIT/Apache-2.0 | Not declared in inspected manifest | Synchronous |
| Oracle | `oracle` 0.6.3 | UPL-1.0/Apache-2.0 | 1.60.0 | Blocking OCI/ODPI-C calls |
| PostgreSQL/MySQL alternative | `sqlx` 0.9.0 | MIT OR Apache-2.0 | 1.94.0 | Asynchronous |

Sources: [postgres manifest](https://github.com/rust-postgres/rust-postgres/blob/master/postgres/Cargo.toml), [tokio-postgres manifest](https://github.com/rust-postgres/rust-postgres/blob/master/tokio-postgres/Cargo.toml), [Tiberius release manifest](https://github.com/prisma/tiberius/blob/v0.13.0/Cargo.toml), [mysql release manifest](https://github.com/blackbeam/rust-mysql-simple/blob/v28.0.3/Cargo.toml), [Oracle release manifest](https://github.com/kubo/rust-oracle/blob/v0.6.3/Cargo.toml), and [SQLx release manifest](https://github.com/launchbadge/sqlx/blob/v0.9.0/Cargo.toml).

The PostgreSQL manifests were read from mutable `master` references. Before pinning, inspect the published archive and record its checksum. ROM currently declares Rust 1.99 in its workspace manifest. That exceeds these declared driver versions, but does not establish transitive dependency compatibility.

## Runtime and TLS implications

`postgres` creates a non-threaded Tokio runtime for each client. TLS is supplied through connector traits. Native TLS and OpenSSL connectors exist separately. A synchronous call from inside an application Tokio runtime therefore needs an explicit execution boundary. See [postgres changelog](https://docs.rs/crate/postgres/0.19.14/source/CHANGELOG.md) and [TLS API](https://docs.rs/postgres/0.19.14/postgres/tls/index.html).

Tiberius uses asynchronous I/O. Tokio streams require a compatibility wrapper for its futures I/O traits. Version 0.13.0 defaults include native TLS and TDS 8.0. Optional Rustls support depends on `tokio-rustls` 0.26.4, `rustls-native-certs` 0.8, and `rustls-webpki` with a 0.103.13 version floor. These requirements do not prove an advisory-free resolved graph. See [Tiberius API](https://docs.rs/tiberius/0.13.0/tiberius/) and [release manifest](https://github.com/prisma/tiberius/blob/v0.13.0/Cargo.toml).

The `mysql` default feature set enables system zlib through `minimal`. It does not select a TLS backend. `minimal-rust` selects Rust compression. TLS choices include native TLS, Rustls with AWS-LC, and Rustls with ring. The provider-free Rustls feature needs a crypto provider. Select compression and TLS deliberately. Review native build requirements for each selected crypto provider. See [mysql release manifest](https://github.com/blackbeam/rust-mysql-simple/blob/v28.0.3/Cargo.toml) and [mysql documentation](https://docs.rs/mysql/28.0.3/mysql/).

MySQL and MariaDB must have separate capability profiles and backend tests. Shared protocol access does not prove identical authentication, locking, isolation, or DDL behavior. This is a design requirement, not an executed compatibility result.

Oracle 0.6.3 requires a C compiler and Oracle Client 11.2 or later. Its manifest pins `odpic-sys` 0.1.1, annotated as ODPI-C 5.4.1. The native client is loaded at runtime. Missing libraries produce DPI-1047. TLS configuration belongs to the native Oracle networking stack, rather than a Rustls feature in this crate. Client packaging and redistribution need their own license review. See [Oracle crate](https://docs.rs/oracle/0.6.3/oracle/), [release manifest](https://github.com/kubo/rust-oracle/blob/v0.6.3/Cargo.toml), and [ODPI-C installation](https://oracle.github.io/odpi/doc/installation.html).

SQLx 0.9.0 supports PostgreSQL, MySQL, and SQLite. Its inspected workspace has no Microsoft SQL Server or Oracle driver. It therefore cannot alone provide the requested six-provider core. See [SQLx release manifest](https://github.com/launchbadge/sqlx/blob/v0.9.0/Cargo.toml).

## CockroachDB ownership and transaction risks

CockroachDB supports PostgreSQL wire protocol version 3.0 and much PostgreSQL syntax. That supports investigating the PostgreSQL driver, but does not establish PostgreSQL storage semantics. See [versioned compatibility source](https://github.com/cockroachdb/docs/blob/main/src/current/v26.2/postgresql-compatibility.md).

Under SERIALIZABLE isolation, default locking reads use unreplicated in-memory locks. Lease transfers and range changes can drop them. Transaction ordering can change, and commits can require a `40001` retry. Documentation calls these locks best-effort. The documented durable-locking setting changes this behavior. SERIALIZABLE guarantees remain separate from those lock guarantees. See [versioned limitations source](https://github.com/cockroachdb/docs/blob/main/src/current/_includes/v26.2/known-limitations/select-for-update-limitations.md).

Do not implement long-lived ROM ownership with a transaction row lock alone. A row lock ends with its transaction. Prefer persisted ownership and a fencing value that every protected mutation validates. This is a proposed architecture consequence, not verified CockroachDB behavior.

PostgreSQL advisory-lock equivalence remains unverified. No authoritative advisory-lock implementation was established in this review. Do not depend on a PostgreSQL ownership lock without a version-specific source review and concurrent backend test.

Current versioned sources support `SKIP LOCKED`, with limitations for multiple column families. An older official blog says it is unsupported. Use the targeted server version and versioned documentation for capability decisions. See [versioned SELECT documentation](https://github.com/cockroachdb/docs/blob/main/src/current/v26.2/select-for-update.md) and [official blog](https://www.cockroachlabs.com/blog/select-for-update/).

## Proposed synchronous boundary

Keep driver runtimes inside the adapter. Keep receipts, revision checks, journal rules, and Work semantics in the shared core.

Use a bounded executor with explicit admission limits. Assign each live transaction to one connection and worker. Avoid one unbounded thread per request. Avoid creating an async runtime for every action.

For Tiberius, investigate a dedicated worker runtime or connection-owning worker. For blocking drivers, use bounded blocking workers. These are proposed approaches. Validate nested-runtime behavior, queue exhaustion, shutdown, and connection loss before choosing one.

Cancellation of a waiting caller does not prove database rollback. Retain transaction ownership until the worker establishes an outcome. Treat a lost commit acknowledgement as an unknown outcome. Reconcile through durable receipts before retrying a mutation.

For CockroachDB, retry the complete transaction only when its outcome is known to permit retry. Preserve action identity and receipt semantics across retries. Test contention and node movement on a real cluster.

## Required evidence before support claims

- Inspect published crate archives and record exact checksums.
- Resolve an explicit feature matrix into a lockfile.
- Audit licenses for the complete resolved graph, selected crypto providers, and Oracle native redistribution.
- Run advisory checks against that lockfile with a recorded database revision and date.
- Compile all feature profiles with the declared ROM toolchain. Record native packages and target architecture.
- Test certificate verification, incorrect hostnames, expired certificates, and private CA configuration.
- Test transactions, conflicting revisions, receipt reconciliation, restart, ownership fencing, and lost commit acknowledgements on every backend.
- Test Oracle client/server combinations actually offered for support.
- Test MySQL and MariaDB separately, including authentication plugins.
- Record unsupported capabilities and failed checks explicitly.

No license approval, clean advisory report, resolved MSRV result, TLS test, or real-backend compatibility result is supplied by this assessment.
