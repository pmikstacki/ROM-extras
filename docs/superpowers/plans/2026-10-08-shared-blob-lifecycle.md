# Shared blob lifecycle implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Run the existing native BlobService lifecycle assertions on real S3 and Azure providers through one shared test harness.

**Architecture:** Extract Azure's provider-neutral scenarios into tests/common/blob_lifecycle. The observation wrapper delegates every physical operation to a generic public BlobStore. Azure and independent S3 consumer tests supply their own public constructors; no production implementation is copied.

**Tech Stack:** Published ROM d7ef529040eec60dc869034c2d33130219db85fe, SQLite/redb, existing AzureBlob and public S3 Adapter, Tokio, futures-util 0.3.34.

**Spec:** Task 7 of [the approved complete plan](2026-10-08-rom-extras.md), [complete design](../specs/2026-10-08-rom-extras-design.md).

## Global constraints

- Every project and technology decision must use web research.
- Do not depend on another chat's dirty working tree or copy its private modules.
- A lost commit acknowledgement is an unknown outcome until authoritative recovery establishes the result.
- Keep public provider constructors, limits, deadline, and retry policy unchanged.
- Preserve Azure assertions and all four existing S3 scenarios.
- Preserve object and native database evidence; no implicit cleanup.

## Research and decisions

[Lifecycle source assessment](../../research/s3-blob-lifecycle-assessment.md) establishes the shared public BlobStore boundary and required drain/drop order.
[Existing Azure assessment](../../research/azure-blob-lifecycle-assessment.md) explains tombstone Denied and post-publication Unattached outcomes.
Select shared test orchestration because providers implement the same public contract. Keep wire fault, retention, TLS, and cloud tests separate.
No production algorithm changes; this increment adds acceptance tests and refactors existing test orchestration.
Use the current green full-verifier baseline before extracting Azure's unchanged scenarios.

## Review focus

- Wrong digest or failed input stream must not invoke provider create.
- Unauthorized reads must fail before invoking provider get.
- Repeated Ready upload and detach must not duplicate physical creation or change receipt identity.
- Provider success followed by deleted reservation must retain a real unattached object and original Denied cause.
- Native reopen requires service drain, Runtime drain, and owner release; neither orphan receipt nor detached state authorizes immediate deletion.

## Task 1: Shared real-provider lifecycle acceptance

Files:
- Add tests/common/blob_lifecycle/mod.rs as a facade.
- Add observed.rs for generic physical-operation observation.
- Add helpers.rs for native Runtime, reserve, staged input, drain, and unique artifact paths.
- Add attached.rs and unattached.rs for the two preserved scenario functions.
- Reduce crates/rom-azure-blob/tests/lifecycle.rs to provider-specific wrappers; remove its superseded active observation helper.
- Add two feature-gated wrappers to tests/public-consumer/tests/s3_blob.rs using its existing fixture mutex.
- Add optional native adapters and futures-util to the consumer's s3-lifecycle feature.
- Require s3-lifecycle in scripts/check-s3; document and record actual results.

Steps:
- [x] Record the unchanged green baseline and both dependency graphs before extraction.
- [x] Extract the existing Azure scenarios with provider label and artifact-root parameters; preserve their checks.
- [x] Add independent S3 lifecycle wrappers, feature dependencies, and required gate.
- [x] Resolve only required dependency additions; compare all pre-existing package identities and checksums.
- [x] Run Azure and S3 affected gates; require all existing assertions and new lifecycle cases.
- [x] Temporarily omit the observed wrapper's real create call; require S3 lifecycle failure, then restore the exact source and rerun acceptance.
- [x] Run dependency advisory/license checks for added packages and Clippy with warnings denied.
- [x] Freeze all runtime inputs, run the complete configured verifier, and confirm identical inputs afterward.
- [x] Obtain independent review, correct findings, record limits, and commit on codex/initial-extras.

This task does not complete wire acknowledgement loss, cancellation/drain faults, corrupt-provider reads, cloud TLS, trusted grace cleanup, or packaged release acceptance.
