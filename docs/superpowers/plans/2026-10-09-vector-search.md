# Authorized vector search implementation plan

> **For agentic workers:** Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add real bounded vector queries with current ROM authorization and public Resource hydration.

**Architecture:** Add parallel typed vector contracts. Preserve text signatures and share private candidate admission/hydration.
The Qdrant target uses its fixed generation, verified HTTP and native dense query API.

**Tech Stack:** Existing Rust/ROM, SQLite/redb, Qdrant 1.19.2, Tokio and serde_json; no new dependency versions.

**Spec:** `docs/superpowers/specs/2026-10-09-vector-search.md`.

## Global constraints

Keep existing text and Qdrant write public APIs stable.
Admit finite 1..=4096 vectors, 1..=64 results, result-limit..=256 candidates and exact original ROM keys.
Derive disclosure fields from the exact DocumentMapping. Only core mints approved query tokens.
Use existing isolated Qdrant TLS fixture on 55471; preserve ports 55467/55468 for maps.

## Review focus

- Hidden secondary embedding fields must suppress an otherwise readable Resource.
- Dot versus distance ordering must not reverse nearest-neighbor ranking.
- Policy denial after backend failure must still control disclosure.
- Duplicate/foreign/excluded candidates must reject before any Runtime hydration.
- Malformed native revision halves must never become a native version or rounded ROM revision.

## Task 1: Generic core contracts and shared hydration

Create `vector_query.rs`, `vector_search_contract.rs`, `vector_search.rs`, `search_context.rs` and vector tests.
Modify only the existing `search.rs` internals and facade exports; keep public text paths/signatures.
Produces VectorQuery::new(vector, result_limit, candidate_budget), excluding(key), typed VectorMetric,
and VectorSearch::new(runtime, actor, kind, target, policy, mapping), execute(query).

- [x] Write public input/approval tests and actual SQLite/redb controlled-candidate scenarios; record missing API RED.
- [x] Implement validated contracts and shared current hydration, preserving the original text field predicate.
- [x] Run core vector and all existing text tests, doctests and Clippy.

## Task 2: Qdrant native query adapter

Create provider `search.rs`, `search_response.rs`, separate parser tests and independent consumer vector cases.
Reuse native generation inspection and original-key point hashing instead of copying encoders or policies.
Produces VectorSearchTarget implementation with bounded actual native candidates.

- [x] Write response/ranking tests and native expectations before adapter implementation.
- [x] Implement live/profile/kind filters, fixed embedding/exact mode, read consistency and total deadline.
- [x] Run native metric ranking and current SQLite/redb authorization cases, including revocation during actual held protocol.
- [x] Extend independent direct/archive consumers and malformed/timeout/refusal evidence; run affected checks.

## Task 3: Review, verification and publication

Update guides, support matrix and measured verification record.

- [x] Obtain one independent source review; fix important findings with concrete RED/GREEN tests.
- [x] Freeze maintained sources and run the complete local verifier.
- [x] Record native versus controlled evidence and remaining limitations; check staged whitespace and publish.

## Results

Core suite, existing text cases, doctests and Clippy passed.
The independent consumer qualified actual Qdrant 1.19.2 queries on SQLite/redb with all three admitted metrics.
Direct and normalized archive consumers passed native filter, held-response authority and corrupted metadata cases.
One final review found two qualification gaps; both have concrete RED/GREEN evidence and are fixed.
The complete `./scripts/check-all` exited 0 with all 1270 frozen maintained files unchanged.
See [measured verification](../../verification/vector-search-2026-10-09.json).
Cosine, other dimensions, distributed consistency, rebuild/cutover and whole-family acceptance remain open.
