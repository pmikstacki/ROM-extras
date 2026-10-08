# Azure Blob adapter

`rom-azure-blob` implements the published ROM `BlobStore` port with object_store 0.14.2.
Construct and use the adapter inside Tokio. Use ROM `BlobService` for staging, digest verification, and Resource attachment.
The adapter adds no Resource type, attachment ledger, or automatic retry scheduler.

## Explicit host configuration

Provide account, existing container, endpoint, endpoint policy, credentials, and finite limits.
`Credentials::SharedKey` accepts a base64 account key. `Credentials::BearerToken` accepts an explicit static token.
Credentials and configuration have no public Debug implementation. Keep dependency logs and credential provisioning outside Resource state.
Token acquisition and refresh remain host responsibilities. SAS, managed identity, and Azure CLI discovery are not implemented.

`EndpointPolicy::HttpsOnly` requires an HTTPS root endpoint without URL credentials, query, or fragment.
`LoopbackEmulator` permits HTTP only on a numeric loopback address and the exact account path.
This policy validates host configuration; it is not a public-address or exclusive-trust-root policy.
The SDK's HTTP connector retains its default proxy and redirect behavior.

The SDK emulator branch reads an ambient endpoint variable.
The adapter instead uses an explicit account-path endpoint and selects the credential type explicitly.
See [the pinned builder source](https://github.com/apache/arrow-rs-object-store/blob/v0.14.2/src/azure/builder.rs).

| Limit | Accepted range | Default |
| --- | --- | --- |
| Object bytes | 1..=16 MiB | 16 MiB |
| Concurrent operations | 1..=64 | 4 |
| Operation deadline | 1 ms..=60 s | 5 s |

SDK retries are zero. Admission is immediate; a full adapter returns Overloaded.
The connection timeout is at most three seconds and never exceeds the operation deadline.
The application deadline includes metadata, GET, and body collection.
Caller payload allocation and SDK/parser overhead are outside the configured object-byte limit.

## Operation semantics

Create uses the provider's conditional create operation. Existing objects return Conflict without overwrite.
Reads first obtain size and ETag, then issue an ETag-conditioned GET.
The adapter bounds collected bytes and checks the final length against the metadata snapshot.
A changed ETag returns Conflict. Missing objects return Missing; confirmed credential rejection returns Denied.
Empty reads return the empty metadata snapshot without a second request.

Write timeout or an unclassified write failure returns Unknown. Cancellation does not prove that a write failed.
Delete treats an already missing object as success.
Deletion is trusted maintenance: detach references and satisfy ROM's grace and quiescence requirements first.
See [Azure conditional headers](https://learn.microsoft.com/en-us/rest/api/storageservices/specifying-conditional-headers-for-blob-service-operations).

## Executed profile and remaining gates

The local profile uses persistent Azurite 3.37.0, an explicit private test key, and a pre-created namespace.
Published blob conformance, restart retention, credential denial, local interruption, and independent-consumer tests pass.
The local interruption test withholds a response; it does not prove a write committed before acknowledgement loss.
See [verification evidence](verification/azure-blob-2026-10-08.md).

Run the profile with the four ROM_EXTRAS_AZURE variables set by your private fixture configuration:

```sh
./scripts/check-azure
```

Missing fixture variables fail the profile. Do not place credentials in command arguments or checked-in files.

Azure cloud support is not established. Real cloud TLS, authorization, and conformance remain pending.
BlobService lifecycle, failed attachment/orphan handling, and actual post-write acknowledgement loss require separate integration tests.
Bearer-token authentication is configuration-only evidence so far. Packaged-consumer and redistribution checks remain pending.
The full ROM-extras goal remains active.
