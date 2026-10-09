# Fixed-generation Qdrant target

`rom-qdrant` implements `ProjectionTarget` for one host-owned concrete collection.
The existing `PreparedWrite::new/as_bytes` API remains available.
This increment adds `Generation`, `Distance`, `Qdrant` and an owned `PreparedPage`.

## Host configuration

Use an explicit HTTPS origin, CA certificate, collection-scoped `prw` JWT and operation deadline.
Create a unique collection name and generation nonce. Never reuse the name.
Build `Generation` from the exact `DocumentMapping::profile()`, dimensions and native distance metric.
Admit Dot, Euclid or Manhattan. Cosine returns `Error::Unsupported` because upload normalization changes the approved vector.
Dimensions are bounded to 1..=4096. Names and nonces admit ASCII letters, digits, hyphens and underscores within 128 bytes.

`Generation::definition()` produces a pure administrator provisioning document.
The host sends it through a separate administrator path before constructing the writer.
The definition uses one shard, one replica, one float32 `embedding` vector and a profile/generation marker.
Keep administrator credentials out of `Qdrant`.
Changing the metric, embedding model or representation requires a new profile and physical collection.

```rust,ignore
let generation = Generation::new(
    mapping.profile().clone(), "documents_generation_42", "host_nonce_42", 3, Distance::Dot,
)?;
let definition = generation.definition()?;
// The trusted host provisions this definition through its separate administrator connection.
let config = TlsConfig::api_key(endpoint, ca_pem, collection_prw_token, deadline)?;
let mut target = Qdrant::new(config, generation)?;
target.verify_generation().await?;
let prepared = target.prepare(&[&approved_document])?;
let observations = target.apply(prepared).await?;
```

The [independent executable host](../tests/qdrant-target-consumer/src/main.rs) uses these public contracts.
Its [Runtime case](../tests/qdrant-target-consumer/src/recovery.rs) demonstrates authorized history and deterministic host vectors on SQLite/redb.
The host owns Resource selection, actors, grants, embeddings and generation administration.
No browser or presentation dependency enters Rust.

## Write and reconciliation contract

Preparation is pure. A page admits at most 64 unique documents and 1 MiB of total encoded requests.
Profile, physical name, nonce, dimensions and metric bind each owned page to its preparing target.
A mismatched target refuses the page before network dispatch.

Each conditional whole-point upsert preserves exact original ROM kind/id and both u32 halves of the u64 revision.
The native filter permits replacement only for an older revision with the same original key.
Selected JSON remains a string to avoid native payload numeric coercion.
Tombstones retain identity and revision but remove selected values and all vectors.

The adapter requires completed native operations using `wait=true&ordering=strong`.
It then retrieves every actual point with payload and vectors.
Every payload field and each finite float32 component must match the prepared point.
Equal-revision conflicts, superseded points, missing points and hash collisions return `Rejected`.
A successful HTTP response alone never creates an observation.

`inspect_prepared` performs the same readback without another write.
The adapter checks native configuration before dispatch and after readback under one total operation deadline.
Transport uses fixed HTTPS origins, bounded 1 MiB bodies, explicit authentication, and no proxy, redirect or automatic retry.
Errors contain finite classifications rather than wire bodies or credentials.
Future cancellation stops local waiting. It does not establish native rollback.
Revision filters fence older requests that survive cancellation. Pending checkpoint intent remains available for explicit recovery.

## Collection identity limit

Qdrant REST does not expose its internal immutable collection UUID.
The generation nonce is mutable application metadata. It detects marker loss or drift; it cannot detect recreation with copied metadata.
The host must exclude aliases, name reuse, administrative deletion and snapshot restoration while work is pending.
Collection-scoped `prw` denies writer lifecycle/configuration operations, but another administrator can still violate this host contract.
This target does not provide OpenSearch's native index UUID fence.
See the [official-source research](research/qdrant-target-2026-10-09.md).

## Verification

Run the preserved isolated TLS/JWT fixture at loopback port 55471 before this command:

```sh
source .superpowers/qualification-env-2026-10-09.sh
./scripts/check-qdrant-target
```

The gate executes pure tests and an independent consumer against native Qdrant 1.19.2.
It repeats the consumer against normalized Cargo archives with the public ROM revision explicitly patched.
All three admitted metrics are tested with three-dimensional non-unit, zero, maximum finite and subnormal vectors.
Native cases cover exact retries, full-u64 boundaries, stale/conflicting revisions, tombstones, resurrection, changed vectors,
original-key corruption, missing points, wrong generation and marker drift.
Native scoped requests prove lifecycle, metadata, named-vector, payload-index and other-collection access denial.

The controlled TLS relay forwards a real native write and discards its completed response.
SQLite/redb processes exit with code 70 after confirming pending intent and unchanged cursors.
The fixture server restarts on persistent storage before separate processes recover through public authorized ROM history.
Every database, collection, private host configuration and process log remains retained.
Parser and 429/oversize/redirect/timeout/cancellation cases use authored responses.
They are separate from actual native-service results.
A two-document page loses the second completed response; its durable cursor remains unchanged.
GET-only inspection before replay proves native persistence instead of letting replay conceal missing points.
A separate loss probe removes a newly created fixture point and proves refusal before checkpoint completion.
Another checkpoint reopen verifies both original-key digests and completed cursors.
Two loopback receivers verify that fixture clients reject redirects without contacting the destination.

[Authorized dense vector queries](vector-search.md) now use the same fixed target with current ROM hydration.
Cosine representation, distributed consistency, rebuild/cutover and release acceptance remain incomplete.
The server's copied-marker identity limitation remains an operational exclusion, not a qualified immutable fence.

The [verification record](verification/qdrant-target-2026-10-09.json) records the successful complete verifier and archive hashes.
It also preserves the first failed run, caused by a collision with the map browser port.
The isolated Qdrant fixture now uses port 55471. The original fixture and its data remain archived.
