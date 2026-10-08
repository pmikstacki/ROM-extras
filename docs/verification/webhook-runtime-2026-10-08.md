# Durable webhook Runtime verification

Date: 2026-10-08. Base commit: `f8deb222bd09b91f57c2bc9c510b31f14c2f4740`.
This increment adds public-ROM integration tests and a durable independent receiver fixture.
Product transport behavior is unchanged. Shared receiver setup was extracted without changing public API paths.

Command: `CARGO_BUILD_JOBS=2 ./scripts/check-all`. Final terminal exit: 0.
The command used host Rust 1.99, GCC 13.3, CMake 3.30.5, and OpenSSL 3.5.8.
Private fixture access scripts supplied PostgreSQL and MSSQL credentials without printing them.
The gate ran formatting, Clippy, tests, doctests, rustdoc, no-default/MSRV checks, independent consumer,
and all required actual HTTPS, PostgreSQL, and MSSQL profiles. No required backend test was silently skipped.

Three Runtime tests run on both pinned ROM SQLite and redb adapters:

- A committed intent remains Pending before dispatch, survives reopening, and records Accepted/Done durably after real HTTPS.
- Service revocation and current source-field denial prevent external requests and produce a durable Denied work record.
- A receiver commits the ID/body and attempt count before losing acknowledgement. ROM persists Unknown, then reopens and retries.

The receiver is killed and waited for, then restarted with the same SQLite database and a newly bound port.
The reopened Runtime uses a rebuilt pinned transport. The receiver preserves one effect for the same ID and exact false payload.
Both attempts are recorded. ROM records Accepted on the second attempt; no internal HTTP retry is involved.
A separate always-disconnected delivery exhausts a two-attempt budget and persists Unknown with Stopped(Attempts).
Processing after another Runtime reopen produces zero work and no additional receiver request.

The independent receiver uses Node v22.16.0, SQLite 3.49.1, WAL, and synchronous=FULL.
The test-only Node SQLite API is experimental. Its warnings are retained in raw output, not suppressed.
This demonstrates process-restart recovery on the executed filesystem; it is not a power-loss or production Node support claim.
Fixture files and private TLS material stay in unique private temporary directories and are preserved outside Git.

Initial compile experiments exposed the public idempotency method's borrowed string argument and policy fn-pointer contract.
A runnable red test then reached the real receiver but was rejected by its old manual-ID fixture restriction.
The receiver now independently checks 64-character hex Runtime IDs for Runtime routes and retains the original manual-ID profile.
Raw failed and successful output is retained. No permissive product ID normalization was added.

Independent source review found no actionable defects. Its reopen-budget coverage suggestion was implemented and verified.
The reviewer inspected source; it did not independently rerun tests.
Cargo-audit 0.22.2 found zero vulnerabilities/warnings in 211 workspace packages.
RustSec commit: `550efd3d587a29b2e2c2b21b17a440da4fede999`.
The workspace manifest license inventory has no absent declarations; release redistribution review remains open.
The independent consumer lockfile is unchanged from its previous 251-package scan.

The post-command witness records source, lockfile, compiler, command, and evidence hashes.
It is not a complete before/after source fence or release admission.
Production receiver compatibility, packaged consumers, migration/redistribution review, and broker/notification implementations remain pending.
