# Verified external blob recovery

Inspection date: 2026-10-10. Primary-source research; executed trials are recorded separately. Read repository writing/quality rules. Public ROM pin: d7ef529040eec60dc869034c2d33130219db85fe.

## Physical key correction

The first research recommendation incorrectly treated an opaque ObjectKey as the content digest.
An actual BlobService/native archive consumer exposed this error; the retained initial run failed with Missing.
The public pinned service derives publication identity from reservation context, separately from payload hashing.
The implemented Entry therefore carries key, digest and bytes separately.
The controlled host observes exact keys through the public create port. It does not duplicate private ROM code.
Source: [pinned service publication/read contract](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-blob/src/service/operations.rs).

## Recommended shared boundary

Implement an extras-owned bounded recovery operation over two public BlobStore values. Accept a host-approved inventory of exact ObjectKey, separate content Digest and expected byte length. Use separate physical ObjectKey and content Digest values. Both use public ROM parsers. Accept explicit limits for entries, per-object bytes, aggregate bytes and deadline. Validate the complete inventory before provider calls, including duplicate keys and arithmetic overflow. Keep one sequential object operation initially; avoid unnecessary concurrency and hidden replay.

Public BlobStore exposes create(key, Vec<u8>), get(key, max_bytes), head(key), and trusted delete(key). create promises complete publication, Conflict without overwrite and Unknown without rollback implication. get returns a whole Vec, so this seam supports bounded whole-object recovery, not arbitrary streaming or multipart resumability. head returns only byte length. Sources: [exact public storage port](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-blob/src/storage.rs), [exact identity API](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-blob/src/identity.rs).

For each key, bounded-get source bytes, require exact length and `Digest::of(bytes)==entry.digest()`, then issue exactly one destination create. After success, bounded-get destination and recheck exact length/digest before reporting verified completion. On Conflict, bounded-get existing destination and compare with inventory digest/length. Equal content is a verified resume; mismatched content refuses recovery without overwrite. HEAD can support early rejection but cannot replace content verification. These are proposed tool guarantees conditional on conforming providers.

Do not retry after Unknown automatically. Stop and return the verified prefix plus the uncertain key/phase. A later explicit host invocation may reconcile by a fresh bounded read or another create-only attempt. It never interprets Unknown as nonpublication. If post-create verification fails, distinguish created-but-unverified from definitely absent; do not pretend an earlier acknowledged create was undone.

## Provider conditions and integrity

