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
Run the independent consumer with `cargo test --manifest-path tests/public-consumer/Cargo.toml --locked --features projection-checkpoints --test projection_checkpoints`.
Run `./scripts/check-projection-package` for a consumer of the normalized Cargo archive.
The archive uses explicit patches to immutable public ROM `d7ef529040eec60dc869034c2d33130219db85fe`; it does not claim registry publication.
Both consumer gates are required by the repository verifier.

Initial bounds include 64 operations/kinds, 128-KiB records, 2-KiB key records, 8-MiB native cache, and 256-MiB private-copy recovery admission.
The native engine can allocate pages before application length checks. The cache setting is not a universal process-memory ceiling.
Startup validates records incrementally without collecting the key table. Startup cancellation is checked between scan segments and 64-KiB private-copy chunks.

Remaining Task 2 work includes the bounded worker, retained authorized-history reconstruction, and worker lifecycle/cancellation.
Actual OpenSearch/Qdrant delivery, native ROM feeds, current authorized search, alias switching, and rebuild remain separate required tasks.
The existing native service probes do not qualify those missing runtime integrations.

The [full verification record](verification/projection-checkpoint-full-verifier-2026-10-08.json) contains the tested source hashes and acceptance limits.
The independent consumer retains an informational warning for an existing unmaintained Oracle-driver dependency; see the decision log.

## Cooperative startup cancellation

Use `Cancellation::new()`, clone its token, and call `cancel()` from the controlling thread. The request cannot be reset.
`open_cancellable` and `reopen_cancellable` check the request before admission, during application scans, and between copy chunks.
Native repair also receives a redb abort callback. Native I/O and repair steps have no fixed cancellation deadline.
A request present before reopening leaves the existing engine usable. Cancellation after reopening starts leaves it retired with ownership retained.
A failed open releases its reservation after native handles close. Successful later admission uses a fresh, unrequested token.
Cancellation does not classify a commit as absent. The API never interrupts checkpoint writes.
Original repair is permitted only after private-copy acceptance. Cancellation during permitted original repair can occur after native bytes change.
The dedicated worker thread, bounded command admission, shutdown/join, and retained-history reconstruction are still required.

The [cancellation verification record](verification/projection-cancellation-full-verifier-2026-10-08.json) identifies actual tests, unchanged dependency graphs, and missing worker acceptance.

## Bounded storage owner

`StorageWorker` creates or opens its checkpoint on one dedicated native thread. Its queue admits one waiting command plus active work.
Only fixed typed methods submit checkpoint work; callers cannot submit arbitrary native transaction closures.
Await `StorageResponse::receive` for the classified operation result. Queue admission is not a commit acknowledgement.
A full queue returns `StorageFailure::Overloaded` before admitting that command. Closed admission returns `Closed`.
A classified checkpoint failure preserves its exact error and uncertain transaction token. A lost admitted response returns `Unclassified`, without rollback permission.
Dropping a response or its awaiting future does not cancel admitted work. Inspect durable state after later ordered load or reopening.
`shutdown` closes admission, drains accepted work and joins through native engine destruction. Dropping the owner also joins.
Creation, opening, shutdown and drop can block on native I/O. Call them from a bounded blocking host context.
No hard I/O or shutdown deadline is qualified. Queue bounds exclude caller-held responses and native engine allocations.
The storage owner reuses `rom-sql-core::Executor`; it enforces join instead of the generic executor's detach-on-drop behavior.
The normalized archive consumer extracts both crates and exercises an asynchronous storage response before shutdown and independent reopen.
Page orchestration, actual provider writes, retained authorized-history reconstruction, and the host's asynchronous lifecycle bridge remain required.

The [storage-owner verification record](verification/projection-storage-worker-full-verifier-2026-10-08.json) preserves the failed fixture stop and successful full retry with exact source identity.

## Pending authorized-history reconstruction

