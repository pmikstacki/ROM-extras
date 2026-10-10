# Verified external blob recovery

This increment implements host-approved recovery over public `rom_blob::BlobStore`.
It does not replace the complete ROM-extras goal or the blocked SQL Storage bridge.

The host supplies a checkpoint SHA-256 and exact opaque physical keys, separate content SHA-256 digests and lengths.
The tool validates all entries before I/O. Limits cover entries, object bytes, aggregate bytes and JSON input.
Use ROM's exact ObjectKey format; the physical key is not the content digest. Preserve supplied order. Reject duplicates, overflow and unknown JSON fields.
Debug and errors omit keys, payloads, checkpoint values and provider diagnostics.

For each entry, verify existing destination bytes first. If absent, verify source and invoke create once.
Verify acknowledged destination content. Reconcile Conflict through a read. Refuse mismatches without overwriting or deleting.
Unknown publication stops. Explicit host re-invocation can verify the existing destination without another write.
One absolute deadline and cancellation token cover provider waits. Publication cancellation has an unknown outcome.
Failure reports include verified prefix, failing entry index, phase and publication certainty.
Progress is in memory. Dropping the whole future or process termination loses that progress report.

The host authenticates the manifest and proves checkpoint completeness. It retains immutable source objects and suspends garbage collection.
The host owns Resource selection, authorization, credentials, database restore and deployment cutover.
This port cannot pin S3 versions or Azure snapshots. Completion verifies only supplied entries at their observation points.
Whole-object recovery is bounded; streaming, multipart and persisted recovery checkpoints remain outside this increment.

Test public API control flow separately from native RustFS and Azurite protocol observations.
Retain SeaweedFS's failed contention evidence. No live AWS or Azure qualification is implied.
Use the existing adapters unchanged. Run affected checks, packaged consumer and full local verifier before integration.
