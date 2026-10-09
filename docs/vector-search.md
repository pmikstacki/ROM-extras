# Authorized vector search

`rom-projection-core` supplies `VectorQuery`, `VectorSearch`, `VectorSearchScope`, `VectorSearchPolicy` and `VectorSearchTarget`.
The existing text search API remains available. Both coordinators share private current Resource hydration.
The [Qdrant target](qdrant-target.md) implements native dense queries through this public capability.

## Host integration

The host selects the Resource kind, actor, fixed collection, metric, model and exact `DocumentMapping`.
It implements current `VectorSearchPolicy` for that scope. The policy must use bounded authoritative checks.
The host computes the query embedding. No automatic model execution or cross-collection lookup occurs.
Keep query vectors and service credentials on the server.
Pass only approved `ProjectedView` results to presentation components.

```rust,ignore
let mut search = VectorSearch::new(
    runtime, actor, Document::KIND, qdrant, host_policy, mapping,
)?;
let query = VectorQuery::new(host_embedding, 10, 64)?;
let approved_current_resources = search.execute(query).await?;
```

The [independent executable consumer](../tests/qdrant-target-consumer/src/vector.rs) demonstrates this sequence on actual SQLite/redb Resources.
It exports authorized public Runtime history with host vectors before querying the native service.
The [fixture runner](../scripts/check-qdrant-target-consumer.py) supplies private configuration and retained source databases.
Run `./scripts/check-qdrant-target` with the documented qualification environment and owned TLS fixture.
The gate also runs the same consumer against normalized Cargo archives.

## Query and disclosure contract

A query admits 1..=4096 finite float32 components, 1..=64 results and result-limit..=256 native candidates.
Negative zero is canonicalized. Debug output omits vectors, keys, model scope and credentials.
An optional exclusion preserves the exact original Resource kind/id, including Unicode and separators.
Dimension, mapping, model, selected input fields and metric remain bound to the coordinator.
Only the core can mint `ApprovedVectorQuery`, after current host authorization.

The coordinator checks query authority before dispatch, after the backend response and before final disclosure.
Current denial takes precedence over a backend failure.
Foreign, duplicate, excluded or excessive candidates are refused before Runtime hydration.
Each candidate passes a current public `Runtime::read_projected` read.
Missing, denied, deleted and changed-revision Resources are omitted.
Every field selected for the embedding must remain present in the current field projection.
This also applies to secondary embedding fields. An otherwise readable Resource does not authorize hidden embedding inputs.

The operation supplies no global authorization snapshot, recall or pagination guarantee.
Concurrent policy changes after an individual public read remain subject to ROM's public read semantics.
Omitted candidates can underfill results. The coordinator does not fetch unbounded additional pages.
The policy, provider and embedding implementation remain trusted host boundaries.

## Native Qdrant contract

Use the fixed standalone generation described in [Qdrant configuration](qdrant-target.md).
Queries accept a collection-scoped `r` or `prw` JWT. Use a separate administrator connection for provisioning.
A read-only `r` credential is qualified independently and cannot write points.

The adapter uses `POST /collections/{name}/points/query`, `using=embedding` and exact search.
Native filters require the exact kind, profile and live marker. The exclusion uses the shared original-key hash.
The request asks for only six identity/revision payload fields and no stored vector or selected values.
Dot scores descend. Euclid and Manhattan distances ascend. Ties preserve native order.
Scores must be finite, but are never disclosed through the Resource result.
The adapter verifies original-key hashes and reconstructs the u64 ROM revision from two checked u32 halves.
It never substitutes Qdrant's internal point version for a Resource revision.

Fixed native configuration and markers are checked before and after the query under one total deadline.
The shared transport enforces verified HTTPS, 1 MiB bodies, fixed endpoints, and no proxy, redirect or automatic retry.
The native query has an explicit seconds-based timeout and read consistency of one for the qualified single replica.
Dropping the future stops local waiting. A read-only native query can continue until its server timeout.
Errors contain finite classifications; wire bodies, query vectors and credentials are excluded.
See the [official API research](research/qdrant-vector-query-2026-10-09.md).

## Verification boundaries

Core tests cover 34 controlled-candidate authorization scenarios on actual SQLite/redb databases, plus public input validation.
These cases do not qualify a native vector service.
Pure response tests cover revision boundaries, invalid ordering, duplicates, payload leakage and bounded admission.

Native Qdrant 1.19.2 cases qualify three-dimensional Dot, Euclid and Manhattan ranking, exact Unicode/slash keys,
empty results, bounded output, native exclusion, current hydration and read-only JWT enforcement.
The relay holds an actual native query response while the host revokes query, row or embedding-field authority.
A separate held-response case changes actual collection metadata and verifies refusal.
Administrator-seeded native points verify live, kind and profile filters.
Native payload corruption tests reject oversized revision halves and mismatched original keys.
All databases, collections and process logs remain retained.

429, oversize, redirects, malformed JSON, timeout and cancellation use authored protocol responses, separately from native service evidence.
The relay counts requests to prove no automatic query retry.
Required gates reject diagnostic-only replay mode; direct loss-probe invocation remains available. Logs are checked for both read/write tokens and administrator credentials.
Cosine normalization, other dimensions, distributed replication, approximate recall, rebuild/cutover and release acceptance remain unqualified.
REST cannot detect recreation with copied markers; exclusive host administration and no name reuse remain mandatory.

The [verification record](verification/vector-search-2026-10-09.json) identifies the frozen sources, successful complete verifier and archive hashes.
