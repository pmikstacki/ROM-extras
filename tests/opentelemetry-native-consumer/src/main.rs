//! Explicit bounded native OTLP host fixture; no global providers.
#[allow(
    dead_code,
    reason = "Shared fixture also exposes its metrics-only entry point"
)]
#[path = "../../opentelemetry-public-consumer/src/host.rs"]
mod host;
mod transport;
use opentelemetry::{
    Context,
    global::BoxedTracer,
    metrics::MeterProvider,
    trace::{
        SpanContext, SpanId, TraceContextExt, TraceFlags, TraceId, TraceState, TracerProvider,
    },
};
use opentelemetry_otlp::{RetryPolicy, WithExportConfig, WithHttpConfig};
use opentelemetry_sdk::{
    Resource,
    metrics::{PeriodicReader, SdkMeterProvider},
    trace::{BatchConfigBuilder, BatchSpanProcessor, SdkTracerProvider},
};
use rom_opentelemetry::{HostObservation, HostTelemetry, Operation, Outcome, RuntimeMetrics};
use std::time::{Duration, Instant};
struct Traced<'a>(&'a HostTelemetry, &'a Context);
impl host::Observer for Traced<'_> {
    type Scope<'a>
        = HostObservation<'a>
    where
        Self: 'a;
    fn start(&self) -> Self::Scope<'_> {
        self.0.observe(Operation::Action, self.1)
    }
    fn finish<'a>(scope: Self::Scope<'a>, outcome: Outcome)
    where
        Self: 'a,
    {
        scope.finish(outcome);
    }
}
fn value(name: &str) -> String {
    std::env::var(name).unwrap()
}
fn main() {
    assert!(
        !std::env::vars_os().any(|(name, _)| name.to_string_lossy().starts_with("OTEL_")),
        "SDK environment overrides are forbidden in this explicit host fixture"
    );
    let case = value("ROM_EXTRAS_OTEL_CASE");
    let endpoint = value("ROM_EXTRAS_OTEL_ENDPOINT");
    let client = transport::client(&case);
    let headers = transport::headers(&case);
    let limit = if case == "client_limit" { 128 } else { 65536 };
    let metric_exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_http()
        .with_endpoint(format!("{endpoint}/v1/metrics"))
        .with_timeout(Duration::from_secs(2))
        .with_http_client(client.clone())
        .with_headers(headers.clone())
        .with_retry_policy(RetryPolicy::disabled())
        .with_max_request_body_size(limit)
        .build()
        .unwrap();
    let trace_exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_http()
        .with_endpoint(format!("{endpoint}/v1/traces"))
        .with_timeout(Duration::from_secs(2))
        .with_http_client(client)
        .with_headers(headers)
        .with_retry_policy(RetryPolicy::disabled())
        .with_max_request_body_size(limit)
        .build()
        .unwrap();
    let meter = SdkMeterProvider::builder()
        .with_resource(Resource::builder_empty().build())
        .with_reader(
            PeriodicReader::builder(metric_exporter)
                .with_interval(Duration::from_secs(3600))
                .build(),
        )
        .build();
    let tracer = SdkTracerProvider::builder()
        .with_resource(Resource::builder_empty().build())
        .with_max_attributes_per_span(4)
        .with_max_events_per_span(0)
        .with_max_links_per_span(0)
        .with_span_processor(
            BatchSpanProcessor::builder(trace_exporter)
                .with_batch_config(
                    BatchConfigBuilder::default()
                        .with_max_queue_size(32)
                        .with_max_export_batch_size(32)
                        .with_scheduled_delay(Duration::from_secs(3600))
                        .build(),
                )
                .build(),
        )
        .build();
    let telemetry = HostTelemetry::new(
        RuntimeMetrics::new(meter.meter("rom-extras-native-fixture")),
        BoxedTracer::new(Box::new(tracer.tracer("rom-extras-native-fixture"))),
    );
    let parent = Context::new().with_remote_span_context(SpanContext::new(
        TraceId::from_bytes([1; 16]),
        SpanId::from_bytes([2; 8]),
        TraceFlags::SAMPLED,
        true,
        TraceState::default(),
    ));
    let root = std::path::PathBuf::from(value("ROM_EXTRAS_OTEL_RUNTIME_PATH"));
    std::fs::create_dir_all(&root).unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    for redb in [false, true] {
        runtime.block_on(host::qualify_with(
            &root.join(if redb { "redb" } else { "sqlite" }),
            redb,
            telemetry.metrics(),
            &Traced(&telemetry, &parent),
        ));
    }
    drop(runtime);
    telemetry
        .observe(Operation::Work, &parent)
        .finish(Outcome::Unknown);
    telemetry
        .observe(Operation::Work, &parent)
        .finish(Outcome::NotCommitted);
    telemetry
        .observe(Operation::Work, &parent)
        .finish(Outcome::Accepted);
    drop(telemetry.observe(Operation::Recovery, &parent));
    let deadline = Instant::now();
    let metrics_result = meter.force_flush();
    let traces_result = tracer.force_flush();
    if matches!(
        case.as_str(),
        "happy" | "partial_success" | "malformed_success"
    ) {
        assert!(metrics_result.is_ok());
        assert!(traces_result.is_ok());
    } else {
        assert!(
            metrics_result.is_err(),
            "negative metric export must reject"
        );
    }
    // Batch processor flush acknowledges draining; inspect receiver evidence, never infer export success from this alone.
    let shutdown_result = meter.shutdown();
    if matches!(
        case.as_str(),
        "happy" | "partial_success" | "malformed_success"
    ) {
        assert!(shutdown_result.is_ok());
    } else {
        assert!(shutdown_result.is_err());
    }
    assert!(tracer.shutdown_with_timeout(Duration::from_secs(3)).is_ok());
    assert!(
        deadline.elapsed() < Duration::from_secs(8),
        "explicit blocking client bound and disabled retries exceeded"
    );
    println!("{case}: actual SQLite/redb host outcomes preserved; bounded flush/shutdown finished");
}
