# KMS migration implementation plan

> **For agentic workers:** Use superpowers:executing-plans inline. Root owns maintained edits; research and final reviewer own private reports only.

**Goal:** Authenticated bounded envelope migration with explicit destination versions and real OpenBao evidence.
**Architecture:** Add compatible optional Kms methods and one reusable Migrator. Share OpenBao request encoding and authenticated response parsing.
**Tech Stack:** Existing Rust1.99, Tokio1.53.1, OpenBao2.7.1 and current bounded HTTPS transport; no new registry package/version.
**Spec:** ../specs/2026-10-10-kms-migration.md

## Global constraints

Preserve APIs, original envelopes, context/AAD, service data and historical evidence. No production provisioning or ROM private APIs.
Migration deadline1ms..=60s; concurrency1..=64; plaintext4096bytes; OpenBao portable pinned range1..=i32::MAX.

## Review focus

- Unsupported/unapproved selection fails before plaintext acquisition.
- Cancellation and Busy cover the decrypted buffer between calls; no detached work.
- Late provider completion cannot publish after absolute deadline.
- Response profile/key/version drift rejects publication.
- Latest remains compatible; pinned versions never silently become latest.

## Task1: Generic contract and OpenBao adapter

Consumes Kms, Binding, Envelope and SecretBytes. Produces EncryptionVersion, MigrationLimits, Migrator and compatible optional Kms methods.
Files: new rom-kms encryption_version/migration modules and public tests; existing provider/façade/manifest; OpenBao requests/transit plus unit tests.

- [x] Write public boundary, optional-capability, binding, mismatch, admission/deadline/cancellation tests. Run `cargo test -p rom-kms`; expect missing API RED.
- [x] Implement named modules and OpenBao shared encoder/parser; version validations before I/O.
- [x] Run tests/Clippy/docs and existing independent consumers; expect terminal0, inspect output.

## Task2: Native and archive qualification

Consumes Task1 public methods. Produces actual-service evidence, public example and accurate support documentation.
Files: native migration consumer/test support, package preparation/gate, secrets docs/support/research/verification.

- [x] Test actual rotated version2 after latest3, cross-key/AAD, denial/floors/revocation/expiry and same-volume recovery.
- [x] Run representative source and extracted archive independent consumers; preserve runs and redact secrets.
- [x] Check source/archive dependency integrity/advisories/licenses/MSRV; run fresh final review.
- [x] Run affected gate and full frozen verifier; expect terminal0 and unchanged inputs.
- [x] Record exact evidence/limitations and prepare authorized branch publication.

## Executed evidence

Affected gate passed 30 cases and the independent archive smoke test.
The extracted archive service test target compiled. Full frozen `./scripts/check-all` returned 0; all 1520 captured inputs remained unchanged.
The final archive consumer uses `/tmp/secrets-packaged-OaTJe8/consumer`.
Source and archive consumer lockfiles match; 157 registry packages passed checksum, source-byte, license and MSRV checks.
The final review archive finding was reproduced and fixed. No Minor finding was deferred.
See [verification metadata](../../verification/kms-migration-2026-10-10.json).
The complete ROM-extras goal remains active.
