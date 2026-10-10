# Verified blob recovery implementation plan

> **For agentic workers:** Use superpowers:executing-plans. Root owns all maintained files; final reviewer is read-only.

**Goal:** Copy and verify a host-approved bounded blob manifest without overwriting existing objects.
**Architecture:** A transport-free crate consumes the public BlobStore port. Host adapters retain credentials and endpoint policy.
**Tech Stack:** Public pinned ROM, existing Tokio, tokio-util, serde and serde_json; no new registry dependency.
**Spec:** ../specs/2026-10-10-blob-recovery.md

- [x] Add failing external API tests for manifest limits, exact keys, no overwrite and unknown publication.
- [x] Implement `rom-blob-recovery` façades, validated manifest codec, count-only reports and bounded sequential copy/verify.
- [x] Add deadline/cancellation, corrupt content, prefix progress and redaction tests.
- [x] Run independent source and normalized archive consumers against RustFS and Azurite.
- [x] Record source and emulator evidence separately; document host checkpoint/restore obligations and remaining gaps.
- [x] Review source once, run affected checks and full verifier, then commit and publish.

The [verification record](../../verification/blob-recovery-2026-10-10.json) identifies the executed source and native runs.
The complete ROM-extras goal remains active. This plan covers the bounded external-object recovery increment only.

File ownership: root edits Cargo manifests, crate modules/tests, consumer, scripts and documentation.
Review focus: cancellation after publication, missing destination versus denied reads, corrupted equal-length content, duplicate keys and secret diagnostics.
