# Typed Qdrant vector query research

Date: 2026-10-09. Read-only source research. No native fixtures or builds ran.

## Existing boundaries

The current core `TextQuery`, `SearchScope`, `ApprovedTextQuery`, `SearchPolicy`, `SearchTarget`, and `Search` are explicitly text contracts. Search checks current policy before dispatch, after dispatch, and before returning. It validates candidate count/kind/duplicates, then hydrates through public `Runtime::read_projected`. Missing/denied/stale candidates are omitted. It additionally requires the text field to survive field projection.

A vector request cannot honestly use a dummy text query. Preserve these public paths. Add `VectorQuery`, a vector authorization scope, private `ApprovedVectorQuery`, and a vector target seam. Share the internal candidate validation and public hydration helper. The vector path must supply its own explicit disclosure predicate rather than inventing `field()` semantics.

Option A adds `VectorSearch<T, P>` and `VectorSearchTarget`/`VectorSearchPolicy`. It has the smallest compatibility surface. Option B introduces a private generic query coordinator and keeps existing text types as public wrappers. Both can share count/key/revision checks and policy timing. Do not replace the existing SearchPolicy signature with an enum unless compatibility is deliberately managed.

The vector scope should expose kind, fixed profile/generation, dimensions, model/mapping identity, admitted metric, and bounded result/candidate limits. Sensitive vector access must be explicit and Debug omitted. The host policy can decide current authority from this scope. Approvals must be core-minted; transports cannot accept unapproved vectors directly as authorized requests.

## Native request

