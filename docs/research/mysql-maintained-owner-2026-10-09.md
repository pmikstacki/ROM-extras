# mysql_async maintained adoption

Inspection date: 2026-10-09. Source review only. Baseline reported by root: `5f5c19d`. No maintained edits, dependencies installed, or backend calls occurred. Exact inspected upstream tag: mysql_async v0.37.1.

## Decision and minimal host API

Use one unpooled MySQL-protocol implementation for MySQL8.4 and MariaDB11.8. Keep separate native profiles. The shared core remains transport-free. Adopt the SDK only after root's quarantined exact-package graph audit and build evidence.

Expose constructed host configuration with bounded hostname/port, username/password/database, explicit TLS choice, optional resolved IPs, validated packet cap and declared deadlines. Redact Debug. Do not accept arbitrary SDK Opts or a DSN that can enable hidden behaviors. Provide a separate explicit loopback plaintext constructor for synthetic native fixtures.

Construct default OptsBuilder internally. Expose no after_connect callback, init/setup SQL, pool, local-infile handlers, socket switching, compression or arbitrary connection attributes. Set prefer_socket=false, secure_auth=true, enable_cleartext_plugin=false, explicit bounded statement cache and packet size. These SDK options otherwise include execution and transport behaviors beyond the ownership contract. See [exact Opts source](https://github.com/blackbeam/mysql_async/blob/v0.37.1/src/opts/mod.rs) and [OptsBuilder0.37.1](https://docs.rs/mysql_async/0.37.1/mysql_async/struct.OptsBuilder.html).

Do not build a SQL policy scanner. Prepared persistence is a trusted driver extension with fixed native statements and bound parameters, not application SQL. Enforce limits and native results at this boundary.

## Cancellation and deferred Drop

Conn's Drop implementation is in `src/conn/pool/mod.rs`, including the unpooled path. It removes infile handling, transfers its inner state, and delegates cleanup. The helper in `src/conn/mod.rs` can spawn detached cleanup/rollback/disconnect on the current Tokio runtime. Thus unpooled does not mean Drop is synchronously complete. See [exact Conn Drop](https://github.com/blackbeam/mysql_async/blob/v0.37.1/src/conn/pool/mod.rs).

`take_stream` is private. Public `disconnect(self)` marks disconnected before awaiting COM_QUIT and closes the stream afterward. Connect runs handshake/authentication, settings reads, optional socket reconnect, and configured commands. Bound the entire Conn::new future, not only TCP establishment. See [exact Conn transport source](https://github.com/blackbeam/mysql_async/blob/v0.37.1/src/conn/mod.rs).

A borrowed request timeout leaves the owned Conn available for retirement. After runtime.block_on returns, take/drop it outside any entered runtime context. That avoids starting asynchronous cleanup through Handle::try_current. If retirement runs inside a runtime, simply taking Conn can leave a hidden cleanup task/socket behind.

Connect cancellation is harder: a partially constructed Conn is dropped inside the connection future while a runtime context exists. That can schedule hidden cleanup. Retire the entire private runtime after uncertain connect/request lifecycle, and reject subsequent jobs. Runtime destruction must occur on the synchronous worker, outside block_on/entered contexts. Do not retain an affected runtime while claiming no transport tasks remain.

Calling disconnect with a deadline is an alternative graceful close path. Its early disconnected flag avoids another deferred cleanup on cancellation after first polling. It still cannot prove rollback or undo an uncertain COMMIT. Verify this lifetime behavior through socket-closure and bounded runtime-drop tests before claiming adoption.

NoPool and no external Conn clones simplify ownership. Do not introduce recyclers or background pool queues merely to reuse a connection after failure.

## TLS and infile behavior

Require a nonempty TLS configuration for verified remote connections. SslOpts must retain domain and certificate validation. Reject skip_domain_validation and accept_invalid_certs. Host-selected roots/client identity must remain explicit. Resolved IPs can avoid DNS while hostname remains the TLS identity; verify that profile separately. Do not let a server lacking TLS silently downgrade a required profile. See [SSL options and host resolution](https://docs.rs/mysql_async/0.37.1/mysql_async/struct.OptsBuilder.html) and [SslOpts source](https://github.com/blackbeam/mysql_async/blob/v0.37.1/src/opts/mod.rs).

Default global infile handler is None. However the SDK capability set still advertises CLIENT_LOCAL_FILES, as well as multi-statement/result capabilities. None is a refusal to provide file contents, not proof that the capability was disabled. A server infile request returns NoHandler when neither local nor global handler exists. Treat it as a protocol rejection and retire the connection. Never set a filesystem handler. See [capability source](https://github.com/blackbeam/mysql_async/blob/v0.37.1/src/opts/mod.rs) and [infile request handling](https://github.com/blackbeam/mysql_async/blob/v0.37.1/src/conn/routines/helpers.rs).

These defaults require negative malicious-request tests. Do not claim the SDK advertises no infile capability.

## Absolute operation and result bounds

Use one worker-owned current-thread runtime. Wrap the full connect operation and each full native query/execute/commit/rollback future in an absolute timeout. All response parsing and draining must occur inside that future. A caller's Executor ticket deadline remains separate and does not cancel admitted work.

OptsBuilder max_allowed_packet saturates to protocol limits. Validate allowed inputs and confirm the effective cap; do not assume an arbitrarily tiny requested value remains unchanged. Packet caps do not bound total rows or complete-stream bytes. See [packet option](https://docs.rs/mysql_async/0.37.1/mysql_async/struct.OptsBuilder.html#method.max_allowed_packet).

QueryResult exposes columns_ref, next, is_empty and affected_rows. drop_result drains rows and subsequent sets; it does not reject an unexpected rowset. Do not call exec_drop and report success merely because everything was discarded. See [exact QueryResult](https://github.com/blackbeam/mysql_async/blob/v0.37.1/src/queryable/query_result/mod.rs).

For prepared execute, inspect result metadata and reject nonempty columns before accepting affected rows. Reject unexpected additional results and bound all transitions/draining under the absolute deadline. If protocol cleanup cannot be established, retire. For lock_owner, require one fixed rowset, bounded columns, at most2 rows and fixed total returned bytes.

Decode through get_opt/from_value_opt equivalents. Numeric expressions and metadata can be unsigned even when host expects i64. Read into a representation that checks signedness and range explicitly; never panic using infallible conversion. Token CASE projection must preserve separate nullable OCTET_LENGTH evidence so malformed data cannot become idle.

## Shared native SQL

Use explicit READ COMMITTED transaction and InnoDB control table. Existing singleton uniqueness must be validated. Both target families support the basic locking read:

```sql
SELECT format_version,
       CASE WHEN OCTET_LENGTH(store_identity)=32 THEN store_identity ELSE NULL END,
       generation,
       CASE WHEN OCTET_LENGTH(owner_token)=32 THEN owner_token ELSE NULL END,
       OCTET_LENGTH(owner_token)
FROM fixed_schema.fixed_table
WHERE singleton_key = ?
LIMIT 2 FOR UPDATE;
```

MySQL documents LIMIT before the locking clause. MariaDB requires transaction/autocommit discipline to retain InnoDB FOR UPDATE locks. See [MySQL8.4 SELECT](https://dev.mysql.com/doc/refman/8.4/en/select.html) and [MariaDB FOR UPDATE](https://mariadb.com/docs/server/reference/sql-statements/data-manipulation/selecting-data/for-update).

Set innodb_lock_wait_timeout in seconds using constructed numeric values outside host SQL input. It does not limit all statements or COMMIT. MySQL/MariaDB server statement timeout options differ, so SDK absolute timeout remains the shared elapsed bound. Lock timeout normally aborts only the statement; request full rollback. Deadlock handling and confirmed rollback are separate from transport uncertainty. See [MySQL error handling](https://dev.mysql.com/doc/refman/8.4/en/innodb-error-handling.html) and [MariaDB InnoDB variables](https://mariadb.com/docs/server/server-usage/storage-engines/innodb/innodb-system-variables).

Set client_found_rows deliberately. Matched-row semantics can simplify singleton updates; verify exact one-row behavior on both native servers, including no-op writes. Do not infer it from UPDATE success or conflate warning counts with failure. See [CLIENT_FOUND_ROWS option](https://docs.rs/mysql_async/0.37.1/mysql_async/struct.OptsBuilder.html#method.client_found_rows).

## Commit and acceptance

Poison on every prepared failure, even if a caller catches it. The facade must own native transaction state. Require exact current fence inside the retained row lock. Commit errors/deadlines are Unknown unless complete rollback is independently established; retire native transport/runtime. Explicit rollback must finish under its deadline. Drop is fallback, not a caller-visible result.

Root must qualify both native profiles: claim race, ordered takeover, old surviving connection, malformed state, caught-error poison, timeout, packet cap, rowset rejection, unsigned length decoding, restart and actual COMMIT response loss. Add handshake cancellation, dribbling response, no delayed cleanup socket after retirement, malicious infile refusal, TLS unsupported-server rejection and wrong CA/name negatives.

This note is not a dependency audit or implementation result. Exact package licensing/MSRV/features/advisories must come from the quarantined resolved graph. Native TLS crypto adoption and the public/archive consumer gates remain required. Full ROM Storage and Work policy are outside this ownership increment.

## Maintained implementation decisions and observed results

Root verified the exact SDK source and adopted mysql_async0.37.1 with minimal-rust/native-tls-tls.
The native-tls feature alone does not enable the SDK TLS implementation.
The private generic Executor owns one runtime/client; retirement drops Conn outside Tokio before shutting down the owned runtime.
There are no caller-visible Executor clones, callbacks, pools or arbitrary SDK options.
The raw synchronous SDK in the independent consumer is fixture provisioning/observation only.

The target locking read acquires metadata protection before the InnoDB engine check.
Metadata locks survive through the transaction; schema replacement cannot race the retained target lock.
See [MySQL8.4 metadata locking](https://dev.mysql.com/doc/refman/8.4/en/metadata-locking.html).
MariaDB locking reads likewise require an explicit transaction; see [MariaDB FOR UPDATE](https://mariadb.com/docs/server/reference/sql-statements/data-manipulation/selecting-data/for-update).
The host remains responsible for immutable indexed schema and transactional protected tables.

A separate root fixture observer checks exact waiting-query/table matching through INNODB_TRX.
Application credentials lack PROCESS and remain unchanged.
Poll outside the native snapshot refresh window; see [MySQL transaction observation consistency](https://dev.mysql.com/doc/refman/8.4/en/innodb-information-schema-transactions.html).
Never report an inferred sleep as an observed lock wait.

The 60KiB parameter budget includes scalar/byte-prefix reserves under a64KiB native packet cap.
Fixed SQL preparation is separately capped16KiB. Packet limits do not imply a cumulative transaction limit.
The SDK does not publicly re-export PathOrBuf; use host trust values converted through SslOpts without a private API export.

Direct consumers ran independently on MySQL8.4.11 and MariaDB11.8.9.
Native COMMIT succeeded before the adapter reported Unknown under suppressed responses and250ms-per-byte response pacing.
The relay saw11 real response bytes; the paced profile delivered7 before the outer2s deadline.
An independent committed read preceded Unknown. Peer closure preceded adapter destruction and relay cleanup.
A negative source mutation retaining the client/runtime failed the peer-close assertion.
These are native protocol results, separate from malicious-wire mock tests and production TLS qualification.

The actual root adoption added24 registry identities, removed0 and changed0 prior checksums.
All24 cached archives and extracted bytes matched registry checksums. Root and independent consumer audits reported0 advisories and0 warnings.
Licenses have permissive alternatives for OR expressions; retain Apache, MIT, Unicode and relevant notices.
Rust1.99 compiled the exact root and independently resolved consumer graphs.
SDK licensing is MIT OR Apache-2.0. Server licensing/distribution remains separate from the Rust client dependency.
Malicious infile injection, verified native TLS profiles, deadlock-victim and native maintained crash recovery remain separate qualification work.

## Shared consumer regression

The first full verifier rejected a new relay method as dead_code in the private MSSQL consumer module.
The legacy arm(bool) now delegates to one arm_delivery implementation with mutually exclusive suppression/pacing flags.
This removes duplicate barrier setup and keeps every consumer under warnings-denied Clippy.
No production driver or existing arm(bool) signature changed.
See [Rust dead_code](https://doc.rust-lang.org/rustc/lints/listing/warn-by-default.html#dead-code).
The original full-verifier failure remains in evidence; affected MSSQL/PostgreSQL/MySQL/archive gates and the full verifier must pass afterward.
