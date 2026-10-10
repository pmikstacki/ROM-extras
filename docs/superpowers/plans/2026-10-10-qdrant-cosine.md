# Qdrant Cosine implementation plan

> **For agentic workers:** Use superpowers:executing-plans inline. Root owns all maintained edits. Research owns its private note; one final reviewer owns its private report.

**Goal:** General finite nonzero Cosine vectors with unchanged original metadata and exact tagged-native reconciliation.
**Architecture:** Explicit version-bound generation policy prepares float64-normalized wire vectors. Safe scalar/SIMD references generate bounded complete readback candidates.
**Tech Stack:** Existing Rust, serde_json, projection HTTP and pinned Qdrant1.19.2; no new registry package.
**Spec:** ../specs/2026-10-10-qdrant-cosine.md

## Global constraints

Keep public API paths, general constructor refusal and existing metrics unchanged.
Keep native endpoint controls, total deadlines, payload checks and exact revision fencing.
Preserve all historical data/evidence. No production provisioning.

## Review focus

- Original digest cannot become a digest of normalized provider representation.
- Signed zero and subnormal components must survive JSON/native qualification or fail exact reconciliation.
- Mixed scalar/SIMD candidate components must never produce an accepted point.
- Wrong server version must prevent write/query dispatch; matching version does not prove arbitrary build arithmetic.
- Zero/nonfinite queries must fail before network work.

## Task1: Representation and adapter

Consumes existing `ApprovedDocument`, `ApprovedVectorQuery`, `Generation` and `Entry` contracts.
Produces `Generation::cosine_v1_19_2` and internal owned representation candidates without public core changes.

- [x] Write public constructor/preparation and internal arithmetic/reconciliation tests; preserve missing API RED.
- [x] Implement named preparation/reference modules, internal shared encoder parameter, explicit marker and exact complete-vector comparison.
- [x] Apply the same preparation to queries; add version checks within existing operation budgets.
- [x] Run rom-qdrant tests, Clippy, docs and independent consumer compilation; inspect every result.

## Task2: Native qualification

Consumes Task1 public generation constructor and unchanged `ProjectionTarget`/`VectorSearchTarget` interfaces.
Produces bounded source/archive native service evidence and a runnable public example.

- [x] Extend independent consumer/driver with separate Cosine dimensional writes, corruption, scope and exact identity cases.
- [x] Run source/archive Cosine queries through public SQLite/redb hydration and native loss/restart/checkpoint recovery.
- [x] Keep authored protocol fault evidence separate from actual native qualification.
- [x] Verify dependency graphs, advisories, source review, affected gates and full frozen local verifier.
- [x] Document measured profiles, attribution and remaining limits; prepare publication on the authorized existing branch.

Full frozen verifier: exit0 on1510 unchanged files. Source/archive Cosine and baseline profiles passed.
Two Minor review findings remain deferred: fused/cutoff regression cases and enum prose.
See [verification record](../../verification/qdrant-cosine-2026-10-10.json).
