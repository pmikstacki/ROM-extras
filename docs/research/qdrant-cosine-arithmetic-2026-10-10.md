# Qdrant Cosine arithmetic and service-version research

Inspection date: 2026-10-10 UTC. Official source tag: Qdrant `v1.19.2`.
Extras baseline now includes published OIDC `f97d701`; this agent owns only this ignored research note.
No maintained files, service data, caches or fixtures were changed. No builds or native probes were run.

## Conclusion

Safe Rust float32 operations and `mul_add` can model the tagged scalar/SSE/AVX/NEON normalization sequences.
They cannot establish universal bit equivalence under arbitrary server/compiler floating-point environments.
The candidate contract needs pinned native qualification, normal rounding/gradual-underflow assumptions and honest architecture/compiler limits.
In particular, SIMD scalar tails use `powi(2)`, whose Rust API precision is unspecified.
Do not claim that replacing it with multiplication proves universal equivalence across every build.

Use exact whole-vector candidates from the submitted representation. Never accept approximate components or mix components between candidates.
Keep original approved content identity separate from submitted and native normalized representations.
The root version endpoint is available without widening a collection JWT because the tagged server exempts it from authentication.
Its response establishes the server's asserted version over verified TLS, not an attested CPU, binary or floating-point environment.

## Exact tagged arithmetic

