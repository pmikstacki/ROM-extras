# SQL Server recovery qualification plan

> **For agentic workers:** Use superpowers:executing-plans. Preserve historical fixtures and evidence.

**Goal:** Qualify native deadlock rollback and acknowledged ownership across a retained SQL Server restart.

**Architecture:** Independent public consumer cases exercise the maintained driver; fixture-side native evidence establishes actual1205. An owned orchestrator restarts only the existing fixed fixture.

**Tech stack:** Rust1.99; existing Tiberius0.13.0; SQL Server2025; Python owned-process helpers.

**Spec:** ../specs/2026-10-09-mssql-owner-recovery.md

## Global constraints

No public API changes, private policy copies, automatic runtime retry or removal of historical evidence.
No production API solely for test diagnostics. No weakening of TLS, limits or terminal cleanup.

## Review focus

Wrong native session or1222 must not qualify a deadlock. A changed owner must reject retry.
A copied owner state must not become acknowledged Ownership. Restart must retain the same native data volume.
A failed child or interrupted parent must terminate only owned processes and preserve fixture data.

### Task 1: Actual deadlock evidence

Files: tests/mssql-owner-consumer/src/{deadlock,fixture,cases,main}.rs; scripts/prepare-mssql-owner-package.mjs.

- [x] Add a real C/S cycle, bounded Extended Events and victim/code checks.
- [x] Verify prefix rollback, explicit takeover, stale retry rejection and new-owner success.
- [x] Require the new case from both direct and archive consumers.

### Task 2: Retained native restart

Files: tests/mssql-owner-consumer/src/restart.rs; scripts/check-mssql-owner-consumer.py.

- [x] Observe the missing restart handshake against the previous consumer before adding the profile.
- [x] Keep acknowledged guards in memory; close connections; implement prepare/restart/resume orchestration.
- [x] Verify exact state/data, stale fencing, current writes, release and generation3 reclaim on the same volume.
- [x] Run formatting, Clippy and complete direct/archive native qualification.

### Task 3: Review and full integration

Files: native guide, support/README, research and verification record.

- [x] Request one source review and fix substantive findings with concrete regression checks.
- [x] Freeze sources and run the full local verifier.
- [x] Record actual evidence and excluded profiles; commit and publish the verified increment.

Ruling: Existing autonomous implementation/publication authorization applies; no repeated approval menu.
