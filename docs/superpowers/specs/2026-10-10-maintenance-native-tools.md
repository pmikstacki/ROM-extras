# Shared native maintenance tools

Date:2026-10-10. Approved whole-goal execution applies; this specification preserves all SQL maintenance requirements.

## Intent and authority

Implement a shared host-facing maintenance facade and operator CLI over the published ROM archive/native maintenance contracts.
Pin public ROMd7ef529040eec60dc869034c2d33130219db85fe. Keep existing APIs, working trees and evidence.
SQLite and redb are explicitly native profiles; they are not labels for external SQL providers.
Full SQL Storage/export/restore remains required and incomplete.

## Boundary

rom-extras-maintenance delegates archive parsing, coherent collection, validation, migration and staged publication to ROM.
Use optional sqlite/redb features; the default archive inspection/migration-preview core does not compile native database dependencies.
A sealed BackupSource is implemented only by enabled native adapters. backup accepts an already-open handle, never a path to open/create.
inspect fully validates the private archive but returns only backend, format and aggregate counts, with explicit external-object/delivery exclusions.
No Resource values, paths, keys, provenance or callback diagnostics enter reports, Debug or CLI errors.
Limits are validated positive bounded budgets, at most128MiB and400000 aggregate records.
Host typed MigrationPlan uses the existing public pure converters and unfinished-work validator.
preview_migration reads/validates/converts the archive without filesystem publication and returns counts of checked records and changed schemas.
It is a point-in-time preflight, not a cutover permit or physical operation deadline.
restore and migrate_from dispatch enabled native profiles and return Arc<dyn rom::Storage> for host Runtime composition.
Unsupported capability fails before filesystem access. Native Error::Unknown remains distinct; never retry or replace an uncertain destination.
No test hooks, native unsafe code, private-module copy or second archive/publication format.

## Operator CLI and host example

CLI supports inspect and restore with explicit backend/path and optional bounded budget flags.
No offline backup opener: upstream constructors may create a missing source. Host backup uses its existing native handle.
No dynamic converter language in CLI; migration is compiled host policy.
Print count-only JSON success or fixed structured error codes; Unknown has a distinct exit code.
Trusted local Linux directories and ownership sidecars remain upstream prerequisites. No automatic sidecar removal, source deletion, overwrite or deployment switch.
Host must stop admission, drain and close the source before routing traffic to the destination. Destination cursor/claim fencing does not stop old deployments.
Archives exclude external blobs and deliveries; external inventories, copy and digest qualification remain required separately.

## Qualification

Independent public source and normalized packaged consumers execute both actual native engines.
Create/action/history/replay/protected attribution/tombstones survive backup, restore and reopen.
Native backup runs with live host writes; each validated snapshot is coherent. Preserve exact Resource identifiers and receipt identity.
Typed migration validates old/new codecs, historical receipt replay and source immutability; reject failing converters and unfinished Work without a validator.
Check restore cursor/claim fencing, fresh-destination refusal, wrong backend, corruption, incompatible formats, byte/record budgets and private permissions.
Missing or owned migration source must fail without creating a source/destination database.
CLI negatives never print private paths or data; no-feature consumers demonstrate capability refusal.
Test interruption before publication through upstream test-support only in independent native fixtures; production API has no fault hook.
Do not claim native publication uncertainty injection, complete external-object backup or SQL acceptance without executed proof.
Run affected feature/core tests, Clippy/docs/MSRV, dependency audits, independent package consumers and full local verifier before integration.
