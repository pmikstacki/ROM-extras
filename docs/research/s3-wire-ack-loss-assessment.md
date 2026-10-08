# S3 wire acknowledgement loss assessment

Evidence category: primary-source review and proposed experiment, 2026-10-08. This agent did not execute a proxy or lifecycle test. Published ROM baseline: `d7ef529040eec60dc869034c2d33130219db85fe`.

## Expected public behavior

The unchanged public object-store adapter sends `PutMode::Create`. It maps ordinary uncertain write failures to `rom_blob::Error::Unknown`. Its S3 constructor selects ETag conditional creation, zero SDK retries, a three-second timeout, and a one-second connection timeout. [Adapter](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-blob-object-store/src/adapter.rs), [error mapping](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-blob-object-store/src/errors.rs), [configuration](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-blob-object-store/src/configuration.rs)

BlobService returns a provider create error before committing Ready. With no concurrent Resource changes, an unknown publication outcome leaves the reservation Pending. A later explicit upload reconstructs the same receipt from reservation identity, revision, store, digest, and length. A create conflict triggers verified retrieval before conditional attachment. The retry must use the same reservation and verified input. [Upload and receipt construction](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-blob/src/service/operations.rs)

Unknown does not establish rollback. A successful direct read after the fault must establish object existence and independently verify bytes. The intended second attempt verifies existing bytes and attaches once; it must not overwrite them.

## Proposed loopback wire fault

Use a bounded HTTP/1 proxy between the unchanged adapter and the existing loopback RustFS fixture. Bind only a literal loopback address. Configure the adapter endpoint to the proxy. Forward to the existing backend without changing the original Host header, path, signed headers, or body. SigV4 signs the endpoint host, including its port. Replacing it with the backend address invalidates the signed request. [object_store signing source](https://github.com/apache/arrow-rs-object-store/blob/v0.14.2/src/aws/credential.rs#L261)

Select exactly one create PUT for the fault. Forward its complete bounded request. Read and validate the backend response before dropping the client-facing connection. Suppress every final response byte when the backend returns success. Do not replace the successful response with a fabricated 500 or 412. Forward unrelated requests normally.

Handle HTTP framing explicitly. A partial header read is not a complete response. Bound request headers, body, connections, tasks, and observation waits. Handle or transparently pass interim responses before the final response. Fail the fixture when framing or backend status differs from the intended case.

The exact RustFS source commits storage before completing the normal PUT usecase. The proxy must nevertheless record the observed final success status; source inspection does not prove that a selected request succeeded. A success response plus independent retrieval establishes this experiment's publication evidence. [RustFS PUT usecase](https://github.com/rustfs/rustfs/blob/6de965ae3c965a78ff819fbcd7acd4aa44177d92/rustfs/src/app/object/put.rs), [storage commit](https://github.com/rustfs/rustfs/blob/6de965ae3c965a78ff819fbcd7acd4aa44177d92/crates/ecstore/src/set_disk/ops/object.rs#L4603)

Export only method counts, final status counts, fault count, elapsed time, and named outcome categories. Never export raw requests, authorization headers, credentials, signed URLs, or payloads. Private parsing does not authorize public raw logs.

## Retry layers

`object_store` 0.14.2 stops its retry loop when `max_retries=0`. Its reqwest builder does not explicitly disable reqwest's default protocol retry policy. Therefore zero SDK retries is not proof that every transport layer disables retries. [SDK retry loop](https://github.com/apache/arrow-rs-object-store/blob/v0.14.2/src/client/retry.rs), [HTTP client construction](https://github.com/apache/arrow-rs-object-store/blob/v0.14.2/src/client/mod.rs#L867)

The inspected consumer lockfile resolves reqwest 0.13.5. Its default classifier retries selected protocol NACKs. The source recognizes HTTP/2 graceful GOAWAY and REFUSED_STREAM; an HTTP/3 timeout branch is feature-dependent. Ordinary HTTP/1 response EOF is not selected by this classifier. Use an HTTP/1 fault and verify the actual forwarded PUT count is one. This is a narrow source finding, not a guarantee about all lower-level connection recovery. [reqwest retry source](https://github.com/seanmonstar/reqwest/blob/v0.13.5/src/retry.rs)

If another create is forwarded before the caller observes Unknown, fail the intended fault scenario. Do not hide implicit retry by reporting only the first request. No production client change is proposed.

## Required scenario assertions

Run independently with native SQLite and redb:

1. Reserve Pending and record its public revision.
2. Upload through the armed proxy and await the result.
3. Require one forwarded create, one backend success, one suppressed response, and caller Unknown.
4. Verify Pending and unchanged revision through public ROM reads.
5. Verify object bytes through a direct provider read.
6. Retry explicitly after disarming the fault. Require conflict verification and one Ready attachment.
7. Repeat Ready upload and assert no additional provider create.
8. Drain, drop owners, reopen the native database, and verify persisted Ready bytes.

The current post-SDK observation pause is a different scenario: the adapter already returned success there. Keep both scenarios and evidence labels distinct.

## Quiescence and limits

Await the failed upload rather than cancelling its waiter as the fault mechanism. BlobService supervises admitted work independently of caller cancellation. Drain it before Runtime shutdown. Then drain proxy tasks and drop all ownership before native reopen. [BlobService lifecycle](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-blob/src/service/lifecycle.rs), [Runtime lifecycle](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/execution/lifecycle.rs)

This fault would establish a lost HTTP success response on the executed loopback profile. It would not establish cloud TLS, backend crash durability, universal retry behavior, or safe orphan deletion. Physical cleanup still requires trusted detachment, grace, and quiescence. [BlobStore contract](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-blob/src/storage.rs)