All paths apply the same helper before scaling:
`length < f32::EPSILON || abs(length - 1.0) <= 1.0e-6` returns the original vector unchanged.
Here `length` is the float32 squared norm, not its square root.
[Tagged threshold helper](https://raw.githubusercontent.com/qdrant/qdrant/v1.19.2/lib/segment/src/spaces/tools.rs).

### Scalar

Compute `vector.iter().map(|x| x*x).sum::<f32>()`.
After the threshold check, compute the square root once and divide each component by that root.
This path uses separate multiplication and summation, not fused multiply-add.
Dispatch uses scalar when no eligible architecture/SIMD path is selected.
SSE/NEON require dimensions >=16; AVX requires x86_64 AVX and FMA plus dimensions >=32.
[Tagged scalar implementation and dispatch](https://raw.githubusercontent.com/qdrant/qdrant/v1.19.2/lib/segment/src/spaces/simple.rs).

For a stable reference, encode sequential left-associated accumulation explicitly rather than assuming a future iterator implementation's order.
Current native compiler/artifact qualification must confirm equivalence with the server's compiled scalar `sum` behavior.

### SSE

Process aligned blocks of 16 components through four independent four-lane accumulators initialized to positive zero.
For every lane, multiply `x*x`, then add that rounded product to the old lane accumulator; do not use `mul_add`.
Reduce each accumulator as `(a0+a2)+(a1+a3)`.
Combine the four reduced scalars left-associatively: `((s0+s1)+s2)+s3`.
Add remaining components in original order using `x.powi(2)`.
After the shared threshold, take one square root and divide every component by it.
[Tagged SSE source](https://raw.githubusercontent.com/qdrant/qdrant/v1.19.2/lib/segment/src/spaces/simple_sse.rs).

### AVX/FMA

Process aligned blocks of 32 components through four independent eight-lane accumulators.
Update each lane with fused `x*x+old`, corresponding to `x.mul_add(x, old)` in a safe reference.
Combine groups per lane as `(group0+group1)+(group2+group3)`.
For combined lanes `t`, reduce low/high pairs first: `p_i=t_i+t_(i+4)` for i=0..3.
Then reduce `(p0+p1)+(p2+p3)`.
Add remaining components in original order using `x.powi(2)`.
After the shared threshold, compute `inv=1.0/sqrt(length)` once and multiply every component by `inv`.
Do not substitute direct division: reciprocal multiplication can round differently.
[Tagged AVX source](https://raw.githubusercontent.com/qdrant/qdrant/v1.19.2/lib/segment/src/spaces/simple_avx.rs).

### AArch64 NEON

Process aligned blocks of 16 components through four independent four-lane accumulators with fused `x*x+old`.
Combine groups per lane as `(group0+group1)+(group2+group3)`.
Reduce combined lanes as `(t0+t1)+(t2+t3)`; this differs from SSE's cross-lane reduction.
Add tail components in original order using `x.powi(2)`.
After the threshold, compute reciprocal square root scaling and multiply every component, including the tail.
[Tagged NEON source](https://raw.githubusercontent.com/qdrant/qdrant/v1.19.2/lib/segment/src/spaces/simple_neon.rs).
Arm's intrinsic reference maps `vaddvq_f32` to paired FADDP instructions, confirming that reduction order.
[Arm NEON intrinsic reference](https://arm-software.github.io/acle/neon_intrinsics/advsimd.html).

## Safe Rust fidelity and its limits

Rust documents primitive float32 addition, subtraction, multiplication and division as roundTiesToEven operations.
`mul_add` has one rounding and returns the rounded infinite-precision fused result.
`sqrt` likewise specifies a rounded infinite-precision result.
`powi`, however, has unspecified precision that can vary by platform, Rust version and invocation.
[Rust float32 contract](https://doc.rust-lang.org/std/primitive.f32.html).

Thus explicit parenthesized `f32` arithmetic, `mul_add`, division and square root match the intended IEEE sequence.
Calling host `powi(2)` copies the source-level tail operation but does not prove equality with another compiler's native binary.
Using `x*x` is an implementation inference about constant-power lowering, not a universal Rust language guarantee.
Qualify tails at dimensions 17, 31, 33 and 4095/4096 against the pinned native artifact.
Whole-block dimensions remove the SIMD-tail issue but are not a justified restriction on the entire intended adapter scope.

On x86, MXCSR controls rounding, flush-to-zero and denormals-are-zero for scalar and vector SSE/AVX instructions.
FTZ flushes subnormal results; DAZ treats subnormal inputs as signed zero. These settings can alter exact results.
[Intel floating-point environment documentation](https://www.intel.com/content/www/us/en/docs/dpcpp-cpp-compiler/developer-guide-reference/2025-0/set-the-ftz-and-daz-flags.html),
[Intel architecture manual](https://cdrdv2-public.intel.com/671436/253665-sdm-vol-1.pdf).
LLVM's denormal environment contract does not itself ensure that runtime hardware state agrees with compiler assumptions.
[LLVM language reference](https://llvm.org/docs/LangRef.html).

Do not infer the server environment from the adapter's CPU or from `/` version output.
The inspected normalization files do not set a floating-point environment; this inspection did not audit every dependency/thread initialization path.
A safe arithmetic reference does not read or constrain the server's MXCSR/Arm FPCR.
Different server builds can preserve the same semantic version while changing compiler flags, lowering or environment behavior.

Recommended qualification scope: exact retained Linux x86_64 native Qdrant artifact, recorded hash/version and measured arithmetic cases.
State default round-to-nearest ties-even and gradual underflow as host/server profile assumptions.
Do not introduce unsafe register reads into maintained code merely to create an incomplete remote-environment assertion.
Native ARM, modified floating-point modes, custom fast-math builds and other Qdrant releases remain unqualified.
If native output matches no complete candidate, return rejection/unknown according to existing semantics; never relax equality to force compatibility.

## Datatype and preparation bounds

Tagged dense `VectorElementType` is `f32`; separate aliases exist for float16 and uint8.
[Tagged vector representations](https://raw.githubusercontent.com/qdrant/qdrant/v1.19.2/lib/segment/src/data_types/vectors.rs).
Tagged storage datatype enum includes Float32, Float16, Uint8 and Turbo4.
[Tagged datatype enum](https://raw.githubusercontent.com/qdrant/qdrant/v1.19.2/lib/segment/src/types.rs).
The selected ROM-extras profile must continue admitting only one named Float32 dense vector, 1..=4096 dimensions, without quantization/multivectors/sparse vectors.
Those dimension bounds are local adapter limits; do not report them as the universal Qdrant maximum.
Keep existing finite-component admission and wire/page byte limits.

For nonzero finite float32 inputs, float64 sum-of-squares spans the full 4096-component local range without overflow or underflow.
This follows from float64 range versus float32 maximum/subnormal exponents; it is mathematical reasoning, not executed evidence.
Normalize in float64 and round submitted components to float32, then derive native candidates from those actual submitted bits.
This prevents the retained raw maximum-finite norm overflow case; it does not guarantee native preprocessing is a no-op.
Preserve original document digest and finite typed query identity even when tiny submitted components round to signed zero.
Zero input needs an explicit rejected-input or native-zero-similarity policy.
Do not silently assign mathematical Cosine meaning to the zero vector.

## Service root and scoped JWT feasibility

`src/actix/mod.rs` registers GET `/` and returns `VersionInfo::default()`.
Its authentication whitelist explicitly includes exact `/`.
[Tagged root registration and whitelist](https://raw.githubusercontent.com/qdrant/qdrant/v1.19.2/src/actix/mod.rs).
The auth middleware returns directly to the service for whitelisted routes, skipping token validation.
[Tagged auth middleware](https://raw.githubusercontent.com/qdrant/qdrant/v1.19.2/src/actix/auth.rs).

Therefore a writer can request the root without administrator privilege or a broader JWT grant.
Even an invalid JWT can be ignored there, so success at `/` is not credential or permission qualification.
Continue proving native collection-scoped denials through collection/lifecycle endpoints.
The retained `rom-projection-http::Http::request` clears the origin path and pushes supplied segments.
An empty segment list provides the root path while retaining HTTPS, CA, body, deadline, no-proxy and no-redirect policy.
Do not parse it as a collection response: root has a direct version object rather than the normal result envelope.
The exact version-field schema must be strictly validated by the adapter; missing/malformed/unsupported values must refuse Cosine admission.

Version equality is a compatibility gate, not build attestation or immutable collection fencing.
Bind a native artifact hash and owned fixture metadata in qualification evidence separately.
Actual `prw` root dispatch, custom reverse-proxy policy and version parsing still require implementation/native tests; no requests were run here.

## Retained experiments and requested proof

Read-only inspection found `.superpowers/qdrant-cosine-reference-2026-10-09.rs` follows the arithmetic grouping above.
It calls `powi(2)` for SIMD tails and uses `mul_add` for AVX/NEON lanes.
`docs/verification/qdrant-cosine-reference-comparison-2026-10-09.json` reports 32 exact native comparisons on Linux x86_64.
Two dimension4096 host-normalized counterexamples matched the AVX candidate only.
These disprove using host normalization as an unconditional native preprocessing bypass.
They do not prove every vector, tail/compiler environment or native Arm behavior.

Keep those artifacts unchanged. Qualify maintained code against new owned collections and direct/packaged consumers.
Test 1/3/15/16/17/31/32/33/4095/4096 dimensions, non-unit, maximum finite, subnormal, mixed-sign and signed-zero cases.
Include retained dominant-component counterexamples and threshold-boundary cases.
Prove complete-vector matching: a componentwise mixture of two valid candidates must fail unless it coincides with another whole candidate.
Prove one-bit corruption outside the candidate set fails without checkpoint advancement.
Retain lost-response restart/recovery and current authorized hydration checks.
No approximate native reconciliation, increased authorization or universal cross-CPU claim follows from this research.

Qdrant source is Apache-2.0; preserve upstream attribution and applicable notices when deriving maintained arithmetic code.
[Tagged license](https://raw.githubusercontent.com/qdrant/qdrant/v1.19.2/LICENSE).
