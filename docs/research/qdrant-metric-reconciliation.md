# Qdrant metric reconciliation requirements

Inspection date: 2026-10-09. This is an actual protocol experiment and a design requirement, not Rust adapter acceptance.

Qdrant [documents automatic Cosine normalization](https://qdrant.tech/documentation/manage-data/collections/).
The [pinned metric implementation](https://github.com/qdrant/qdrant/blob/v1.19.2/lib/segment/src/spaces/simple.rs) performs different preprocessing for each metric.
The core's original vector contributes to deterministic document identity. Native retrieval need not return those original bytes.

The [execution record](../verification/qdrant-metric-observation-2026-10-09.json) contains sixteen actual point observations across four fresh persistent collections.
Verified HTTPS writes used the private writer key. Read-only retrieval established native values. The actual service reported version 1.19.2.
The script converted synthetic input to float32 before dispatch and required completed writes. It preserved all collections.

| Metric | `[3,4,0]` retrieval | Tested large finite vector | Tested zero and small vector |
| --- | --- | --- | --- |
| Cosine | `[0.6,0.8,0]` | Returned zeros | Preserved the tested float32 representations |
| Dot | Unchanged | Preserved float32 | Preserved float32 |
| Euclid | Unchanged | Preserved float32 | Preserved float32 |
| Manhattan | Unchanged | Preserved float32 | Preserved float32 |

The large input was the float32 representation of `[3e38,0,0]`. The small input represented `[1e-38,0,0]`.
JSON decimal spelling can differ while representing the same float32 value. Compare typed values, not decimal strings.
These observations do not establish all dimensions, CPU implementations, metrics, input ranges or query behavior.

Define metric-specific preparation and reconciliation before implementing the Rust target.
Preserve original approved content identity. Treat provider normalization as an explicit contract.
Do not silently clip values or infer the original sidecar from normalized native vectors.
Do not substitute a Dot-only final scope to avoid Cosine qualification.
Qualify zero, non-unit, extreme finite and dimension-boundary vectors through actual writes and queries.
Current protocol probes, checkpoint orchestration and public search tests remain separate evidence categories.

## Query and host-normalization experiments

A second verified-HTTPS experiment wrote 80 synthetic points across all four metrics at dimensions 1, 16, 32 and 4096.
Each write required completed status. Each point was retrieved, then queried using the read-only credential.
For Cosine, the candidate preparation computed a float64 norm and rounded normalized components to float32.
Original host-vector hashes remained separate from prepared-vector hashes.
All 80 retrieved float32 vectors matched this experiment's prepared values. All 80 queries returned successful responses.
Dot queries with the tested large finite input returned some scores as JSON null.
Query success does not establish mathematical ranking accuracy for overflow cases.

A third experiment tested dominant components followed by small components at dimensions 16, 32 and 4096.
Two of twelve retrieved vectors differed from the candidate prepared vector, both at dimension 4096.
Therefore, float64 host normalization alone cannot establish exact native-vector reconciliation.
These experiments preserve nineteen fresh collections and all preceding fixtures.
The [numeric experiment record](../verification/qdrant-numeric-query-observation-2026-10-09.json) includes hashes and actual observations.

The pinned [normalization threshold](https://github.com/qdrant/qdrant/blob/v1.19.2/lib/segment/src/spaces/tools.rs) uses a float32 length test.
The [AVX implementation](https://github.com/qdrant/qdrant/blob/v1.19.2/lib/segment/src/spaces/simple_avx.rs) groups accumulation and uses fused multiplication/addition.
These sources explain why a mathematically unit-length candidate can still trigger native normalization.
Do not declare host normalization an exact bypass of native preprocessing.
Keep numeric reconciliation and overflow query semantics explicit before Rust adapter acceptance.

## Exact arithmetic reference comparison

A safe Rust reference reproduced tagged scalar, SSE, AVX and NEON arithmetic sequences without CPU intrinsics or unsafe code.
Rust [mul_add](https://doc.rust-lang.org/std/primitive.f32.html#method.mul_add) supplies the required fused rounding.
The [SSE source](https://github.com/qdrant/qdrant/blob/v1.19.2/lib/segment/src/spaces/simple_sse.rs) uses a different reduction and final division from AVX.
The [NEON source](https://github.com/qdrant/qdrant/blob/v1.19.2/lib/segment/src/spaces/simple_neon.rs) uses fused accumulation and reciprocal multiplication.
The [Arm intrinsic reference](https://arm-software.github.io/acle/neon_intrinsics/advsimd.html) specifies paired reduction for vaddvq_f32.

GET-only inspection compared the reference with 32 existing Cosine points. Every stored float32 component matched an eligible reference bit pattern.
The two earlier dimension4096 counterexamples matched the AVX reference only.
The [comparison record](../verification/qdrant-cosine-reference-comparison-2026-10-09.json) identifies the compiler, source hashes and observations.
No point was written or removed during this comparison.

This supplies an exact candidate reconciliation method rather than an approximate equality tolerance.
It does not qualify a production adapter, all finite inputs, native ARM execution or other Qdrant versions.
Retain original host-document identity separately from provider-normalized vectors when implementing the target.
