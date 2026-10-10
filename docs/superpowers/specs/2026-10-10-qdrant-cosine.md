# General finite-input Qdrant Cosine profile

## Outcome

Add an explicitly selected Cosine representation profile to the existing Qdrant projection and vector query adapter.
Application authors provide ordinary nonzero finite float32 vectors. The adapter owns numerical preparation and native reconciliation.
Keep original approved Resource identity, revision, values, profile and digest unchanged.

## Contract

Keep `Generation::new`, `PreparedWrite::new`, and all existing public paths stable.
`Generation::cosine_v1_19_2(profile, physical, nonce, dimensions)` selects a distinct generation marker and Qdrant1.19.2 representation policy.
The existing general constructor continues to refuse Cosine, preventing silent changes to older deployments.

Accept1..=4096 dimensions. Prepare nonzero finite components with a sequential float64 squared norm and one float32 rounding after division.
Reject zero vectors before dispatch; mathematical Cosine of zero is undefined. Preserve signed zeros resulting from preparation.
Reject nonfinite inputs and dimension/profile conflicts through existing errors. Tombstones remain empty-vector whole-point replacements.
The prepared request retains submitted vectors separately from original metadata.

Reconcile native readback against one complete eligible tagged preprocessing result, never a mixture of component-wise candidates.
Reference scalar at every dimension, SSE and NEON at dimensions>=16, and AVX/FMA at dimensions>=32.
Use safe Rust references for the exact tagged accumulation, reductions, cutoff and scaling sequence.
The tolerance in Qdrant's preprocessing cutoff is not a readback tolerance. Readback comparison remains float32 bit equality.
No NaN, infinity, extra vector, changed payload, wrong identity or one-bit corruption outside candidate results is accepted.
Total wire/page limits remain unchanged; candidate memory is bounded by four vectors per point and64 points.

Inspect exact server version before dispatch and after readback under the existing total operation deadline.
Retain fixed verified TLS, no redirect/proxy/retry, private collection-scoped credentials, original conditional revision fencing and generation checks.
Matching version alone does not qualify an arbitrary server build or floating-point environment.
Native proof applies to the controlled pinned image and default IEEE rounding/gradual-underflow environment.
Safe references model NEON, but actual ARM execution is not qualified by x86 evidence.

Queries use the same host representation policy without changing approved query/scope contracts.
Current Resource and field authorization, exact keys and public hydration remain in the existing search core.

## Proof

Test scalar/SIMD dimension transitions and tails, fused rounding, cutoff edges, signed zero, maximum finite and subnormal inputs.
Test complete-candidate equality and reject mixed candidates, one-bit corruption and malformed geometry.
Retain prior experiments separately; they do not qualify maintained implementation.

Use independent source and normalized archive consumers against the owned Qdrant1.19.2 TLS/JWT fixture.
Qualify actual writes at1,3,15,16,17,31,32,33,4096 dimensions, ordinary non-unit and extreme finite inputs.
Verify equal retries, stale/conflicting revisions, tombstones, resurrection, scoped denial and exact readback corruption.
Qualify native finite Cosine ranking and current public SQLite/redb hydration.
Exercise actual lost write responses, persistent service restart, inspect-before-replay and durable checkpoint recovery.
Run affected checks, advisory/license/dependency review, one final source review and the full local verifier before publication.

## Boundaries

No SQL/ROM core or UI change. No new registry dependency or production service provision.
Preserve historical collections, fixture data, prototypes and test evidence.
No distributed consistency, generation cutover, custom floating-point mode, arbitrary Qdrant version or production topology qualification is inferred.
Full projection-family and complete ROM-extras acceptance remain open.
