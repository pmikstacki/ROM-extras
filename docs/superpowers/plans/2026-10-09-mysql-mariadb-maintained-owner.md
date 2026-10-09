# Shared MySQL/MariaDB ownership implementation plan

> For agentic workers: use superpowers:executing-plans. Root implements maintained files; one final reviewer owns a named ignored note.

**Goal:** Implement and qualify one maintained protocol driver over the shared SQL ownership contract on both native servers.
**Architecture:** A private generic SQL Executor owns the unpooled async SDK/runtime. A borrowed transaction facade preserves shared arbitration.
**Tech stack:** mysql_async0.37.1 minimal-rust/native-tls-tls, Tokio1.53.1, Rust1.99, MySQL8.4 and MariaDB11.8.
**Spec:** ../specs/2026-10-09-mysql-mariadb-maintained-owner.md

## Constraints and review focus

No SQL-core API or existing provider API changes. Keep full public ROM pin and all historical fixtures/evidence.
Check detached SDK cleanup, partial-result success, malformed binary types, vendor mixing and hidden host callbacks.

### Task1: Bounded protocol implementation

Files: crates/rom-mysql/{Cargo.toml,src/lib.rs,src/config.rs,src/connection.rs,src/worker.rs,src/transaction.rs}; workspace manifest/lock.

- [x] Write and execute failing public configuration/native expectations.
- [x] Implement validated minimal configuration and worker-owned absolute deadlines/retirement.
- [x] Implement native ownership projection, exact updates, prepared limits/result checks and poison/rollback semantics.
- [x] Run public native success/failure cases on each server.

### Task2: Independent qualification and archive consumer

Files: tests/mysql-owner-consumer; scripts/check-mysql-owner; scripts/check-mysql-owner-consumer.py; scripts/prepare-mysql-owner-package.mjs; scripts/check-all.

- [x] Add exact arbitration/stale/ordered-lock tests, malformed-state independence and native prefix rollback.
- [x] Verify actual response loss, peer-close retirement, dribbling deadline and async Drop.
- [x] Execute source and normalized archive consumers separately for MySQL/MariaDB.
- [x] Audit exact executed dependency graphs, licenses, source integrity and Rust declarations; preserve earlier lock identities.

### Task3: Integration

Files: primary research, native guide, README/support and verification record.

- [x] Request final source review and resolve substantive findings.
- [x] Freeze all sources and run full local verifier; compare membership and hashes.
- [x] Record precise native profiles and remaining recovery/TLS/Storage limits; commit/push only after verification.
