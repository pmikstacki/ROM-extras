# Qdrant 1.19.2 revision fencing

Inspection date: 2026-10-08. Evidence category: source review only.

Inspected tag: `v1.19.2`, commit `016542aa5deb6c66380bb137badf73d54f742bde`. No runtime code, dependency, or backend was changed. No concurrency fixture was executed. This report qualifies the proposed design, not a supported provider.

## Finding

A local revision ledger alone is insufficient. An old HTTP request can remain delayed outside the worker, then reach Qdrant after a newer request. Local acknowledgement loss does not cancel that request. The provider must reject its stale revision when it reaches the native submission boundary.

Exact 1.19.2 source provides a native conditional-upsert fence. This is stronger than client arrival ordering or a read-then-write sequence. The first defensible profile is a single-node collection with persistent point tombstones, explicit conditional upserts, and exact bounded revision operands. Native behavior still needs real race and restart tests.

## Source-confirmed submission boundary

`LocalShard::submit_update` classifies conditional upserts as filter-resolving operations. They enter `submit_update_filter_resolving`. Ordinary operations instead hold `update_lock.read()` across WAL append and enqueue. [Exact dispatch](https://github.com/qdrant/qdrant/blob/v1.19.2/lib/collection/src/shards/local_shard/shard_ops.rs#L59), [classification](https://github.com/qdrant/qdrant/blob/v1.19.2/lib/shard/src/resolve.rs#L35).

The conditional path holds `update_lock.write()`. It sends a `Plunger` through the update queue and awaits its response. It then resolves the filter against current segments, rewrites the accepted subset into a plain upsert, and appends/enqueues that record before releasing the fence. The WAL lock is not held during draining. The source explicitly documents this ordering contract. [Exact fenced submission](https://github.com/qdrant/qdrant/blob/v1.19.2/lib/collection/src/shards/local_shard/resolve_submit.rs#L73).

This means the predicate is not merely tested when HTTP parsing begins. Earlier appended updates are processed before resolution. No regular or conditional submission can append between resolution and the resolved operation's append. A later conditional request first drains the resolved earlier operation. This is an inference from the actual lock and queue sequence, not executed evidence.

The update worker awaits operation application before receiving its next queue signal. `Plunger` sends its response when reached. Apply also holds the operation write lock and segment update mutex. `CollectionUpdater` dispatches point processing within those guards. These prevent ordinary concurrent apply from splitting predicate selection and point replacement on the legacy conditional apply path. The normal new submission path resolves before WAL instead. [Worker loop](https://github.com/qdrant/qdrant/blob/v1.19.2/lib/collection/src/update_workers/update_worker.rs#L69), [Plunger](https://github.com/qdrant/qdrant/blob/v1.19.2/lib/collection/src/update_workers/update_worker.rs#L254), [apply guards](https://github.com/qdrant/qdrant/blob/v1.19.2/lib/collection/src/collection_manager/collection_updater.rs#L43).

Resolved accepted points become a single plain-upsert WAL record. Replay therefore does not reevaluate the predicate against a different state. Whole-point upsert replaces both vectors and payload. Use one operation containing identity, revision, state, and vectors; do not follow it with an unguarded payload revision write. [Resolution](https://github.com/qdrant/qdrant/blob/v1.19.2/lib/shard/src/resolve.rs#L200), [whole-point replacement](https://github.com/qdrant/qdrant/blob/v1.19.2/lib/shard/src/update/points/upsert.rs#L144).

The source notes per-replica predicate resolution. Equal replica state is assumed for equal results. This report does not establish distributed linearizability, resharding behavior, or failover safety. `ordering=strong` is an additional profile setting, not a replacement for revision predicates. [Submission caveat](https://github.com/qdrant/qdrant/blob/v1.19.2/lib/collection/src/shards/local_shard/resolve_submit.rs#L1), [official ordering guarantees](https://qdrant.tech/documentation/scaling/consistency-guarantees/).

## Missing points and acknowledged no-ops

The default `Upsert` mode inserts absent points regardless of the filter. It updates existing points only when the filter matches. `InsertOnly` excludes existing points. `UpdateOnly` excludes absent points and existing points failing the filter. These decisions read current segment state. [Exact retention branches](https://github.com/qdrant/qdrant/blob/v1.19.2/lib/shard/src/update/points/upsert.rs#L74).

The exclusion helper finds existing points that do not match the predicate. Absence cannot satisfy that exclusion. It handles deferred segment copies when selecting IDs. These source facts explain why an absent point can be inserted by a stale conditional upsert. [Exact filter helper](https://github.com/qdrant/qdrant/blob/v1.19.2/lib/shard/src/update/helpers.rs#L60).

Predicate rejection can be a completed no-op, not an HTTP conflict. The upstream test expects a successful response while a rejected update leaves its prior version unchanged. Do not map HTTP 200 to “this incoming revision replaced the point.” [Tagged conditional tests](https://github.com/qdrant/qdrant/blob/v1.19.2/tests/openapi/test_conditional_update.py#L15).

After completed acceptance, reconcile the stored point's identity and revision. If the stored revision is greater than or equal to the incoming revision, the operation is superseded or already applied. Equal revisions must represent the same deterministic projection. Different equal-revision content is an invariant error, not permission to overwrite. Unknown outcomes retain the previous local checkpoint.

`wait=true` reaches a worker path that explicitly flushes WAL before application. Success returns `Completed`; timeout and other statuses remain distinct. This is source evidence for the proposed acceptance boundary, not a reproduced crash guarantee. [Native wait handling](https://github.com/qdrant/qdrant/blob/v1.19.2/lib/collection/src/update_workers/update_worker.rs#L346), [HTTP update wait result](https://github.com/qdrant/qdrant/blob/v1.19.2/lib/collection/src/shards/local_shard/shard_ops.rs#L159).

The worker can continue after an operation error. Plunger completion alone does not certify that every prior operation succeeded. Malformed or failed writes need reconciliation. Storage errors, partial application, and subsequent recovery must be tested separately; do not claim the lock sequence proves those failure profiles.

## Exact u64 revision representation

ROM revisions are u64. Qdrant payload integer matching uses i64; numeric range operands and fallback numeric comparisons use f64. A single range comparison against a large u64 can lose precision above 2^53. Do not cast or truncate the revision. [Exact payload types](https://github.com/qdrant/qdrant/blob/v1.19.2/lib/segment/src/types.rs#L60), [range representation](https://github.com/qdrant/qdrant/blob/v1.19.2/lib/segment/src/types.rs#L3351), [numeric checking](https://github.com/qdrant/qdrant/blob/v1.19.2/lib/segment/src/payload_storage/condition_checker.rs#L324).

Propose two scalar integer fields: `rom_revision_hi = revision >> 32` and `rom_revision_lo = revision & 0xffffffff`. Each value is in 0..=4,294,967,295. Each fits i64 and is exactly representable in f64. Lexicographic comparison preserves every u64 revision. This is a mathematical design inference; its actual indexed and unindexed filters still need conformance.

Use a predicate equivalent to:

```text
stored_hi < incoming_hi
OR (stored_hi == incoming_hi AND stored_lo < incoming_lo)
```

Encode it with a filter `should` list containing a high-word range and a nested `must` filter. Use integer equality for the high word. Never add a branch that permits a missing revision field on an existing point. Reject foreign or malformed existing points during reconciliation.

Send one point per predicate, or group only points sharing exactly the same incoming revision. A request-level update_filter applies to the operation's point set; it does not derive a different comparison threshold from every incoming point. Explicitly use `update_mode=upsert`, `wait=true`, and the selected ordering. Keep the raw full revision as a decimal string for diagnostics if needed; do not use it as the native numeric fence. [Exact request schema](https://github.com/qdrant/qdrant/blob/v1.19.2/lib/api/src/rest/schema.rs#L1497).

## Tombstone policy

Physical point deletion removes the payload revision predicate's state. The next stale default upsert can create that absent point. A client ledger cannot prevent an already forwarded old request from doing so. Keep a native tombstone point under the same ID with its newer revision and `rom_live=false`.

A tombstone must remove sensitive text, embeddings, and other source payload. Every query must require `rom_live=true`. A named-vector collection with an empty vector map is a candidate tombstone representation; acceptance must prove the exact server permits it and retains the payload fence. If unavailable, specify an inert vector plus mandatory live filter and test it. Do not physically delete the fence as routine cleanup.

Tombstone retirement requires a separate proven request-quiescence and history/replay horizon, or retirement of the entire physical generation. Preserve generation identity so old requests cannot target a newly reused collection name. Native token revocation or host queue drain alone does not prove all forwarded requests have ended.

## Required real-service qualification

Pin Qdrant 1.19.2's executed image digest and persistent configuration. Restrict the first profile to a single node and one writer authority. Use real HTTP requests with no adapter retry. Capture only sanitized method counts, response statuses, revision outcomes, and fault counts.

1. Complete a newer conditional upsert before forwarding an older held HTTP request. Verify the older request cannot replace vectors or payload.
2. Suppress the old request's real response after backend processing. Submit newer state and then replay the old request. Verify the final newer state.
3. Race initial inserts into an absent point with differing revisions. Verify the highest revision survives regardless of arrival order.
4. Repeat equal revisions with identical content. Send conflicting equal-revision content and verify the adapter rejects reconciliation.
5. Exercise revisions around 2^32, 2^53, i64::MAX, and u64::MAX. Test indexed and unindexed payload conditions.
6. Write a newer tombstone, then release a stale pending live upsert. Verify absence from query results and retained tombstone revision after restart.
7. Verify the absent-point counterexample after physical deletion as negative evidence. Keep it outside the supported tombstone profile.
8. Restart after acknowledged completion and after lost acknowledgement. Reconcile through a fresh native client before publishing the local checkpoint.
9. Inject unavailable service, worker/storage failures where practical, malformed revision fields, and bounded response failures. Do not advance checkpoints on unconfirmed results.

These tests establish only their executed profile. Multi-node writes, shard changes, alias/generation switches, physical tombstone cleanup, and severe storage failures require separate qualification. A durable local ledger remains useful for checkpoints and recovery, but it is not the native stale-write fence.
