# S3 BlobService lifecycle assessment

Evidence category: primary-source review, 2026-10-08. Baseline: published ROM `d7ef529040eec60dc869034c2d33130219db85fe`. This agent did not run lifecycle tests.

## Reuse boundary

Extract test scenarios and observation wrappers into provider-neutral test modules. Supply an `Arc<dyn BlobStore>` through the public BlobService builder. Keep provider construction in each fixture. Reuse the public S3 adapter and native SQLite/redb constructors. Do not copy lifecycle production code. [Public composition](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-blob/src/service/configuration.rs), [storage port](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-blob/src/storage.rs)

## Shared assertions

Preserve these assertions for both providers and both native databases:

- Reservation establishes the expected pending Resource without provider publication.
- Valid upload attaches Ready with the expected digest, length, and revision.
- Repeated Ready upload verifies input and returns attachment without another provider create.
- Wrong digest or length rejects before provider create.
- Unauthorized read rejects before provider get.
- Detach returns the original object receipt and changes logical state; provider bytes remain present.

These assertions follow the public operations flow. Counts must measure real calls through the wrapper, not inferred network traffic. [Operations](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-blob/src/service/operations.rs)

Partial input failure is a separate staging scenario. Yield one successful chunk, then a typed stream error. Assert unchanged pending state and zero create calls. The staging loop propagates that error before complete-input hashing or publication. [Staging](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-blob/src/staging.rs)

## Post-publication deletion

Pause the observation wrapper after the underlying create succeeds, before returning that success to BlobService. Delete the reservation through public ROM operations. Release the wrapper and expect `Unattached` with cause `rom::Error::Denied`. Verify provider object existence and no Ready attachment. [Attachment flow](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-blob/src/service/operations.rs), [tombstone read](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/query.rs)

This wrapper pause is an application scheduling test after confirmed provider success. It does not simulate wire acknowledgement loss. Provider existence does not make a returned receipt cleanup authorization.

## Drain and reopen

Drain BlobService first. Drain its Runtime second. Drop all service, Runtime, store, and wrapper owners before reopening the same database path. Accepted BlobService work runs in a separately supervised task; cancellation of its waiter does not cancel that work. [Blob lifecycle](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-blob/src/service/lifecycle.rs), [Runtime lifecycle](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/execution/lifecycle.rs)

Run reopen independently for SQLite and redb. Re-register the same public Resource definition and provider name. Read the persisted Ready Resource and independently verify provider bytes. Preserve existing database ownership semantics. [SQLite ownership](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-sqlite/src/store.rs), [redb ownership](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-redb/src/store.rs)

No physical deletion belongs in these lifecycle scenarios. The public delete port requires trusted proof of detachment, grace, and quiescence. Keep cleanup qualification separate. [Maintenance contract](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-blob/src/storage.rs)

## Additional independent scenarios

A test-only wrapper can substitute wrong bytes from get while delegating actual publication. Expect digest/length rejection. Label this service integrity validation, not actual S3 corruption. Do not use an ETag as a content digest. [Verified read](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-blob/src/service/operations.rs#L208), [SHA-256 identity](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-blob/src/identity.rs)

Pause a successful underlying get before returning bytes. Revoke the actor's read authorization through the fixture's public model, then release get. Expect rejection before bytes reach the caller. The service rereads the Resource with the original actor after I/O. If revocation is represented by a Resource revision change, distinguish `Denied` from `Conflict` according to the model's authorization result. Keep this scenario separate from tombstone deletion and corruption. [Post-I/O read check](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-blob/src/service/operations.rs#L127)

## Evidence limits

Report each provider/database result separately. Preserve Azure assertions during extraction. S3 success cannot establish Azure success, cloud TLS, crash durability, or acknowledgement-loss recovery. Record the pinned ROM revision, dependency lockfile, backend identity, test commands, and observed failures. Keep wrapper simulation and real provider calls explicit.
