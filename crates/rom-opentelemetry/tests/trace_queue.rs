//! Controlled SDK queue evidence, separate from native HTTP export qualification.
#![cfg(feature = "trace")]
use opentelemetry::{Context, global::BoxedTracer, metrics::MeterProvider, trace::TracerProvider};
use opentelemetry_sdk::{
    Resource,
    error::OTelSdkResult,
    metrics::{
        InMemoryMetricExporter, PeriodicReader, SdkMeterProvider,
        data::{AggregatedMetrics, MetricData},
    },
    trace::{BatchConfigBuilder, BatchSpanProcessor, SdkTracerProvider, SpanData, SpanExporter},
};
use rom_opentelemetry::{HostTelemetry, Operation, Outcome, RuntimeMetrics};
use std::{
    sync::{Arc, Condvar, Mutex},
    time::{Duration, Instant},
};
#[derive(Default, Debug)]
struct State {
    started: bool,
    released: bool,
    exported: usize,
}
#[derive(Clone, Default, Debug)]
struct Held(Arc<(Mutex<State>, Condvar)>);
impl SpanExporter for Held {
    async fn export(&self, batch: Vec<SpanData>) -> OTelSdkResult {
        let (lock, signal) = &*self.0;
        let mut state = lock.lock().unwrap();
        state.started = true;
        signal.notify_all();
        let (next, deadline) = signal
            .wait_timeout_while(state, Duration::from_secs(2), |s| !s.released)
            .unwrap();
        state = next;
        assert!(
            !deadline.timed_out(),
            "bounded test exporter was not released"
        );
        state.exported += batch.len();
        Ok(())
    }
}
#[test]
fn full_trace_queue_loses_telemetry_without_blocking_or_rewriting_host_outcomes() {
    let metrics = InMemoryMetricExporter::default();
    let meter = SdkMeterProvider::builder()
        .with_resource(Resource::builder_empty().build())
        .with_reader(
            PeriodicReader::builder(metrics.clone())
                .with_interval(Duration::from_secs(3600))
                .build(),
        )
        .build();
    let exporter = Held::default();
    let tracer = SdkTracerProvider::builder()
        .with_resource(Resource::builder_empty().build())
        .with_span_processor(
            BatchSpanProcessor::builder(exporter.clone())
                .with_batch_config(
                    BatchConfigBuilder::default()
                        .with_max_queue_size(2)
                        .with_max_export_batch_size(1)
                        .with_scheduled_delay(Duration::from_secs(3600))
                        .build(),
                )
                .build(),
        )
        .build();
    let telemetry = HostTelemetry::new(
        RuntimeMetrics::new(meter.meter("queue-fixture")),
        BoxedTracer::new(Box::new(tracer.tracer("queue-fixture"))),
    );
    telemetry
        .observe(Operation::Action, &Context::new())
        .finish(Outcome::Success);
    let (lock, signal) = &*exporter.0;
    let (state, deadline) = signal
        .wait_timeout_while(lock.lock().unwrap(), Duration::from_secs(1), |s| !s.started)
        .unwrap();
    assert!(!deadline.timed_out());
    drop(state);
    let start = Instant::now();
    for _ in 0..64 {
        telemetry
            .observe(Operation::Action, &Context::new())
            .finish(Outcome::Success);
    }
    let elapsed = start.elapsed();
    {
        lock.lock().unwrap().released = true;
        signal.notify_all();
    }
    assert!(
        elapsed < Duration::from_secs(1),
        "host producer blocked behind the held export"
    );
    tracer.force_flush().unwrap();
    let exported = lock.lock().unwrap().exported;
    assert!((1..=3).contains(&exported));
    meter.force_flush().unwrap();
    let snapshots = metrics.get_finished_metrics().unwrap();
    let metric = snapshots
        .last()
        .unwrap()
        .scope_metrics()
        .flat_map(|s| s.metrics())
        .find(|m| m.name() == "rom.host.operations")
        .unwrap();
    let AggregatedMetrics::U64(MetricData::Sum(sum)) = metric.data() else {
        panic!("unsigned counter required")
    };
    assert_eq!(sum.data_points().map(|p| p.value()).sum::<u64>(), 65);
    tracer
        .shutdown_with_timeout(Duration::from_secs(2))
        .unwrap();
    meter.shutdown().unwrap();
}
