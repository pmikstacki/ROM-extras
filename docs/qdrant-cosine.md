# Qdrant Cosine representation

`Generation::cosine_v1_19_2` selects general nonzero finite Cosine vectors for the tagged Qdrant1.19.2 profile.
The existing `Generation::new(..., Distance::Cosine)` still returns `Error::Unsupported`.
Use a new physical collection and nonce. Do not change an existing generation's metric or copy its markers.

```rust,ignore
let generation = Generation::cosine_v1_19_2(
    mapping.profile().clone(), "documents_cosine_42", "host_nonce_42", 3,
)?;
let definition = generation.definition()?;
// The host provisions this definition with separate administrator credentials.
let config = TlsConfig::api_key(endpoint, ca_pem, collection_prw_token, deadline)?;
let target = Qdrant::new(config, generation)?;
```

The host supplies ordinary model vectors through existing `HostVectors` and authorized history contracts.
For example, an approved `[3,4,0]` remains unchanged in ROM and its digest.
The adapter submits `[0.6,0.8,0]`. It computes the norm in float64, then rounds each component once to float32.
This also handles maximum finite and subnormal inputs without float32 norm overflow or squared-input underflow.
Zero vectors are rejected before dispatch because mathematical Cosine has no zero direction.

## Exact native reconciliation

Qdrant selects scalar, SSE, AVX/FMA or NEON preprocessing according to dimensions and CPU capability.
The adapter computes bounded complete candidates using safe references to the tagged arithmetic sequence.
It accepts readback only when every component matches one complete eligible candidate by float32 bits.
It does not combine components from different candidates or accept approximate equality.
The upstream preprocessing cutoff does not become a readback tolerance.
Original payload, exact Resource keys, revision, digest and tombstone checks remain unchanged.

The marker is `cosine-f64-qdrant-1.19.2-v1`. The adapter inspects the exact server version within every generation check.
Version checks occur before dispatch and after readback under the existing total operation deadline.
A matching version is a compatibility check, not native build attestation or immutable collection identity.
Keep exclusive host administration, fixed verified TLS and collection-scoped credentials.
The root version endpoint is public; successful root access does not prove token permissions.

The core canonicalizes author-supplied negative zero before it produces `ApprovedDocument`.
The adapter preserves the approved input and signed zero produced by its own normalization.
Native discrepancies reject reconciliation; they do not rewrite the approved digest or advance a checkpoint.

## Public queries and recovery

`VectorSearchTarget` uses the same representation policy for query vectors.
The native response parser requires finite scores in descending order and validates exact identity/revision fields.
Scores are not exposed as authority. The existing core hydrates candidates through current public ROM reads.
Current Resource/field authorization and excluded exact keys remain part of that core contract.
A zero query is rejected before any native request.

The independent [consumer](../tests/qdrant-target-consumer/src/cosine.rs) validates observations with public `PageIntent::reconcile`.
It does not inspect private metadata or use mutable ROM source.
The [native gate](../scripts/check-qdrant-target) runs existing metrics and the explicit Cosine profile on source and normalized archive consumers.
Each invocation creates new collections and preserves every prior run.

```sh
# Configure the private owned fixture through the existing qualification environment first.
./scripts/check-qdrant-target
```

The gate covers1,3,15,16,17,31,32,33,4096-dimensional writes, maximum finite, subnormal, mixed-sign and dominant-component vectors.
It checks signed-zero output, exact original metadata, retries, stale/conflicting revisions, tombstones and resurrection.
Actual SQLite/redb cases cover ranking, current authorization, write-response loss, persistent native restart and inspect-before-replay recovery.
Authored unsupported-version,429, oversized-response, redirect, timeout and cancellation cases remain separate from actual native-service evidence.

## Qualification limits and attribution

Native evidence applies to the controlled pinned Qdrant1.19.2 image on Linux x86_64 and its default floating-point environment.
Safe NEON references do not qualify actual ARM execution.
Rust does not guarantee universal cross-build precision for the tagged SIMD tail's `powi(2)`.
Unknown compiler builds, rounding modes and FTZ/DAZ settings require separate qualification; exact readback still fails closed.
Other versions, distributed replication, custom vector datatypes, generation switching and production topology remain unqualified.
The existing three-dimensional query consumer does not establish high-dimensional ranking or approximate recall.

The arithmetic references derive from [Qdrant's tagged spaces implementation](https://github.com/qdrant/qdrant/tree/v1.19.2/lib/segment/src/spaces).
The package includes [upstream attribution](../crates/rom-qdrant/QDRANT-NOTICE.md) and the [Apache2.0 license](../crates/rom-qdrant/QDRANT-LICENSE.txt).
The package declares `MIT AND Apache-2.0`. No new registry dependency is required.
See the [research record](research/qdrant-cosine-arithmetic-2026-10-10.md) for source and arithmetic limits.
