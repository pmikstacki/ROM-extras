# ROM-extras public boundary audit

Date: 2026-10-08. Evidence category: source inspection, not executed integration tests.

## Source identity

ROM HEAD: `1a2b7934139f3d7bb4cb04aaf4a90bd9405ab470`.
The working tree contains concurrent changes. Findings below distinguish committed interfaces from working-tree additions.
No ROM-extras repository exists locally or at the proposed GitHub location during this inspection.

Commands used:

```sh
git rev-parse HEAD
git ls-tree -r --name-only HEAD crates/rom/src
git show HEAD:crates/rom/src/lib.rs
git show HEAD:crates/rom/src/channels/registration.rs
git show HEAD:crates/rom/src/source.rs
git show HEAD:crates/rom-backup/src/model.rs
git show HEAD:crates/rom-blob-object-store/src/adapter.rs
```

Working-tree files were also read to identify forthcoming interfaces. Their existence does not establish a released support contract.

## Extension boundary map

| Extension | Committed interface or existing behavior | Required next action |
| --- | --- | --- |
| SQL storage | `Storage`, `Bundle`, `StorageOwnership`, `StorageOwner` | Select a commit containing incremental adapter support before implementing shared storage |
| Incremental Work | `storage_support.rs` is absent from HEAD | Check `StorageMetadata`, `prepare_bundle`, `WorkRead`, and `WorkDelta` exports in a later immutable revision |
| Webhooks and brokers | `Delivery<P>`, `DeliveryOutcome`, `ChannelRegistration<P>`, `RuntimeBuilder::channel_with` | Implement host callbacks; preserve persisted delivery profiles and reconciliation semantics |
| Notification connectors | Same channel boundary | Choose specific providers and payload contracts in the delivery-family spec |
| OpenTelemetry | Diagnostic modules and builder integration exist only in the working tree | Wait for a commit exporting the diagnostic drain; do not invent a separate action pipeline |
| S3-compatible blobs | `rom_blob::BlobStore` and `rom_blob_object_store::Adapter::s3` | Extend host presets and verify additional deployment profiles through the existing implementation |
| Azure Blob | Existing adapter enables the `object_store` AWS feature; no Azure constructor was found | Add an Azure implementation using `BlobStore`; verify create-only and unknown-outcome behavior independently |
| OIDC presets | `OidcIdTokenAdapter::configured` and `with_trusted_audiences` | Reuse the verifier; define exact issuer, client, key-source, and audience inputs |
| OpenSearch and vectors | Bounded `Storage::journal` and Runtime authorization | Persist independent read-model checkpoints and reauthorize disclosure; specify gap recovery |
| Secrets and KMS | No selected provider contract in the ROM-extras design | Define host secret-reference resolution and encryption-context contracts before provider code |
| Backup and migrations | `rom_backup::{Snapshot, Collector, MigrationPlan, Backend}` | Resolve the closed backend discriminator before SQL archive support |
| Import | `SourcePermit::trusted`, `Runtime::invoke_sourced`, `source_state` | Use service actors and revision-bound source permits; preserve ordinary mutation authorization |

## Incremental storage and ownership

The committed `StorageOwner` is an opaque guard around a process-local atomic claim.
It has no public callback constructor for a database-session lock or lease.
The SQL adapter therefore needs a separately owned native guard and persistent generation checks.
The Runtime guard still determines when another Runtime can acquire the local store claim.
The design must specify the native guard's lifetime and handoff when an adapter handle outlives its Runtime.

Every mutating transaction must validate ownership, including registration, action commit, reaction updates, operator control, and maintenance.
Checking a fence only in `Storage::commit` would leave other writes unprotected.
A connection-loss test must preserve a second pooled connection and prove that takeover rejects its old generation.

The working tree exposes `rom::storage_support::metadata::{StorageMetadata, BundleDelta, prepare_bundle}`.
It also exposes incremental Work readers, editors, accounting, and deltas through `rom::storage_support::work`.
These exports are not present in the inspected commit.
Do not replace them with copied private code or a serialized complete Work ledger.

