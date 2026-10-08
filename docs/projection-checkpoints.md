# Projection checkpoints

Status: checkpoint-store implementation; projection worker and provider adapters remain incomplete.

`rom-projection-core` owns a separate redb 4.3.0 file and uses public ROM keys, cursors, and native ownership.
It does not mutate private ROM tables. ROM remains authoritative for Resources and authorization.
The [transaction contract](superpowers/specs/2026-10-08-projection-checkpoints.md) defines its format, bounds, and pending family requirements.

## Page protocol

Validate the entire ordered page before preparing its immutable intent.
Preparation persists metadata and digests before network dispatch. It does not advance the journal cursor.
Repeated keys retain their highest validated revision. Conflicting equal revisions fail before dispatch.
Filtered pages can advance without a backend operation.

The host adapter supplies actual stored identity, target/profile, revision, digest, and tombstone observations.
The core requires exact matches for the complete page before it constructs `ReconciledPage`.
A write acknowledgement, successful HTTP status, or conditional no-op is insufficient.
The observation constructor does not authenticate a host or backend. Qualified verified transport remains the adapter's responsibility.

Completion atomically publishes acknowledged key states, the next cursor, and retained completion metadata.
The last completed page remains bounded. Startup verifies its resulting cursor and every affected key before accepting its receipt.
Older receipts are not retained indefinitely. A new unresolved intent excludes conflicting later work.

## Commit and restart

Every write transaction explicitly uses Immediate durability. Fresh initialization also synchronizes its parent directory before acknowledgement.
Only trusted local Linux storage is admitted through public `rom_backup::NativeOwnership`.
The ownership sidecar persists. Do not delete it to recover ownership.
Live replacement, rename, hard-link deployments, network filesystems, and uncooperating writers are outside this profile.

An uncertain commit returns `Unknown` with an immutable 83-byte transaction token.
The engine closes while its ownership reservation remains held. The retired handle cannot accept more transactions.
Retain the token, reopen the store, and reconcile against durable transition-specific records.
Do not infer rollback from an I/O error. A controlled failure after actual synchronization can leave the transition applied.
If the token and an uncommitted intent are both lost, the store cannot classify that absent attempt.

Read-only preflight validates clean files before writable open.
When native repair is required, a bounded private copy is repaired and validated before original recovery is permitted.
Rejected future or malformed formats leave original bytes unchanged. Accepted-format dirty process recovery is implemented.
This does not establish arbitrary hardware power-loss behavior; the selected filesystem and devices must honor synchronization.

## Checks and limits

Run `cargo test -p rom-projection-core --locked` for real files, controlled native I/O faults, and bounded child-process interruption.
The child-only test entry point is invoked by parent tests; its harness ignore marker is not an absent-backend skip.
Run the independent consumer with `cargo test --manifest-path tests/public-consumer/Cargo.toml --locked --test projection_checkpoints`.
Run `./scripts/check-projection-package` for a consumer of the normalized Cargo archive.
The archive uses explicit patches to immutable public ROM `d7ef529040eec60dc869034c2d33130219db85fe`; it does not claim registry publication.
Both consumer gates are required by the repository verifier.

Initial bounds include 64 operations/kinds, 128-KiB records, 2-KiB key records, 8-MiB native cache, and 256-MiB private-copy recovery admission.
The native engine can allocate pages before application length checks. The cache setting is not a universal process-memory ceiling.
Startup validates records incrementally without collecting the key table. Worker cancellation between scan segments remains unimplemented.

Remaining Task 2 work includes the bounded worker, retained authorized-history reconstruction, and worker lifecycle/cancellation.
Actual OpenSearch/Qdrant delivery, native ROM feeds, current authorized search, alias switching, and rebuild remain separate required tasks.
The existing native service probes do not qualify those missing runtime integrations.

The [full verification record](verification/projection-checkpoint-full-verifier-2026-10-08.json) contains the tested source hashes and acceptance limits.
The independent consumer retains an informational warning for an existing unmaintained Oracle-driver dependency; see the decision log.
