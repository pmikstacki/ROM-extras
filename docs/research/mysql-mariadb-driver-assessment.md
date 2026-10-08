# MySQL and MariaDB connection driver assessment

Date: 2026-10-08. Evidence: primary-source and local-code inspection only. No dependencies were installed. No database tests, advisory audit, or image pull was executed by this research agent.

## Inspected candidates

| Driver | Inspected version | License declaration | Declared `rust-version` | Boundary fit |
| --- | --- | --- | --- | --- |
| `mysql` | 28.0.3 release tag | MIT/Apache-2.0 | Absent | Synchronous connection operations fit the existing worker directly |
| `mysql_async` | 0.37.1 release tag | MIT OR Apache-2.0 | Absent | Requires a Tokio runtime owned by the worker |
| `sqlx` | 0.9.0 release tag | MIT OR Apache-2.0 | 1.94.0 | Requires an async runtime; broader abstraction and feature graph |

Sources: [mysql manifest](https://github.com/blackbeam/rust-mysql-simple/blob/v28.0.3/Cargo.toml), [mysql_async manifest](https://github.com/blackbeam/mysql_async/blob/v0.37.1/Cargo.toml), and [SQLx workspace manifest](https://github.com/launchbadge/sqlx/blob/v0.9.0/Cargo.toml). An absent declaration does not establish compatibility with an old Rust compiler. Validate the complete locked dependency graph against ROM's declared toolchain.

`mysql` defaults enable system zlib, derive support, and the buffer pool. They do not select a TLS backend. `minimal-rust` enables Rust compression. TLS options include native TLS, Rustls with AWS-LC, and Rustls with ring. Bare Rustls requires a crypto provider. See [mysql feature definitions](https://github.com/blackbeam/rust-mysql-simple/blob/v28.0.3/Cargo.toml).

`mysql_async` defaults enable system zlib and derives, with no TLS feature. Its Rustls presets select AWS-LC or ring. Its provider-free Rustls feature requires a provider. Native TLS uses `native-tls` and `tokio-native-tls`. See [async feature definitions](https://github.com/blackbeam/mysql_async/blob/v0.37.1/Cargo.toml).

## Existing executor constraints

The local connection boundary is `rom_sql_core::Executor<C>`, not a type named `ConnectionExecutor`. It creates and destroys `C` on one dedicated worker. `C` need not implement `Send`. Submitted closures and results cross threads. The bounded queue counts queued operations, not captured bytes. See [executor source](../../crates/rom-sql-core/src/executor.rs).

A ticket timeout returns `Unknown` while admitted work continues. Dropping a ticket does not cancel a database operation. Shutdown drains admitted work and waits for connection destruction. Drivers must provide connect and operation deadlines. See [ticket source](../../crates/rom-sql-core/src/ticket.rs) and [executor source](../../crates/rom-sql-core/src/executor.rs).

Recommendation: use one worker-owned synchronous `mysql::Conn` for the first shared MySQL/MariaDB connection driver. Run a complete transaction inside one admitted operation. Return owned data and checked terminal outcomes. Do not expose driver-borrowed rows or transactions across the executor boundary.

Use a worker-owned runtime if adopting an async driver later. Keep its connection and futures on that runtime. Avoid creating a runtime per query or adding a pool whose independent concurrency bypasses the executor's admission limit. This recommendation is architectural inference from the inspected executor, not a tested driver integration.

## Commit, rollback, and ambiguous outcomes

The synchronous transaction's `commit` executes `COMMIT` and sets its committed flag only after success. Drop attempts `ROLLBACK` when neither committed nor rolled back. Drop discards rollback errors. See [synchronous transaction implementation](https://github.com/blackbeam/rust-mysql-simple/blob/v28.0.3/src/conn/transaction.rs).

The async transaction executes `COMMIT` and attempts rollback after a commit error. It returns the original error. Dropping an open transaction marks the connection as requiring rollback; subsequent connection processing performs cleanup. See [async transaction implementation](https://github.com/blackbeam/mysql_async/blob/v0.37.1/src/queryable/transaction.rs).

SQLx transaction drop initiates rollback through the database transaction manager. That generally queues cleanup for a subsequent asynchronous invocation. It is not a confirmed rollback response. See [SQLx transaction implementation](https://github.com/launchbadge/sqlx/blob/v0.9.0/sqlx-core/src/transaction.rs).

Inference: none of these cleanup paths prove noncommit after a lost commit acknowledgement. A server may commit before the client loses the response. Classify uncertain commit errors separately from confirmed rejection. Retire an uncertain connection rather than silently reusing it. Reconcile through future durable receipts before replaying ROM mutations.

Use explicit rollback and check its result. Keep schema-changing statements outside atomic mutation transactions. MariaDB documents implicit commits for many DDL and administrative statements. See [MariaDB implicit commits](https://mariadb.com/docs/server/reference/sql-statements/transactions/sql-statements-that-cause-an-implicit-commit).

## TLS, deadlines, and version capability profiles

The synchronous options require SSL when `ssl_opts` is present. Dangerous invalid-certificate and hostname-skip switches default to false. TCP connect, read, and write timeouts default to absent. Set explicit nonzero deadlines and preserve verification. Disable preferred local socket routing when a test must establish TCP/TLS behavior. See [mysql connection options](https://github.com/blackbeam/rust-mysql-simple/blob/v28.0.3/src/conn/opts/mod.rs).

Socket read/write deadlines are per-I/O settings. They do not alone prove a total transaction deadline. Define bounded result sizes, lock waits, and query limits separately. Do not interpret executor observation timeout as enforced database timeout.

The projects' versioned CI definitions include multiple MySQL versions and MariaDB major versions. These are declared upstream test targets, not reproduced test results or a complete ROM support matrix. See [mysql CI](https://github.com/blackbeam/rust-mysql-simple/blob/v28.0.3/azure-pipelines.yml) and [mysql_async CI](https://github.com/blackbeam/mysql_async/blob/v0.37.1/azure-pipelines.yml).

Keep distinct MySQL and MariaDB profiles. Record actual server version, engine, authentication plugin, SQL mode, character set, isolation, and locking behavior. Upstream protocol access does not prove identical atomic mutation semantics. Test MySQL 8.4 and MariaDB 11.8 independently before announcing those profiles.

## Official container sources and initialization

The inspected Docker Official Images source lists `mysql:8.4.11` and aliases `8.4` and `8`. Its build provenance points to docker-library/mysql commit `01f90d87012e46cd174073bba02d64e9fc693ed3`, directory `8.4`, file `Dockerfile.oracle`. See [official MySQL image manifest](https://github.com/docker-library/official-images/blob/master/library/mysql).

The same source lists `mariadb:11.8.9` and aliases `11.8` and `11`. Its provenance points to MariaDB/mariadb-docker commit `bdfe641466a5312bb97d06a1a4e1fb411c49e6b6`, directory `11.8`. See [official MariaDB image manifest](https://github.com/docker-library/official-images/blob/master/library/mariadb). MariaDB identifies 11.8 as a long-term release. See [release-series documentation](https://mariadb.com/docs/release-notes/community-server/11.8/what-is-mariadb-118).

These are inspected source-listed tags, not registry resolution evidence. Cached release documentation can list older patches. Pull the chosen exact tag and record its digest and `SELECT VERSION()` output. Do not use `latest` or infer that a major alias remains fixed.

MySQL initialization supports `MYSQL_ROOT_PASSWORD`, `MYSQL_DATABASE`, `MYSQL_USER`, and `MYSQL_PASSWORD`, with file-based secret alternatives. Initialization variables do not modify an already initialized data directory. `/docker-entrypoint-initdb.d` scripts initialize fresh instances. See [official image instructions](https://hub.docker.com/_/mysql).

MariaDB's entrypoint supports the corresponding `MARIADB_` initialization variables and checks whether its data directory already contains a database. Keep restart volumes intact and do not expect changed environment values to replace stored credentials. See [MariaDB entrypoint](https://github.com/MariaDB/mariadb-docker/blob/master/11.8/docker-entrypoint.sh).

Use `/var/lib/mysql` persistent storage and explicit InnoDB tables. Record `innodb_flush_log_at_trx_commit=1` and, when binary logging is enabled, `sync_binlog=1`. MySQL recommends both for greatest replication durability. Hardware and operating-system flush behavior still constrain that guarantee. See [MySQL durability guidance](https://dev.mysql.com/doc/refman/8.4/en/replication-options-binary-log.html) and [MariaDB InnoDB variables](https://mariadb.com/docs/server/server-usage/storage-engines/innodb/innodb-system-variables).

## Adoption and remaining evidence

Proposed first profile: `mysql` 28.0.3, default features disabled, Rust compression, and exactly one explicit TLS backend. Resolve the selected graph before deciding ring, AWS-LC, or native TLS. Disable unused derive, binlog, and authentication-plugin features unless a required scenario needs them.

Audit the resolved lockfile for advisories, declared and effective MSRV, all licenses, and native crypto build requirements. Upstream release notes mention fixes for advisories; they do not establish a clean current graph. Source inspection here supplies no advisory verdict.

Required backend evidence includes verified TLS, wrong credentials, wrong CA/hostname, bounded overload, explicit rollback, conflicting updates, restart persistence, and lost commit acknowledgement. Public-consumer tests must use only exported APIs. A shared host connection driver remains a prerequisite, not a complete ROM Storage provider. Atomic state/event/receipt/Work semantics and the stable public ROM storage helpers require their own implementation and conformance evidence.
