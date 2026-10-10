# Maintained Oracle ownership implementation plan

> For agentic workers: use superpowers:executing-plans. Root owns maintained edits; one final source reviewer owns a named ignored note.

**Goal:** Implement and qualify native Oracle ownership while retaining the complete ROM-extras objective.
**Architecture:** Private OCI worker over existing SQL executor; shared arbitration and strict owner validator; separate native session/error semantics.
**Tech stack:** rust-oracle0.6.3, external OCI23.26.3.0.0, Rust1.99, Tokio1.53.1 runtime admission, retained Oracle fixture55458.
**Spec:** ../specs/2026-10-10-oracle-maintained-owner.md

## Global constraints

Preserve public ROMd7ef529040eec60dc869034c2d33130219db85fe, every public API, worktree, native dataset and failure artifact.
No unsafe adapter code, library bundle, new administrative grant, automatic callback replay or claim of whole-operation containment/full Storage.
Root alone edits maintained files; researcher/reviewer may write only their named ignored notes.

## Review focus

- Oracle numeric getters coerce strings and fractions: independent malformed cases must prove strict type and exact integer checks.
- DDL can implicitly commit: reject prepared DDL before native execution and prove prefix rollback/no new table.
- OCI cancellation/close is not absolute containment: retain explicit unqualified limits rather than a timed-out ticket claim.
- Caught ORA-00001 leaves preceding writes pending: require explicit whole rollback, then safe fresh reuse.
- Descriptor injection and secret diagnostic text: reject structured field injection and demonstrate redacted public errors/Debug.

### Task1: Maintained native owner boundary

Files: crates/rom-oracle/{Cargo.toml,src/lib.rs,src/config.rs,src/connection.rs,src/worker.rs,src/transaction.rs,src/value.rs,src/metadata.rs}; public admission tests.

- [x] Write a missing-public-API consumer RED for Config/Deadlines/ControlTable/Value and ownership.
- [x] Implement validated configuration, native-owned capacity-one worker, strict metadata and poisoned native transaction.
- [x] Execute native owner transitions and statement/descriptor/numeric limits through the public crate.
- [x] Run affected tests, Clippy and documentation with warnings denied.

### Task2: Native qualification and independent archive

Files: tests/oracle-owner-consumer/{Cargo.toml,Cargo.lock,src/main.rs,src/fixture.rs,src/cases.rs,src/restart.rs}; scripts/check-oracle-owner, scripts/check-oracle-owner-consumer.py, scripts/prepare-oracle-owner-package.mjs.

- [x] Add native failure/rollback/stale-owner/lock/metadata cases with exact independent observer evidence.
- [x] Preserve real in-memory ownership through same-image/same-volume graceful restart; require FREEPDB1 READ WRITE readiness.
- [x] Run source and normalized archive consumers with explicit external native loader prerequisites.
- [x] Audit exact root/source/archive dependencies, licenses and existing checksums.

### Task3: Evidence and integration

Files: docs/research/oracle-maintained-owner-2026-10-10.md, docs/oracle-ownership.md, docs/support.md, README.md, docs/verification/oracle-maintained-owner-2026-10-10.json; scripts/check-all.

- [x] Request final source review and resolve substantive findings with observed RED→GREEN.
- [x] Freeze source/graph identity and execute affected checks plus full local verifier.
- [x] Record actual qualifications and remaining containment/TLS/Storage gaps; publish the verified increment.
