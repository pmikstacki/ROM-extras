# S3 fixture profiles implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Run the same public S3 qualification against independently selected SeaweedFS and RustFS fixtures.

**Architecture:** Keep one acceptance suite and the unchanged published ROM constructor. A private test helper binds each explicit profile to its exact loopback endpoint and restart target. Preserve the failed SeaweedFS evidence while qualifying RustFS separately.

**Tech Stack:** Existing independent Rust consumer, Tokio, ROM revision d7ef529040eec60dc869034c2d33130219db85fe, unchanged object_store 0.14.2, SeaweedFS 4.48, RustFS 1.0.1.

**Spec:** [Approved complete design](../specs/2026-10-08-rom-extras-design.md), Task 7 of [the complete implementation plan](2026-10-08-rom-extras.md).

## Global constraints

- Every project and technology decision must use web research.
- Do not depend on another chat's dirty working tree or copy its private modules.
- A lost commit acknowledgement is an unknown outcome until authoritative recovery establishes the result.
- Keep the public three-second timeout, one-second connect timeout, and zero SDK retries.
- Keep eight barrier-released candidates over sixteen fresh keys; accept only one winner and seven confirmed conflicts in the strict test.
- No dependency or production adapter changes.

## Research and decision

[Exact RustFS source review](../../research/rustfs-conditional-create-assessment.md) establishes conditional-write checking under the commit guard.
[SeaweedFS recovery assessment](../../research/seaweedfs-race-assessment.md#recovery-locks-after-routed-rejection) identifies recovery locking after routed rejection.
[Garage source review](../../research/garage-conditional-create-assessment.md) excludes that candidate because its inspected PUT path lacks this condition.
Select RustFS for an additional qualification attempt, not as an already supported provider.
The existing gate continues to run the same tests for the configured profile; no scenario is removed or marked ignored.

## Review focus

- Unexpected endpoint or profile must fail before provider access or restart.
- A profile must never restart another fixture; validate its exact container label first.
- A configured wrong secret must produce Denied, and the correct account must confirm no created object.
- A timed-out write stays Unknown; reconciliation cannot replace strict responsiveness qualification.
- Passing RustFS must not imply passing SeaweedFS, cloud TLS, crash durability, or lifecycle acceptance.

## Task 1: Select a fixture and run identical acceptance

Files:
- Add `tests/public-consumer/tests/s3_fixture/mod.rs` as a facade.
- Add `tests/public-consumer/tests/s3_fixture/profile.rs` for validated fixture identity and restart.
- Modify `tests/public-consumer/tests/s3_blob.rs` to use the helper and generic test names.
- Update `docs/s3-compatible.md`, `docs/support.md`, and `docs/research/decision-log.md` with separate profile results.
- Retain verification logs and frozen input identities under `docs/verification/`.

Steps:
- [x] Run the existing conformance test with the RustFS environment; retain its expected endpoint assertion failure.
- [x] Add the profile helper: legacy unset/seaweedfs uses port 55455; rustfs uses port 55456. Both require bucket rom-extras and region us-east-1.
- [x] Map restart targets only through fixed profile identities; verify fixture label before restart.
- [x] Run all four S3 tests and Clippy through scripts/check-s3 with RustFS selected.
- [x] Run a mismatched endpoint and unknown profile; require failure before backend access.
- [x] Re-run SeaweedFS basic conformance without substituting its strict failed result.
- [x] Freeze runtime inputs, run scripts/check-all with all existing fixture environments and RustFS selected, then compare input identities.
- [x] Review source and evidence independently, correct findings, record actual results, and commit on codex/initial-extras.

The overall extension goal and additional S3 acceptance work remain active after this increment.