`PendingHistory` starts from a persisted intent's original expected cursor. Fetch each batch through public `Runtime::journal` with the export actor.
Pass the authorized batch and immutable mapping to `push`. The component retains metadata, not projected values.
It validates the entire batch before selecting events within the original endpoint, including historical tombstones.
The mapper cannot substitute keys, positions, revisions or tombstone status. Digest determinism and export authorization remain host responsibilities.
Repeated keys collapse across batches, with nondecreasing revisions and identical content at equal revisions.
`finish` requires inspected coverage and complete reconstructed intent equality. Changed disclosure or mapping returns `RebuildRequired`.
This adds an error variant to the unreleased crate. Update exhaustive `Error` matches to handle `RebuildRequired`.
Changed generation, invalid ordering or insufficient coverage returns `HistoryGap`. A cursor alone cannot prove unchanged disclosure.
Limits are 64 fetches, 4096 returned views, 4096 global inspected positions and 64 distinct retained keys.
The inspected-position budget includes final overrun, unrelated kinds and denied events. Larger spans return `TooLarge` without publication.
A pre-cancelled push preserves the component for a fresh token. Other failures make the reconstruction terminal.
A mapper unwind leaves a terminal `RebuildRequired` state if the host catches the panic. Panics are not caught internally.
This component performs no checkpoint write or network dispatch. A successful finish is not a provider acknowledgement.
Synthetic public-shaped batches and actual pending files test the core seam. Actual SQLite/redb feed integration remains unqualified.
Approved documents, full page orchestration, actual providers and the bounded asynchronous host lifecycle bridge remain required.

## Immutable approved documents

Create `DocumentMapping` with an input profile, unique selected top-level field names and optional fixed vector dimension.
Use its derived `profile()` for checkpoint creation/opening and provider configuration. Selection or dimension changes produce a different durable mapping identity.
`document` selects only fields already present in an authorized historical `JournalView`. It does not grant export authority.
Missing fields remain absent; null, booleans, integers, floats, strings, arrays and objects retain separate content identities.
A tombstone removes selected values and vectors. A live vector mapping requires the exact configured dimension and a profile model identifier.
The document is immutable. Explicit getters expose approved values to the qualified transport; Debug exposes no values, keys or vectors.
Approved selected values have a 16 KiB canonical limit, 4096 nodes and 32 recursive levels. Vectors have 1..4096 finite f32 values, with normalized zero.
The complete encoded wire request/response still requires the separate1MiB provider gate. Canonical value size does not prove wire size.
Numeric admission preserves u64/i64 exactly and accepts canonical finite f64 representations. Numeric formatting is bounded to 64 bytes before conversion.
Arbitrary-precision values outside these domains, including non-roundtripping decimal floats, return `Invalid`; overlong numeric tokens return `TooLarge`.
This is a binary digest format, not JCS. Provider numeric compatibility, metric/profile checks and real feed authorization remain required.

The SHA-256 mapping domain is `ROM-extras/projection-mapping/v1`; the document domain is `ROM-extras/projection-document/v1`.
Lengths and counts are u32 big-endian. A frame is length followed by exact bytes. Profile bytes use the checkpoint profile encoding.
Mapping identity hashes framed input profile, selected-field count, framed sorted names and dimension (zero means absent). Its lowercase hex replaces mapping identity.
Document identity hashes framed derived profile, the same selector/dimension configuration, framed kind/ID, u64 revision and one tombstone byte.
It then hashes framed selected-value encoding, a vector-presence byte and finite f32 big-endian bits. Journal position is excluded.
Selected-value encoding is an object: tag8,count, then sorted framed UTF-8 names and typed values. No Unicode normalization occurs.
Value tags are0 null,1 false,2 true,3 u64,4 negative i64,5 finite f64 bits,6 framed UTF-8 string,7 array and8 object.
Integer/float words are big-endian; float zero bits normalize to positive zero. Arrays encode count and ordered values; objects encode count and sorted entries.
Tombstones encode an empty selected-value frame and no vector. The profile/configuration remains bound even when values are absent.

## Core page orchestration

Worker::apply_page validates selected documents and prepares an owned target request before durable intent.
It awaits native preparation before target dispatch. Exact stored-state observations allow atomic local completion.
Unknown target outcomes retain pending intent and the old cursor. Later pages require recovery first.
Worker::recover reconstructs and compares the complete old interval before replay. It never publishes an overrun endpoint.
Admission uses the same 4096-position interval limit as recovery. Recovery checks the 64-fetch budget before fetching.

Cancellation after target acceptance retains pending work. Dropping a future does not prove remote or native rollback.
An admitted local completion returns its classified outcome despite late cancellation. Native uncertainty tokens remain unchanged.
The target must enforce wire/numeric/metric bounds and fencing. The source must enforce immutable export authorization.
These trusted interfaces do not establish native adapter correctness.
Construction, error cleanup, destruction and shutdown require a blocking host context. The asynchronous lifecycle bridge remains unfinished.
Actual SQLite/redb authorization/history and Rust OpenSearch/Qdrant transport acceptance remain required.

The [worker verification record](verification/projection-worker-full-verifier-2026-10-09.json) records core composition and independent consumer evidence.
