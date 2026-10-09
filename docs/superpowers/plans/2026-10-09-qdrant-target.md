# Qdrant target implementation plan

> **For agentic workers:** Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement bounded fixed-generation Qdrant writes with actual stored-state observations.

**Architecture:** Generation configuration is pure. Prepared pages own exact expected points and conditional requests.
The adapter uses shared projection HTTP and verifies native configuration before writes and after retrieval.

**Tech Stack:** Existing Rust, reqwest, Tokio, serde_json, SHA-256 and native Qdrant 1.19.2.

**Spec:** `docs/superpowers/specs/2026-10-09-qdrant-target.md`.

## Global constraints

Preserve `PreparedWrite::new/as_bytes`, public ROM revision and historical evidence.
Use verified TLS, fixed origin, 64 documents, 1 MiB aggregate request and one total deadline.
No administrative credential in the writer. Never reuse a physical collection name.

## Review focus

- A different generation must reject an otherwise compatible prepared request before dispatch.
- Native float32 JSON rendering must preserve bits across small and large finite components.
- Equal revisions with changed content must not overwrite or yield an observation.
- Partial writes and uncertain responses must leave durable pending intent recoverable.
- Mutable markers must not be described as immutable collection identity.

## Task 1: Pure generation and page admission

Files: `crates/rom-qdrant/src/{generation,page,lib}.rs`, public admission tests.
Produces: `Generation::new(profile, physical, nonce, dimensions, Distance)`, `definition()` and target-bound `PreparedPage`.

- [x] Write public tests for invalid configuration, unsupported Cosine, duplicate keys, profile/dimension mismatch and limits.
- [x] Run the tests; record the absent API as RED.
- [x] Implement generation validation and preparation, reusing the existing conditional encoder.
- [x] Run the crate suite and formatting/Clippy.

## Task 2: Native target and qualification

Files: `crates/rom-qdrant/src/{adapter,reconcile}.rs`, `tests/qdrant-target-consumer`, native fixture helper and scripts.
Produces: `Qdrant::new(TlsConfig, Generation)`, `ProjectionTarget`, `verify_generation`, `inspect_prepared`.

- [x] Write native public-consumer expectations and response validation tests before implementation.
- [x] Implement bounded writes and exact readback; run native and negative response cases.
- [x] Prove scoped `prw` writes and denied lifecycle/configuration operations on an isolated persistent fixture.
- [x] Run direct and normalized archive consumers, including pending checkpoint recovery.

## Task 3: Review and integration

Files: provider documentation, support matrix, verification record and check scripts.

- [x] Obtain one independent source review and resolve important findings with RED/GREEN evidence.
- [x] Freeze maintained sources; run affected checks and the complete `./scripts/check-all`.
- [x] Record tested scope and remaining gaps. Check the staged diff and publish the verified increment.

## Execution ledger

Task 1: public missing Generation/Distance and target API RED preceded implementation.
Local admission, exact response parsing and aggregate wire-bound tests pass.
Task 2: native standalone Dot/Euclid/Manhattan writes and normalized archives pass.
Real forwarded-response loss causes SQLite/redb process exit 70 before checkpoint completion.
Task 3 review: three important qualification/security findings; no deferred minors.
Final fixed: replay concealed missing native state — actual point removal produced replay exit 0 in RED;
GET-only inspection before replay returns exit 71 in GREEN, without advancing pending work.
Final fixed: fixture redirects forwarded credentials — two-receiver RED followed 302;
existing shared redirect refusal produces GREEN with zero destination requests.
Final fixed: singleton recovery missed partial-page acknowledgement — withholding the second response produced RED;
two-document pages preserve pending intent and recover both actual points after native restart in GREEN.
Reopened checkpoints validate both original-key digests and cursor position 2.
Historical evidence remains retained. The user instruction overrides skill cleanup of test evidence.

Full verifier attempt 1: failed at maps-ui startup because this increment occupied its reserved port 55467.
The 1252 frozen files stayed unchanged. Earlier gates passed; this is not a successful full verifier.
Ruling: allocate port 55471 after maintained-source and listener checks; preserve the old container/data as an archive.
The new isolated fixture gets separate private credentials and storage. Repeat affected and full checks.
Cost if wrong: another local fixture startup failure; no production endpoint or unrelated browser configuration is changed.

Task 3: complete local verifier `./scripts/check-all` exit 0 on the port55471 source snapshot.
All 1252 maintained files stayed unchanged during this successful run.
Snapshot SHA-256: `9d6a1ec1dab521d685ddd32e1837e6d83da8421bce271c52ca544ee092e6e1f4`.
Both native and normalized archive consumers passed on the isolated fixture.
Standalone `check-map-ui` and the repeated complete verifier each passed all five browser tests.
Post-verifier edits only record results, mark this plan and link the guide to evidence.
Integration remains a commit/push of this verified increment; the complete ROM-extras goal remains active.
