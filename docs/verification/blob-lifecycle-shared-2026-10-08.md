# Shared Azure and S3 native blob lifecycle verification

Date: 2026-10-08. Baseline: ROM-extras f38cc1fb20f07dbe9d0b060482a52f8218ef59d8 plus the recorded shared-harness changes.
Public ROM revision: d7ef529040eec60dc869034c2d33130219db85fe. Production provider and core implementations are unchanged.

## Shared boundary and preserved behavior

[Source assessment](../research/s3-blob-lifecycle-assessment.md) establishes use of public BlobStore and BlobService without copying private ROM modules.
The previous full-verifier baseline's 104 runtime inputs remained unchanged before extraction.
Azure's active observation helper and scenario bodies now live under tests/common/blob_lifecycle.
Its existing test names remain stable; provider wrappers supply logical store label, artifact root, and real adapter factory.
Independent review found no removed Azure assertions or unintended provider abstraction.

The observation wrapper delegates create/get/head/delete to the actual provider.
It records create and read calls, and can pause only after actual create success.
The same two scenario functions run on Azure, RustFS, and SeaweedFS, each with SQLite and redb.
Native database artifacts and detached/unattached provider objects remain preserved.

The attached scenario checks wrong digest and failed stream before provider create, Pending revision 1, valid false-byte attachment, and private authorized reads.
It verifies denied reads make no provider GET, repeated Ready upload makes no new create, and Ready state survives orderly native reopen.
Detachment returns a stable receipt; Detached survives another reopen, prevents upload, and retains provider bytes.

The unattached scenario pauses after real create success, independently reads provider bytes, then deletes the Pending Resource reservation.
After release, attachment returns Unattached with the original Denied cause and exact key/digest/byte-count receipt.
Tombstone reads remain Denied and queries exclude the Resource, before and after native reopen.
Provider bytes remain retrievable. This is not wire acknowledgement loss, grace expiration, or physical cleanup evidence.

## Executed gates and mutation sensitivity

| Command/profile | Result |
| --- | --- |
| scripts/check-azure, persistent Azurite 3.37.0 | Exit 0; ten workspace tests, one independent-consumer test, Clippy |
| scripts/check-s3, RustFS 1.0.1, s3-lifecycle | Exit 0; six tests, no failures or ignored cases, Clippy |
| Filtered S3 BlobService cases after exact mutation restoration | Exit 0; two tests |
| Filtered S3 BlobService cases on SeaweedFS 4.48 | Exit 0; two tests; does not replace strict failed qualification |

Each lifecycle test loops independently over SQLite and redb.
See [Azure gate](blob-lifecycle-azure-shared-2026-10-08.log), [S3 gate](blob-lifecycle-s3-shared-2026-10-08.log), [restored cases](blob-lifecycle-s3-restored-2026-10-08.log), and [SeaweedFS cases](blob-lifecycle-seaweedfs-2026-10-08.log).

A temporary observer mutation omitted the real create call while retaining counters and reporting success.
The test compiled, then failed with runtime Missing on BlobService's verified read.
The exact correct source was restored by hash and both lifecycle cases passed again.
[Mutation facts](blob-lifecycle-mutation-2026-10-08.json) and [failed run](blob-lifecycle-real-create-mutant-2026-10-08.log) distinguish this meaningful failure from compilation errors.
The mutant source and failed native artifact remain preserved locally; no production adapter was mutated.

## Dependency identity and checks

The consumer graph grows from 427 to 440 packages with published native adapters and their required dependencies.
All earlier package identities and checksums remain present. The workspace lockfile remains unchanged at 360 packages.
Thirteen added package manifests have license declarations and available Rust-version fields recorded in [dependency facts](blob-lifecycle-dependencies-2026-10-08.json).
The installed pinned cargo-audit command reports zero vulnerabilities and warnings against the recorded RustSec database.
See [audit result](blob-lifecycle-consumer-audit-2026-10-08.json).
These manifest declarations are not a redistribution certificate.

The offline dependency-resolution command completed with one unused-import warning; its output is retained.
The unused import was removed before all affected Clippy gates passed with warnings denied.
See [resolution log](blob-lifecycle-consumer-resolution-2026-10-08.log).

## Complete configured verifier

The complete `./scripts/check-all` run exited 0 with the persistent local SQL, messaging, Azure, and RustFS fixtures configured.
All 108 frozen runtime inputs had identical paths and SHA-256 hashes after the run.
The final S3 gate ran all six cases successfully with no ignored tests, followed by Clippy.
See the unmodified [full log](blob-lifecycle-full-verifier-2026-10-08.log) and [input hashes](blob-lifecycle-source-inputs-2026-10-08.json).
Independent source review found no actionable issues; it did not claim to rerun these commands.

## Limits

These are basic native lifecycle results against exact local backend profiles, using orderly drain and owner release.
They do not establish current revocation during I/O, corrupt-provider reads, caller cancellation/drain faults, abrupt process crash, or actual lost write acknowledgement.
Grace/quiescence cleanup, live cloud authorization, verified S3 TLS, and packaged-consumer acceptance remain pending.
SeaweedFS's eight-way operational qualification remains failed; filtered lifecycle success does not alter it.
The complete ROM-extras goal remains active.
