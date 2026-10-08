# MySQL and MariaDB execution profiles

This increment verifies the shared bounded executor against two separate real servers.
It does not replace the planned shared SQL persistence engine or establish ROM Storage conformance.
Public ROM main remains `d7ef529040eec60dc869034c2d33130219db85fe` during this increment.
The required incremental Work boundary is not published there.

## Researched boundary

Use a single native connection created and destroyed on each executor worker.
The blocking [mysql driver](https://docs.rs/mysql/28.0.3/mysql/) fits this existing boundary without another pool or runtime.
Disable socket preference to preserve the selected TCP fixture endpoint.
Restrict plaintext fixture endpoints to explicit loopback IP addresses.
Set finite connection and per-read/write deadlines. These are not whole-transaction cancellation guarantees.
[Driver options](https://docs.rs/mysql/28.0.3/mysql/struct.OptsBuilder.html) define their individual scope.

Use distinct MySQL 8.4 and MariaDB 11.8 fixture profiles and persistent volumes.
Select immutable executed digests after pulling the official image tags.
[MySQL image metadata](https://github.com/docker-library/official-images/blob/master/library/mysql)
and [MariaDB image metadata](https://github.com/docker-library/official-images/blob/master/library/mariadb) identify their maintainers and source repositories.
Keep random passwords in ignored mode-0600 files. Do not print driver configuration or credentials.

## Required scenarios

- Missing profile configuration fails the required job.
- A Tokio caller executes driver I/O on the dedicated worker.
- Server identity, InnoDB flush-at-commit and binary-log synchronization are checked for each profile.
- A wait timeout remains Unknown while admitted native work can later commit.
- A fresh connection observes that later commit.
- A job panic retires its connection; uncommitted InnoDB writes are absent after reconnection.
- A duplicate-key statement leaves earlier transaction work visible inside the transaction; explicit rollback removes all operation writes.
- Binary identities preserve case, Unicode, zero and empty byte distinctions.
- An acknowledged row survives labelled server restart and bounded readiness.

A duplicate-key statement failure does not establish whole-operation rollback.
[MySQL InnoDB error handling](https://dev.mysql.com/doc/refman/8.4/en/innodb-error-handling.html) defines statement and transaction rollback differences.
[MariaDB ROLLBACK](https://mariadb.com/docs/server/reference/sql-statements/transactions/rollback) defines explicit transaction rollback.

Full receipts, revision arbitration, metadata, journal, incremental Work, owner fencing and interruption recovery remain required.
Verified CA/name TLS, packaged consumers and the shared `SqlStore<D>` remain pending.
