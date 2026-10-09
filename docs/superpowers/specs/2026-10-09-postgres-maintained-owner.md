# Maintained PostgreSQL ownership mapping

This is an architectural increment within the user-authorized full extras plan.
It replaces no historical prototype and implements no private ROM persistence policy.

## Outcome and boundary

Add rom-postgres with public Connection, Transaction, Deadlines and ControlTable.
Reuse rom-sql-core claim/takeover/release/with_owner arbitration and worker confinement.
The maintained driver must supply native row locks and complete bounded operation responses.
Full ROM Storage, incremental Work and opaque StorageOwner lifetime integration remain open.
Preserve existing MSSQL and SQL-core public paths, fixtures, consumers and evidence.

## Transport and configuration

Use already locked tokio-postgres0.7.18 and tokio1.53.1.
A current-thread runtime drives one retained native transport task.
Wrap complete connect, query, execute, commit and rollback futures in positive finite deadlines.
Transport uncertainty retires the client and its owned transport task; no implicit reconnect or retry.
A host-supplied MakeTlsConnect connector handles certificate identity and trust.
Normal connect forces SslMode::Require; one explicit host and no hostaddr overrides are admitted.
A separate explicit connect_loopback permits NoTls only for one literal loopback TCP IP.
Calls from an active Tokio runtime fail before blocking. Runtime Drop must be safe after host async handoff.

Deadlines distinguish connect, total I/O, native statement and native lock limits.
Require whole-millisecond native limits with lock < statement < I/O; connect/I/O at most 60 seconds.
Server timeouts do not substitute for the outer I/O timeout.
Trust only prepared driver SQL, bounded 16KiB/64 parameters; host bounds parameter bytes and complete statement work.
Never expose SQL to Resource actions; no session/transaction changes, rowsets, DDL, external effects or triggers.

## Native state

Provision a logged singleton table separately with fixed columns singleton_key,format_version,store_identity,generation,owner_token.
Use READ COMMITTED and FOR UPDATE; retain the exact control row lock through native terminal acknowledgement.
Bound returned rows to2 and BYTEA materialization to32 with CASE/octet_length guards.
Validate format1, exact 32-byte identity, nonnegative i64 generation and optional nonzero32-byte token.
Reject malformed metadata and poisoned transactions; no automatic provision/migration/reset.
SET LOCAL lock_timeout, statement_timeout and synchronous_commit=on during each admitted transaction.
Any caught persistence error must prevent commit/re-lock. Confirm complete rollback before retaining a usable native connection.
Unknown commit/rollback cannot report success or return fabricated acknowledged Ownership.
Public errors and Debug must contain no native messages, SQL, endpoint or credentials.

## Acceptance

Independent direct and normalized-archive consumers execute actual PostgreSQL 18.6 operations.
Cover arbitration/races/stale connections, native lock ordering, prefix rollback, Drop, native timeouts,
metadata/limits, caught-error poison, outer connect timeout and retirement as applicable.
Keep old synthetic acknowledgement fault evidence explicitly separate from wire response loss.
Production TLS, actual response-loss and restart qualification require their own later native profiles.
Run affected checks, dependency audit/license/MSRV inspection, one final source review, and full frozen verifier before publication.

## Decision

Selected bounded async-native transport over synchronous SDK to cover blackholed responses.
A generic host TLS seam avoids imposing a new certificate backend; fixture plaintext is explicitly isolated.
Native timeout semantics differ from MSSQL, so they must not be forced into its contract.
The public ROM pin and all historical work remain unchanged. Existing autonomous implementation authorization applies.
See the accompanying primary-source research; source recommendations are not execution evidence.

Common portable identifier and connect/I/O validation belongs to rom-sql-core and is reused by both maintained drivers.
Native quoting and statement/lock guarantees remain driver-specific. MSSQL public paths and signatures remain unchanged.
