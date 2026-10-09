# Shared SQL ownership implementation plan

> **For agentic workers:** Use superpowers:executing-plans task-by-task in this chat.

**Goal:** Implement and natively qualify reusable ownership transaction fencing for the SQL core.

**Architecture:** Typed owner state and transitions use a driver transaction port. Drivers own locks, rollback and commit classification.

**Tech Stack:** Rust1.99, dependency-free SQL core; existing postgres0.19.14 fixture and PostgreSQL18.6.

**Spec:** ../specs/2026-10-09-sql-ownership.md

## Global constraints

Preserve APIs, original fixtures and all evidence. No complete Work ledger, private ROM policy, automatic retry or automatic takeover.
Use primary web sources and finite errors. Full SQL Storage remains a separate required deliverable.

## Review focus

- Old pooled connections must fail after takeover commits.
- Unknown claim commit must not issue usable ownership.
- An old transaction can finish before takeover acquires the control lock.
- Failed operations must roll back all staged writes.
- Generation overflow must fail without resetting ownership.

### Task1: Generic transitions and native consumer

Files: crates/rom-sql-core/src/{owner,owner_transaction}.rs and facade; tests/ownership.rs; tests/sql-owner-consumer/{Cargo.toml,Cargo.lock,src/{main,driver,cases}.rs}; scripts/check-sql-owner and archive preparation.

Interfaces: OwnerToken::new([u8;32]); OwnerState::new(generation,Option<OwnerToken>); Ownership; OwnerTransaction::{lock_owner,write_owner,commit,rollback}; claim_owner; takeover_owner; release_owner; with_owner.

- [x] Record failing public transition tests before implementation.
- [x] Implement typed validation, exact transitions and rollback/unknown handling in named modules.
- [x] Execute native PostgreSQL races, ordered held transactions, stale separate writer/release, rollback, exhaustion and actual committed acknowledgement loss.
- [x] Run the independent consumer against the normalized archive and preserve tables/logs.

### Task2: Integration gate

- [x] Run one final source review, affected checks and dependency audit.
- [x] Freeze sources and run the complete local verifier.
- [x] Record exact results and limitations; update support; commit and publish.

This plan progresses SQL Task2. It does not complete public ROM incremental Work publication or six-provider Storage conformance.
