# Maintained PostgreSQL ownership implementation plan

> For agentic workers: use superpowers:executing-plans. Root owns all maintained edits; reviewers own only explicitly named ignored notes.

**Goal:** Deliver a real maintained PostgreSQL ownership mapping over the shared SQL core.
**Architecture:** One bounded runtime/client/transport task; native transaction driver; shared arbitration stays in rom-sql-core.
**Tech stack:** tokio-postgres0.7.18, tokio1.53.1, Rust1.99, PostgreSQL 18.6.
**Spec:** ../specs/2026-10-09-postgres-maintained-owner.md

## Constraints and review focus

Preserve public paths/history/evidence and public ROM pin. No Storage stub or private policy copy.
Check swallowed statement errors, malformed oversized BYTEA, partial connect/terminal responses, active-runtime Drop and unintended plaintext/failover.

### Task1: Bounded maintained adapter

Files: crates/rom-postgres/{Cargo.toml,src/lib.rs,src/config.rs,src/connection.rs,src/transaction.rs,src/tests.rs}; workspace manifest/lock.

- [x] Add failing public configuration/native consumer expectations before implementation.
- [x] Implement exact validated native limits/table config and safe errors.
- [x] Implement bounded connection/owned task retirement and native transaction protocol.
- [x] Confirm real protected writes, stale fencing, rollback and malformed-state denial.

### Task2: Independent native and packaged qualification

Files: tests/postgres-owner-consumer; scripts/check-postgres-owner; scripts/prepare-postgres-owner-package.mjs; scripts/check-all.

- [x] Keep historical SQL-owner consumer intact; require maintained public consumer.
- [x] Exercise native success/failure, limits, caught-error poison and runtime boundaries.
- [x] Execute normalized archives with exact dependencies and no workspace feature reliance.
- [x] Audit dependency changes, licenses and declared Rust versions; run all affected gates.

### Task3: Integration

Files: primary-source research, native guide, README/support and verification record.

- [x] Request one final source reviewer; correct meaningful findings.
- [x] Freeze complete source and execute full local verifier.
- [x] Preserve logs and limits; publish under existing authorization only after verification.
