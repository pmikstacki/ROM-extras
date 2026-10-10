# Verified external blob recovery

`rom-blob-recovery` copies a host-approved inventory through public `rom_blob::BlobStore` values.
It uses existing S3 and Azure adapters. It does not open endpoints or obtain credentials.

## Inventory and host approval

Each `Entry` contains an exact opaque `ObjectKey`, a separate SHA-256 content `Digest`, and an expected byte length.
A physical ROM key identifies a publication. It is not necessarily the content digest.
A `Manifest` binds entries to a host-selected checkpoint digest. SHA-256 does not authenticate the manifest or prove completeness.

The host can preserve exact publication keys when objects are created.
The [checkpoint inventory tool](blob-checkpoint.md) can observe exact keys through public reads of current Ready built-in Blob rows.
Do not copy the private key algorithm. Do not replace physical keys with content digests.

The host must authorize recovery, authenticate checkpoint provenance and prove inventory completeness.
Keep source objects immutable and suspend garbage collection throughout recovery and cutover.
Protect encoded manifests and archive files. Their keys and checkpoint values are private operational data.
Debug reports and errors contain counts, entry indices, fixed phases and safe causes only.

## Bounds and operation

Default limits allow 10,000 entries, 64 MiB per object, 1 GiB aggregate content and 4 MiB encoded JSON.
The host can select explicit limits. The tool rejects duplicates, overflow, unknown fields and invalid identifiers before provider calls.
This implementation uses bounded whole-object reads. It does not implement streaming or multipart transfers.

The tool first reads and verifies existing destination content. Matching content counts as an explicit resume without a write.
If the destination is missing, it verifies source bytes and invokes create once.
It reads back acknowledged writes. A concurrent-create Conflict also requires verified destination content.
A content mismatch stops recovery. The tool never overwrites or deletes an object.

Unknown publication stops without automatic retry. The host can explicitly invoke recovery again to reconcile matching destination content.
A failure reports the verified prefix and publication certainty for the failing entry.
An acknowledged write followed by failed verification remains Confirmed; it was not rolled back.

`Context` carries one absolute Tokio deadline and a cancellation token for the complete invocation.
Cancellation before admission performs no write. Cancellation or timeout during an admitted create has an unknown publication outcome.
The deadline bounds cooperative caller waiting. Arbitrary providers can continue hidden work after future cancellation.
Progress exists in memory. Dropping the whole future or terminating the process loses its returned progress report.

## Public API example

The host supplies already-configured stores and a validated private manifest:

```rust,ignore
use rom_blob_recovery::{Context, copy, verify};
use tokio_util::sync::CancellationToken;

let context = Context::new(
    tokio::time::Instant::now() + std::time::Duration::from_secs(30),
    CancellationToken::new(),
);
let copied = copy(source.as_ref(), destination.as_ref(), &manifest, &context).await?;
let checked = verify(destination.as_ref(), &manifest, &context).await?;
assert_eq!(copied.verified, checked.verified);
```

Authorization, Resource selection and deployment cutover remain host operations.
No Resource actions, attachment transitions, database restores or automatic external connections occur in this crate.

## Executable host example and evidence

The independent consumer is `tests/blob-recovery-public-consumer`.
Its controlled host uploads through the real BlobService and observes exact keys through the public BlobStore create port.
It binds captured publication facts to Ready Blob rows in a coherent SQLite or redb archive.
It copies Azure objects to S3, restores the native database and reads the same Resource ID through the destination BlobService.
It also checks an unauthorized caller and reopens each restored native database.

The example captures one approved publication in memory. The checkpoint inventory tool adds private durable inventories with an explicit current-row completeness scope.
A successful run does not qualify recovery after losing that inventory or an automatic deployment cutover.

Run the affected gate with explicit dedicated fixture configuration:

```sh
./scripts/check-blob-recovery
```

The gate runs public control-flow tests and independent source and normalized archive consumers.
Native consumers test RustFS/Azurite transfer in both directions, exact readback and explicit equal resume.
A bounded HTTP relay drops a real successful S3 PUT response. An independent client proves publication.
Explicit recovery then reads existing content without a second PUT. Fresh clients verify retained S3 content after fixture restart.
Fixture readiness retries bounded reads outside the recovery operation. They do not enable production retries.

Azurite is emulator evidence. RustFS is a local S3 profile; these tests do not qualify AWS S3 or live Azure.
SeaweedFS's failed strict contention profile remains recorded and is not overridden by this recovery tool.
No provider-wide coherent snapshot, version-pinned source, durable recovery ledger, streaming restore or atomic cross-store cutover is provided.
Source and normalized archive consumers passed these native scenarios. All 16 public control-flow tests passed.
The full local verifier passed with all 1451 frozen files unchanged during execution.
See the [research](research/blob-recovery-2026-10-10.md) and [executed verification record](verification/blob-recovery-2026-10-10.json).
