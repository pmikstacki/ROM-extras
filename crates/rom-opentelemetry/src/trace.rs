use crate::{Operation, Outcome, RuntimeMetrics};
use opentelemetry::{
    Context, KeyValue,
    global::{BoxedSpan, BoxedTracer},
    trace::{Span, SpanBuilder, Tracer},
};
use std::{fmt, time::Instant};

/// Explicit host-owned tracer and fixed metrics for host observations.
/// No provider, global context, exporter or background task is installed.
/// These observations do not establish physical commits or diagnostic coverage.
pub struct HostTelemetry {
    metrics: RuntimeMetrics,
    tracer: BoxedTracer,
}
impl fmt::Debug for HostTelemetry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HostTelemetry").finish_non_exhaustive()
    }
}
impl HostTelemetry {
    /// Combine existing fixed instruments with an explicit host tracer.
    /// The host owns provider configuration, sampling, export and lifetime.
    pub fn new(metrics: RuntimeMetrics, tracer: BoxedTracer) -> Self {
        Self { metrics, tracer }
    }
    /// Borrow fixed instruments for explicit status sampling or untraced operations.
    /// Do not also record an operation which an observation scope records.
    pub fn metrics(&self) -> &RuntimeMetrics {
        &self.metrics
    }
    /// Begin one finite-name observation with an explicitly approved parent context.
    /// The host must approve parent IDs and trace state; baggage is not copied to attributes.
    /// The current thread context is not implicitly attached or selected.
    /// Finishing records the chosen outcome; dropping records Abandoned, not cancellation.
    pub fn observe(&self, operation: Operation, parent: &Context) -> HostObservation<'_> {
        let name = match operation {
            Operation::Action => "rom.host.action",
            Operation::Query => "rom.host.query",
            Operation::Work => "rom.host.work",
            Operation::Recovery => "rom.host.recovery",
        };
        let builder = SpanBuilder::from_name(name)
            .with_attributes([KeyValue::new("operation", operation.label())]);
        HostObservation {
            metrics: &self.metrics,
            operation,
            start: Instant::now(),
            span: Some(self.tracer.build_with_context(builder, parent)),
        }
    }
}

/// One host observation. Completion consumes it, preventing a second result.
/// Dropping an unfinished scope records an abandoned observation only.
/// The scope neither cancels the underlying operation nor asserts rollback.
#[must_use = "finish the observation with a typed outcome, or drop it as abandoned"]
pub struct HostObservation<'a> {
    metrics: &'a RuntimeMetrics,
    operation: Operation,
    start: Instant,
    span: Option<BoxedSpan>,
}
impl fmt::Debug for HostObservation<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HostObservation").finish_non_exhaustive()
    }
}
impl HostObservation<'_> {
    /// Complete exactly one host metric and span observation with a finite outcome.
    /// Span status stays Unset: typed outcome carries caller observation, not commit certainty.
    pub fn finish(mut self, outcome: Outcome) {
        self.complete(outcome);
    }
    fn complete(&mut self, outcome: Outcome) {
        if let Some(mut span) = self.span.take() {
            self.metrics
                .record_operation(self.operation, outcome, self.start.elapsed());
            span.set_attribute(KeyValue::new("outcome", outcome.label()));
            span.end();
        }
    }
}
impl Drop for HostObservation<'_> {
    fn drop(&mut self) {
        self.complete(Outcome::Abandoned);
    }
}
