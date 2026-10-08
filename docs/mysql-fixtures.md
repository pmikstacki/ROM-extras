# MySQL and MariaDB connection fixtures

These profiles verify `rom_sql_core::Executor<mysql::Conn>` through an independent public consumer.
They do not implement ROM Storage or establish supported database providers.
See [the execution spec](superpowers/specs/2026-10-08-mysql-execution.md) and [driver research](research/mysql-mariadb-driver-assessment.md).

The executed MySQL profile is 8.4.11 on `127.0.0.1:55452`.
The separate MariaDB profile is 11.8.9 on `127.0.0.1:55453`.
Both use database `rom_extras_tests`, one CPU, 768 MiB, and distinct persistent named volumes.
Image digests and selected actual settings are recorded in [fixture evidence](verification/mysql-mariadb-fixtures-2026-10-08.json).
Passwords are randomly generated in ignored mode-0600 access files.
Do not publish configuration, environment dumps, or credential-bearing connection strings.

Both profiles require InnoDB flush-at-commit, binary-log synchronization and enabled binary logging.
The fixture uses InnoDB tables. It tests acknowledged persistence after controlled container restart.
This does not establish power-loss, replica-loss or failover behavior.

## Execute the required profiles

Supply `ROM_EXTRAS_MYSQL_DSN` and `ROM_EXTRAS_MARIADB_DSN` through private host configuration.
Use `mysql://USER:PASSWORD@127.0.0.1:PORT/rom_extras_tests` syntax with each profile's exact port.
The test configuration refuses another IP, port, database or TLS mode before connection.
This increment uses plaintext loopback fixtures. Verified TLS remains required before production support.

Run both profiles:

```sh
./scripts/check-mysql
```

The script runs the same five tests separately on each server, followed by feature-enabled Clippy.
It fails when either DSN is absent. `scripts/check-all` includes this required job.
Tests preserve their tables, volumes and failed-run evidence.
The restart test checks the container's fixture label before controlling it.

## Boundaries and dependencies

The consumer pins `mysql` 28.0.3 with defaults disabled, `minimal-rust` and `native-tls` features.
The selected Linux build uses the separately verified external OpenSSL 3.5.9 installation.
See [native build requirements](kafka.md) and [dependency decisions](research/decision-log.md).
A compiled TLS backend does not prove a verified MySQL or MariaDB TLS handshake.

One connection belongs to one dedicated executor worker. No additional pool is introduced.
Connect waits are bounded to three seconds. Each native read/write has a five-second deadline.
The fixture session has a three-second InnoDB lock wait limit.
These settings do not impose a total transaction deadline.
A caller timeout preserves `Unknown`; admitted work can commit later.

A duplicate-key statement does not roll back earlier transaction writes.
The test observes the earlier row inside the transaction, then checks explicit whole-operation rollback.
The panic test observes absent uncommitted writes after retirement and reconnection.
Neither result establishes noncommit after a lost commit acknowledgement.
Full receipt recovery, revisions, metadata, journal, Work and owner fencing remain pending.
