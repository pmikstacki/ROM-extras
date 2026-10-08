# ROM-extras: database adapters and extension roadmap

Status: proposed design. No provider implementation or database verification is claimed.
Date: 2026-10-08.

## Intent and baseline

Create a separate ROM-extras repository with PostgreSQL, Microsoft SQL Server, and additional relational providers.
Share persistence orchestration. Preserve ROM's Resource → action → atomic commit → event contract.
Keep database dependencies outside the ROM domain core.

Reviewed ROM HEAD: `1a2b7934139f3d7bb4cb04aaf4a90bd9405ab470`.
The working tree contains concurrent, uncommitted storage changes. This revision alone does not identify those changes.
The referenced architecture chat is still implementing incremental SQLite and redb storage.
Select a committed ROM revision containing the required public interfaces before implementing ROM-extras.
Do not depend on another chat's dirty working tree or copy its private modules.

Relevant interfaces are `Storage`, `Bundle`, `StorageOwner`, `StorageMetadata`, `prepare_bundle`, and the public incremental Work interfaces.
The shared storage conformance entry point is `rom_conformance::storage`.
Check actual public exports and packaged-consumer compilation at the selected revision.

## Alternatives

| Approach | Benefit | Cost |
| --- | --- | --- |
| Shared SQL orchestration with typed driver ports | One commit protocol; explicit backend guarantees | Requires a carefully bounded transaction interface |
| Separate complete adapters | Fast first implementation | Duplicates arbitration, limits, recovery, and tests |
| One universal SQL/NoSQL engine | Uniform surface | Hides incompatible atomicity and query guarantees |

Select shared SQL orchestration. Nonrelational providers can share ROM semantics and conformance without sharing SQL transactions.

## Proposed repository

Proposed remote: `pmikstacki/ROM-extras`, matching the existing ROM owner.
The repository visibility remains an explicit publication setting.

| Package | Responsibility |
| --- | --- |
| `rom-sql-core` | Implement ROM storage operations through typed transaction ports |
| `rom-postgres` | PostgreSQL connections, SQL, schema, locks, and error classification |
| `rom-mssql` | SQL Server connections, T-SQL, schema, locks, and error classification |
| `rom-mysql` | MySQL/InnoDB connections, SQL, schema, locks, and error classification |
| `rom-extras-conformance` | Fresh databases, reopen fixtures, failures, and external consumers |

Use named modules for behavior. Keep crate roots as facades.
Each provider is independently selectable. PostgreSQL consumers must not compile MSSQL drivers.
Pin ROM by immutable Git revision until a compatible package release is available.
Select driver versions only after checking advisories, licenses, MSRV, TLS, and required features.

## Semantic and physical boundaries

ROM owns Resource validation, action semantics, authorization, receipt meaning, and Work transitions.
The SQL core orchestrates public ROM helpers inside one native transaction.
Drivers expose typed reads, conditional writes, bounded scans, and commit outcomes.
Do not expose raw SQL or a database-specific transaction through the Resource API.

Store Resources, receipts, events, effects, Work records, indexes, and operator records separately.
Use incremental Work reads and deltas. Do not serialize the entire retained Work ledger for each mutation.
Preserve protected metadata without exposing it as public fields or journal payloads.
Compare keys and fingerprints using explicit byte semantics, independent of database default collation.
Check numeric ranges and encoded sizes before conversion or allocation.

## Commit protocol

1. Acquire a native transaction and verify the active owner fence.
2. Read the authoritative metadata and retry boundaries.
3. Arbitrate receipt identity and fingerprint using ROM's replay rules.
4. For a new operation, validate registration, references, expected revision, and Work claims.
5. Prepare the canonical bundle delta through the public ROM helper.
6. Persist state, receipt, events, effects, Work changes, and metadata in this transaction.
7. Commit before returning success or publishing an invalidation.

Use a transactionally locked metadata row for journal publication order in the initial profile.
Do not use an identity column or sequence allocation as proof of commit order.
This serialization is an explicit throughput tradeoff. Benchmark it before proposing a more concurrent protocol.

A receipt replay must not produce another event or effect.
A rejected mutation must leave every bundle component unchanged.
A lost commit acknowledgement is an unknown outcome until authoritative recovery establishes the result.
An absent receipt alone does not prove rollback while the original transaction can still finish.
Normalize confirmed rollback, conflict, unsupported behavior, and unknown outcome separately.
Retry only confirmed retryable transactions, with bounded attempts and the original identity.
Never rerun application actions or external effects inside driver retries.

## Ownership and execution

