# Next extension: exact blob checkpoint inventories

Research date: 2026-10-10. This record contains source review and design inferences, not executed behavior tests.
ROM-extras baseline: `9809d5df398be7af115b9b5fb6f16d4d1a0f2026`.
Public ROM dependency: `d7ef529040eec60dc869034c2d33130219db85fe`.
The local ROM working tree is not an implementation input.

## Recommendation

Implement exact checkpoint inventory collection and private durable publication before generic projection switching.
Treat the online append-before-create journal as a separate optional follow-up.
Keep two separate contracts: publication facts and exact checkpoint Resource bindings.
A create recorder alone cannot establish those bindings.
A dedicated read observer can establish them through the current public BlobService API.
This read-observation route is a design inference from pinned source. It needs a public-consumer proof before adoption.

## Public storage and archive facts

The public BlobStore receives physical keys and bytes. It receives no Resource ID, actor, revision, or checkpoint.
It has no enumeration method. Create must not overwrite existing keys. An unknown result does not establish rollback.
These are the complete public method signatures:

```rust
fn create<'a>(&'a self, key: &'a ObjectKey, bytes: Vec<u8>) -> StoreFuture<'a, ()>;
fn get<'a>(&'a self, key: &'a ObjectKey, max_bytes: usize) -> StoreFuture<'a, Vec<u8>>;
fn head<'a>(&'a self, key: &'a ObjectKey) -> StoreFuture<'a, Metadata>;
fn delete<'a>(&'a self, key: &'a ObjectKey) -> StoreFuture<'a, ()>;
```

Source: [pinned public storage contract](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-blob/src/storage.rs), checked 2026-10-10.

`Blob` publicly exposes `owner`, `store`, `digest`, `bytes`, `state`, and `upload_revision`.
The built-in worker actor can read Blob resources under the built-in definition.
Custom policies transfer responsibility to the host.
Source: [pinned Blob Resource](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-blob/src/resource.rs), checked 2026-10-10.

The public archive API is:

```rust
pub fn read(
    path: impl AsRef<Path>,
    backend: Backend,
    limits: BackupLimits,
) -> Result<(Manifest, Snapshot)>;
```

It validates the archive and returns public snapshot rows.
The manifest explicitly excludes external blobs and external deliveries.
Sources: [archive read](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-backup/src/archive.rs), [archive model](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-backup/src/model.rs), checked 2026-10-10.

`BlobService::upload` returns `Attached(Snapshot<Blob>)` or `Unattached { object, cause }`.
Only Unattached exposes the physical receipt. The private receipt helper must not become an extras key recipe.
`BlobService::read(&self, actor: &Actor, id: &str) -> Result<Vec<u8>>` computes its key internally.
It uses `upload_revision`, reads the object, verifies content, and checks the Resource revision again.
It rejects non-Ready states and zero upload revisions.
Source: [pinned service operations](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-blob/src/service/operations.rs), checked 2026-10-10.

