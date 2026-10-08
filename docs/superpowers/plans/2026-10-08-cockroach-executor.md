# CockroachDB executor qualification plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Exercise the shared SQL executor and existing PostgreSQL driver against real CockroachDB, including SERIALIZABLE retry and restart retention.

**Architecture:** Share worker connection construction and existing deadline/panic scenarios between PostgreSQL and CockroachDB. Keep server-specific settings and serialization tests separate. This qualifies a driver execution boundary while full ROM Storage remains incomplete.

**Tech Stack:** Public rom-sql-core Executor, postgres 0.19.14, CockroachDB v26.3.2, persistent loopback development fixture.

**Spec:** Task 4 of [complete plan](2026-10-08-rom-extras.md), [primary assessment](../../research/cockroachdb-executor-assessment.md).

## Constraints and review focus

- Every decision uses recorded primary research.
- Reuse the common executor, not private ROM storage modules.
- No waiting timeout or SQL40003 outcome may be treated as rollback.
- Require actual40001; retry the complete SQL transaction including reads, without external side effects.
- Explicitly rollback23505 and verify both absence and fresh transaction usability.
- Literal loopback only for NoTls; no admin port publication.
- Preserve previous PostgreSQL checks and all persistent artifacts.
- Resource limits2CPU/2GiB are experimental, not vendor minimum; single-node nonproduction licensing scope only.

## Task 1: Real shared-driver acceptance

Files: tests/common/pgwire/{mod,connection,cases}.rs; tests/public-consumer/tests/{postgres_executor,cockroach_executor}.rs; consumer feature manifest; scripts/check-cockroach and check-all; evidence/docs.

- [x] Inspect exact official image identity/help; start persistent fixed loopback fixture and verify SQL availability.
- [x] Add shared connection and unchanged existing deadline/panic assertions; preserve PostgreSQL-specific checks.
- [x] Add Cockroach typed worker, deadline, panic,23505 rollback/fresh use, actual40001 whole-unit retry, byte-key identity, and acknowledged restart retention.
- [x] Run affected PostgreSQL/Cockroach gates and warnings-denied Clippy; failures remain evidence.
- [x] Review independently; freeze runtime sources and run full configured verifier.
- [x] Record limitations, source hashes and actual results; commit and push working branch.

No dependency version changes, complete Storage, ownership fence,40003wire injection,TLS,quorum or crash durability claim is included.
