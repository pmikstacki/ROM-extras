# Maintained Oracle ownership research

Inspection date: 2026-10-10. Source review and local native observations are separate categories.
This decision advances native ownership, not full Oracle Storage or production TLS acceptance.

## Selected boundary and alternatives

Use exact rust-oracle0.6.3 and external retained OCI23.26.3.0.0 on the existing capacity-one SQL executor.
Keep native statements, results and connection handles private. Share ownership arbitration and primitive validation through SQL core.
Owned typed binds permit payload limits before worker admission; arbitrary SDK callbacks do not.
The [tagged SDK source](https://github.com/kubo/rust-oracle/blob/v0.6.3/src/connection.rs) and [SDK connection contract](https://docs.rs/oracle/0.6.3/oracle/struct.Connection.html) define native handle lifetime.

OCI timeout bounds each round trip. Cleanup can require a further interval; native close can fail while statement/LOB handles remain open.
A watchdog break request or executor ticket timeout cannot establish full rollback or retract an acknowledged commit.
The chosen in-process ownership foundation therefore keeps absolute operation/socket containment explicitly open.
An isolated helper process with bounded IPC/confirmed termination is an alternative requiring separate design and native qualification.
Do not detach a timed-out thread and claim it canceled a mutation.
Sources: [exact SDK timeout/break APIs](https://github.com/kubo/rust-oracle/blob/v0.6.3/src/connection.rs), [OCI attributes](https://docs.oracle.com/en/database/oracle/oracle-database/26/lnoci/handle-and-descriptor-attributes.html), [ODPI-C5.4.1 close](https://github.com/oracle/odpi/blob/v5.4.1/src/dpiConn.c).

## Native profile and descriptor control

Use explicit host, port, service and credentials, with one internally generated ADDRESS and retry count zero.
Connect and transport bounds apply per address/IP; DNS expansion and listener redirection are not absolute endpoint containment.
No arbitrary descriptor, implicit native endpoint or administrator role is accepted.
Sources: [Oracle Net timeout rules](https://docs.oracle.com/en/database/oracle/oracle-database/26/haovw/oracle-net-tns-string-parameters.html), [naming parameters](https://docs.oracle.com/en/database/oracle/oracle-database/26/netrf/local-naming-parameters-in-tns-ora-file.html).

Plaintext construction requires literal loopback. TCPS configuration requires explicit wallet and exact certificate DN matching without bypass.
The retained OCI23 configuration uses SSL_SERVER_DN_MATCH, SSL_SERVER_CERT_DN and MY_WALLET_DIRECTORY.
Current documentation prefers TLS-prefixed fields and deprecates MY_WALLET_DIRECTORY in26ai; do not infer new fields are qualified on OCI23.
The requested /23 reference redirects to /26, which limits version-specific web evidence.
Native TCPS trust/hostname qualification remains required; constructor source checks are not that qualification.
Sources: [older official TLS reference](https://docs.oracle.com/en/database/oracle/oracle-database/21/dbseg/configuring-secure-sockets-layer-authentication.html), [current wallet/identity reference](https://docs.oracle.com/en/database/oracle/oracle-database/26/netrf/local-naming-parameters-in-tns-ora-file.html).

## Ownership read and strict native values

Use READ COMMITTED, an indexed singleton and ROWNUM<=2 FOR UPDATE WAIT n.
Oracle forbids combining row_limiting_clause with FOR UPDATE. SKIP LOCKED would conceal contention as missing state.
Keep schema DDL outside application transactions because DDL can implicitly commit.
Sources: [Oracle SELECT](https://docs.oracle.com/en/database/oracle/oracle-database/26/sqlrf/SELECT.html), [transaction control](https://docs.oracle.com/en/database/oracle/oracle-database/26/lnpls/transaction-processing-and-control.html).

Default SDK queries allocate100 rows; set explicit small fetch/prefetch sizes.
Generic numeric getters can accept character values. Require native NUMBER/RAW metadata before conversion.
Check generation=TRUNC(generation) and signed range before i64 extraction, and exact format equality before a potentially truncating getter.
RAW(32) is only a maximum width; require exact stored32-byte identity/token and preserve bytes without character normalization.
SQL cannot recover original fractional input previously rounded by a constrained NUMBER column; persisted validation is a separate guarantee.
Sources: [SDK buffering](https://docs.rs/oracle/0.6.3/oracle/struct.Connection.html), [native type mapping](https://github.com/kubo/rust-oracle/blob/v0.6.3/src/sql_type/oracle_type.rs), [SDK conversion](https://github.com/kubo/rust-oracle/blob/v0.6.3/src/sql_value.rs), [Oracle data types](https://docs.oracle.com/en/database/oracle/oracle-database/26/sqlrf/Data-Types.html).

## Prepared writes and uncertainty

After prepare, require native is_dml() and deny is_returning() before execution.
This rejects DDL, SELECT, PL/SQL and transaction control without a SQL scanner, but does not sandbox database-side routines/triggers.
Oracle statement rejection can leave earlier writes pending. Poison the transaction and require complete rollback before reuse.
COMMIT timeout, break or transport failure is Unknown; later rollback is not proof that commit failed.
Only a small native code allowlist represents ordinary rejection. Unknown codes conservatively retire the connection.
Sources: [exact statement classification](https://github.com/kubo/rust-oracle/blob/v0.6.3/src/statement.rs), [COMMIT](https://docs.oracle.com/en/database/oracle/oracle-database/26/sqlrf/COMMIT.html), [ORA-00060](https://docs.oracle.com/en/error-help/db/ora-00060/), [ORA-01013](https://docs.oracle.com/en/error-help/db/ora-01013/).

## Actual local observations and remaining proof

The independent source consumer executed real ownership, rollback, forbidden DDL, RAW/NUMBER and WAIT cases on the retained native fixture.
An endpoint negative exposed unspecified IP falling through DNS validation; the focused test failed, then passed after a strict IP branch.
Same-volume graceful recovery preserved acknowledged generation2/token/data while generation1 was denied and generation3 was freshly acknowledged.
Archive, audit, final review and full-verifier results belong in the maintained verification record after those commands execute.
Real COMMIT response suppression, native physical containment, abrupt recovery, TCPS and full Storage remain unqualified.
External client/license integrity evidence remains in [the prior fixture record](../oracle-fixture.md); Rust graph audit does not cover OCI native advisories.

## Dependency adoption ruling

Root, source-consumer and archive-consumer audits report zero known vulnerabilities and one unmaintained warning for paste1.0.15.
[RUSTSEC-2024-0436](https://rustsec.org/advisories/RUSTSEC-2024-0436.html) is informational and has no patched versions.
The exact SDK requires paste; the inspected [upstream development manifest](https://github.com/kubo/rust-oracle/blob/master/Cargo.toml) still requires it.
Disabling optional SDK features cannot remove this dependency.
Retain the pinned SDK for this ownership foundation and preserve the warning without audit suppression.
paste is a compile-time procedural macro; this limits its role, but does not establish build-system safety.
An SDK fork replacing paste would introduce a separately maintained driver and requires its own adoption and qualification.
Track that alternative before broader acceptance. Native OCI security review remains separate.
The root graph adds10 registry packages; all prior package checksums remain unchanged.
The21 consumer registry packages have verified archive hashes, extracted source, licenses and declared compiler requirements.

## Original column type correction

A focused native VARCHAR2 identity case failed the Invalid assertion: the CASE projection raised ORA-00932 and returned Unknown.
The observer independently described VARCHAR2 identity and RAW token columns. No malformed row was admitted.
Describe original columns with a zero-row SELECT before bounded CASE projections; reject non-NUMBER/RAW types as Invalid.
The host must keep schema stable; this does not protect against concurrent administrative DDL or hostile native allocation.
Sources: [exact SDK result metadata](https://github.com/kubo/rust-oracle/blob/v0.6.3/src/row.rs), [Oracle CASE types](https://docs.oracle.com/en/database/oracle/oracle-database/26/sqlrf/CASE-Expressions.html).
