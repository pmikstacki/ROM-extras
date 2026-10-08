# Object storage extension assessment

Date: 2026-10-08. Evidence: primary-source and published ROM source review only. No dependency installation, backend fixture, cloud operation, or advisory audit was executed.

The approved plan now places blob extensions in Task 7; Task 6 is OpenTelemetry. The design requires extending existing ROM blob capabilities. See [plan](../superpowers/plans/2026-10-08-rom-extras.md) and [design](../superpowers/specs/2026-10-08-rom-extras-design.md).

## Existing public ROM contract

Published ROM revision `d7ef529` already exports `BlobStore`, `BlobService`, `Blob`, `Digest`, `ObjectKey`, and upload outcomes. `Digest` implements the public custom `Field` contract and uses SHA-256 independently of provider ETags. `Blob` records owner, logical store, digest, bytes, state, and upload revision. Reuse these fields rather than inventing a parallel reference format. See [identity source](https://github.com/pmikstacki/ROM/blob/d7ef529/crates/rom-blob/src/identity.rs), [Resource source](https://github.com/pmikstacki/ROM/blob/d7ef529/crates/rom-blob/src/resource.rs), and [public facade](https://github.com/pmikstacki/ROM/blob/d7ef529/crates/rom-blob/src/lib.rs).

The existing object-store adapter pins `object_store` 0.14.2 with AWS support. It implements create-only writes, bounded reads, size metadata, and idempotent missing-object deletion. Its public constructors support S3-compatible stores and trusted folders. There is no public arbitrary-`ObjectStore` constructor in this revision. Consequently, Azure requires a reviewed upstream constructor or a new Azure implementation of the public `BlobStore` port. Do not access private adapter fields. See [manifest](https://github.com/pmikstacki/ROM/blob/d7ef529/crates/rom-blob-object-store/Cargo.toml), [adapter](https://github.com/pmikstacki/ROM/blob/d7ef529/crates/rom-blob-object-store/src/adapter.rs), and [constructors](https://github.com/pmikstacki/ROM/blob/d7ef529/crates/rom-blob-object-store/src/configuration.rs).

`BlobService` reserves a Pending Resource, stages and hashes the input, creates or verifies the content object, then conditionally commits Ready state. Upload success and Resource attachment are separate outcomes. Cancellation of the waiter does not cancel admitted work. Preserve unattached outcomes and orphan reconciliation. This is source-established behavior, not reproduced conformance evidence. See [operations](https://github.com/pmikstacki/ROM/blob/d7ef529/crates/rom-blob/src/service/operations.rs).

## SDK comparison

| Candidate | Inspected source | License | Declared Rust version | Fit |
| --- | --- | --- | --- | --- |
| `object_store` | v0.14.2 | MIT/Apache-2.0 | 1.85 | Already used by ROM; shared async S3/Azure boundary |
| `aws-sdk-s3` | main manifest identifying 1.152.0 | Apache-2.0 | 1.94.1 | Broader AWS-specific API; separate Azure implementation needed |
| `azure_storage_blob` | azure_storage_blob@1.0.0 release | MIT | 1.88, inherited workspace | Azure-specific client; separate S3 implementation needed |

Sources: [object_store manifest](https://github.com/apache/arrow-rs-object-store/blob/v0.14.2/Cargo.toml), [AWS manifest](https://github.com/awslabs/aws-sdk-rust/blob/main/sdk/s3/Cargo.toml), [Azure release manifest](https://github.com/Azure/azure-sdk-for-rust/blob/azure_storage_blob%401.0.0/sdk/storage/azure_storage_blob/Cargo.toml), and [Azure workspace manifest](https://github.com/Azure/azure-sdk-for-rust/blob/azure_storage_blob%401.0.0/Cargo.toml). The AWS source is mutable; inspect its published archive before pinning.

`object_store` AWS/Azure convenience features select Reqwest, Rustls, and AWS-LC. Base features permit a host-supplied transport and crypto provider. The AWS SDK defaults include Tokio and HTTPS transport. Azure defaults inherit Reqwest/Rustls and Tokio through `azure_core`. Select only required providers and inspect the resolved crypto graph. See [object_store features](https://github.com/apache/arrow-rs-object-store/blob/v0.14.2/Cargo.toml), [AWS features](https://github.com/awslabs/aws-sdk-rust/blob/main/sdk/s3/Cargo.toml), and [Azure core features](https://github.com/Azure/azure-sdk-for-rust/blob/azure_storage_blob%401.0.0/sdk/core/azure_core/Cargo.toml).

Recommendation: keep `object_store` as the shared host integration mechanism. Extend the maintained adapter's public construction seam for Azure where possible. Use direct SDKs only for required operations that this boundary cannot provide. Keep SDK types and credentials out of Resource state and public domain fields.

## Conditional operations and content identity

`object_store::PutMode::Create` provides atomic create-if-absent semantics where implemented. Its update mode carries provider version/ETag information. Unsupported provider capabilities must fail explicitly. Do not replace conditional creation with HEAD followed by unconditional PUT. See [shared operation contract](https://github.com/apache/arrow-rs-object-store/blob/v0.14.2/src/lib.rs) and [S3 conditional configuration](https://docs.rs/object_store/0.14.2/object_store/aws/enum.S3ConditionalPut.html).

Amazon S3 supports `If-None-Match:*` on object creation and multipart completion. It also supports ETag-conditioned writes. Concurrent operations can produce precondition or conflict errors. Each S3-compatible product needs its own evidence for these operations. See [AWS conditional writes](https://docs.aws.amazon.com/AmazonS3/latest/userguide/conditional-writes.html).

Azure Blob REST supports conditional headers including ETags. An ETag is a provider concurrency token, not ROM's SHA-256 digest. Preserve the content digest and byte count independently. See [Azure conditional headers](https://learn.microsoft.com/en-us/rest/api/storageservices/specifying-conditional-headers-for-blob-service-operations) and [ROM identity](https://github.com/pmikstacki/ROM/blob/d7ef529/crates/rom-blob/src/identity.rs).

S3 supports conditional deletion through `If-Match`. The inspected shared `ObjectStore::delete` and ROM `BlobStore::delete` do not accept an ETag precondition. Keep deletion restricted to trusted maintenance with proven detachment, grace, and quiescence. A future conditional-delete extension requires explicit API design. See [S3 conditional deletion](https://docs.aws.amazon.com/AmazonS3/latest/userguide/conditional-deletes.html), [shared API](https://github.com/apache/arrow-rs-object-store/blob/v0.14.2/src/lib.rs), and [ROM maintenance preconditions](https://github.com/pmikstacki/ROM/blob/d7ef529/crates/rom-blob/src/storage.rs).

## Multipart and atomicity limits

Object-store multipart completion is not an atomic commit with ROM storage. Failed attachment can leave a complete orphan. Interrupted uploads can leave staged parts. Reconcile each state explicitly; do not infer rollback from an interrupted caller.

S3 documents up to 10,000 parts, with 5 MiB minimum except the final part and 5 GiB maximum per part. Azure service versions from 2019-12-12 permit 50,000 blocks of up to 4,000 MiB, with a 5,000 MiB single Put Blob limit. Application limits should be much smaller and explicitly bounded. See [S3 multipart limits](https://docs.aws.amazon.com/us_en/AmazonS3/latest/userguide/qfacts.html) and [Azure upload limits](https://learn.microsoft.com/en-us/rest/api/storageservices/put-blob).

The existing ROM adapter uses buffered create, not a streaming multipart contract. Keep its published size and staging limits. Do not silently switch to multipart until conditional completion, abort, orphan handling, and memory bounds have conformance evidence. See [existing adapter](https://github.com/pmikstacki/ROM/blob/d7ef529/crates/rom-blob-object-store/src/adapter.rs) and [shared multipart options](https://github.com/apache/arrow-rs-object-store/blob/v0.14.2/src/lib.rs).

## Backend profiles and evidence

ROM already provides a real local MinIO verification script. Reuse its scenario suite and preserve historical evidence. Verify the exact server binary or image provenance independently. See [existing S3 verifier](https://github.com/pmikstacki/ROM/blob/d7ef529/crates/rom-blob-object-store/verify-s3).

Garage is another real S3-compatible server. Its documentation supplies fixed-tag Docker deployment and a compatibility matrix. Treat it as a separate profile, not automatic S3-wide support. See [Garage deployment](https://github.com/deuxfleurs-org/garage/blob/main-v2/doc/book/cookbook/real-world.md) and [compatibility matrix](https://github.com/deuxfleurs-org/garage/blob/main-v2/doc/book/reference-manual/s3-compatibility.md).

Microsoft provides `mcr.microsoft.com/azure-storage/azurite` for local emulation, with persistent workspace and HTTPS options. Pin an exact version and image digest. Microsoft documents functional differences from cloud Azure. Azurite evidence must remain labeled emulator evidence. See [Azurite source](https://github.com/Azure/Azurite/blob/main/README.md) and [Microsoft emulator limitations](https://learn.microsoft.com/en-us/azure/storage/common/storage-use-azurite).

A live Azure profile needs an actual Azure storage account and separate conformance execution. Test upload, content digest, metadata, deletion, missing objects, conflicts, denied credentials, TLS, interrupted requests, orphan reconciliation, and restart/retry. An unavailable account blocks the live support claim, not continued local implementation. Do not substitute Azurite success for Azure success.

## Proposed family specification and adoption gates

Specify S3-compatible presets, Azure host construction, endpoint policy, credential ownership, immutable publication, bounded staging/read behavior, error mapping, and maintenance lifecycle. Keep the full requested S3-compatible and Azure family scope. Document missing cloud evidence as outstanding work.

Resolve credentials only in host configuration. Store logical aliases and content identities in Resources. Never store SAS URLs, access keys, bearer tokens, connection strings, or SDK configuration dumps in Resource values or journals. Bound diagnostics and redact secrets before logging. These are proposed acceptance requirements.

Audit the full resolved lockfile for licenses, advisories, transitive MSRV, crypto providers, and native build dependencies. Review storage-server licenses separately from client licenses. Run published-consumer conformance and the full local verifier after implementation. No dependency approval or supported-provider claim follows from this source review.

## Azure seam decision

Preferred proposal: add a reviewed, public host-owned constructor to the maintained adapter. It must validate limits and preserve the create-only and bounded-read contract. ROM-extras can then build an Azure `object_store` provider and delegate blob operations to that adapter. Publish the upstream seam before pinning it from ROM-extras. This preserves one implementation of storage adaptation and the existing domain protocol.

If that upstream change is unavailable, implement Azure through the already public `BlobStore` trait. Reuse `BlobService`, `Digest`, and `Blob`; do not copy their Resource protocol. A separate provider implementation must supply its own bounded reads and create-only translation. Review that duplication against the DRY gate before selecting this route. It is an implementation alternative, not an existing capability.

The existing adapter depends on `object_store` with AWS enabled. Azure construction therefore also needs deliberate `azure` or `azure-base` feature selection in the host dependency graph. Feature availability alone does not add a public Adapter constructor. The inspected `object_store` v0.14.2 Git tag resolves to `279572ea60f3a7a6e5237e066a6f4a35eee611e0`. Published archive checksums and locked graph remain unverified by this agent. See [tag source](https://github.com/apache/arrow-rs-object-store/tree/v0.14.2) and [ROM dependency](https://github.com/pmikstacki/ROM/blob/d7ef529/crates/rom-blob-object-store/Cargo.toml).