Use POST `/collections/{collection_name}/points/query` with a dense numeric `query` array and `using:"embedding"`. Set a positive candidate limit and offset zero. Request only original key/profile/live/revision-half payload metadata. Set `with_vector:false`. Omit prefetch, recommendation, discovery, ID lookup, inference, formulas, and arbitrary native DSL. This is a dense nearest-neighbor increment. [Official v1.19.x query API](https://api.qdrant.tech/api-reference/search/query-points).

Validate 1..=4096 finite float32 components against the configured exact dimension. Keep the current target's Dot/Euclid/Manhattan metric profile. Cosine remains separately unsupported by its exact-bit write profile; a query-only workaround cannot fix stored representation qualification. Suggested public limits match text search: 1..=64 results, candidate budget at least result limit and at most 256. These are extras design bounds, not Qdrant native maxima.

Always filter `rom_live:true`, original Resource kind, and exact profile fingerprint. Query the fixed concrete collection and verify generation configuration before and after query. The host still excludes aliases/name reuse/admin restoration. No REST native UUID fence appeared in the prior research. Candidate filters reduce contamination; they do not grant ROM disclosure authority.

Parse `result.points` under the shared transport byte/deadline bound. Reject excess candidates, duplicate native IDs/original keys, malformed metadata, wrong original kind/profile/live state, invalid UUID, or UUID inconsistent with the original-key hash. Reconstruct positive u64 revision from checked u32 high/low halves. Native point `version` is Qdrant's internal version and must not substitute for ROM revision. Retain no backend values or score in SearchCandidate.

If the API adds an excluded origin Resource, validate a bounded same-kind original Key and derive its native ID. Encode a `must_not` `has_id` condition and reject any returned excluded key defensively. Raw dense-vector queries have no automatic origin. ID-based queries internally exclude referenced IDs, but that is a separate native lookup contract and should not be used to bypass public ROM reads. [Tagged query resolution and reference exclusion](https://raw.githubusercontent.com/qdrant/qdrant/v1.19.2/lib/collection/src/operations/universal_query/collection_query.rs).

## Score and ordering

Dot returns inner-product similarity; higher is better. Euclid returns positive Euclidean distance after square-root postprocessing; lower is better. Manhattan returns positive L1 distance; lower is better. Cosine scores are normalized inner-product similarity. These scores have different units and ranges. They must not share a generic higher-is-better check. [Tagged metric postprocessing](https://github.com/qdrant/qdrant/blob/v1.19.2/lib/segment/src/spaces/simple.rs), [distance ordering](https://raw.githubusercontent.com/qdrant/qdrant/v1.19.2/lib/segment/src/types.rs).

Validate score finiteness and monotonic native order using the configured metric. Finite components alone do not guarantee finite accumulation for extreme magnitudes. Reject nonfinite native scores. Keep native candidate order during authorized hydration. Do not reorder ascending distance as descending similarity. Tie order is not an established stable pagination contract; allow ties and avoid pagination claims.

Omit score thresholds initially. Native thresholds are strict: Dot/Cosine `score > threshold`; Euclid/Manhattan `score < threshold`. A future typed threshold must specify similarity versus distance and finite metric-appropriate semantics. [Tagged threshold implementation](https://raw.githubusercontent.com/qdrant/qdrant/v1.19.2/lib/segment/src/types.rs).

ANN candidate bounds are not recall guarantees. Explicit `params.exact:true` is a possible fixed initial profile, but it can increase work substantially. Finite response size does not bound collection scan cost. Fix server timeout plus client whole-operation deadline and qualify the selected exact/approximate mode. Do not silently expose native HNSW tuning or claim complete authorized nearest neighbors after filtering denied/stale candidates.

## Consistency and scoped authentication

Set an explicit read consistency. For the current standalone one-replica target, `consistency=1` matches its profile. Native default is Factor(1). Tagged semantics query N replicas and intersect results; `all` queries all, `majority` queries N/2+1 and intersects, and `quorum` queries all and retains majority-present points. This is replica agreement, not a transaction with ROM authorization or immutable collection identity. [Tagged consistency definitions](https://raw.githubusercontent.com/qdrant/qdrant/v1.19.2/lib/collection/src/operations/consistency_params.rs).

Qdrant CollectionQueryRequest requires read access and no write/manage/extras permission. Collection-scoped `prw` therefore permits query. A read-only scoped token also suffices for a separate reader. Cross-collection lookup has extra scope checks, which the dense-only API avoids. [Tagged query RBAC](https://raw.githubusercontent.com/qdrant/qdrant/v1.19.2/lib/storage/src/rbac/ops_checks.rs).

## Current public hydration and vector disclosure

Hydrate each candidate through public ROM at the exact original key. Require current revision to equal indexed revision and live projected value. Continue past denied, missing, stale, and deleted records without revealing omission reasons. Recheck current query policy after remote completion and before return. Preserve target profile/generation identity checks. No unpublished ROM reader is needed.

The existing text field predicate prevents ranking from returning a projected value that hides the searched field. Vector embeddings can encode multiple fields. A vector policy must address inference from ranking and require the approved mapping's relevant selected fields to remain disclosed. A current projected object alone does not establish authority over every embedded input. Reuse public field projection and an explicit host-selected field set; do not duplicate Resource authorization or trust Qdrant payload as permission.

Changing export/model/field grants requires profile invalidation and generation handling. This increment cannot establish automatic rebuild or safe active-generation cutover. Scores and native payload values should remain internal; return only current ProjectedView values.

## Required real-service evidence

- Dense named-vector query for Dot, Euclid, and Manhattan with distinguishable expected ranking and actual positive distance values.
- Empty result, result/candidate bounds, nonfinite/wrong-dimension input, wrong vector name, and invalid metric/configuration.
- Live/kind/profile filtering, tombstone exclusion, origin exclusion, hash/original-key mismatch, malformed revision halves, and full-u64 reconstruction.
- Policy denied before dispatch and revoked during query; current Resource denied/deleted/changed after indexing; hidden embedded input fields.
- Oversized/malformed response, duplicated candidates, invalid/nonmonotonic scores, timeout, TLS/auth failure, and scope for another collection denied.
- Drifted generation marker/configuration, missing collection, and documented copied-marker recreation limitation.
- Independent package consumer using only public exact ROM pin and extras exports. Protocol response fixtures supplement actual Qdrant 1.19.2 fixtures; they do not establish native ranking or RBAC.

Remaining uncertainty: CPU score accumulation/tie behavior, ANN recall, and replicated read semantics were not executed. Public authorization must not imply a globally coherent snapshot across all hydrated Resources. Candidate budget can underfill results. No new dependency, Cosmos work, or unpublished source API is required for this proposed increment.
