# OpenTelemetry research

Reviewed 2026-10-09. Research only. No builds, installs, manifests, service changes, or source edits were performed.

## Published ROM boundary and scope

The authoritative ROM source is pin `d7ef529040eec60dc869034c2d33130219db85fe`.
Its `Runtime::status()` returns `RuntimeStatus`: intake, failed, owned work, permit availability, and registered definition counts.
`IntakeState` contains Open, Draining, and Stopped. Readiness means open intake without terminal failure; it does not imply spare capacity.
Permit counts are advisory under concurrency. Owned work includes tracked I/O batches and worker lifetimes, not detached host tasks.
[Published lifecycle](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/execution/lifecycle.rs).

`Runtime::observation_status(actor)` is an authority check. It is not a telemetry stream.
`Runtime::observe` is private. The pin has no public committed `DiagnosticReader` contract.
Do not copy private lifecycle internals or treat dirty upstream additions as published APIs.
[Authority check](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/execution/authority.rs),
[public exports](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/lib.rs).

Status gauges and typed host operation summaries are a provisional increment.
They do not complete Task 6's committed diagnostic bridge, action/commit/Work/recovery correlation, or dropped host-local sequence semantics.
Those requirements remain explicit until upstream publishes the necessary reader and event contracts at an approved pin.

## Exact Rust dependencies