The existing extras consumer captures successful creates in a Mutex vector.
It uses one object and one store. Therefore digest/length matching does not qualify multiple equal-content publications.
The recorder also loses process-crash evidence and records nothing after an unknown create result.
Source: [capture consumer](https://github.com/pmikstacki/ROM-extras/blob/9809d5df398be7af115b9b5fb6f16d4d1a0f2026/tests/blob-recovery-public-consumer/src/capture.rs).

## Implementable increment

1. Add a durable append-before-create publication journal around public BlobStore.
2. Record store identity, exact opaque key, digest, length, and an operation identifier before provider admission.
3. Record confirmed, conflict, or unknown outcomes separately. Never claim that missing final journal data proves provider absence.
4. Recover interrupted records by explicit reads and content verification. Do not automatically repeat create.
5. Read the validated archive and enumerate every live built-in Ready Blob row.
6. Bind each row to a key through a dedicated BlobService read observer.
7. Publish a bounded private inventory only after every required Ready row has verified binding evidence.

Steps 1–4 follow the public create uncertainty contract. Steps 5–7 are proposed completeness rules.
The inventory must retain exact Resource kind, ID, snapshot revision, upload revision, owner, store, digest, and length.
Its archive digest binds file bytes. Host authorization and provenance remain separate requirements.

The read observer is more useful than digest lookup. A host supplies an isolated archive-derived Runtime with its Resource definitions.
For each Ready row, a dedicated wrapper observes the physical key passed to get.
The service uses its own supported key logic. The wrapper delegates the read and verifies the expected digest and length.
The host compares the runtime row against the archive row before and after the read.
Only a successful service read and an exact row match produce a binding.
All calls through that observer must be exclusive. A shared last-key variable or timing correlation is insufficient.
SQLite and redb retain existing descriptors when they register a supplied subset.
A Blob-only Runtime can inspect mixed archives if the stored Blob descriptor matches the built-in definition.
Qualify this source-reviewed conclusion through an independent public consumer.

A missing original create journal does not necessarily prevent checkpoint recovery through this read-observation route.
It cannot restore deleted source content. It also does not discover unattached or unreferenced provider objects.
State the completeness scope explicitly: all current Ready built-in Blob rows in this validated archive.
Pending rows, Detached rows, historical journal values, and externally managed blob kinds need separate documented policies.
A native archive's external-blobs flag remains false; extras must not silently rewrite its format.

Use private durable journal storage with a documented sync policy.
Rust File drop ignores close errors; `sync_all` reports synchronization failures.
SQLite provides atomic transactions, but its synchronous setting affects power-loss guarantees.
Sources: [Rust File](https://doc.rust-lang.org/std/fs/struct.File.html#method.sync_all), [SQLite atomic commit](https://www.sqlite.org/atomiccommit.html), [SQLite synchronous](https://www.sqlite.org/pragma.html#pragma_synchronous), checked 2026-10-10.
A SQLite journal avoids inventing a second torn-record recovery format, but dependency and durability profiles need review.
Neither SQLite nor a synced append log atomically commits with a remote object store.

Acceptance scenarios should include equal bytes under distinct IDs, owners, revisions and stores; unrelated concurrent reads; missing source content;
archive mismatch; unsupported Blob schema; zero-length blobs; malformed upload revision; process interruption before and after provider admission;
confirmed create followed by journal failure; and restart reconciliation without another write.
No such tests were run during this research.

## Projection switching alternative

OpenSearch supports atomic `POST /_aliases` actions. A switch combines remove and add.
`must_exist: true` can require the named old association to exist. It is not a universal administrator fencing token.
Avoid `remove_index`, which deletes retained history.
Source: [official Manage Aliases API](https://docs.opensearch.org/latest/api-reference/alias/aliases-api/), checked 2026-10-10.

Qdrant supports atomic alias changes through `POST /collections/aliases`.
Delete names an alias only; create names an alias and collection. The documented API has no expected-old-collection predicate.
Thus a read-before-switch check does not provide compare-and-swap against concurrent administrators.
The API timeout is a wait-for-commit timeout; a service error does not establish rollback.
Sources: [official collection aliases](https://qdrant.tech/documentation/manage-data/collections/), [official alias API](https://api.qdrant.tech/master/api-reference/aliases/update-aliases), checked 2026-10-10.
These pages use current/latest documentation. A future implementation must pin and test its actual service versions.

The current ProjectionTarget port only exposes profile, physical target, prepare, and apply.
It has no alias-administration method, switch intent, expected alias association, or cross-backend readiness contract.
Source: [current public target port](https://github.com/pmikstacki/ROM-extras/blob/9809d5df398be7af115b9b5fb6f16d4d1a0f2026/crates/rom-projection-core/src/worker_contract.rs).

Qdrant Generation is host-owned and requires physical-name non-reuse plus exclusive administration during pending work.
Its generation check verifies metadata and configuration, not an immutable native UUID.
An alias named differently from the concrete physical target can be used by external readers without changing that writer binding.
However, alias switching does not strengthen pending-write or name-reuse guarantees.
Sources: [Generation contract](https://github.com/pmikstacki/ROM-extras/blob/9809d5df398be7af115b9b5fb6f16d4d1a0f2026/crates/rom-qdrant/src/generation.rs), [generation inspection](https://github.com/pmikstacki/ROM-extras/blob/9809d5df398be7af115b9b5fb6f16d4d1a0f2026/crates/rom-qdrant/src/reconcile.rs).

| Alternative | Immediate value | Required additional contract |
| --- | --- | --- |
| Durable create recorder only | Survives publication interruption | Exact Resource binding remains absent |
| Durable recorder plus Ready checkpoint audit | Recoverable publication facts and demonstrable current-row completeness | Isolated observed reads and explicit archive scope |
| Host-scripted alias switch | Native atomic reader routing | Host serialization, readiness and uncertainty handling |
| Generic projection generation switch | Reusable deployment workflow | New privileged port, durable intent, reconciliation, readiness and fencing |

The highest-impact increment closes the blob recovery consumer's concrete completeness gap.
Generic projection switching remains feasible as a later extras feature, but cannot honestly be claimed through existing ports alone.

## Follow-up: concrete checkpoint collector seam

The preferred first increment needs no online journal. Use an explicit fresh native inspection database and the existing extras restore tool.
Then register only `rom_blob::definition()`. Never register reactions or delivery channels. Never invoke Work execution.
Runtime construction creates no autonomous worker. The storage adapters merge supplied descriptors with the existing persisted catalog.
They validate the complete catalog and reject incompatible supplied descriptors.
Sources: [Runtime builder](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/execution/builder.rs), [SQLite registration](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-sqlite/src/references.rs), [redb registration](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-redb/src/references.rs), checked 2026-10-10.

The tool can use public `rom_blob::worker_actor()` with the built-in policy.
This is a privileged offline maintenance path. It must require trusted host-supplied archives and configured stores.
It must not accept a client-selected worker identity or expose protected archive rows through Resource endpoints.

The read-only BlobStore wrapper should reject create and delete without delegation.
It should observe exactly one get for each sequential service read and preserve the configured store label.
Different store labels can contain the same physical key. Recovery inventories must stay partitioned by logical store identity.
Reject unknown store labels before provider reads. Explicitly count excluded Pending, Detached and tombstone rows.

Cancellation needs particular care. BlobService::run spawns accepted operations; dropping a read waiter does not terminate its storage read.
Do not reuse observation state after cancellation. Drain the service, or retain isolated owned state until accepted work finishes.
An arbitrary provider can ignore cooperative cancellation. Do not promise a hard bound on shutdown completion.
Source: [BlobService lifecycle](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-blob/src/service/lifecycle.rs), checked 2026-10-10.

Reuse public `rom_backup::Stage` to persist the encoded inventory:

```rust
pub fn new(destination: &Path) -> rom::Result<Stage>;
pub fn path(&self) -> &Path;
pub fn publish(&self) -> rom::Result<()>;
```

Stage creates a private sibling directory and a mode-0600 file on Unix.
After the caller writes encoded bytes, publish syncs the file, hard-links it without overwrite, and syncs the parent directory.
A parent-directory sync failure returns Unknown after the destination link exists.
Use a trusted parent directory and a filesystem that supports hard links and directory synchronization.
No NativeOwnership algorithm needs duplication for this ordinary file publication.
Sources: [public Stage export](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-backup/src/lib.rs), [Stage implementation](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-backup/src/publication.rs), checked 2026-10-10.

The synchronous restore and Stage operations are not bounded by an asynchronous Context timeout alone.
Bound archive sizes before work and report filesystem publication uncertainty separately from source-read cancellation.
Preserve the inspection database as explicit test or operator evidence. Never automatically remove an existing database or inventory.
A private durable manifest is still unsigned. Authentication and garbage-collection suspension remain host responsibilities.
