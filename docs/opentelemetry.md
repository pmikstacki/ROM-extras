# OpenTelemetry host metrics

`rom-opentelemetry` records fixed instruments through an explicit host-owned OpenTelemetry `Meter`.
It has no SDK, exporter, global registration, environment configuration or background sampler in its production dependencies.
No recording method accepts Resource IDs, values, actor names, credentials or raw error strings.

## Current scope

The implemented increment records public `RuntimeStatus` samples and finite host operation summaries.
It does not implement traces or export OTLP.
Native Collector qualification remains pending.
The committed diagnostic bridge remains required for full family acceptance.
Published ROM pin `d7ef529` does not expose that reader.

A successful host invocation can return a stored idempotency receipt.
The operation counter counts that observation; it does not establish an additional physical commit.
`Unknown`, `NotCommitted`, provider acceptance, retry, permanent rejection, timeout and panic remain separate classes.
Timeout and panic do not establish rollback or absence of an external effect.

## Host configuration

Choose the SDK provider, instrumentation scope, readers and export policy in the host.
Pass its `Meter` to `RuntimeMetrics::new`.
Call `record_status(&runtime.status())` at an explicit host cadence.
Measure an operation with `Instant` and pass its typed outcome and `Duration` to `record_operation`.
Authorization and Resource selection remain in the host.

```rust,ignore
use opentelemetry::metrics::MeterProvider;
use rom_opentelemetry::{Operation, Outcome, RuntimeMetrics};
use std::time::Instant;

let metrics = RuntimeMetrics::new(provider.meter("my-host"));
metrics.record_status(&runtime.status())?;
let start = Instant::now();
let result = runtime.execute(&actor, command).await;
metrics.record_operation(Operation::Action, Outcome::from_result(&result), start.elapsed());
```

Provider lifetime, force flush and shutdown remain host responsibilities.
A `Meter` without an SDK reader produces no external export.
Select telemetry resource attributes carefully; arbitrary host SDK attributes can still disclose sensitive data.

## Instruments

| Name | Type and unit | Finite attributes |
| --- | --- | --- |
| `rom.runtime.status_available` | Unsigned gauge, `1` | None |
| `rom.runtime.ready` | Unsigned gauge, `1` | None |
| `rom.runtime.failed` | Unsigned gauge, `1` | None |
| `rom.runtime.intake` | Unsigned gauge, `1` | `state`: open, draining, stopped |
| `rom.runtime.owned_work` | Unsigned gauge, `{work}` | None |
| `rom.runtime.available_permits` | Unsigned gauge, `{permit}` | `pool`: action, io, subscription |
| `rom.runtime.registered` | Unsigned gauge, `{definition}` | `definition`: resource, reaction, channel |
| `rom.runtime.status_failures` | Monotonic unsigned counter, `{failure}` | None |
| `rom.host.operations` | Monotonic unsigned counter, `{operation}` | `operation`, `outcome` |
| `rom.host.operation.duration` | Floating-point histogram, `s` | `operation`, `outcome` |

Operation has four values: action, query, Work and recovery.
Outcome has 15 fixed values, so the operation/outcome attribute space has at most 60 combinations.
The host can apply additional SDK views and cardinality limits.

Each successful status sample replaces gauge values and resets all three intake series.
Readiness does not imply available capacity; permit counts are advisory.
Failed sampling records availability zero and increments the failure counter.
Other gauges retain last-known values; do not display them as current when availability is zero.
Count conversion occurs before measurement, but collection can race individual gauge updates.
The contract does not promise an atomic multi-instrument snapshot.

## Executable examples and evidence

Run `./scripts/check-opentelemetry-metrics` with the repository qualification toolchain.
The gate runs SDK collection tests and an independent public consumer on SQLite and redb.
It also consumes the normalized Cargo archive outside the workspace.
Both consumers select only API/SDK metrics features and use a bounded host capture exporter.
They exercise actual create, action, rejected mutation, denied authority, receipt replay and shutdown.
Canaries in Resource keys, values, actor text and errors must be absent from collected metric data.
Test databases and logs remain preserved under `.superpowers`.

These are local SDK and durable Runtime checks, not native OTLP protocol qualification.
Actual unknown commits, Work retry diagnostics and dropped-reader sequence accounting remain pending.
See [the design](superpowers/specs/2026-10-09-opentelemetry-design.md) and [primary research](research/opentelemetry-2026-10-09.md).

The [2026-10-09 verification record](verification/opentelemetry-metrics-2026-10-09.json) records the successful full local verifier and frozen source hashes.
