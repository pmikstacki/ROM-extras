# ROM-extras Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement and verify every extension family in the ROM-extras goal, with one shared SQL persistence engine.

**Architecture:** ROM retains domain semantics and authorization. ROM-extras implements native persistence, delivery, observation, and host integrations through public ROM boundaries. Each family has an independently testable package and an explicit backend support profile.

**Tech Stack:** Rust 2024, the compiler version required by the selected ROM revision, native database drivers, and real backend fixtures.

**Spec:** [ROM-extras design](../specs/2026-10-08-rom-extras-design.md).

## Global Constraints

- Every project and technology decision must use web research from primary sources and record its evidence.

- Preserve Resource → action → atomic commit → event semantics.
- Keep database dependencies outside the ROM domain core.
- Pin ROM by immutable Git revision until a compatible package release is available.
- Do not depend on another chat's dirty working tree or copy its private modules.
- Use incremental Work reads and deltas. Do not serialize the entire retained Work ledger for each mutation.
- Retain ROM's one-Runtime-per-store contract.
- Native advisory locks alone are not sufficient when independent pool connections perform writes.
- A deadline after submission cannot imply that the transaction rolled back.
- A missing database must fail the required verification job; it must not silently count as a passing test.
- Run the full local verifier before integration. Keep GitHub Actions disabled, following ROM's current policy.
- Do not label a provider supported until its declared profile passes these gates.
- Implement tasks in this chat. Do not dispatch agents without authorization.

## Review Focus

- A pooled connection survives owner-session loss: reject its stale generation before mutation.
- An unknown commit is followed by an absent receipt: preserve uncertainty while the original transaction can finish.
- Database collation merges distinct IDs: use explicit byte semantics and test non-ASCII and case differences.
- An executor deadline expires after admission: keep native completion and caller acknowledgement distinct.
- A projection receives a duplicate or a journal gap: deduplicate or rebuild without exposing unauthorized data.

## Current evidence and dependency gate

On 2026-10-08, local `/root/ROM-extras` does not exist. GitHub lookup reports no `pmikstacki/ROM-extras` repository.
ROM HEAD is `1a2b7934139f3d7bb4cb04aaf4a90bd9405ab470`. Its workspace declares Rust `1.99`.
The `rom-dev` container provides `rustc 1.99.0 (b940084d7 2026-09-28)`.
The host has Docker. No goal-specific database containers were running during inspection.

The working tree exposes `rom::storage_support::{metadata, work}`. HEAD does not contain `crates/rom/src/storage_support.rs`.
The referenced architecture chat is still measuring and changing native storage.
SQL integration must wait for a committed revision with these exports and verified consumer compatibility.
Other families can use an earlier immutable revision when their required public boundary is present.
Do not replace incremental storage with complete-ledger serialization to bypass this gate.

## Task 1: Repository and reproducible boundary consumer

**Files:** Create `/root/ROM-extras/{Cargo.toml,LICENSE,README.md,.gitignore}`, `docs/{quality.md,writing.md,support.md}`, `scripts/check`, `tests/public-consumer/{Cargo.toml,src/main.rs}`.

**Interfaces:** Consume the chosen ROM Git revision. Produce an independently buildable workspace and `./scripts/check`.

- [ ] Verify the chosen commit exports `Storage`, `Bundle`, `StorageOwner`, `StorageMetadata`, `prepare_bundle`, `WorkRead`, and `WorkDelta`.
- [ ] Record that revision, toolchain, dependency license/advisory checks, and feature selections in `docs/support.md`.
- [ ] Create `pmikstacki/ROM-extras`, preserving the existing ROM project's MIT license and public open-source intent.
- [ ] Configure GitHub Actions as disabled.
- [ ] Add an external consumer that declares a custom field using only public ROM interfaces and imports the adapter support exports.
- [ ] Run the consumer independently of workspace feature unification. Require a successful build before storage integration.
- [ ] Add formatting, Clippy with denied warnings, workspace tests, doctests, warning-free docs, MSRV, and consumer checks to `scripts/check`.
- [ ] Commit only this repository's sources and evidence. Do not commit concurrent ROM changes.

