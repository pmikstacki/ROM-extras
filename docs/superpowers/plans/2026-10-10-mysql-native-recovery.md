# MySQL and MariaDB native recovery implementation plan

> For agentic workers: use superpowers:executing-plans. Root owns maintained files; final reviewer owns one ignored source-review note.

**Goal:** Qualify retained acknowledged native ownership across actual process death on both InnoDB vendors.
**Architecture:** Public independent consumer holds exact guards; private runner validates identities and owns lifecycle/deadlines.
**Tech stack:** Existing pinned MySQL28.0.3 and Rust1.99; existing Python/Docker fixture helpers; no new dependencies.
**Spec:** ../specs/2026-10-10-mysql-native-recovery.md

## Global constraints

Preserve public APIs, exact ROMd7ef529040eec60dc869034c2d33130219db85fe and all historical fixture data.
Exact retained loopback55452/55453 containers only; no automatic remote restart or schema repair.
No retry of prepared writes, ownership transitions or release. No full Storage or power-loss claim.

## Review focus

- No-op lifecycle signals must fail native restart evidence.
- Guard values must originate before process death, not be reconstructed after restart.
- Uncommitted writes must be independently observed before kill and absent afterward.
- Failure paths must leave the retained fixture available and child/socket groups joined.
- Source/archive execution must use the same cases with bounded private output.

### Task1: Required actual native recovery cases

Files: tests/mysql-owner-consumer/src/{main,restart}.rs, scripts/check-mysql-owner-consumer.py, scripts/prepare-mysql-owner-package.mjs.

- [x] Observe missing recovery module RED; implement exact retained guard scenario.
- [x] Extend private runner with exact identity validation, SIGKILL and bounded startup/readiness lifecycle.
- [x] Execute no-op negative, both actual native profiles and normalized archive equivalents.

### Task2: Acceptance evidence and integration

Files: docs/mysql-ownership.md, docs/support.md, docs/research/mysql-native-recovery-2026-10-10.md, docs/verification/mysql-native-recovery-2026-10-10.json.

- [x] Record primary research, actual dependency audit and final review; resolve substantive findings.
- [x] Freeze source, run affected checks and the full local verifier, preserving failed attempts.
- [x] Publish actual evidence with remaining full Storage/TLS/deadlock/uncertainty/power-loss gaps.
