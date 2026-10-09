# Typed authorized vector search

Add real vector queries to ROM-extras without changing existing text search public paths.
Keep authorization, Resource policy and field projection in the host and published ROM Runtime.
Use the current pinned public ROM revision. No new registry versions or unpublished interfaces are required.

## Shared core

Add `VectorQuery`, `VectorMetric`, `VectorSearchScope`, private `ApprovedVectorQuery`,
`VectorSearchPolicy`, `VectorSearchTarget` and `VectorSearch`.
A query contains 1..=4096 finite float32 values, 1..=64 results and a candidate budget between result limit and 256.
Canonicalize negative zero. Optional exclusion retains an exact validated same-kind original Resource key.
No raw provider DSL, score threshold, inference request or native ID lookup is exposed.

Bind the coordinator to Runtime, host actor, Resource kind, target, current policy and the exact DocumentMapping.
Require matching profile and vector dimension. Derive all disclosure fields from that mapping, without caller-selected omissions.
An empty field selector represents a declared model without Resource field inputs; current row authorization remains required.
Expose mapping/model, fields, metric, dimensions, generation and limits in the authorization scope.
Sensitive query data must not appear in Debug or errors. Only the coordinator can mint approval.

Check policy before dispatch, after remote completion even on backend error, and before disclosure.
Check target identity and shape before and after backend work.
Share candidate admission and current public Runtime hydration with text search in a named internal module.
Reject excess, foreign-kind, duplicated or explicitly excluded candidates before hydration.
Omit missing, denied, deleted, revision-mismatched or insufficiently projected Resources.
Return only current ProjectedView values in native candidate order. Never return native payloads or scores.
Preserve all existing text search scenarios and its public signatures.

## Qdrant adapter

Use the already qualified fixed-generation standalone target and shared HTTP transport.
Query named `embedding` with a dense float32 array, explicit `consistency=1`, native timeout and `params.exact=true`.
Filter live state, original kind and exact profile. Encode an optional exclusion using the derived original-key hash.
Request only key/profile/live/revision metadata, never selected values or stored vectors.
Check generation configuration before and after the query under one total operation deadline.

Parse bounded `result.points`. Validate native UUID against the exact original key and positive revision from checked u32 halves.
Native point `version` is not a ROM revision. Reject excess or duplicate candidates and malformed metadata.
Require finite ordered scores: descending Dot, ascending nonnegative Euclid/Manhattan; allow ties.
Preserve the exact-bit write profile's Cosine refusal. No ANN recall, stable pagination, or replicated consistency claim is made.

## Evidence

Core tests use actual SQLite/redb and explicitly controlled candidate providers.
Native Qdrant tests verify all three metrics with distinguishable ranking, live/kind/profile filtering and exclusions.
Verify current policy revocation during held native queries, hidden embedding fields, stale/deleted/denied Resources and finite limits.
Authored malformed/oversized responses supplement native tests, without being labeled native-service qualification.
Extend the independent consumer and its normalized archive test. Preserve old fixtures, ports and all historical evidence.
Run affected checks, one independent source review and the complete local verifier before integration.

Rebuild/cutover, Cosine, distributed qualification, globally coherent hydration and release acceptance remain open.
