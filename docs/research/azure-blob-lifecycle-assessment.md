# Azure blob lifecycle acceptance assessment

Date: 2026-10-08. Evidence: source review of public ROM revision `d7ef529`, official Azure documentation, and Tokio documentation. This agent did not execute lifecycle tests or cloud operations. The parent reports a deleted-reservation scenario returned `Denied`; that report is consistent with the inspected source.

## Deleted reservations and attachment outcomes

`Runtime::read` loads the stored row and returns `Error::Denied` when `row.value` is absent. It does this before ordinary read authorization. A tombstoned reservation therefore produces `Denied`, even when a test expected a revision conflict. See [public read implementation](https://github.com/pmikstacki/ROM/blob/d7ef529/crates/rom/src/query.rs).

After successful object publication, `BlobService::upload` reads the reservation again using the original actor. If that read fails, it returns `UploadOutcome::Unattached { object, cause }`. Thus deletion after publication can yield an unattached object with cause `rom::Error::Denied`. Preserve that cause; do not change production semantics to satisfy an incorrect expected `Conflict`. See [public upload implementation](https://github.com/pmikstacki/ROM/blob/d7ef529/crates/rom-blob/src/service/operations.rs).

A revision change before publication and a failed conditional attachment have separate conflict paths. Deleted reservation, revoked read authorization, and stale revision scenarios need separate assertions. The acceptance invariant is that an unauthorized or deleted reservation cannot become Ready. Provider object existence does not establish successful ROM attachment. See [upload control flow](https://github.com/pmikstacki/ROM/blob/d7ef529/crates/rom-blob/src/service/operations.rs).

Proposed deleted-reservation test: block the provider create response after the object is committed, delete the reservation, then release the response. Assert the unattached receipt and `Denied` cause. Assert the reservation remains unreadable and the provider object exists. That object is maintenance evidence, not an instruction to delete immediately.

## Shutdown and native store reopen

`BlobService` supervises accepted work in separately spawned tasks. Cancelling a waiting caller does not cancel admitted work. `shutdown` closes admission and waits until active work reaches zero. The source releases operation-owned provider/runtime references before advertising quiescence. Its documentation requires draining the service before shutting down its core Runtime. See [blob lifecycle](https://github.com/pmikstacki/ROM/blob/d7ef529/crates/rom-blob/src/service/lifecycle.rs).

The core `Runtime::shutdown` closes its admission, I/O, and subscriptions, then waits for active work. It does not consume the Runtime handle. Existing service and Runtime clones can still own storage after successful shutdown. See [Runtime lifecycle](https://github.com/pmikstacki/ROM/blob/d7ef529/crates/rom/src/execution/lifecycle.rs).

Proposed reopen sequence: await blob-service shutdown; await Runtime shutdown; drop all service, Runtime, provider wrapper, and store owners. Then reopen the same database path using the public native adapter. Keep the same Azure object container. Validate that persisted Ready references still retrieve independently verified bytes.

The public SQLite adapter owns its connection and native exclusion guard. Its declaration ensures the connection closes before exclusion ends. The public redb adapter owns its database and native ownership guard. Its documentation directs callers to share an Arc rather than reopen concurrently. See [SQLite ownership](https://github.com/pmikstacki/ROM/blob/d7ef529/crates/rom-sqlite/src/store.rs) and [redb ownership](https://github.com/pmikstacki/ROM/blob/d7ef529/crates/rom-redb/src/store.rs).

redb 4.3.0 documents that a live write transaction can defer database close and keep the file locked. Reopen success requires releasing such ownership, not merely completing an application shutdown call. See [redb close semantics](https://docs.rs/redb/4.3.0/redb/struct.Database.html).

Tokio documents that started blocking tasks cannot be aborted. A shutdown timeout stops waiting but does not cancel them. Do not use a runtime timeout as proof of native-store release or transaction rollback. See [Tokio blocking-task behavior](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html).

## Trusted orphan cleanup

The public `BlobStore::delete` contract restricts deletion to trusted maintenance. The caller must prove detachment, grace, and quiescence. `BlobService::detach` commits logical detachment first. Its returned receipt explicitly supplies maintenance evidence without authorizing deletion. See [storage contract](https://github.com/pmikstacki/ROM/blob/d7ef529/crates/rom-blob/src/storage.rs) and [detach implementation](https://github.com/pmikstacki/ROM/blob/d7ef529/crates/rom-blob/src/service/operations.rs).

An unattached upload receipt also cannot establish global quiescence. Other admitted operations may still publish or reconcile the same reservation-derived key. An observation that the Resource is missing at one instant does not exclude an in-flight actor. Maintain the existing grace policy and stop or fence relevant admission before cleanup. This is an architectural consequence of the inspected supervision and delete contract.

Proposed acceptance separates logical lifecycle from cleanup. Confirm the orphan remains after denied attachment. Await required grace. Drain the relevant service and Runtime. Establish detached/deleted reservation state using trusted maintenance evidence. Only then delete through the provider, and verify a bounded read returns missing.

Azure Delete Blob behavior depends on snapshots, versions, soft-delete settings, and permissions. A successful delete can make the current blob unavailable without physically removing retained versions. Record account retention configuration. Do not claim immediate irreversible erasure from a missing-object check. See [Azure Delete Blob API](https://learn.microsoft.com/en-us/rest/api/storageservices/delete-blob) and [soft-delete documentation](https://learn.microsoft.com/en-us/azure/storage/blobs/soft-delete-blob-overview).

## ETags, digests, and conditional publication

Azure ETags support optimistic concurrency. A mismatched `If-Match` condition returns HTTP 412. The ETag is a service concurrency token; it does not replace ROM's content digest. See [Azure concurrency model](https://learn.microsoft.com/en-us/azure/storage/blobs/concurrency-manage).

ROM's `Digest` uses SHA-256. Its object receipt includes a separate digest and byte count. The physical key hashes reservation identity, revision, logical store, digest, and byte count. It is not simply the payload digest or Azure ETag. See [digest implementation](https://github.com/pmikstacki/ROM/blob/d7ef529/crates/rom-blob/src/identity.rs) and [receipt construction](https://github.com/pmikstacki/ROM/blob/d7ef529/crates/rom-blob/src/service/operations.rs).

Duplicate publication conflicts are accepted only after retrieving and validating digest and length. Reads also verify those values. Test corrupted content separately from ETag conflicts. A matching HEAD size alone does not establish content integrity. See [verified-read implementation](https://github.com/pmikstacki/ROM/blob/d7ef529/crates/rom-blob/src/service/operations.rs).

Preserve create-only provider publication and conditional ROM attachment as distinct operations. Azure supports conditional headers, but there is no distributed atomic transaction between Azure Blob and ROM's native storage. Retain unattached and unknown outcomes rather than translating an interrupted provider request into rollback. See [Azure conditional headers](https://learn.microsoft.com/en-us/rest/api/storageservices/specifying-conditional-headers-for-blob-service-operations) and [ROM upload protocol](https://github.com/pmikstacki/ROM/blob/d7ef529/crates/rom-blob/src/service/operations.rs).

## Required evidence categories

Run the same public BlobService lifecycle scenarios with SQLite and redb separately. Identify the exact published ROM revision, dependency lockfile, backend configuration, command, and observed outcomes. A source inspection is not a passing acceptance test.

Azurite can provide local lifecycle evidence, but Microsoft documents functional differences from cloud Azure. Keep emulator evidence distinct from a real Azure account. Neither local reopen nor emulator upload proves live Azure authorization, retention, or interruption behavior. See [Microsoft emulator limits](https://learn.microsoft.com/en-us/azure/storage/common/storage-use-azurite).

Do not copy production lifecycle code into ROM-extras. Tests and host presets should use the published public `BlobService`, field, storage, and native adapter interfaces. Any missing seam requires an explicit upstream proposal or public-port implementation.