## Task 2: Shared SQL protocol and bounded executor

**Files:** Create `crates/rom-sql-core/src/{lib.rs,ports.rs,store.rs,commit.rs,reads.rs,schema.rs,journal.rs,work.rs,ownership.rs,executor.rs}` and `tests/{protocol.rs,executor.rs}`.

**Interfaces:** Produce `SqlStore<D>: rom::Storage`; `SqlDriver: Send + Sync + 'static` supplies native transaction operations. Transaction ports consume canonical ROM keys, rows, receipts, descriptors, metadata, and Work deltas. Final port signatures must be checked against the committed ROM API before implementation.

- [ ] Define typed transaction operations for bounded authoritative reads, receipt arbitration, conditional Resource writes, metadata locking, Work deltas, and commit classification.
- [ ] Add a deterministic transactional fixture with failpoints at each write and acknowledgement boundary.
- [ ] Assert atomic all-or-none bundles, fingerprint mismatch, stale revision rejection, and replay without duplicate events.
- [ ] Assert create-if-absent, tombstones, protected metadata, references, schema registration, retry epochs, and journal retention boundaries.
- [ ] Assert atomic completed Work plus new Work enqueue, and reject stale claim tokens.
- [ ] Implement through ROM's public preparation helpers. Keep policies and application codecs outside native locks.
- [ ] Implement a bounded executor with explicit queue capacity and connection limits. Report `Overloaded` before admission and honest outcomes after admission.
- [ ] Test shutdown, queue saturation, cancellation, delayed acknowledgement, and calling Storage from a Tokio worker.
- [ ] Implement owner generations checked in every mutating transaction. Test a surviving pooled connection after takeover.
- [ ] Apply the fence to registration, reactions, operator controls, and maintenance as well as action commits.
- [ ] Define native guard lifetime when a store handle outlives its Runtime; `StorageOwner` has no public native release callback.
- [ ] Use a locked metadata publication boundary for journal order. Test that a delayed earlier writer cannot be skipped by a cursor.
- [ ] Run focused tests, independent consumer compilation, and the local verifier before committing.

## Task 3: PostgreSQL and MSSQL real-server adapters

**Files:** Create `crates/rom-{postgres,mssql}/src/{lib.rs,driver.rs,transaction.rs,schema.rs,errors.rs,owner.rs}`, `tests/sql-conformance/{src/lib.rs,tests/postgres.rs,tests/mssql.rs}`, `infra/sql/compose.yaml`, and `scripts/check-sql`.

**Interfaces:** Produce provider stores implementing `Storage` through `rom-sql-core`. Each provider supplies a fresh `rom_conformance::storage::StorageFixture` factory.

- [ ] Research and pin driver and server versions. Record TLS, licensing, advisory, MSRV, and durability requirements.
- [ ] Provision isolated databases with task-specific names and persistent test evidence.
- [ ] Run `rom_conformance::storage::basic(factory)` against each actual server.
- [ ] Add SQL races for identical identities, distinct fingerprints, and the same expected revision.
- [ ] Test exact byte identity semantics for case differences, Unicode, empty IDs where allowed, and maximum key lengths.
- [ ] Test subprocess interruption before commit and after commit before acknowledgement; reopen and recover using the original identity.
- [ ] Restart each server and verify acknowledged state, receipts, journal, and Work.
- [ ] Test owner connection loss, reconnect, takeover, stale generation rejection, and coherent bounded query reads.
- [ ] Test old, malformed, and future format markers without schema writes on rejection.
- [ ] Test confirmed rollback versus unknown acknowledgement errors. Do not infer rollback from transport failure.
- [ ] Publish separate PostgreSQL and MSSQL support profiles only after their required checks pass.

