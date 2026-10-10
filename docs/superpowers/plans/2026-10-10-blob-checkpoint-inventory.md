# Blob checkpoint inventory implementation plan

> **For agentic workers:** Use superpowers:executing-plans. Root owns all maintained files; one final reviewer owns only its private report.

**Goal:** Recover exact complete current Ready Blob bindings from a checkpoint and persist a private recovery inventory.
**Architecture:** An isolated restored native Runtime performs sequential public BlobService reads through read-only observers. Existing recovery manifests and public Stage provide transfer and publication semantics.
**Tech Stack:** Public pinned ROM/rom-backup/rom-blob, existing extras maintenance/recovery, serde/serde_json and Tokio. No new registry dependency.
**Spec:** ../specs/2026-10-10-blob-checkpoint-inventory.md

## Constraints and ownership

Keep existing APIs stable. Add public Context check/wait access without changing existing behavior.
Root owns `crates/rom-blob-checkpoint`, the Context visibility addition, Cargo files, one independent consumer, scripts and documentation.
No source edits in `/root/ROM`; use the immutable public pin only. Preserve all fixture data, worktrees and evidence.

## Review focus

- Cancellation while a supervised read still runs must not reuse observer state or publish a partial inventory.
- Two Resources with equal content must retain their distinct exact physical keys.
- Different aliases may contain the same physical key; limits apply across the complete inventory.
- Mixed archives must retain their full schemas while inspection registers only Blob.
- Publication uncertainty must preserve artifacts; decoded JSON does not authenticate checkpoint provenance.

## Task1: Collection and private durable inventory

Files: new crate modules `limits`, `error`, `model`, `codec`, `inspection`, `observer`, `publication`; thin `lib.rs` façade.
Interfaces: `Inspection::prepare(Backend,&Path,&Path,Vec<(String,Arc<dyn BlobStore>)>,Limits)->Result<Inspection,Error>`;
`Inspection::collect(self,Context)->Result<Inventory,Failure>`;
`Inventory::{bindings,counts,checkpoint,encode,decode,for_store,publish,read}`.

- [x] Write failing public-consumer tests for native archive binding, equal content, limits, excluded rows and durable artifact refusal.
- [x] Run the tests and preserve the missing-API RED result.
- [x] Implement validation, isolated native preparation, per-invocation read-only observation, exact row checks, cleanup and safe failures.
- [x] Implement strict codec, per-alias recovery manifests and Stage-based no-overwrite publication.
- [x] Run both native feature profiles, the disabled-feature consumer, all failure cases, fmt, Clippy and docs.

## Task2: Independent actual protocol and integrated qualification

Files: `tests/blob-checkpoint-public-consumer`, `scripts/check-blob-checkpoint`, `scripts/prepare-blob-checkpoint-package.mjs`, `scripts/check-all`, guides and verification record.
Reuse existing fixture adapters/protocol helpers; do not copy native clients or private key generation.

- [x] Build independent source and normalized archive consumers; qualify SQLite/redb mixed checkpoints, equal-content owners and multi-store recovery against RustFS/Azurite.
- [x] Reopen durable inventory in a fresh process; read exact restored Resources and verify denied caller behavior.
- [x] Record dependency graph integrity, licenses/MSRV and audits. Preserve native evidence separately from mocks and authored failures.
- [x] Run one final source review, affected checks and the full verifier on a frozen source.
- [x] Document capabilities, configuration and remaining gaps; prepare integration on the authorized branch.

Commit and publication results are recorded in the private progress ledger after execution.
