# Qdrant ProjectionTarget research

Date: 2026-10-09. Source review only. No builds, credentials, provisioning, or service mutations.

## Native identity limitation

Qdrant 1.19.2 stores an internal collection UUID. Its REST CollectionConfig conversion explicitly discards that UUID. GET collection information exposes configuration and application metadata, but not an immutable physical UUID. A copied application marker cannot distinguish delete/recreate. Do not claim the OpenSearch index UUID guarantee. [Tagged REST conversion](https://raw.githubusercontent.com/qdrant/qdrant/v1.19.2/lib/collection/src/operations/types.rs), [internal configuration](https://raw.githubusercontent.com/qdrant/qdrant/v1.19.2/lib/collection/src/config.rs).

A fixed collection name plus explicit host generation nonce/profile marker supports an operationally owned generation. Check the expected marker and vector configuration before write and after readback. This detects missing/replaced markers and configuration drift. It does not fence administrative recreation with copied metadata, an administrator race between checks, or a restored snapshot carrying old markers. Never call a mutable nonce genuine native identity.

## Scoped credential enforcement

Tagged RBAC requires global Manage for CreateCollection, UpdateCollection, DeleteCollection, aliases, and shard-management operations. A collection-scoped JWT `rw` cannot perform those operations, including collection PATCH metadata. However `rw` can create/delete named vectors and payload indexes through `write().extras()`. [Exact permission checks](https://github.com/qdrant/qdrant/blob/v1.19.2/lib/storage/src/rbac/ops_checks.rs).

Prefer collection-scoped `prw` (PointsReadWrite) for the writer. It permits point mutation/read but denies extras. Check the exact collection name in the token grant. Prepare collection configuration separately with an administrator. Neither `rw` nor `prw` binds authorization to internal UUID; grants are name-based. Another global administrator can still recreate that name. [Access modes and enforcement](https://github.com/qdrant/qdrant/blob/v1.19.2/lib/storage/src/rbac/mod.rs), [REST-to-dispatcher checks](https://github.com/qdrant/qdrant/blob/v1.19.2/lib/storage/src/dispatcher.rs).

Native qualification should prove writer GET/upsert succeeds while create/delete/PATCH metadata/named-vector mutation fails. Do not infer effective token restrictions from client configuration alone. Credential contents must remain host-only.

## Concrete target contract

Use a host-selected concrete collection, never an alias. Fix profile fingerprint, generation marker, named vector `embedding`, dimensions, metric, float32 datatype, and normalization policy. For the initial native profile, declare one shard and one replica on a standalone server. Reject sparse/multivector/extra named-vector configuration and unexpected datatype. Quantization requires its own explicit profile and tests; do not infer original-vector preservation from quantized search scores.

The local `PreparedWrite` already hashes the original kind/id into a UUID-shaped point ID. Its payload retains original key, profile, digest, live marker, revision high/low halves, and selected JSON as a string. Preserve that string to avoid payload number coercion. Retain exact expected payload and approved vector in a target-bound prepared page. Reject duplicate point IDs and profile mismatch before intent. Bound document count and total encoded request.

The high and low revision halves each fit u32. Compare them lexicographically: older high half, or equal high half with older low half. This represents the full u64 range without relying on native signed-u64 filtering. Equal revision must not overwrite. An exact duplicate can reconcile; equal revision with different data must conflict. Verify original kind/id even when hashed IDs coincide.

Qdrant upsert supports `update_filter` on existing points; missing points are inserted under upsert mode. Original-key filtering prevents replacing an existing hash collision. It cannot prevent initial insertion into a newly recreated empty collection. Native filter indexes and strict-mode settings must permit these conditional filters. [Upsert API](https://api.qdrant.tech/api-reference/points/upsert-points), [tagged conditional operation](https://github.com/qdrant/qdrant/blob/v1.19.2/lib/shard/src/operations/point_ops.rs).

Use PUT `/collections/{name}/points?wait=true&ordering=strong`. Require a completed native operation response, then retrieve the actual points with payload and vectors. Operation acknowledgement alone is not an observation. Inspect all IDs, original keys, profile, both revision halves, live state, digest, and exact stored selected JSON. For tombstones, verify stale selected values and vectors are removed by whole-point replacement. [Retrieve API](https://api.qdrant.tech/api-reference/points/get-points).

RemoteObservation must follow actual readback. Do not merely copy prepared metadata after a successful HTTP response. Greater stored revision, wrong key, wrong profile, changed fields, or equal-revision changed digest must reject that prepared operation. Preserve Unknown for lost acknowledgement, malformed response, timeout, or uncertain readback. Keep a separate inspect-prepared method for reconciliation without another write.

The tagged ordering definition describes strong writes through a permanent leader. This is not a general cross-request transaction, collection-incarnation fence, or proof of distributed linearizability. Native qualification here should stay standalone unless replica consistency, leader changes, and conditional concurrent writes are independently established. [Tagged ordering](https://github.com/qdrant/qdrant/blob/v1.19.2/lib/collection/src/operations/point_ops.rs).

## Metrics, transformation, and digest

Tagged float32 Euclid, Dot, and Manhattan preprocess methods return the supplied vector unchanged. Cosine preprocess routes CPU-specific SIMD or scalar normalization. Scalar normalization uses float32 sum-of-squares, skips zero/already-normalized lengths, and divides by the square root. Original bit equality is not valid for arbitrary Cosine upload. [Exact metric implementation](https://raw.githubusercontent.com/qdrant/qdrant/v1.19.2/lib/segment/src/spaces/simple.rs).

Qdrant documents automatic Cosine upload normalization. Named vectors each have their own dimensions and distance metric. [Official collections documentation](https://qdrant.tech/documentation/manage-data/collections/).

Local ApprovedDocument digest includes the original approved vector bits, profile, original key, revision, selected fields, and tombstone state. Thus it cannot be recomputed from a transformed Cosine readback without a defined representation contract. A payload digest alone does not establish that actual vector matches the approved vector.

Defensible choices are: retain original vector bits in bounded payload and validate the actual native vector against a qualified transformation policy; or prepare a documented host-normalized vector whose accepted native representation remains unchanged. The latter needs native proof across admitted dimensions/norms and CPU behavior. A universal scalar reproduction must not assume SIMD bit equality. A tolerance comparison must be explicit in the profile and cannot claim exact-bit corruption detection. Keep Dot/Euclid/Manhattan as real admitted metrics where float32 storage readback preserves values; do not narrow to one metric solely for easier tests.

For every admitted metric, reject nonfinite components, dimensional mismatch, unsupported datatype, and invalid normalization contract. Zero vectors need an explicit Cosine policy. Test subnormal/large components, negative zero normalization, JSON f32 roundtrip, and actual native retrieval. Search scores are not a substitute for stored-vector inspection.

## Existing module seams

`ProjectionTarget::prepare` requires numeric, metric, wire, and profile validation before intent. `apply` requires actual stored observations and bounds/deadlines. Existing OpenSearch demonstrates target-bound PreparedPage and inspect-prepared; reuse that contract shape rather than HTTP acknowledgement shortcuts.

The shared projection HTTP transport already owns explicit TLS authentication, fixed origin, bounded JSON bodies, and safe TargetFailure classifications. Reuse it for Qdrant endpoints. Keep Qdrant response parsing, collection configuration, conditional preparation, and vector reconciliation in named Qdrant modules. Do not add backend-specific behavior to the module facade or transport.

## Required native tests and claims

- Fresh live upsert, exact retry, stale revision, equal revision changed content, tombstone, attempted resurrection, and original-key mismatch.
- Full-u64 boundary revisions, including 2^32, 2^63, and u64::MAX, with actual stored halves.
- Lost response and inspect-only reconciliation; process restart after write before checkpoint; pending page retained on failure.
- Payload/vector mutation, malformed retrieval, missing point, extra IDs, unsupported metric/config, profile/marker drift.
- Delete/recreate with absent marker must fail. Recreation with a copied marker must be recorded as undetectable under REST and excluded by host policy.
- Collection-scoped `prw` denies lifecycle/config/named-vector changes; different collection access fails.
- Every admitted metric: retrieve actual stored vector and qualify its representation; Cosine non-unit input proves normalization occurs.

This increment can qualify fixed-generation standalone writes and exact stored reconciliation under exclusive administrative ownership. It does not complete search authorization, automatic rebuild, active-generation cutover, distributed safety, or an immutable native collection-identity fence. Those remain separate whole-goal requirements.
