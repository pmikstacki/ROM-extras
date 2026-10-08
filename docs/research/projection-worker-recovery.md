# Projection worker recovery research

Inspection date: 2026-10-08. Status: implementation requirements for the next Task 2 increment, not executed worker acceptance.

## Public historical source

The pinned [public journal implementation](https://raw.githubusercontent.com/pmikstacki/ROM/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/journal.rs) returns authorized historical views and an inspected cursor.
It applies the runtime's row and byte limits. Denied or missing-current-state events can disappear while the cursor advances.
Recovery must compare the reconstructed immutable intent, including selected revisions, positions, tombstones, and mapping digests.
A successful page fetch alone cannot establish unchanged authorization or mapping output.

The [projection implementation](https://raw.githubusercontent.com/pmikstacki/ROM/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/projection.rs) checks both current and historical field access.
Its historical view can carry a tombstone; current reads reject tombstones. A current read cannot replace exact historical recovery.

The [retained-state journal implementation](https://raw.githubusercontent.com/pmikstacki/ROM/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/storage_state.rs) rejects cursors below retention or across generations.
It scans global positions while selecting the requested kind. Recovery cannot interpret gaps between returned positions as missing kind events.

## Required interval reconstruction

Select public `Runtime::journal`, with an explicit export actor and immutable mapping profile, over direct storage access.
Start at the pending intent's expected cursor. Stop at its stored next cursor, not the current head.
A replayed batch can extend beyond the stored endpoint because later events now exist.
Validate its generation/kind and monotonic positions, then use only events within the immutable interval.
Do not accept a new later cursor as the pending intent's acknowledgement.
If several bounded batches are required, bound total fetches and cumulative inspected work; cancellation must preserve the pending intent.
A filtered empty interval still needs inspected-cursor coverage through its endpoint.

Reconstruct the collapsed operation metadata using the same mapping and exact old checkpoint.
Compare the full resulting `PageIntent` before any replay or completion. Do not compare only its last cursor or operation count.
Return `HistoryGap` for lost continuity. Return `RebuildRequired` for changed disclosure, content, profile, or unreproducible mapping.
A newer observed provider revision is divergence, not an exact acknowledgement.
These categories require implementation and native tests; this document does not add them to the compiled API.

## Storage-thread lifecycle

[Tokio's blocking-task contract](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html) does not permit aborting a started blocking job.
Use a dedicated storage owner thread for the long-lived checkpoint engine, with bounded admission and explicit shutdown.
The [standard bounded channel](https://doc.rust-lang.org/std/sync/mpsc/fn.sync_channel.html) supplies bounded queue semantics.
Prefer private typed checkpoint commands over a public arbitrary transaction closure. No native transaction crosses network work.
The [join contract](https://doc.rust-lang.org/stable/std/thread/struct.JoinHandle.html) requires joining; dropping a join handle detaches it.
Shutdown must finish admitted commands, close the engine, and release ownership before its join returns.
Cooperative startup cancellation is implemented separately. It does not supply a hard I/O deadline or prove thread shutdown.

## Required acceptance

Test recovery after intent commit and after actual remote acceptance, before completion acknowledgement.
Use both public SQLite and redb history with retention loss, current access revocation, multiple updates per key, and filtered cursor progress.
Use actual OpenSearch/Qdrant observation for provider acknowledgement; a synthetic trusted host seam is only a core test.
Test bounded admission, cancelled caller response, monotonic recovery, storage-thread panic, and shutdown while an operation is admitted.
Preserve uncertain transaction tokens and distinguish cancellation from rollback.
Keep native protocol, core orchestration, public ROM authorization, and package qualification as separate evidence scopes.

The existing extras `rom-sql-core::Executor` has explicit shutdown/join, but its last-handle drop detaches admitted work.
Do not claim that executor alone establishes projection-owner shutdown. A wrapper must enforce the projection lifecycle and bounded payloads.
Source inspection of this existing component is separate from web contract research and native worker qualification.

## Storage-owner execution checkpoint

The typed storage owner now reuses the executor and overrides detach-on-drop with shutdown/join.
Native file tests and normalized archive consumers qualify this limited execution layer.
The bounded asynchronous host lifecycle bridge, page orchestration and retained-history reconstruction are still required.
