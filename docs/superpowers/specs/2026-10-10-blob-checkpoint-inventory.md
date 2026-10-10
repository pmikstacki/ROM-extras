# Exact blob checkpoint inventories

This increment implements the approved backup/recovery goal. The complete ROM-extras goal remains active.
Research: ../../research/blob-checkpoint-inventory-2026-10-10.md. Public ROM pin: d7ef529040eec60dc869034c2d33130219db85fe.

## Contract

Add `rom-blob-checkpoint` with independently selected `sqlite` and `redb` features, disabled by default.
`Inspection::prepare(backend, archive, inspection_database, stores, limits)` is a synchronous host maintenance operation.
The host authorizes access, authenticates the archive, retains immutable source objects and suspends garbage collection.
Restore into an explicit fresh inspection database through `rom-extras-maintenance`. Preserve that database after every outcome.
Register only the built-in Blob definition. Do not register reactions, execute Work or mutate Resources.
Validate all current archive rows before provider reads. Count other kinds, tombstones, Pending and Detached Blob rows separately.
Enumerate every current Ready built-in Blob row. Reject malformed rows, zero upload revisions, unknown stores and exceeded budgets.
Preserve exact Resource IDs, snapshot revisions, upload revisions, owners, store aliases, digests and lengths.
The source archive must remain immutable through preparation. Bind its bounded original bytes to a SHA-256 checkpoint.

`Inspection::collect(self, context)` consumes the isolated inspection. No observer or Runtime is exposed for concurrent operations.
For each Ready row, compare current public row data before and after a successful `BlobService::read(worker_actor(), id)`.
Observe exactly one public get call for the selected alias. Reject create, head and delete without delegation.
Verify actual returned bytes and the observed key. Equal-content Resources must retain separate physical bindings.
Partition recovery manifests by store alias. The same physical key in different aliases is not a duplicate across stores.
Use the existing recovery Context for cooperative waits. Do not weaken provider HTTP contracts or enable automatic retry.
Accepted BlobService work survives waiter cancellation. Never reuse an aborted collector; drain its service before Runtime shutdown.
Synchronous native I/O, hashing and cleanup have no hard termination deadline. Dropping the whole future returns no completion proof.

## Durable artifact

Only complete collection returns an Inventory. Errors contain counts, fixed phases and causes; no private identity or provider text.
Expose private bindings through explicit getters. Debug output is count-only.
Use strict version-one bounded JSON with unknown-field rejection, exact identities and checked aggregate limits.
`Inventory::for_store(alias)` returns the existing validated recovery Manifest bound to this checkpoint.
`Inventory::publish(path)` uses public `rom_backup::Stage`: mode0600 staging, synchronization and no-overwrite publication.
Preserve Unknown after publication uncertainty. Do not retry, delete or replace existing artifacts automatically.
`Inventory::read(path, limits)` admits only bounded regular private files in a trusted parent.
Decoded artifacts establish structural validity, not authenticity or archive completeness. The host must compare the expected checkpoint.

## Limits and scope

Defaults: 128MiB archive, 400000 archive records, 10000 Ready rows, 16MiB per object, 1GiB aggregate object bytes, 4MiB JSON.
Store count is1..32; aliases are1..64 bytes. Resource IDs and owner labels have4096-byte limits.
The16MiB object ceiling follows the pinned public BlobService limit. Zero-byte objects are allowed.
Completeness covers current Ready built-in Blob rows only. Other resource kinds, historical values and unattached objects are counted or excluded explicitly.
Native archives still declare external objects excluded. No archive-format mutation, online publication journal or atomic cross-store cutover is implied.
AWS/Azure cloud qualification remains separate from local RustFS/Azurite evidence.

## Acceptance

Test multiple owners and equal-content Unicode/slash IDs, mixed Resource kinds, all excluded Blob states and both native inspection engines.
Test invalid/unknown alias, duplicate binding, malformed/future JSON, size/aggregate limits, missing/corrupt/denied source, timeout and cancellation.
Check no provider writes or deletes and no secret/identity diagnostics. Test existing inventory refusal, permissions and symlink/file limits.
Independent source and normalized archive consumers must reconstruct inventory without any create recorder.
Persist inventory, reopen it in a fresh process, copy per-store objects and read restored Resources through BlobService.
Keep mock, actual native database, authored protocol and real RustFS/Azurite evidence separate.
Run affected checks, dependency integrity/audits, one final source review and the full local verifier before integration.
