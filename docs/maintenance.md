# Native maintenance tools

rom-extras-maintenance provides host-only backup, archive inspection, fresh restore and typed migration over published ROM contracts.
It reuses ROM archive version6 and storage format8 at public commit d7ef529040eec60dc869034c2d33130219db85fe.
This increment supports the existing native SQLite/redb boundary. It does not implement external SQL backup or complete family acceptance.

## Select capabilities

The default feature set provides archive inspection and typed migration preview without native database dependencies.
Enable sqlite, redb or both to select backup, restore and native migration adapters. Disabled restore/migration fails before filesystem access.

| API | Contract |
| --- | --- |
| backup | Accepts an already-open native handle; coherent snapshot and private non-overwriting archive publication. |
| inspect | Fully validates the archive and returns backend, format and aggregate counts; no Resource data. |
| preview_migration | Executes the compiled host plan in memory; no destination publication. |
| restore | Validates and reconstructs a fresh native destination; returns Arc<dyn rom::Storage> for host composition. |
| migrate_from | Requires an offline existing source and fresh destination; applies the compiled host plan without changing source bytes. |

Backup accepts only sealed enabled native adapters. Public native open constructors can create a missing source; this tool supplies no offline backup opener.
The host can back up its already-owned database while writes continue. The native engine supplies the coherent transaction.

## Host integration

Authorize maintenance outside Resource-facing routes. Use a bounded blocking executor; these synchronous operations have no physical cancellation guarantee.
Keep the source handle alive through backup. Close source admission and drain the deployment before native migration or application cutover.

```rust
use rom_extras_maintenance::{backup, Error, Limits, Report};
use std::path::Path;

fn checkpoint(store: &rom_sqlite::Sqlite, private_archive: &Path) -> Result<Report, Error> {
    backup(store, private_archive, Limits::new(32 * 1024 * 1024, 100_000)?)
}
```

Typed migration uses the public MigrationPlan and ResourceMigration. Converters must be pure and bounded; they are not sandboxed.
An explicit unfinished-work validator must attest frozen contract compatibility. It cannot rewrite obligations.
A preview validates one archive at one time. It does not authorize later publication or ensure the archive remains unchanged.

Successful restore fences destination journal cursors and old Work claims. It does not stop the old deployment or disable its external deliveries.
The host must close the source deployment before routing writes to the destination. Never use both deployments as an active-active pair.

## Operator CLI

Build the enabled native profiles before restoring:

```sh
cargo build --locked -p rom-extras-maintenance --features sqlite,redb
target/debug/rom-extras-maintenance inspect sqlite /srv/private/checkpoint.rombk --max-bytes 33554432 --max-records 100000
target/debug/rom-extras-maintenance restore sqlite /srv/private/checkpoint.rombk /srv/private/fresh.db
```

Use redb instead of sqlite for that exact archive backend. No SQL backend alias is accepted.
Migration stays in the compiled host library; the CLI does not execute converter scripts.
Successful inspection prints count-only JSON. Successful restore prints the required host-cutover flag and explicit external-data exclusions.
Failure prints a fixed error code and exits2. Unknown exits3 and retains uncertainty; never automatically replace, delete or retry that destination.
Private paths, values, keys, attribution and native/converter diagnostics are omitted. Non-UTF8 CLI arguments fail with Invalid.

## Budgets and filesystem contract

Limits require positive values through128MiB and400000 aggregate records; defaults equal these ceilings.
Budgets cover serialized archive size and aggregate record admission, not absolute heap allocation or elapsed operation time.
Use trusted local Linux parent directories and private archives. Archives require permissions0600.
The upstream contract excludes network filesystems, live path replacement and hard-link aliases. Never remove a persistent .rom-owner sidecar to recover access.
Publication refuses an existing destination and uses upstream staged hard-link/fsync semantics.
A post-publication failure may be Unknown; rollback or destination absence must not be inferred from that result.
SHA-256 detects accidental corruption. It is not encryption, origin authentication or protection from a hostile archive author.

## External objects and packaging

Archives include internal durable intents/Work. They exclude external blob bytes, delivery execution and provider state.
Separate bounded inventories, copy/restore and digest checks remain required; database restore alone does not recover object stores.

Cargo normalizes Git dependencies to registry versions in package archives.
The tested archive consumer supplies explicit patches for rom, rom-backup, rom-sqlite and rom-redb to the same public immutable commit.
No local dirty ROM checkout or private API is substituted. Matching registry publication remains a prerequisite for use without those patches.
See [the primary research and alternatives](research/maintenance-native-tools-2026-10-10.md).

## Executed checks and remaining acceptance

The independent working example is tests/maintenance-public-consumer; the no-native consumer is tests/maintenance-core-consumer.
Run ./scripts/check-maintenance for feature variants, public consumers, normalized archives and actual CLI/native scenarios.
Run ./scripts/check-all before integration. Preserve each private run directory and failure log.

Source trials executed actual Runtime create/action/tombstone, restore replay, cursor fencing, typed preview/migration and native reopen on both engines.
Native trusted persistence fixtures also checked protected provenance, claimed Work restoration and explicit unfinished-work validation.
Backup windows witnessed acknowledged write progress and fully validated coherent snapshots. Native collection and write transactions remain serialized by the shared handle.
The CLI ran real inspect/restore plus private-path, non-UTF8, limit and repeated-destination negatives.
A separately labeled upstream test-support callback interrupted staged native migration before publication; both source files remained unchanged.
This callback is not a production facade API or a real post-publication uncertainty test.

Source and normalized archive consumers passed the same native and CLI scenarios, including future-format, permissions and symlink negatives.
Registry graphs use 85 packages for native consumers and 30 for the no-native consumer; no root registry package was added.
Audits found zero known Rust vulnerabilities. The root retains the existing unsuppressed paste maintenance warning; the four independent graphs have no warnings.
The final source reviewer closed the live-write witness finding after correction.
The full local verifier passed; all 1424 frozen files remained unchanged during execution.
See [the executed verification record](verification/maintenance-native-tools-2026-10-10.json).
All external SQL profiles, external-object restore verification, post-publication failure qualification, legacy-format coverage and host cutover journeys remain required.

## External objects

The [blob recovery tool](blob-recovery.md) verifies a host-approved external inventory through public provider ports.
Its controlled host example binds captured physical keys to SQLite/redb checkpoints and reads restored Resources through the destination provider.
The [checkpoint inventory tool](blob-checkpoint.md) persists exact current Ready built-in Blob bindings after isolated public-service reads.
Completeness across all Resource families, cloud qualification and deployment cutover remain required.
