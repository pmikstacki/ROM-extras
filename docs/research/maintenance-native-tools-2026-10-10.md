# Native maintenance tool research

Inspection date: 2026-10-10. Source review only; no builds, installs or native lifecycle operations. Public ROM pin: d7ef529040eec60dc869034c2d33130219db85fe. Read extras writing and quality rules. Parent independently rechecked upstream HEAD. This note does not extend SQL support.

## Recommendation

Implement a small `rom-extras-maintenance` host library that dispatches existing SQLite/redb maintenance APIs. Add a CLI for bounded archive verification and restore-to-fresh-path. Offer backup on already-open host-owned native handles. Offer typed migration through the library, accepting the upstream MigrationPlan unchanged. This provides useful maintenance without recreating serialization, publication or policy.

Keep capability reporting explicit: SQLite/redb archive/restore/migration are available upstream; six extras SQL families currently expose ownership groundwork, not full maintenance export. Backend is exactly `Sqlite | Redb`. Never label SQL data as either variant. See [public backup model](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-backup/src/model.rs).

## Public seams and guarantees

Both native adapters expose:

```rust
pub fn backup_to(&self, path: impl AsRef<Path>, limits: BackupLimits) -> Result<Manifest>;
pub fn restore_from(archive: impl AsRef<Path>, destination: impl AsRef<Path>, limits: BackupLimits) -> Result<Self>;
pub fn migrate_from(source: impl AsRef<Path>, destination: impl AsRef<Path>, plan: &MigrationPlan, limits: BackupLimits) -> Result<Self>;
```

SQLite collects one coherent read transaction, including committed WAL state, then releases it before archive publication. Redb takes its commit gate and one consistent read transaction, then releases both before publication. These are logical snapshots, not filesystem copies. Sources: [SQLite maintenance](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-sqlite/src/maintenance.rs), [redb maintenance](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-redb/src/maintenance.rs).

Archive APIs are `write(path, Backend, &Snapshot, BackupLimits)->Result<Manifest>` and `read(path, Backend, BackupLimits)->Result<(Manifest, Snapshot)>`. Current archive version is6, native storage format8. Read verifies backend, version, checksum, bounds and logical state. Write refuses overwrite. SHA-256 detects corruption; it does not authenticate hostile modification. Protected values occur in archives. Do not print Snapshot or include it in Resource/operator responses. Sources: [archive](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-backup/src/archive.rs), [backup facade](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-backup/src/lib.rs).

## Ownership and opening side effects

`NativeOwnership::acquire(&Path, NativeAccess)->Result<Self>` provides Existing, Fresh and OpenOrCreate modes. Existing refuses absent source without creating source or sidecar. Fresh rejects existing destination while retaining reservation. Ownership is noncloneable, Linux-local and cooperating-process scoped. Never remove `.rom-owner` to recover a lock. Hard-link aliases, live replacement and network filesystems are excluded. See [native ownership](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-backup/src/native_ownership.rs).

Public adapter open uses OpenOrCreate. `open_owned` is private. Therefore a standalone path-backup command cannot acquire Existing and transfer that guard through a public constructor. A filesystem existence precheck followed by ordinary open has a race; do not claim it is a read-only source admission guarantee. Prefer an already-open host handle now. An upstream public existing-only constructor is an alternative, not an available API. Inspect [SQLite store](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-sqlite/src/store.rs) and [redb store](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-redb/src/store.rs).

Adapter operator helpers in operator.rs are pub(crate), not an external raw-control seam. Do not bypass public Runtime authorization using private helper copies. Offline filesystem maintenance is host administration; ordinary operator controls retain their upstream authorization protocol.

## Restore, publication and cutover

Restore validates archive before fresh destination acquisition. It stages a reconstructed database, validates native reads, closes and publishes it, then reopens under retained ownership. SQLite additionally checkpoints WAL and selects DELETE journaling before publication. Failure after native commit/publication can be Unknown. Do not translate Unknown into safe rollback or delete the destination automatically. See [SQLite restore](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-sqlite/src/maintenance.rs).