Current API, SDK, and OTLP package documentation reports 0.33.0.
All three tagged manifests declare Apache-2.0 and Rust 1.75.0, edition 2021.
API introduction prose still claims Rust 1.70; use the tagged manifests for compatibility claims.
The resolved dependency graph can have a higher effective MSRV. It has not been built or audited here.
[API manifest](https://raw.githubusercontent.com/open-telemetry/opentelemetry-rust/opentelemetry-0.33.0/opentelemetry/Cargo.toml),
[SDK manifest](https://raw.githubusercontent.com/open-telemetry/opentelemetry-rust/opentelemetry-0.33.0/opentelemetry-sdk/Cargo.toml),
[OTLP manifest](https://raw.githubusercontent.com/open-telemetry/opentelemetry-rust/opentelemetry-0.33.0/opentelemetry-otlp/Cargo.toml).

Proposed instrumentation dependency: exact API 0.33.0, default features disabled, `metrics` and `trace`.
Keep SDK and OTLP configuration host-owned; use them for qualified consumer fixtures.
Proposed fixture SDK features: `metrics`, `trace`, with defaults disabled.
OTLP HTTP protobuf fixture features: `http-proto`, `reqwest-blocking-client`, `reqwest-rustls`, with defaults disabled.
This avoids implicit logs and internal logging from default feature groups.
The `tls-*` OTLP features configure tonic TLS, not HTTP reqwest TLS.
[OTLP feature definitions](https://raw.githubusercontent.com/open-telemetry/opentelemetry-rust/opentelemetry-0.33.0/opentelemetry-otlp/Cargo.toml).

## Instrument and recording semantics

A synchronous gauge records the current value; it does not add changes.
A counter adds nonnegative increments. Repeatedly adding a sampled queue size overcounts activity.
A histogram records samples, suitable for operation duration in seconds.
Observable instruments obtain measurements in collection callbacks, not immediately at registration.
Reuse instruments and finite attribute sets.
[Metrics API specification](https://opentelemetry.io/docs/specs/otel/metrics/api/),
[Rust gauge API](https://docs.rs/opentelemetry/latest/opentelemetry/metrics/struct.Gauge.html).

Proposed runtime gauges: readiness, failed, owned work, available permits by finite pool kind, registrations by finite definition kind.
Represent intake as three fixed state series, recording zero for inactive states and one for the active state.
Record every state each sample so a prior active series does not remain misleading.
Never infer operation success, committed counts, durable backlog, or unknown outcomes from permit availability.

Prefer `record_runtime_status(RuntimeStatus)` to an observable callback capturing `Runtime`.
It avoids callback-owned runtime retention and lets the host choose sampling cadence and error handling.
If status acquisition fails, record a distinct fixed collection-failure counter; do not fabricate a stopped/zero snapshot.
No hidden background sampler or network activity is needed.

Typed host operation recording can use finite enums for operation kind and outcome class plus a finite duration.
Do not accept arbitrary labels, metric names, actor names, Resource IDs, payload values, error strings, or URLs.
Reject nonfinite/negative duration. A host summary measures caller observation, not necessarily a durable commit.

The SDK specification recommends default cardinality limit 2000 and an overflow aggregation.
That recommendation is not a promise that arbitrary labels are safe.
Keep the adapter's Cartesian label space finite; host SDK views/cardinality controls remain additional protection.
[Metrics SDK cardinality](https://opentelemetry.io/docs/specs/otel/metrics/sdk/),
[sensitive data guidance](https://opentelemetry.io/docs/security/handling-sensitive-data/).

## Traces and final diagnostic bridge

Accept a host-owned tracer, with a fixed instrumentation scope and finite span names.
Use explicit host context for parent links. Avoid automatic global tracer/provider registration or baggage capture.
Opaque host correlation must not encode Resource IDs, actors, payloads, authorization tokens, or error strings.
Prefer OpenTelemetry parent/span links for per-operation correlation; do not place unbounded correlation values in metric labels.
[Rust trace API](https://docs.rs/opentelemetry/latest/opentelemetry/trace/),
[trace API specification](https://opentelemetry.io/docs/specs/otel/trace/api/).

The final bridge must consume committed public diagnostics, not reconstruct commits from a successful host return.
Keep action attempt, definite commit, unknown commit, Work attempt, recovery, and reconciliation distinct.
Unknown outcome cannot become success or failure through a duration observation.
Span status descriptions should remain empty or controlled finite text; do not export raw ROM errors.

When upstream publishes `DiagnosticReader`, verify event identity, sequence scope, drop accounting, and observation authority from its public contract.
Preserve host-local dropped-sequence semantics. Never treat missing diagnostic events as proof that an operation did not occur.
Do not advance commit counters twice during replay/recovery. Define exactly which committed event increments each counter.
The current public pin cannot qualify this final bridge. Status/host summaries must retain a provisional support claim.

## Host ownership and transport

Construct instrumentation with the host's `Meter` and tracer. Do not construct exporters, install globals, or read endpoint environment variables implicitly.
The host owns provider lifetime, resources, sampling, readers, exporters, credentials, trust roots, and shutdown.
The default meter provider has no readers, so instrumentation can remain local/no-op.
[SDK meter provider](https://docs.rs/opentelemetry_sdk/latest/opentelemetry_sdk/metrics/struct.SdkMeterProvider.html).

SDK 0.33's default `PeriodicReader` runs on its own thread.
It supports blocking HTTP export. Async reqwest/hyper clients are unsupported unless a compatible experimental async reader is selected.
The reader enforces neither collection callback nor exporter timeouts.
Observable callbacks that block can stall exports indefinitely.
Shutdown on a Tokio current-thread runtime can deadlock; run blocking shutdown outside that runtime thread.
[PeriodicReader documentation](https://docs.rs/opentelemetry_sdk/latest/opentelemetry_sdk/metrics/struct.PeriodicReader.html).

Important source limitation: `SdkMeterProvider::shutdown_with_timeout(_timeout)` ignores its timeout parameter in this tagged implementation.
Do not claim bounded cleanup based on that method name.
Use finite HTTP timeouts and retry budgets; keep callbacks bounded. Bound fixture process lifetime independently.
Check flush/shutdown errors; they do not guarantee all telemetry was flushed or resources released.
[Tagged meter provider implementation](https://raw.githubusercontent.com/open-telemetry/opentelemetry-rust/opentelemetry-0.33.0/opentelemetry-sdk/src/metrics/meter_provider.rs).

OTLP HTTP exposes explicit client, headers, retry policy, and maximum request body size controls.
The request size default is 64 MiB, enforced before and after compression.
Explicit host blocking reqwest client permits CA and authentication configuration without a ROM transport redesign.
Avoid redirects and uncontrolled proxy/environment behavior in a local fixture.
Do not reuse an asynchronous core HTTP transport with the default reader solely to reduce duplicate code.
[Tagged HTTP exporter](https://raw.githubusercontent.com/open-telemetry/opentelemetry-rust/opentelemetry-0.33.0/opentelemetry-otlp/src/exporter/http/mod.rs).

OTLP/HTTP protobuf uses POST signal endpoints such as `/v1/metrics` and `/v1/traces`.
Success can include partial rejection; inspect response semantics. Retries do not establish exactly-once backend storage.
Explicit endpoints and host authorization are required. Never enable a default external backend.
[OTLP specification](https://opentelemetry.io/docs/specs/otlp/).

## Real Collector fixture candidate

The official latest Collector release resolved to v0.162.0.
Use pinned `otel/opentelemetry-collector-contrib:0.162.0` with recorded image digest.
The contrib manifest includes OTLP receiver, Basic Auth extension, file exporter, and debug exporter.
Do not assume the core image includes authentication components.
[Release](https://github.com/open-telemetry/opentelemetry-collector-releases/releases/tag/v0.162.0),
[contrib manifest](https://raw.githubusercontent.com/open-telemetry/opentelemetry-collector-releases/v0.162.0/distributions/otelcol-contrib/manifest.yaml).

Configure only OTLP/HTTP on an explicit loopback/private endpoint.
Set TLS `cert_file` and `key_file` with a dedicated fixture CA and matching server SAN.
Require Basic Auth with a local htpasswd file and `auth.authenticator: basicauth/server`.
Enable the extension in `service.extensions`; wire metrics and traces pipelines explicitly.
Keep `include_metadata=false` so client authentication metadata does not propagate into exported attributes.
Set finite read/write timeouts and request body limits; default read timeout is unbounded.
[OTLP receiver](https://raw.githubusercontent.com/open-telemetry/opentelemetry-collector/v0.162.0/receiver/otlpreceiver/README.md),
[Basic Auth](https://raw.githubusercontent.com/open-telemetry/opentelemetry-collector-contrib/v0.162.0/extension/basicauthextension/README.md),
[HTTP controls](https://raw.githubusercontent.com/open-telemetry/opentelemetry-collector/v0.162.0/config/confighttp/README.md),
[TLS controls](https://raw.githubusercontent.com/open-telemetry/opentelemetry-collector/v0.162.0/config/configtls/README.md).

Export received metrics and traces into unique mounted per-run JSON files.
File exporter is alpha for these signals. Field names are not guaranteed stable.
Its default `append=false` truncates existing files; never reuse historical evidence paths.
Use bounded file rotation or finite fixture lifetime and volume size. Avoid resource-based file path grouping.
Debug exporter can provide auxiliary logs; it is not a durable receipt ledger.
[File exporter](https://raw.githubusercontent.com/open-telemetry/opentelemetry-collector-contrib/v0.162.0/exporter/fileexporter/README.md),
[debug exporter](https://raw.githubusercontent.com/open-telemetry/opentelemetry-collector/v0.162.0/exporter/debugexporter/README.md).

## Qualification and unresolved work

Use an independent packaged consumer with published ROM pin and extras public API.
Record gauges, a typed operation counter/histogram, and finite-name spans through host-owned providers.
Force flush, stop producers, shut down providers, then stop Collector with a bounded deadline.
Inspect Collector-exported JSON for exact types, units, values, scope, and trace relationships.
Search output for canary Resource IDs, actor identifiers, payloads, error strings, and credentials; all must be absent.

Test trusted CA success, untrusted CA failure, wrong hostname, missing/wrong authentication, body limits, and refused endpoint.
Do not accept an HTTP response alone as evidence that expected telemetry reached the file exporter.
Record release/image digest, configuration, consumer lockfile, export files, and finite shutdown evidence.
Local fake exporters prove API behavior only. A real Collector fixture proves the tested receive/export path, not production backend durability.

Exact dependency compilation, SDK queue/cardinality configuration APIs, Collector image availability, effective retries, and shutdown bounds remain unexecuted.
Final committed DiagnosticReader integration is blocked by missing published exports, not solved by provisional gauges.
No Collector was provisioned or contacted in this research task.

Root independently verified tagged API/SDK sources and current public ROM exports.
Public main still resolves to d7ef529 on 2026-10-09. Cross-chat reads did not return; no chat contents establish this boundary.

## Host export follow-up

### Registry availability

A read-only `docker manifest inspect otel/opentelemetry-collector-contrib:0.162.0` succeeded on 2026-10-09.
Docker Hub registry namespace is `docker.io/otel/opentelemetry-collector-contrib`; manifest service is `registry-1.docker.io`.
The returned Linux amd64 image manifest digest was:

`sha256:340885ab6f46822374f00c58c067396b53b8d612559842e8821585567579d9e3`

Linux arm64 digest was `sha256:254638b0878b0ef56088f3d580bac163aa7131213c5f7eb6e0cc9d169805503f`.
These are platform manifests, not the multi-platform index digest. No image was pulled or started.
[Official image repository](https://hub.docker.com/r/otel/opentelemetry-collector-contrib),
[release source](https://github.com/open-telemetry/opentelemetry-collector-releases/releases/tag/v0.162.0).

### HTTP export construction

`opentelemetry_otlp::RetryPolicy::disabled()` performs one attempt without retries.
Default `recommended()` permits three retries. Server throttling delays can exceed the configured maximum delay.
Disable retries for deterministic auth/TLS negative tests and bounded initial qualification.
[RetryPolicy 0.33.0](https://docs.rs/opentelemetry-otlp/0.33.0/opentelemetry_otlp/struct.RetryPolicy.html).

`WithExportConfig::with_timeout(Duration)` sets exporter configuration.
`WithHttpConfig::with_http_client(client)`, `with_retry_policy(policy)`, and `with_max_request_body_size(usize)` configure HTTP export.
When supplying a custom reqwest client, set its own finite timeout; the HTTP integration uses the supplied client's timeout.
Set trust roots, no redirects, and no automatic proxy on that host client.
[Export configuration](https://docs.rs/opentelemetry-otlp/0.33.0/opentelemetry_otlp/trait.WithExportConfig.html),
[HTTP integration](https://raw.githubusercontent.com/open-telemetry/opentelemetry-rust/opentelemetry-0.33.0/opentelemetry-http/src/lib.rs).

The OTel 0.33 workspace uses reqwest `0.13.1`, with default features disabled.
This shares the root's reqwest 0.13 semver family. Lockfile unification remains a build-time check.
Use `reqwest::blocking::Client` for the default SDK reader/processor; TLS support needs the selected reqwest feature.
[Workspace dependency versions](https://raw.githubusercontent.com/open-telemetry/opentelemetry-rust/opentelemetry-0.33.0/Cargo.toml).

### Exporter and tracer types

`opentelemetry_sdk::metrics::exporter::PushMetricExporter` requires Send, Sync, and static lifetime.
Its `export(&self, &ResourceMetrics)` returns a Send future yielding `OTelSdkResult`.
It also requires `force_flush`, `shutdown_with_timeout(Duration)`, and `temporality`.
The SDK does not retry failed metric exports; retries belong to the exporter.
[PushMetricExporter](https://raw.githubusercontent.com/open-telemetry/opentelemetry-rust/opentelemetry-0.33.0/opentelemetry-sdk/src/metrics/exporter.rs).

`BoxedTracer::new(Box::new(host_provider.tracer("finite-scope")))` can wrap a host SDK tracer without global registration.
The blanket `ObjectSafeTracer` implementation requires a Send/Sync/static span type.
The boxed tracer constructor requires an object-safe Send/Sync tracer.
[Boxed tracer implementation](https://raw.githubusercontent.com/open-telemetry/opentelemetry-rust/opentelemetry-0.33.0/opentelemetry/src/global/trace.rs).

`InMemorySpanExporter` and its builder require SDK `testing` feature outside the SDK's own tests.
Use that feature only for test dependencies. `get_finished_spans()` provides local assertion evidence.
[Trace testing exports](https://raw.githubusercontent.com/open-telemetry/opentelemetry-rust/opentelemetry-0.33.0/opentelemetry-sdk/src/trace/mod.rs).

### Trace queue and deadline limits

`BatchConfigBuilder` supports `with_max_queue_size`, `with_max_export_batch_size`, and `with_scheduled_delay`.
Pass the result through `BatchSpanProcessor::builder(exporter).with_batch_config(config).build()`.
Queue overflow drops spans. Explicitly bound queue/batch sizes and span attribute/event/link counts.
The dedicated-thread processor ignores export timeout; `with_max_export_timeout` is gated for the experimental async processor.
Its shutdown waits for acknowledgment with `recv_timeout(timeout)` and joins after acknowledgment.
Timeout can return while a blocked export worker remains active; it does not cancel arbitrary exporter code.
[Tagged span processor](https://raw.githubusercontent.com/open-telemetry/opentelemetry-rust/opentelemetry-0.33.0/opentelemetry-sdk/src/trace/span_processor.rs).

Tracer provider propagates `shutdown_with_timeout` to processors; default shutdown uses 5 seconds.
This differs from the ignored metric-provider timeout described above.
Keep exporter HTTP timeout finite and retry disabled. A process-level fixture deadline remains necessary.
Configure `with_max_attributes_per_span`, `with_max_events_per_span`, and `with_max_links_per_span` explicitly.
[Tagged tracer provider](https://raw.githubusercontent.com/open-telemetry/opentelemetry-rust/opentelemetry-0.33.0/opentelemetry-sdk/src/trace/provider.rs).

### Scope reminder

The public delivery outcome includes `TimedOut` and `Panicked`, alongside Accepted, Retryable, Permanent, and Unknown.
Preserve these classes in finite host telemetry outcomes; do not merge them into ordinary failure or success.
Task 3 diagnostic-reader integration remains pending. Provisional runtime/host telemetry cannot satisfy committed diagnostic correlation.
[Published delivery model](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/channels/model.rs).

## Implemented metrics increment

The initial production package selects only API `metrics`; trace is reserved for the next increment.
The standalone public consumer selects only SDK `metrics` and implements bounded host capture through `PushMetricExporter`.
Workspace SDK tests select `testing`, which also enables SDK trace/log support; this feature unification is not the production dependency contract.
Metadata records confirm the standalone API and SDK each resolve only `metrics`.
All newly introduced registry packages have an MIT or Apache license alternative; the selected policy uses those alternatives.
Tagged direct API/SDK MSRV is 1.75; the workspace still requires Rust 1.99.
Native Collector operation has not been tested by this local metrics increment.
