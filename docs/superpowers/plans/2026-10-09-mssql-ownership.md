# SQL Server ownership implementation plan

> **For agentic workers:** Use superpowers:executing-plans. Preserve the ledger and historical evidence.

**Goal:** Implement and qualify maintained SQL Server native ownership over the shared SQL core.

**Architecture:** A bounded worker-owned TDS connection opens native transactions. The existing core owns arbitration; the adapter owns locks, bindings and terminal cleanup.

**Tech Stack:** Rust 1.99; Tiberius 0.13.0; Tokio 1.53.1; tokio-util 0.7.19; native SQL Server 2025 fixture.

**Spec:** ../specs/2026-10-09-mssql-ownership.md

## Global constraints

Preserve API paths, prior fixtures and evidence. Keep lib.rs a facade. Force encrypted TDS; certificate trust belongs to the host.
No automatic DDL, retry, reconnect, private Work policy or full-ledger serialization.

## Review focus

- A timeout must retire the client instead of reusing a partially consumed response.
- A native statement error must roll back earlier prepared writes.
- Malformed binary identity and token data must not be padded or truncated into valid state.
- An old connection must be fenced after a competing takeover.
- A runtime misuse must return an error without a nested-runtime panic.

### Task 1: Maintained driver and native acceptance

Files: crates/rom-mssql/src/{lib,config,connection,transaction}.rs; tests/mssql-owner-consumer/src/{main,fixture,cases}.rs; workspace and consumer manifests.
Interfaces: Connection::connect(Config, Deadlines); Connection::begin(&ControlTable); Transaction implements OwnerTransaction; Transaction::execute for trusted prepared native persistence.

- [x] Add independent native tests and observe failure before the driver exists.
- [x] Implement validated configuration, native transaction mapping and retirement.
- [x] Run native cases, unit boundary tests, formatting, Clippy and rustdoc with warnings denied.

### Task 2: Required package qualification

Files: scripts/check-mssql-owner; scripts/prepare-mssql-owner-package.mjs; scripts/check-mssql-owner-consumer.py; scripts/check-all.

- [x] Consume normalized rom-mssql and rom-sql-core Cargo archives; rerun actual native cases.
- [x] Require the gate in the full verifier. Do not skip missing backend credentials.
- [x] Audit dependency advisories, licenses and Rust version compatibility.

### Task 3: Review, documentation and integration

Files: docs/mssql-ownership.md; research and verification records; support status; README.

- [x] Request one fresh source review with read-only ownership.
- [x] Fix substantive findings with regression checks.
- [x] Freeze source evidence; run the full local verifier and record its exit status.
- [x] Document exact native qualifications and excluded failure profiles; commit and publish the reviewed increment.

Ruling: Existing autonomous execution and publication authorization applies; no repeated approval menu.

Completed against 1,289 unchanged source files. Full local verifier exit0.
The first two full attempts remain recorded; isolated OpenSearch recovered native qualification without source or threshold changes.