Stage publication syncs its file, creates a non-overwriting hard link and syncs the parent directory. Existing destination becomes Conflict; post-link directory-sync failure becomes Unknown. Native cleanup deliberately retains uncertain destinations. Trusted same-filesystem directories with hard-link/fsync support are prerequisites. Reuse Stage through native methods; do not add a second rename/publication protocol. See [publication](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-backup/src/publication.rs).

Restore preparation fences old cursors and in-flight claims in the destination. It does not automatically stop the old deployment or its external deliveries. Host cutover must stop admission and drain/close the source deployment before routing to the destination. Never claim dual deployment writes are fenced by fresh-destination publication alone.

## Typed migration

`ResourceMigration::new<Before: Resource,After: Resource>(fn(Before)->Result<After>)` validates one consecutive version for the same kind. `MigrationPlan::new(Vec<ResourceMigration>)` rejects no steps. `validate_work(fn(&PendingWork)->Result<()>)` attests unfinished-work compatibility. Plans contain executable host converters, not a general JSON mapping language. See [migration plan](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-backup/src/migration_plan.rs).

`migrate_snapshot` validates bounded complete history, receipts and obligations; conversions do not add business events or change revisions. Native migrate_from opens an Existing source without changes and publishes a Fresh destination. SQLite documents legacy format4 through7 plus current. Native publication fences destination claims/cursors. Keep conversion and cutover distinct. Sources: [shared migration](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-backup/src/migration.rs), [SQLite migration](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-sqlite/src/migration.rs).

## External objects

Manifest explicitly excludes external blobs and deliveries. Optional trusted host verification can accept a bounded explicit inventory of store, ObjectKey, digest and expected length. Reuse public BlobStore head/get and Digest, checking bytes and recomputed digest. HEAD length alone is insufficient. Do not promise a coherent external-object snapshot from sequential calls, or infer complete custom blob inventories by scanning arbitrary Resource JSON. Source: [BlobStore](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-blob/src/storage.rs).

Verification is read-only. Physical cleanup remains separate and requires detachment, grace and quiescence. No restore verification should call delete. Backend ETag is not a universal content digest.

## Alternatives and qualification

SQLite Online Backup API copies a native snapshot and can operate incrementally. It does not supply ROM archive migration, fenced restoration or logical validation. It is a possible physical backup profile, not a replacement for logical maintenance. See [official SQLite backup](https://www.sqlite.org/backup.html). Redb durability settings likewise do not replace the shared publication protocol; see [redb4.3.0 Durability](https://docs.rs/redb/4.3.0/redb/enum.Durability.html).

Reuse pinned rom-backup, rom-sqlite and rom-redb without a new archive dependency. Pinned workspace declares MIT and Rust1.99; rom-backup directly depends on rom, serde, serde_json and sha2=0.10.9. This source inspection is not a new advisory/native-library audit. Recheck the effective independent consumer graph and features. Sources: [workspace manifest](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/Cargo.toml), [backup manifest](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-backup/Cargo.toml).

Native public and packaged qualification should execute both backends: coherent backup after mutation, verified manifest, fresh restore, exact historical receipts/events, old cursor/claim rejection, restart, typed migration, existing destination refusal, missing migration source no-creation, source ownership conflict, wrong backend/format, corruption and byte/record bounds. Add malformed explicit blob inventories and actual digest mismatch tests if verification is included. Keep bounded blocking execution and protected-output checks.

Fault injection after publication may require upstream test-support APIs or controlled filesystem faults. Do not introduce production callbacks solely to fake a wire/publication failure. Archive verification alone is not executed restore validation. SQL maintenance remains open until public full Storage/native export integration exists; this increment must report that gap explicitly.

## CLI argument admission

The native CLI test observed non-UTF8 input exit101 instead of the required safe Invalid exit2.
Rust [args](https://doc.rust-lang.org/std/env/fn.args.html) can panic on invalid Unicode; [args_os](https://doc.rust-lang.org/std/env/fn.args_os.html) retains OS arguments.
Use args_os with checked conversion and fixed Invalid error, without retaining or printing the failed path.
This correction still rejects unsupported filesystem argument encoding; it does not silently change path bytes.
