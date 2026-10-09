# PostgreSQL native recovery implementation plan

> For agentic workers: use superpowers:executing-plans. Root owns maintained edits; reviewers own explicitly named ignored notes.

**Goal:** Qualify actual PostgreSQL18.6 same-volume SIGKILL recovery through the maintained public ownership API.
**Architecture:** A public consumer retains acknowledged guards; a bounded owned runner restarts only the pinned fixture.
**Tech stack:** Existing rom-postgres/tokio-postgres0.7.18, PostgreSQL18.6, Python3 owned process helpers.
**Spec:** ../specs/2026-10-09-postgres-native-recovery.md

## Constraints and review focus

No production API/dependency changes. Preserve all fixtures, evidence and public ROM pin.
Check fake readiness, guard reconstruction, wrong/unlogged control tables, unrelated containers and interrupt cleanup.

### Task1: Native recovery consumer

Files: tests/postgres-owner-consumer/src/{main.rs,restart.rs}; scripts/check-postgres-owner-consumer.py; scripts/prepare-postgres-owner-package.mjs.

- [x] Add exact public guard/data observations, cluster/start identity and UNLOGGED negative control.
- [x] Run a no-restart mutation and confirm native observation failure.
- [x] Implement pinned same-container/image/volume SIGKILL runner with bounded cleanup and sanitized crash-start observations.
- [x] Execute direct and archive consumer recovery; retain failure and success evidence.

### Task2: Verification and publication

Files: docs/postgres-ownership.md, docs/support.md, primary research, recovery verification record.

- [x] Run affected checks and request one final source review.
- [x] Freeze tracked and untracked sources; run ./scripts/check-all.
- [x] Verify frozen source hashes; record exact compiler/lockfile/log identities and scope limits.
- [x] Commit and push after verification under existing authorization.
