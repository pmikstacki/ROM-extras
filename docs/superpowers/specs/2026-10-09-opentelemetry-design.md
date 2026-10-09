# OpenTelemetry integration

Date: 2026-10-09. This family implements the approved whole-goal Task 6.
The user authorized execution and research-backed decisions in the current chat.
Root owns implementation, manifests, tests and maintained documentation. Research ownership is separate and ignored.

## Intent and public boundary

Expose payload-free runtime metrics and correlated diagnostic traces through host-owned OpenTelemetry providers.
Preserve ROM mutation, authorization, receipt, Work and unknown-outcome semantics.
No metric or span may contain ROM values, Resource keys, actors, credentials, provider URLs or raw errors by default.

ROM pin `d7ef529040eec60dc869034c2d33130219db85fe` exposes `RuntimeStatus` and typed errors/delivery outcomes.
It does not expose `DiagnosticReader`. Public main still resolves to this pin.
Do not copy private diagnostics or use an upstream dirty tree.
Metrics and host spans are working prerequisites, not completion of the committed diagnostic bridge.
Final family acceptance retains action/commit/Work/recovery correlation, dropped records and host-local sequence semantics.

## Alternatives and selected ownership

| Approach | Consequence |
| --- | --- |
| Explicit host Meter/Tracer and public diagnostics | Preserves provider ownership, finite attributes and absence of automatic network configuration; selected. |
| Global tracing subscriber and arbitrary ROM error fields | Requires global ownership and risks sensitive fields; excluded from this profile. |
| Infer commit traces from public action success | Cannot distinguish replay, unknown commits or lossy diagnostics; insufficient for final acceptance. |

`rom-opentelemetry` depends on the exact OpenTelemetry API 0.33.0 without default features.
Metrics support is unconditional; explicit host traces use optional feature `trace`.
SDK/exporter construction belongs to the host and independent fixtures.
No environment-variable configuration, default exporter, automatic sampler, global provider registration or separate diagnostic ledger is created.
The [research note](../../research/opentelemetry-2026-10-09.md) records primary sources, SDK timeout limitations and Collector selection.

## Runtime metrics

Public API: `RuntimeMetrics::new(Meter)`, `record_status(&rom::Result<RuntimeStatus>) -> Result<(), TelemetryError>`.
Sample acquisition and cadence belong to the host. The adapter does not retain Runtime or install callbacks.
Use fixed synchronous gauges, not counter increments, for snapshot values.

| Instrument | Unit | Fixed attribute space |
| --- | --- | --- |
| `rom.runtime.status_available` | `1` | None |
| `rom.runtime.ready` | `1` | None |
| `rom.runtime.failed` | `1` | None |
| `rom.runtime.intake` | `1` | `state`: open, draining, stopped |
| `rom.runtime.owned_work` | `{work}` | None |
| `rom.runtime.available_permits` | `{permit}` | `pool`: action, io, subscription |
| `rom.runtime.registered` | `{definition}` | `definition`: resource, reaction, channel |
| `rom.runtime.status_failures` | `{failure}` | None |

Record all three intake states each successful sample to reset inactive states to zero.
Readiness follows public `RuntimeStatus::is_ready`; free permits are advisory, not readiness or durable backlog.
Convert all counts to u64 before changing snapshot gauges; an unrepresentable count yields safe `TelemetryError::CountOverflow`.
On sampling error or conversion failure, record availability zero and increment failures without fabricating other snapshot values.
Previous gauges remain last-known observations and must not be treated as current when availability is zero.
A successful sample records availability one. No raw ROM error fields are recorded.

## Host operation summaries and traces

Public `Operation`: Action, Query, Work, Recovery. These describe host observation, not physical commits.
Public `Outcome`: Success, Denied, Conflict, Overloaded, NotCommitted, Unknown, Rejected, Failed, Unavailable, Accepted, Retryable, Permanent, Abandoned, TimedOut, Panicked.
`Outcome::from_result` matches typed ROM errors without retaining or formatting their fields.
`Outcome::from_delivery` preserves Accepted, Retryable, Permanent, Unknown, TimedOut and Panicked distinctly.
Failure classes describe a host return, not proof of rollback. Only NotCommitted carries its explicit ROM meaning.

`RuntimeMetrics::record_operation(Operation, Outcome, Duration)` adds one to `rom.host.operations` with unit `{operation}`.
It records seconds in `rom.host.operation.duration` with unit `s` using the same fixed operation/outcome attributes.
Typed Duration excludes negative and nonfinite inputs. No arbitrary names, labels or IDs are accepted.
There are at most 60 operation/outcome attribute combinations per instrument.
Repeated sampling cannot increment an operation counter. Host replay is an observed invocation, not a second definite commit.

The trace increment accepts an explicit host-owned tracer and parent Context, finite span names and the same outcome classes.
No global context attachment or baggage export is automatic. A dropped unfinished observation is Abandoned, not a rollback or definite cancellation.
Opaque trace links must come from host-approved context, never encoded ROM identities or secrets.
SDK buffering, sampling and processor lifetime remain host responsibilities.

## Final diagnostic bridge

Bind the committed public reader when it exists; qualify against its exact published event and loss contracts.
Keep action attempt, definite commit, unknown commit, receipt replay, Work attempt and recovery/reconciliation distinct.
Report diagnostic loss separately from durable history and business outcomes.
Reader sequence is host-local, potentially lossy and never a durable journal cursor.
Absence of a diagnostic event cannot establish that an operation did not occur.
Missing public exports remain a reported dependency, not an implemented stub or substituted summary API.

## Verification and completion

SDK collection tests inspect actual gauge/counter/histogram types, units, values, reset behavior, finite labels and sensitive canaries.
Test unknown versus not-committed errors, payload-bearing error variants, delivery outcomes, failed status sampling and repeated host operations.
Independent consumers use only public ROM and extras APIs; packaged consumers use normalized archives.
No in-memory exporter result qualifies native transport.

Use an owned Collector contrib 0.162.0 with authenticated TLS OTLP/HTTP, explicit endpoints, no external exporter and unique preserved JSON exports.
Inspect received types, values, units, finite attributes and trace relationships; an HTTP acknowledgment alone is insufficient.
Qualify wrong identity, CA/auth rejection, response/body bounds, refused endpoints, bounded queue behavior and shutdown under collector failure.
SDK 0.33 default periodic metrics reader needs blocking HTTP; its shutdown timeout argument does not establish a deadline.
Host client/export retry policy and fixture process-group lifetime must provide independent bounds.
Native fixture credentials and TLS keys stay outside committed source. No trace or metric should change mutation outcomes.

Run affected checks and the full local verifier before integration.
Final Task 6 completion additionally requires the real reader and successful/rejected action, unknown commit, replay and Work retry scenarios.
Provider account/backend durability and complete diagnostic traces are not implied by local Collector receive/export evidence.
