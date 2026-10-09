//! Local SDK trace evidence; no native Collector or committed diagnostic bridge claims.
#![cfg(feature = "trace")]
use opentelemetry::{
    Context, KeyValue,
    global::BoxedTracer,
    metrics::MeterProvider,
    trace::{
        SpanContext, SpanId, TraceContextExt, TraceFlags, TraceId, TraceState, TracerProvider,
    },
};
use opentelemetry_sdk::{
    Resource,
    metrics::{
        InMemoryMetricExporter, PeriodicReader, SdkMeterProvider,
        data::{AggregatedMetrics, MetricData},
    },
    trace::{InMemorySpanExporter, SdkTracerProvider},
};
use rom_opentelemetry::{HostTelemetry, Operation, Outcome, RuntimeMetrics};
use std::time::Duration;

fn setup() -> (
    HostTelemetry,
    SdkMeterProvider,
    InMemoryMetricExporter,
    SdkTracerProvider,
    InMemorySpanExporter,
) {
    let metrics = InMemoryMetricExporter::default();
    let meter = SdkMeterProvider::builder()
        .with_resource(Resource::builder_empty().build())
        .with_reader(
            PeriodicReader::builder(metrics.clone())
                .with_interval(Duration::from_secs(3600))
                .build(),
        )
        .build();
    let spans = InMemorySpanExporter::default();
    let tracer = SdkTracerProvider::builder()
        .with_resource(Resource::builder_empty().build())
        .with_simple_exporter(spans.clone())
        .build();
    let telemetry = HostTelemetry::new(
        RuntimeMetrics::new(meter.meter("fixture")),
        BoxedTracer::new(Box::new(tracer.tracer("fixture"))),
    );
    (telemetry, meter, metrics, tracer, spans)
}
fn parent() -> Context {
    Context::new().with_remote_span_context(SpanContext::new(
        TraceId::from_bytes([1; 16]),
        SpanId::from_bytes([2; 8]),
        TraceFlags::SAMPLED,
        true,
        TraceState::default(),
    ))
}
#[test]
fn explicit_parent_fixed_spans_and_exactly_one_completed_observation() {
    let (telemetry, meter, metrics, tracer, spans) = setup();
    for (operation, outcome) in [
        (Operation::Action, Outcome::Unknown),
        (Operation::Query, Outcome::NotCommitted),
        (Operation::Work, Outcome::Accepted),
        (Operation::Recovery, Outcome::Success),
    ] {
        telemetry.observe(operation, &parent()).finish(outcome);
    }
    meter.force_flush().unwrap();
    let data = metrics.get_finished_metrics().unwrap();
    let operation_metric = data
        .last()
        .unwrap()
        .scope_metrics()
        .flat_map(|s| s.metrics())
        .find(|m| m.name() == "rom.host.operations")
        .unwrap();
    let AggregatedMetrics::U64(MetricData::Sum(sum)) = operation_metric.data() else {
        panic!("counter required")
    };
    assert_eq!(sum.data_points().map(|p| p.value()).sum::<u64>(), 4);
    let collected = spans.get_finished_spans().unwrap();
    assert_eq!(collected.len(), 4);
    for (span, name, outcome) in collected
        .iter()
        .zip([
            "rom.host.action",
            "rom.host.query",
            "rom.host.work",
            "rom.host.recovery",
        ])
        .zip(["unknown", "not_committed", "accepted", "success"])
        .map(|((s, n), o)| (s, n, o))
    {
        assert_eq!(span.name, name);
        assert_eq!(span.parent_span_id, SpanId::from_bytes([2; 8]));
        assert_eq!(span.span_context.trace_id(), TraceId::from_bytes([1; 16]));
        assert!(span.events.is_empty());
        assert!(span.links.is_empty());
        assert_eq!(span.attributes.len(), 2);
        assert!(span.attributes.contains(&KeyValue::new("outcome", outcome)));
        assert!(span.end_time >= span.start_time);
    }
    meter.shutdown().unwrap();
    tracer.shutdown().unwrap();
}
#[test]
fn dropping_or_unwinding_marks_abandoned_without_inventing_cancellation_or_rollback() {
    let (telemetry, meter, metrics, tracer, spans) = setup();
    drop(telemetry.observe(Operation::Action, &Context::new()));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _scope = telemetry.observe(Operation::Work, &Context::new());
        panic!("private-actor-resource-error");
    }));
    assert!(result.is_err());
    meter.force_flush().unwrap();
    let data = metrics.get_finished_metrics().unwrap();
    let debug = format!("{data:?}");
    assert!(!debug.contains("private-actor-resource-error"));
    let collected = spans.get_finished_spans().unwrap();
    assert_eq!(collected.len(), 2);
    for span in collected {
        assert!(
            span.attributes
                .contains(&KeyValue::new("outcome", "abandoned"))
        );
        assert!(!format!("{span:?}").contains("private-actor-resource-error"));
        assert_eq!(span.status, opentelemetry::trace::Status::Unset);
    }
    meter.shutdown().unwrap();
    tracer.shutdown().unwrap();
}
#[test]
fn current_context_and_baggage_do_not_become_implicit_parents_or_exported_payloads() {
    use opentelemetry::baggage::BaggageExt;
    let (telemetry, meter, _, tracer, spans) = setup();
    let ambient = parent().with_baggage([KeyValue::new(
        "private-context-canary",
        "private-value-canary",
    )]);
    let _attached = ambient.attach();
    telemetry
        .observe(Operation::Query, &Context::new())
        .finish(Outcome::Denied);
    let collected = spans.get_finished_spans().unwrap();
    assert_eq!(collected.len(), 1);
    assert_eq!(collected[0].parent_span_id, SpanId::INVALID);
    assert_ne!(
        collected[0].span_context.trace_id(),
        TraceId::from_bytes([1; 16])
    );
    assert!(!format!("{:?}", collected[0]).contains("canary"));
    meter.shutdown().unwrap();
    tracer.shutdown().unwrap();
}
