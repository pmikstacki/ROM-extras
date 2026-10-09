# Sourced import implementation plan

> **For agentic workers:** Use superpowers:executing-plans task-by-task.

**Goal:** Build validated ordinary action imports without a second source controller.

**Architecture:** Host-approved plans bind immutable bytes and public ROM permits. The host invokes the retained ordinary request.

**Tech Stack:** Rust 1.99; published ROM d7ef529; existing sha2 0.10.9; existing serde_json 1.0.151.

**Spec:** ../specs/2026-10-09-sourced-import.md

## Global constraints

Preserve exact keys, public paths, original identity and retry epoch. Keep transports and source activation outside this core. Preserve all historical fixtures and evidence.

## Review focus

- Expiry or dependency rejection after an unknown commit does not establish rollback.
- Output attribution is host attestation, not input field inference.
- SHA-256 does not authenticate an untrusted producer.
- Structural limits are checked after bounded parsing.
- ROM command limits can reject an admitted document.

### Task 1: Pure import core and public native consumer

Files: crates/rom-import/src/{lib,error,document,action}.rs; crates/rom-import/tests/admission.rs; tests/import-public-consumer; scripts/check-import; docs/import.md.

Interfaces: Limits::new(bytes,depth,nodes,string_bytes); ActionPlan::trusted(ActionBinding,SourceGrant,Limits); ActionPlan::prepare(&[u8]); PreparedAction::request() -> (Invocation,SourcePermit).

- [x] Write admission tests; record the expected absent API failure.
- [x] Implement fixed errors, bounded duplicate-free JSON and SHA-256 binding in named modules.
- [x] Add independent consumer for SQLite/redb domain actions, metadata protection, replay, conflicts and authorization.
- [x] Package the crate and execute the same independent consumer against the extracted archive.
- [x] Run affected checks, dependency audit, source review and full local verifier.
- [x] Record evidence, update support documentation, commit and publish.

### Task 2: Import transport and recovery qualification

- [x] Research explicit file and HTTP endpoint ownership and limits.
- [x] Add bounded file and TLS HTTP readers without automatic retries or implicit deletion.
- [x] Qualify protocol failures, unknown acknowledgement and process restart retaining the original request.
- [x] Integrate only after affected and full local verification.

### Task 3: Maintenance family

- [ ] Complete transactional SQL archive collection and fresh restore for every implemented Storage profile.
- [ ] Qualify migration fencing, frozen Work and receipt replay.
- [ ] Verify external blob manifests separately from database archives.

Tasks 2 and 3 retain the approved Task 9 acceptance criteria. Task 1 does not complete that family.