## Delivery and reconciliation

`Delivery.id` is the stable external delivery identity. `Delivery.attempt` distinguishes attempts.
`DeliveryOutcome::Accepted` represents external acknowledgement, not external exactly-once behavior.
`DeliveryVerification::NotAccepted` asserts that the earlier attempt cannot succeed later.
A lookup that merely reports no current message does not necessarily satisfy that assertion.

Use `ChannelRegistration::verifier` only when a provider can establish the declared result.
Otherwise return `DeliveryVerification::Unresolved` and preserve operator recovery.
The Runtime enforces delivery deadlines; adapter callbacks must yield and avoid blocking Tokio workers.

## Telemetry

The working-tree `Diagnostics::bounded` returns a sink and host-owned `DiagnosticReader`.
The reader supplies `recv`, `try_recv`, `close`, and independent loss statistics.
`DiagnosticEvent` has fixed outcome categories and opaque correlation links, without Resource values or principal text.
Its sequence is host-local and potentially lossy; it is not a business journal cursor.

The OpenTelemetry exporter should drain this stream after the interface is committed.
It must preserve the distinction between diagnostic loss, durable history, and unknown action outcomes.
The exporter cannot promise complete traces from this lossy source.

## Blob integration

The existing object-store adapter implements create-only writes with `PutMode::Create`.
It bounds reads before and during streaming and treats deletion of a missing object as successful maintenance.
The S3 constructor disables automatic SDK retries and validates an explicit HTTPS or loopback-test endpoint policy.

Reuse that contract for S3 presets. Source inspection does not prove a new endpoint satisfies conditional-create behavior.
Azure Blob needs its own constructor or adapter; enabling a feature alone is not evidence of conformance.

## Backup compatibility decision

The committed archive uses archive version 6 and storage format 8.
The working-tree archive uses archive version 7 and storage format 10.
Both inspected `Backend` definitions contain only `Sqlite` and `Redb`.

A SQL archive cannot be labeled as either backend merely to pass decoding.
Select one explicit approach in the maintenance-family spec:

- Add an upstream backend-neutral archive profile with versioned native-layout metadata and backward compatibility tests.
- Add a ROM-extras SQL envelope around canonical public snapshot data, with its own format identity and validation.

The second approach must reuse available snapshot validation and migration logic rather than duplicate domain rules.
Neither approach is currently implemented or verified.

## Provenance import

`SourcePermit::trusted` validates labels, nonzero generations, dependencies, expiry, and bounded field origins.
`Runtime::invoke_sourced` requires a service principal and executes the normal mutation pipeline.
The permit is not deserializable caller authority.
The importer must obtain the permit through trusted host configuration rather than accepting it from an input file.

Use `source_state` for authorized inspection of current source attribution and tombstones.
Do not infer that whole-record field access also permits protected source metadata disclosure.

## External driver findings

The PostgreSQL synchronous driver is a Tokio-backed wrapper. A dedicated executor avoids nested-runtime blocking in arbitrary callers.
[postgres implementation](https://docs.rs/postgres/0.19.14/postgres/#implementation)

Tiberius uses asynchronous TDS I/O and accepts an injected stream. Tokio streams require the documented compatibility adapter.
[Tiberius Tokio integration](https://docs.rs/tiberius/0.13.0/tiberius/#connecting-with-tokio)

These documentation observations do not establish dependency admissibility, performance, TLS interoperability, or backend conformance.
The separate driver assessment records candidates and remaining audit requirements.

## Outcome

Committed delivery, source attribution, OIDC, and blob boundaries allow reuse without a domain rewrite.
Incremental SQL storage and the proposed telemetry integration require a newer committed ROM revision.
SQL backup compatibility and native owner lifetime require explicit design decisions.
No new provider has passed compilation or a real backend test in this audit.
