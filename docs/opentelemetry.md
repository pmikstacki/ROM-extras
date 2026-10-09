# OpenTelemetry host metrics

`rom-opentelemetry` records fixed instruments through an explicit host-owned OpenTelemetry `Meter`.
It has no SDK, exporter, global registration, environment configuration or background sampler in its production dependencies.
No recording method accepts Resource IDs, values, actor names, credentials or raw error strings.

## Current scope

The implemented increment records public `RuntimeStatus` samples and finite host operation summaries.
The optional `trace` feature adds explicit host observation scopes.
Independent and packaged hosts export metrics and traces through a real local Collector.
The instrumentation package itself has no transport dependency.
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

## Explicit host traces

Enable `rom-opentelemetry` feature `trace`.
Construct `HostTelemetry` with the fixed metrics and the host's `BoxedTracer`.
`BoxedTracer` is an API wrapper; its `global` module path does not register a global provider.
Pass an explicitly approved `Context` to each observation.
The adapter does not select or attach the current context.
The host must approve parent identifiers and trace state; baggage is not copied into span attributes.

```rust,ignore
use opentelemetry::{Context, global::BoxedTracer, trace::TracerProvider};
use rom_opentelemetry::{HostTelemetry, Operation, Outcome};

let telemetry = HostTelemetry::new(metrics, BoxedTracer::new(Box::new(tracer_provider.tracer("my-host"))));
let scope = telemetry.observe(Operation::Action, &Context::new());
let result = runtime.execute(&actor, command).await;
scope.finish(Outcome::from_result(&result));
```

Span names are `rom.host.action`, `rom.host.query`, `rom.host.work`, and `rom.host.recovery`.
Each span has only finite `operation` and `outcome` attributes from this adapter.
Span status stays Unset; the typed outcome describes the observation without adding commit certainty.
Finishing consumes the scope and records one metric/histogram sample and one ended span.
Dropping an unfinished scope records `abandoned`, including unwind.
It does not cancel an operation or establish rollback.
Do not also call `record_operation` for the same scope.
Use `telemetry.metrics()` for explicit status sampling or operations without traces.

Host SDK configuration can add resources, attributes or behavior outside this adapter's finite contract.
The host owns sampling, cardinality views, buffers, processors, export policy and lifetime.
Telemetry can be lost when an SDK queue is full.
The controlled queue test observes 65 metric operations and at most three exported spans with a two-entry queue.
That test qualifies SDK loss isolation, not native Collector durability.

## Native OTLP host example

The executable consumer is `tests/opentelemetry-native-consumer`.
The native gate consumes its public API directly and through Cargo's normalized archive.
Both select API/SDK metrics and trace features without default logs or internal logging.
Their host uses explicit blocking reqwest for SDK threads; it disables proxies, redirects and retries.
Client request timeout is two seconds; connect timeout is one second.
Each outgoing body is bounded to 65536 bytes.
The selected HTTP integration bounds collected response bodies to 4 MiB; this is not a bound for every header or allocation.

The fixture host rejects all `OTEL_*` overrides before constructing SDK exporters.
The runner removes inherited overrides and separately tests the rejection guard.
This is fixture policy, not automatic environment configuration in the instrumentation package.
Production hosts must review SDK environment precedence, including headers.

Select an owned Collector explicitly:

```bash
export ROM_EXTRAS_OTEL_NATIVE_FIXTURE=/absolute/path/to/owned/collector-fixture
export ROM_EXTRAS_OTEL_NATIVE_CONTAINER=rom-extras-collector-native-YYYYMMDD
./scripts/check-opentelemetry-native
```

The fixture directory contains `fixture.json`, private `credentials.json`, `tls`, and persistent `exports` directories.
`fixture.json` records container name/ID, image ID/digest, endpoint and native version.
The runner verifies the exact container and image before stopping and restarting this owned fixture.
Credentials contain `username` and `password`; the host supplies the corresponding Basic Auth header privately.
TLS files are `ca.pem`, `server.pem` and `server.key`.
The tested profile uses `collector.fixture.test` and private IP `10.246.72.2` in the certificate SAN.
Wrong-identity testing resolves its deliberate mismatch to this literal owned IP.

Use Collector contrib 0.162.0 with Linux amd64 digest `sha256:340885ab6f46822374f00c58c067396b53b8d612559842e8821585567579d9e3`.
The tested container uses a private internal network, user 1000, read-only root, dropped capabilities and no external exporter.
It has one CPU, 256 MiB memory and 64 process limits.
Its only writable telemetry volume is a dedicated fixture export directory.
Require Basic Auth through an explicit `basicauth/server` extension and OTLP receiver authenticator.
Set `include_metadata: false`, body limit 65536, and read/header/write timeouts of two seconds.
Configure only OTLP/HTTP TLS on port 4318 and metrics/traces file exporters.

The file exporters use `append: true` and 100 ms flush intervals.
Historical files are never truncated or rotated away.
The runner refuses files over 16 MiB and preserves per-run slices, private process logs and Runtime databases.
Use a fresh isolated fixture if that ceiling is reached.
File exporter is alpha; its exact JSON field names can change between versions.

The host limits its trace queue and batch to 32 entries and disables timed export during the bounded scenario.
Producers stop before explicit flush and shutdown, outside Tokio's current-thread runtime.
The metric SDK shutdown timeout argument does not enforce a deadline in this version.
The executed example instead uses bounded blocking HTTP, disabled retries and a 45-second owned process-group ceiling per case.
Every exit closes the newly owned group, including descendants, with two-second TERM grace and one-second reap bounds.
INT/TERM unwind fixture restoration; bounded cleanup defers handlers even when a worker thread receives the signal.
Lifecycle stop/start commands have eight-second ceilings and restore the owned Collector through `finally`.
Tests reproduce interruption while the actual Collector is stopped and verify its restart.
The flush/shutdown phase must finish in eight seconds.
A failing metric export can make shutdown return an error; this does not rewrite a durable Runtime result.
Shutdown performs another metric export, so there are three explicit HTTP attempts across both signals, not two.

## Native scope and remaining work

The successful native case receives exact metrics and 14 parent-linked spans.
SQLite and redb each execute actual create, action, rejected mutation, denied authority, replay and shutdown.
Additional Work Unknown/NotCommitted/Accepted scopes and a dropped recovery scope are typed host observations.
Those additional scopes are not actual diagnostic Work attempts or commit receipts.

Native negative cases cover missing/wrong authentication, untrusted CA, wrong TLS identity, unavailable endpoint and an actual Collector stop/restart.
Authenticated valid protobuf of 65536 bytes returns 200; 65537 bytes returns 400.
The unknown-field padding is valid, so malformed protobuf cannot explain the oversized rejection.
Unauthorized requests return 401.
Controlled TLS HTTP fixtures separately cover outgoing limits, redirects, 429, 503, held responses and oversized response bodies.
The request counter confirms one attempt per explicit export, with retries disabled.
Controlled partial rejection and malformed success-body cases can still return SDK success.
The gate records that limitation and checks actual Collector file contents before claiming native acceptance.

Full diagnostic correlation, unknown commits, Work retry and dropped-reader sequence semantics still require the committed public reader.
Local Collector receive/export does not establish a production telemetry backend's durability or exactly-once delivery.

The [native 2026-10-09 record](verification/opentelemetry-native-2026-10-09.json) separates SDK, real Collector and controlled HTTP evidence.
