# OpenTelemetry Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans. Root owns implementation; source review and primary research have separate file ownership. Steps use checkbox syntax.

**Goal:** Implement and qualify payload-free OpenTelemetry metrics, host traces and the final committed public ROM diagnostic bridge.

**Architecture:** Host-owned Meter/Tracer/providers; finite synchronous instruments; explicit sampling and trace context. Public diagnostics preserve commit, retry and loss meaning.

**Tech Stack:** Rust 1.99, ROM d7ef529, OpenTelemetry API/SDK/OTLP 0.33.0; native Collector contrib 0.162.0.

**Spec:** ../specs/2026-10-09-opentelemetry-design.md

## Global Constraints

- Preserve existing public API paths, working changes and historical evidence.
- No implicit exporter, globals, environment configuration, sampling task or network default.
- No ROM values/keys, actor text, credentials, URLs or raw error strings in instrumentation attributes.
- Metric names/units/finite attributes and 60 operation/outcome combinations follow the spec exactly.
- Real diagnostic bridge remains required; public d7ef529 lacks its exports. Never substitute summaries for completion.
- SDK shutdown and asynchronous/blocking exporter compatibility require executed evidence, not method-name assumptions.

## Review Focus

- Repeated snapshots must reset intake states without accumulating current queue or permit values.
- Failed status sampling must mark unavailable and retain last-known values without presenting them as current.
- Unknown and NotCommitted cannot collapse into successful or definite rollback outcomes.
- Payload-bearing errors must not leak through attributes or Debug.
- Host-exporter failures and abandoned observations must not rewrite mutation/Work outcomes or imply exactly-once telemetry.

### Task 1: Public runtime metrics and finite host operation summaries

Files: crates/rom-opentelemetry/{Cargo.toml,src/lib.rs,src/runtime.rs,src/operation.rs,src/outcome.rs,src/error.rs,tests/metrics.rs}; independent metric consumer and affected gate.
Interfaces: RuntimeMetrics::new(Meter), record_status(&Result<RuntimeStatus>), record_operation(Operation,Outcome,Duration), typed Outcome conversions.

- [x] Write SDK collection tests before implementation; retain the missing-public-API failure.
- [x] Implement fixed gauge/counter/histogram instruments, upfront numeric conversion and availability/failure handling.
- [x] Inspect collected values after state transitions, failures, repeat observations and payload-bearing error canaries.
- [x] Add independent public consumer with only explicitly selected API/SDK features; verify no implicit exporter/global setup.
- [x] Check dependencies, licenses, MSRV, independent feature resolution and advisory database.
- [x] Request source review and fix confirmed findings with regressions.
- [x] Freeze sources, run affected checks/full verifier, publish only verified scope.

### Task 2: Host trace scopes and native Collector qualification

Files: src/trace.rs; native Collector runner, TLS/auth fault cases, independent/package consumers and evidence/docs.
Interfaces: explicit host tracer/parent context; finite operation scope completion/abandonment; no global attachment.

- [ ] Test fixed span names, parent links, safe outcome attributes and abandoned observations without outcome fabrication.
- [ ] Implement scopes with exactly one metrics/trace completion and no raw domain data.
- [ ] Provision a preserved, isolated real Collector with TLS/auth, finite request and export limits and unique file exports.
- [ ] Send real OTLP metrics/traces; inspect native JSON values/units/types/relationships and canary absence.
- [ ] Exercise trusted/untrusted identity, auth, request limits, unavailable Collector, bounded buffers and shutdown.
- [ ] Run independent and normalized archive consumers, affected checks and full local verifier before integration.

### Task 3: Published diagnostic reader and full family acceptance

Dependency: committed public ROM reader/event/loss contracts. No substitute private adapter is authorized.

- [ ] Revalidate public exports and select an immutable compatible ROM pin; preserve existing APIs and run compatibility checks.
- [ ] Consume bounded reader records with explicit host-local sequence and dropped-record accounting.
- [ ] Test successful action, rejected mutation, unknown commit, receipt replay and Work retry against SQLite/redb.
- [ ] Correlate action/commit/Work/recovery spans without double-counting commits or converting absent diagnostics into failure.
- [ ] Qualify native Collector export and failure isolation for these actual Runtime scenarios.
- [ ] Run final family acceptance/full verifier and update support status; retain missing reader work until genuinely implemented.

## Task 1 execution evidence

SDK API RED: `.superpowers/opentelemetry-metrics-api-red-2026-10-09.log`.
SDK GREEN: `.superpowers/opentelemetry-metrics-complete-outcomes-green-2026-10-09.log`, four collection tests.
Affected gate: `.superpowers/opentelemetry-metrics-affected-2026-10-09.log`, exit zero.
Independent and normalized archive consumers each exercise actual SQLite/redb Runtime behavior.
Dependency review: `.superpowers/opentelemetry-dependency-review-2026-10-09.json`.
Both refreshed audits report zero vulnerabilities and warnings at RustSec commit `7eebec69c352c7191b1f13eb95dd510eeca5d1de`.

Task 1 ruling: capture converts/asserts borrowed SDK data within export instead of cloning it.
`ResourceMetrics::Clone` requires the SDK testing feature; the consumer deliberately isolates metrics-only production features.
This costs a fixture-specific capture implementation but avoids claiming feature isolation through a testing-enabled SDK.
Source-only independent review found no actionable findings. Full verifier exited zero on 1157 unchanged frozen files.
Evidence: `docs/verification/opentelemetry-metrics-2026-10-09.json`.