Retain ROM's one-Runtime-per-store contract.
A process-local mutex is insufficient for a remote database.
Persist an owner generation. Verify it under a lock in every mutating transaction.
Coordinate takeover with a database-native exclusive ownership mechanism and an atomic generation change.
A stale owner must fail writes after takeover. Test connection loss and reconnect explicitly.
Native advisory locks alone are not sufficient when independent pool connections perform writes.

The current Storage interface is synchronous.
Confine asynchronous drivers to a dedicated bounded executor with explicit connection and request limits.
Do not call a nested runtime's blocking entry point from arbitrary Tokio workers.
Define queue saturation, deadlines, shutdown, and cancellation outcomes in the implementation plan.
A deadline after submission cannot imply that the transaction rolled back.

## Provider sequence

| Provider | Proposed phase | Admission requirement |
| --- | --- | --- |
| PostgreSQL | First | Real-server conformance, ownership fencing, recovery, and packaged consumer |
| Microsoft SQL Server | First | Same tests, plus T-SQL transaction and error-state coverage |
| MySQL/InnoDB | Second | Same tests, with full rollback after statement-only failures |
| MariaDB | Second, separate profile | Its own server/version matrix; no inherited MySQL support claim |
| CockroachDB | Later | Separate retry, ownership, and distributed durability assessment |
| Oracle | Later | Driver, licensing, transaction, and operational assessment |

Keep SQLite and redb in ROM initially. Moving maintained packages is outside this task.
Treat Azure SQL and hosted PostgreSQL products as deployment profiles that need their own evidence.
Use bounded reference query reads first. Add native query optimization only with semantic equivalence tests.
Do not advertise full-text, vectors, arbitrary SQL writes, or active-active Runtime ownership in the first release.

## Verification and support admission

Run the existing shared storage conformance suite against each real server.
Add races for identical identities, conflicting fingerprints, and stale revisions.
Test process interruption before commit and after commit but before acknowledgement.
Test receipt recovery, server restart, owner loss, stale fences, and bounded reads.
Test schema registration, references, tombstones, retry epochs, journal gaps, and Work completion plus enqueue.
Distinguish missing, null, false, zero, empty strings, and numeric boundaries.
Test migrations against old and future markers. Unknown formats must fail without schema writes.
Run format, Clippy with denied warnings, tests, doctests, warning-free docs, MSRV, and packaged consumers.
Record source revision, dirty state, lockfile, compiler, server versions, settings, command, and result.
A missing database must fail the required verification job; it must not silently count as a passing test.
Run the full local verifier before integration. Keep GitHub Actions disabled, following ROM's current policy.
Do not label a provider supported until its declared profile passes these gates.

## Further extension candidates

These are proposals, not included implementations or verified compatibility claims.

| Candidate | ROM boundary | Priority and constraint |
| --- | --- | --- |
| OpenTelemetry integration | Runtime and adapter observation | Early: correlate action, commit, Work, and unknown-outcome recovery |
| Webhook delivery | Durable effects/Work | Early: signatures, bounded retries, delivery receipts, SSRF controls |
| Kafka, NATS, RabbitMQ delivery | Durable effects/Work | Next: outbox publication and consumer deduplication; no external exactly-once promise |
| S3-compatible and Azure Blob integrations | Existing blob boundary | Extend existing ROM blob capabilities instead of duplicating them |
| OIDC provider presets | Existing authentication boundary | Extend existing ROM authentication; keep authorization in Runtime |
| OpenSearch and vector projections | Event-driven read models | Rebuildable, explicitly eventual, authorized query results |
| Secret/KMS integrations | Host secret and encryption boundary | Reference secrets without storing plaintext in Resource events |
| Notification connectors | Effects/Work | Email, SMS, and collaboration tools with explicit delivery policy |
| Backup and migration tooling | Existing maintenance boundary | Consistent archives, format checks, tested restore and cutover fencing |
| Import/connectors for existing systems | Actions with source provenance | Preserve authorization and idempotency; no direct table mutation |

## Primary references

PostgreSQL documents transaction and session advisory locks separately:
https://www.postgresql.org/docs/18/explicit-locking.html

SQL Server exposes application locks through `sp_getapplock`:
https://learn.microsoft.com/en-us/sql/relational-databases/system-stored-procedures/sp-getapplock-transact-sql?view=sql-server-ver17

InnoDB distinguishes whole-transaction deadlock rollback from statement rollback on a lock timeout:
https://dev.mysql.com/doc/refman/8.0/en/innodb-error-handling.html

The fencing and shared-core design above is a ROM-specific proposal, not a guarantee established by these sources.
