# Private Blob checkpoint inventories

`rom-blob-checkpoint` collects exact physical publications for every current Ready built-in Blob row in a trusted archive.
It restores a fresh SQLite or redb inspection database through public ROM contracts.
An isolated Blob-only Runtime reads each row through the public BlobService and a read-only observer.
The observer records the service-selected opaque key and verifies content digest and byte length.
Before and after each read, the collector checks the exact Resource row.
It does not copy ROM's private key algorithm or infer keys from equal content.

## Host configuration and authorization

Enable `sqlite`, `redb`, or both. Native features are disabled by default.
Supply explicit store aliases and already-configured `Arc<dyn rom_blob::BlobStore>` values.
Credentials and endpoints remain in the host's adapters. No external connection is configured automatically.
Unknown aliases are rejected before inspection or provider reads.

Run this privileged operation outside client-facing Resource routes.
The isolated service uses public `worker_actor()` to read protected builtin Blob rows.
The host must authorize maintenance and authenticate archive provenance.
Keep archives and source objects immutable. Suspend garbage collection throughout inspection, transfer and cutover.
Use trusted private parent directories. The file contract requires Unix permission checks, hard links and directory synchronization.

Preparation is synchronous. Run it in a bounded blocking host executor.
It validates archive, metadata and encoded-size limits before restoring the inspection database.
It compares source archive SHA-256 before and after restore.
This comparison does not replace immutability, host authorization or a signature.
Preserve the explicit inspection database after success or failure.

## Collection and durable publication

The default bounds are 128 MiB archive bytes, 400,000 archive records, 10,000 Ready rows and 32 configured stores.
Each object is limited to 16 MiB; aggregate content is limited to 1 GiB.
Encoded JSON is limited to 4 MiB. The whole-object bound follows the current public BlobService limit.
The host can reduce limits. An empty Ready set is valid.

Collection is sequential and consumes its inspection instance.
One absolute `rom_blob_recovery::Context` deadline and cancellation token cover caller waiting and observed provider reads.
The service can retain accepted work after its caller future is dropped.
Observers are isolated per invocation; a canceled observer is never reused.
Dropping the entire collection future provides no shutdown or completion proof.
The collector drains accepted service work and shuts down the Runtime before returning an inventory.
There is no hard deadline for synchronous restore, filesystem operations or native cleanup.

Successful collection performs provider reads only. Observer create, head and delete methods reject calls without delegation.
A missing, corrupt, oversized or denied object stops collection without returning a partial inventory.
Failures disclose fixed causes, phases and verified-prefix counts. Debug output does not disclose IDs, owners or physical keys.

`Inventory::publish` uses public `rom_backup::Stage` for mode-0600 staging, file synchronization, no-overwrite linking and parent-directory synchronization.
An existing destination returns Conflict and retains its bytes.
A parent synchronization failure can return Unknown after publication. Preserve the destination and reconcile explicitly; do not retry automatically.
`Inventory::read` rejects symlinks, non-private regular files, oversized JSON and unsupported platforms.
Strict decoding rejects unknown fields, future versions, duplicate IDs, duplicate keys within one alias and more than 32 aliases.
Decoded JSON remains unsigned operational data. It does not prove authenticity or completeness independently of trusted collection.

## Host integration example

The host supplies private paths, explicit adapters, limits and an absolute context:

```rust,ignore
use rom_blob_checkpoint::{Inspection, Inventory, Limits};
use rom_extras_maintenance::Backend;

let inventory = Inspection::prepare(
    Backend::Sqlite,
    &archive_path,
    &fresh_inspection_path,
    vec![("media".into(), configured_source.clone())],
    Limits::default(),
)?
.collect(context)
.await?;
inventory.publish(&private_inventory_path)?;

let persisted = Inventory::read(&private_inventory_path, Limits::default())?;
assert_eq!(persisted.checkpoint(), &expected_archive_digest);
let manifest = persisted.for_store("media")?;
rom_blob_recovery::copy(source.as_ref(), destination.as_ref(), &manifest, &copy_context).await?;
```

Keep separate recovery manifests for each store alias.
An identical physical key can exist in different logical stores; it must not merge their namespaces.
Bindings preserve exact Resource IDs, current revisions, upload revisions, owners, stores, physical keys, content digests and lengths.
Host-controlled native restore and destination BlobService configuration follow verified transfers.
Authorization, session handling and deployment cutover remain host operations.

## Working consumers and scope

Run `./scripts/check-blob-checkpoint` with the existing explicit dedicated RustFS and Azurite fixture environment.
The independent example is `tests/blob-checkpoint-public-consumer`; `tests/blob-checkpoint-core-consumer` checks disabled native capabilities.
The native example captures no create-time key journal.
It collects five Ready rows from mixed SQLite/redb archives, including equal-content owners, Unicode IDs and a zero-byte object.
It copies both logical stores between RustFS and Azurite.
Fresh child processes reload the private inventory, restore and reopen the native database, and verify exact authorized reads.
An unauthorized caller is denied. These are controlled local protocols, not live Azure or AWS qualification.

Counts explicitly exclude Pending, Detached and built-in tombstone rows and report other Resource rows.
The inventory covers current Ready builtin Blob rows only.
Historical publications, unattached provider objects, custom external Resource families and deleted source content are outside this scope.
The original ROM archive still reports external blob bytes as absent; its format is not changed.
Provider-wide snapshots, durable online publication journals, streaming transfers and atomic cross-store deployment cutover remain open.

Source and normalized archive consumers passed these local native scenarios.
All feature-profile checks passed. The full local verifier passed with all 1479 frozen files unchanged.
See the [primary-source research](research/blob-checkpoint-inventory-2026-10-10.md), [specification](superpowers/specs/2026-10-10-blob-checkpoint-inventory.md), and [executed verification record](verification/blob-checkpoint-inventory-2026-10-10.json).
