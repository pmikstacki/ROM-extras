# Azure Blob extension

Implement the approved blob family through the published `rom_blob::BlobStore` contract.
Reuse ROM `BlobService`, `Blob`, `Digest` and `ObjectKey`. Do not duplicate the Resource attachment protocol.
The published object-store Adapter has no arbitrary provider constructor. Its private fields remain inaccessible.
Provide an Azure-specific port implementation while preserving the existing S3 adapter for the S3 profile.

## Host configuration

Use the already selected object_store 0.14.2 Azure backend.
Host credentials are explicit shared keys or bearer tokens. Do not resolve ambient environment or implicit identity chains.
Cloud endpoints require HTTPS. Loopback emulator configuration is explicit and isolated from cloud support claims.
Reject URL credentials, query, fragment and unexpected account paths before constructing the client.
Limit objects to 1..=16 MiB. Use finite connect and request deadlines, zero SDK retries and bounded concurrent admission.
Do not expose SDK errors, credentials or provider configuration through Resource state or diagnostics.

The [Azure builder](https://docs.rs/object_store/0.14.2/object_store/azure/struct.MicrosoftAzureBuilder.html) supports explicit credentials and endpoint configuration.
Its emulator branch reads ambient endpoint configuration. Avoid that branch; use an explicit account-path endpoint.
[SDK source](https://github.com/apache/arrow-rs-object-store/blob/v0.14.2/src/azure/builder.rs) establishes this distinction.

## Operations

Create uses `PutMode::Create`, preserving create-if-absent atomicity. No HEAD-then-unconditional-PUT substitute is permitted.
Uncertain writes return Unknown. Caller cancellation does not prove that an object was not created.
Reads use a bounded metadata snapshot and ETag-conditioned GET. They verify the expected byte length.
A changed object cannot silently combine metadata from one version with bytes from another.
Missing reads return Missing; confirmed denied access returns Denied. Unsupported capabilities fail explicitly.
Missing deletion is idempotent. Deletion remains trusted maintenance requiring detachment, grace and quiescence.

[Azure conditional headers](https://learn.microsoft.com/en-us/rest/api/storageservices/specifying-conditional-headers-for-blob-service-operations) define ETag preconditions.
[ROM's public port](https://github.com/pmikstacki/ROM/blob/d7ef529/crates/rom-blob/src/storage.rs) defines create-only and uncertain-write semantics.

## Required evidence

Run the published blob conformance suite and independent public consumers.
Test duplicate create, concurrent arbitration, false/empty/exact-limit payloads, size bounds, missing reads, metadata and idempotent deletion.
Test confirmed credential denial, service unavailability, interruption after publication before acknowledgement and restart retention.
Exercise BlobService staging, digest verification, failed attachment, authorization and orphan reconciliation separately.

Use a pinned persistent Azurite fixture for emulator evidence. Preserve failed runs and private credentials.
[Azurite](https://github.com/Azure/Azurite) is an emulator; its success does not establish live Azure support.
A real Azure account, verified TLS, cloud conformance and packaged-consumer checks remain mandatory before a supported profile claim.
Full S3-compatible, Azure and other ROM-extras family requirements remain active.
