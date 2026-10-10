# Maintained CockroachDB ownership implementation plan

> For agentic workers: use superpowers:executing-plans. Root owns maintained edits; one final source reviewer owns a named ignored note.

**Goal:** Qualify a maintained CockroachDB ownership adapter while preserving existing native provider APIs.
**Architecture:** Share native PGwire lifecycle; keep PostgreSQL/Cockroach session semantics separate. Validate common ownership primitives in SQL core.
**Tech stack:** tokio-postgres0.7.18, Tokio1.53.1, Rust1.99, CockroachDB26.3.2 retained fixture55457.
**Spec:** ../specs/2026-10-09-cockroach-maintained-owner.md

## Global constraints

Keep public ROMd7ef529040eec60dc869034c2d33130219db85fe and all prototypes, worktrees, pending work and native evidence.
No default demo/remote service, cluster-setting mutation, automatic callback retry or claimed full Storage.
Root alone edits maintained files; reviewer is read-only except its named ignored note.
Preserve current native errors/public paths; SQLSTATE details remain transport-level evidence without native messages.

### Task1: Shared PGwire and validated ownership primitives

Files: crates/rom-pgwire/{Cargo.toml,src/lib.rs,src/connection.rs,src/error.rs}; crates/rom-sql-core/src/{lib.rs,owner_row.rs}; core tests.
Adapt crates/rom-postgres/src/{connection.rs,transaction.rs}; workspace/consumer manifests and locks as required.

- [x] Write public primitive validator/transport expectations; execute RED for the missing APIs.
- [x] Extract owned lifecycle and complete native operations without exposing arbitrary runtime/Client callbacks.
- [x] Share pure format/identity/token validation while retaining provider-specific SDK type checks.
- [x] Adapt PostgreSQL facade with unchanged constructors, paths and READ COMMITTED behavior.
- [x] Run native source/archive PostgreSQL ownership, actual response loss and maintained recovery gates.

### Task2: Native Cockroach provider and independent qualification

Files: crates/rom-cockroach/{Cargo.toml,src/lib.rs,src/config.rs,src/connection.rs,src/transaction.rs}; tests/cockroach-owner-consumer.
Files: scripts/check-cockroach-owner, scripts/check-cockroach-owner-consumer.py, scripts/prepare-cockroach-owner-package.mjs; shared relay fixed fixture whitelist.

- [x] Execute missing-provider public RED and required-session negatives before implementation.
- [x] Implement explicit vendor/session validation, SERIALIZABLE durable locking and bounded native ownership.
- [x] Qualify current shared native ownership assertions plus independently observed locks and native40001 rollback/reread.
- [x] Qualify actual COMMIT response loss, peer close, startup timeout and async-handoff Drop.
- [x] Qualify retained-guard native restart on the exact current image/data volume.
- [x] Run normalized archive consumers with independently resolved features.

### Task3: Shared mapping cleanup and integration

Files: maintained MySQL/MSSQL row mappings if shared validation replaces exact duplication; required gates/check-all; research/guide/support/README/verification.

- [x] Preserve native SDK conversion strictness and existing errors while replacing duplicated validation.
- [x] Audit exact root/source/archive dependency graphs and unchanged prior checksums.
- [x] Request final source review and resolve substantive findings with observed RED→GREEN.
- [x] Run affected provider gates; freeze sources and run the full local verifier.
- [x] Record actual profiles, distributed/TLS/Storage gaps and native evidence; commit/push only after verification.
