# SQL Server native ownership

Implement maintained `rom-mssql` over the existing `rom-sql-core::OwnerTransaction` contract.
This increment qualifies native fencing, not ROM Storage or Resource persistence.
Public ROM remains pinned to d7ef529040eec60dc869034c2d33130219db85fe.
Do not copy private Work policy or change existing API paths.

The trusted driver host supplies Tiberius configuration, explicit deadlines, and an existing control table.
Force encryption. Certificate trust remains the host's explicit decision; never enable a bypass inside the adapter.
Keep the native client and current-thread runtime on a SQL executor worker.
Reject invocation inside an active Tokio runtime before blocking.
Do not provision, migrate, reset, reconnect or retry automatically.

The table has `singleton_key INT`, `format_version INT`, `store_identity VARBINARY(32)`, `generation BIGINT`, and nullable `owner_token VARBINARY(32)`.
The host must enforce a singleton key of 1, format 1, exact lengths and nonnegative generations.
The table must have no triggers. The driver verifies exact identity, format and state on every locking read.
Table identifiers accept ASCII letters, digits and underscore, with a letter or underscore first, at most 63 bytes.
Use READ COMMITTED transactions and `UPDLOCK,HOLDLOCK`; retain locks until commit or confirmed rollback.
Use parameterized bindings for binary state. Native generation is distinct from Resource revision.

Prepared native writes execute only after the transaction locks the control row.
This is a trusted driver extension, not an application action or Resource SQL API.
The extension must not execute transaction control, alter session settings, or invoke external effects.
Shared arbitration remains entirely in rom-sql-core.
Commit failures return Unknown. Timeout or incomplete transport retires the client permanently.
Server statement errors require complete native rollback before the shared core preserves their finite error.
Rollback must establish XACT_STATE()=0 and @@TRANCOUNT=0, including a transaction already aborted by SQL Server.
Drop attempts bounded cleanup; an unknown cleanup retires the client.
No error, Debug output or returned result contains credentials, SQL text or native server messages.

Require actual native competing claims, stale surviving connections, held-transaction ordering, release/reclaim,
statement-error prefix rollback, lock timeout cleanup, malformed metadata, Drop cleanup, caller deadline and I/O retirement.
Repeat native acceptance against normalized Cargo archives in an independent consumer.
Retain all previous fixtures, source changes and evidence.
Run affected checks and the full local verifier before integration.

This increment does not qualify deadlock victim recovery, lost COMMIT wire responses, server restart,
production certificate chains, pooled StorageOwner release or full Storage conformance. Report those limits explicitly.