## Task 4: MySQL, MariaDB, CockroachDB, and Oracle

**Files:** Create `crates/rom-{mysql,cockroach,oracle}/src/{lib.rs,driver.rs,transaction.rs,schema.rs,errors.rs,owner.rs}` and corresponding real-server fixtures under `tests/sql-conformance/tests/`.

**Interfaces:** Implement Task 2's ports. MySQL and MariaDB may share transport code but have distinct server profiles and evidence.

- [ ] Research native transaction, isolation, lock, numeric, collation, restart, and durability behavior for each server.
- [ ] Run every Task 3 conformance and recovery scenario against each server independently.
- [ ] For MySQL and MariaDB, test statement-only failure and require explicit whole-operation rollback before retry.
- [ ] For CockroachDB, test transaction restart/retry and prove an ownership fence without assuming PostgreSQL advisory-lock compatibility.
- [ ] For Oracle, verify installed client dependencies, bind limits, empty-string behavior, and supported numeric representations.
- [ ] Reject unsupported representations explicitly. Do not silently coerce ROM values or fingerprints.
- [ ] Record real backend evidence and packaged-consumer results before declaring support.

## Task 5: Durable delivery, webhooks, brokers, and notifications

**Files:** Create `crates/rom-delivery-core/src/{lib.rs,message.rs,outcome.rs,policy.rs}`, `crates/rom-{webhook,kafka,nats,rabbitmq,notifications}/`, and `tests/delivery/`.

**Interfaces:** Consume ROM `Delivery<P>`, `DeliveryOutcome`, and channel registration. Use the stable `Delivery.id` as a provider idempotency key where supported.

- [x] Write the delivery-family spec against the committed channel registration API before adding provider code.
- [ ] Test committed intents survive storage reopen and invoke each real provider.
- [ ] Test bounded retry, duplicate attempts, permanent rejection, and lost external acknowledgement.
- [x] Implement webhook signatures and explicit destination policy. Test redirects, resolved private addresses, payload limits, and non-success responses.
- [ ] Implement Kafka, NATS, and RabbitMQ independently. Require real broker acknowledgement and restart/redelivery tests.
- [ ] Implement SMTP/email, an SMS provider, and a collaboration notification provider with configurable destinations and secret references.
- [ ] Preserve unknown outcomes when the receiver cannot establish acceptance. Do not advertise external exactly-once effects.
- [ ] Run each provider's required fixture and consumer checks before declaring its profile supported.

## Task 6: OpenTelemetry

**Files:** Create `crates/rom-opentelemetry/src/{lib.rs,observer.rs,attributes.rs,export.rs}` and `tests/telemetry/`.

**Interfaces:** Bind the public ROM observation boundary available in the selected revision. If no such boundary exists, document and coordinate a public upstream addition.

- [ ] Define action, commit, Work, and recovery correlation in a focused spec.
- [ ] Use the committed `DiagnosticReader` interface; document dropped records and host-local sequence semantics.
- [ ] Test a successful action, rejected mutation, unknown commit, receipt replay, and Work retry.
- [ ] Send spans and metrics to a real local OTLP collector.
- [ ] Assert Resource values, credentials, and protected metadata are absent from exported attributes by default.
- [ ] Test bounded buffering and collector failure without changing mutation outcomes.

## Task 7: Blob, identity presets, secrets, and KMS

**Files:** Create `crates/rom-{blob-presets,oidc-presets,secrets,kms}/` and `tests/{blob,identity,secrets}/`.

**Interfaces:** Extend `rom_blob::BlobStore`, existing `rom-blob-object-store::Adapter`, and `rom_auth::OidcIdTokenAdapter`. Secret resolution belongs to host configuration.