S3 If-None-Match:* evaluates existing current objects and prevents overwrite. Versioned delete markers and concurrent deletes complicate create behavior; 409 is not the same as an existing immutable object. Provider mappings must preserve this distinction. Real S3 documentation is a protocol reference, not proof every compatible server implements atomic conditional creation. Source: [official S3 conditional writes](https://docs.aws.amazon.com/AmazonS3/latest/userguide/conditional-writes.html).

S3 ETag is not a general SHA-256 content identity. Multipart and encryption modes affect checksum interpretation. Download and recompute the public Digest rather than substituting ETag or a composite checksum. Native transport integrity can supplement this, but does not establish inventory completeness or trusted origin. Source: [S3 checksum documentation](https://docs.aws.amazon.com/AmazonS3/latest/userguide/checking-object-integrity-upload.html).

Azure Put Blob supports conditional headers; If-None-Match:* prevents replacement when the object exists. Azure ETag supports concurrency comparison, not the ROM SHA-256 identity. Use the maintained adapter's create-only semantics and verify downloaded content. Sources: [Azure Put Blob](https://learn.microsoft.com/en-us/rest/api/storageservices/put-blob), [Azure conditional headers](https://learn.microsoft.com/en-us/rest/api/storageservices/specifying-conditional-headers-for-blob-service-operations), [Azure concurrency](https://learn.microsoft.com/en-us/azure/storage/blobs/concurrency-manage).

## Source consistency and host responsibility

BlobStore cannot select S3 VersionId or Azure snapshot/version IDs and cannot express conditional GET by ETag. Therefore the generic tool proves bytes observed for each key, not one coherent provider-wide snapshot. Host-approved immutable content namespaces and GC quiescence remain prerequisites. A head/get race must not bypass final digest validation. Even a verified object can be deleted after verification without host protection. Provider-specific pinned-version recovery is a future boundary, not silently available through this port. Azure version/snapshot reads are documented separately in [Get Blob](https://learn.microsoft.com/en-us/rest/api/storageservices/get-blob).

SHA-256 verifies content against the inventory; it does not authenticate an attacker-supplied inventory. Host approval must establish provenance and completeness. SQL/native archive backup excludes external blobs. The host must bind archive checkpoint to its complete approved inventory and ensure source object retention. The tool must not scan arbitrary Resource JSON and call that a complete inventory. Encryption-at-rest, credentials, TLS, KMS access, authorization, retention and cutover remain host/provider obligations.

Do not invoke delete, Resource actions, receipts, attachment state transitions or deployment cutover. The public delete contract independently requires detachment, grace and quiescence. Recovery completion means every supplied entry was verified at its observation point; it does not mean all deployment objects were supplied or remain forever accessible.

## Deadlines and cancellation

Use one absolute caller deadline across provider calls, while stating this bounds caller waiting only. Arbitrary BlobStore futures can hide work after drop. Tokio timeout also depends on cooperative polling; it does not bound blocking hashing/native code. Bound object sizes and avoid advertising strict physical work termination without a qualified provider/runtime. Source: [Tokio timeout contract](https://docs.rs/tokio/latest/tokio/time/fn.timeout.html).

If create was polled before timeout/cancellation, its effect may be Unknown. Distinguish cancellation before create admission from cancellation during create and verification after acknowledged publication. Dropping the whole recovery future cannot deliver its final progress report. A returned timeout result can include a verified prefix; external cancellation requires host-owned observation/checkpointing or later explicit reconciliation. Do not promise a persisted recovery ledger unless one is implemented and qualified.

A successful create followed by cancellation before read-back is created-but-unverified. Never erase its effect by returning an undifferentiated no-progress result. If checkpoint durability is future work, state progress is in-memory and can be lost on process exit. No automatic retries, cancellation-triggered cleanup or speculative deletes belong in this core.

## Qualification and alternatives

Public consumer tests should use only BlobStore/Digest/ObjectKey, exercise equal resume, mismatched same-length bytes, wrong length, missing/denied source, oversized response, inventory overflow/duplicates, deadline phases, create Unknown and post-create read failure. Mocks test control flow only.

Real native tests should exercise maintained Azure against Azurite and unchanged public S3 adapter against RustFS. Preserve SeaweedFS's failed high-contention qualification profile: successful sequential recovery would qualify that narrow profile only. Do not promote SeaweedFS to universal concurrent-create support. Test bidirectional S3/Azurite transfer where interfaces support it, exact destination bytes, explicit replay and persistent backend restart. Azurite remains emulator evidence; it does not establish live Azure support.

For HTTP response loss, a bounded loopback proxy must forward the actual create request and suppress its successful response. Independently read destination to prove object publication, require Unknown with no automatic second write, then explicitly rerun and verify equal resume. Record sanitized request counts/status only; never log authorization or payloads. This is distinct from a mock or a post-SDK pause.

Packaged consumers must compile/run independently of workspace feature unification. Prefer public rom-blob Digest implementation and already-locked Tokio, avoiding a new checksum/archive graph. Record exact lock/compiler/license/advisory scope in implementation verification; source review here is not that audit.

Provider-native copy can reduce network traffic but cannot be expressed through BlobStore and may have different version, encryption and destination-precondition semantics. Streaming multipart restore requires a new bounded public seam and abort/orphan policy. The proposed whole-object core is substantive external recovery groundwork; it does not complete native database backup, atomic cross-store restoration or full requested maintenance tooling.