- [ ] Review existing S3 and Azure support before selecting genuinely missing operations or presets.
- [ ] Verify upload, digest, metadata, deletion, missing objects, and interrupted requests against real S3-compatible and Azure Blob profiles.
- [ ] Add OIDC presets for explicit issuers and audiences. Test expiry, issuer mismatch, key rotation, and principal-kind preservation.
- [ ] Define a bounded secret resolver and KMS encryption/decryption API in a family spec.
- [ ] Test invalid secret references, denied access, rotation, KMS context mismatch, and unavailable services.
- [ ] Verify credentials and plaintext secret material cannot enter Resource values, journal, or logs through these integrations.

## Task 8: OpenSearch and vector projections

**Files:** Create `crates/rom-projection-core/`, `crates/rom-opensearch/`, `crates/rom-vector/`, and `tests/projections/`.

**Interfaces:** Consume bounded ROM journal pages and stable event identities. Persist projection checkpoints only after acknowledged index writes.

- [ ] Specify rebuild, checkpoint, deletion, authorization, and query limits before choosing a vector backend.
- [ ] Implement OpenSearch and one real vector backend, with independently tested profiles.
- [ ] Test duplicate delivery, crash between indexing and checkpoint, stale revisions, tombstones, and journal retention gaps.
- [ ] Rebuild into a fresh generation and atomically switch the active projection after verification.
- [ ] Apply current ROM authorization before returning projected results. Test denied and newly revoked Resources.
- [ ] Document eventual consistency and prove bounded query and payload behavior.

## Task 9: Backup, migration, and provenance-preserving import

**Files:** Create `crates/rom-extras-maintenance/`, `crates/rom-import/`, `tools/rom-extras/`, and `tests/maintenance/`.

**Interfaces:** Extend existing ROM maintenance and archive boundaries. Imports invoke ordinary actions with `SourceProvenance` through the committed public source-authoring API.

- [ ] Specify SQL archive/export contracts, format markers, dry-run reports, and cutover fencing.
- [ ] Resolve `rom_backup::Backend`, which currently permits only SQLite and redb; never disguise SQL data as either backend.
- [ ] Test backup, restore to a fresh destination, interrupted migration, checksum failure, and incompatible versions for every SQL profile.
- [ ] Keep external blob backup scope explicit and test referenced object verification.
- [ ] Implement bounded file and HTTP imports using stable source identities and revision conditions.
- [ ] Test repeated imports, changed source revisions, denied actions, malformed records, and unknown outcomes.
- [ ] Verify source attribution survives commit and import never bypasses Runtime authorization with table writes.

## Task 10: Whole-goal acceptance and release

**Files:** Maintain `docs/support.md`, `docs/verification/`, package examples, migration notes, and `scripts/check-all`.

- [ ] Map every named goal extension to implemented packages, examples, real backend evidence, and a declared profile.
- [ ] Run the full local verifier, every required service fixture, and independently packaged consumers.
- [ ] Record source revision, dirty changes, lockfile, compiler, command, server version, settings, and result.
- [ ] Review compatibility, dependency advisories/licenses, limits, and upgrade instructions.
- [ ] Commit and publish reviewed sources to ROM-extras. Preserve unsuccessful experiments and historical evidence.
- [ ] Mark the goal complete only when every named extension is implemented and its required evidence is present.

## Plan review and execution

This is the whole-goal plan. Family-specific specs must pin final APIs and provider selections before those families are implemented.
Read the [public boundary audit](../../research/rom-extras-public-boundary-audit-2026-10-08.md) before selecting the baseline or implementing integration seams.
Read the [SQL driver assessment](../../research/rom-extras-sql-driver-assessment-2026-10-08.md) before resolving driver dependencies and backend profiles.
Execute the tasks in this chat. Keep independent integration work moving when a particular backend or upstream revision is unavailable.
The user approved this plan and execution in the current chat on 2026-10-08.
The user also requires research-backed decisions; see ../../goal.md and ../../research/decision-log.md.
