//! SDK collection evidence, separate from native Collector or diagnostic-reader qualification.
use opentelemetry::{KeyValue, metrics::MeterProvider};
use opentelemetry_sdk::{
    Resource,
    metrics::{
        InMemoryMetricExporter, PeriodicReader, SdkMeterProvider,
        data::{AggregatedMetrics, Metric, MetricData, ResourceMetrics},
    },
};
use rom::{DeliveryOutcome, Error, IntakeState, RuntimeStatus};
use rom_opentelemetry::{Operation, Outcome, RuntimeMetrics};
use std::{collections::BTreeMap, time::Duration};

fn setup() -> (RuntimeMetrics, SdkMeterProvider, InMemoryMetricExporter) {
    let exporter = InMemoryMetricExporter::default();
    let reader = PeriodicReader::builder(exporter.clone())
        .with_interval(Duration::from_secs(3600))
        .build();
    let provider = SdkMeterProvider::builder()
        .with_resource(Resource::builder_empty().build())
        .with_reader(reader)
        .build();
    (
        RuntimeMetrics::new(provider.meter("rom-extras-fixture")),
        provider,
        exporter,
    )
}
fn status(intake: IntakeState, failed: bool) -> RuntimeStatus {
    RuntimeStatus {
        intake,
        failed,
        owned_work: 3,
        available_action_permits: 0,
        available_io_permits: 4,
        available_subscription_permits: 5,
        registered_resources: 6,
        registered_reactions: 7,
        registered_channels: 8,
    }
}
fn metric<'a>(data: &'a ResourceMetrics, name: &str) -> &'a Metric {
    data.scope_metrics()
        .flat_map(|s| s.metrics())
        .find(|m| m.name() == name)
        .unwrap()
}
fn attributes<'a>(values: impl Iterator<Item = &'a KeyValue>) -> String {
    let mut pairs: Vec<_> = values.map(|v| format!("{}={}", v.key, v.value)).collect();
    pairs.sort();
    pairs.join(",")
}
fn gauge(data: &ResourceMetrics, name: &str) -> BTreeMap<String, u64> {
    let AggregatedMetrics::U64(MetricData::Gauge(gauge)) = metric(data, name).data() else {
        panic!("expected unsigned gauge")
    };
    gauge
        .data_points()
        .map(|p| (attributes(p.attributes()), p.value()))
        .collect()
}
fn collect(provider: &SdkMeterProvider, exporter: &InMemoryMetricExporter) -> ResourceMetrics {
    provider.force_flush().unwrap();
    exporter.get_finished_metrics().unwrap().pop().unwrap()
}

#[test]
fn runtime_snapshots_replace_values_reset_intake_and_keep_readiness_distinct_from_capacity() {
    let (observer, provider, exporter) = setup();
    for (intake, failed, ready, state) in [
        (IntakeState::Open, false, 1, "open"),
        (IntakeState::Draining, false, 0, "draining"),
        (IntakeState::Stopped, false, 0, "stopped"),
        (IntakeState::Open, true, 0, "open"),
    ] {
        let sample = Ok(status(intake, failed));
        observer.record_status(&sample).unwrap();
        observer.record_status(&sample).unwrap();
        let data = collect(&provider, &exporter);
        assert_eq!(
            gauge(&data, "rom.runtime.ready"),
            BTreeMap::from([(String::new(), ready)])
        );
        assert_eq!(gauge(&data, "rom.runtime.status_available")[""], 1);
        assert_eq!(gauge(&data, "rom.runtime.failed")[""], u64::from(failed));
        assert_eq!(gauge(&data, "rom.runtime.owned_work")[""], 3);
        assert_eq!(
            gauge(&data, "rom.runtime.available_permits"),
            BTreeMap::from([
                ("pool=action".into(), 0),
                ("pool=io".into(), 4),
                ("pool=subscription".into(), 5)
            ])
        );
        assert_eq!(
            gauge(&data, "rom.runtime.registered"),
            BTreeMap::from([
                ("definition=resource".into(), 6),
                ("definition=reaction".into(), 7),
                ("definition=channel".into(), 8)
            ])
        );
        assert_eq!(
            gauge(&data, "rom.runtime.intake"),
            ["open", "draining", "stopped"]
                .map(|v| (format!("state={v}"), u64::from(v == state)))
                .into()
        );
        assert_eq!(
            metric(&data, "rom.runtime.available_permits").unit(),
            "{permit}"
        );
    }
    provider.shutdown().unwrap();
}

#[test]
fn failed_sampling_marks_unavailable_without_fabricating_a_stopped_snapshot_or_exporting_errors() {
    let (observer, provider, exporter) = setup();
    observer
        .record_status(&Ok(status(IntakeState::Open, false)))
        .unwrap();
    observer
        .record_status(&Err(Error::Unsupported(
            "private-resource-key-and-password".into(),
        )))
        .unwrap();
    let data = collect(&provider, &exporter);
    assert_eq!(gauge(&data, "rom.runtime.status_available")[""], 0);
    assert_eq!(gauge(&data, "rom.runtime.ready")[""], 1);
    assert_eq!(gauge(&data, "rom.runtime.intake")["state=open"], 1);
    let AggregatedMetrics::U64(MetricData::Sum(sum)) =
        metric(&data, "rom.runtime.status_failures").data()
    else {
        panic!("expected monotonic failure counter")
    };
    assert!(sum.is_monotonic());
    assert_eq!(sum.data_points().next().unwrap().value(), 1);
    assert!(!format!("{data:?}").contains("private-resource-key-and-password"));
    provider.shutdown().unwrap();
}

#[test]
fn operation_summaries_keep_unknown_not_committed_and_relay_acceptance_distinct() {
    let (observer, provider, exporter) = setup();
    let cases = [
        (
            Operation::Action,
            Outcome::from_result(&Err::<(), _>(Error::Unknown)),
            "action",
            "unknown",
        ),
        (
            Operation::Action,
            Outcome::from_result(&Err::<(), _>(Error::NotCommitted)),
            "action",
            "not_committed",
        ),
        (
            Operation::Work,
            Outcome::from_delivery(DeliveryOutcome::Accepted),
            "work",
            "accepted",
        ),
        (
            Operation::Work,
            Outcome::from_delivery(DeliveryOutcome::Retryable),
            "work",
            "retryable",
        ),
        (
            Operation::Work,
            Outcome::from_delivery(DeliveryOutcome::Permanent),
            "work",
            "permanent",
        ),
        (
            Operation::Recovery,
            Outcome::from_result(&Ok(())),
            "recovery",
            "success",
        ),
        (
            Operation::Query,
            Outcome::from_result(&Err::<(), _>(Error::Invalid {
                kind: "secret-actor".into(),
                field: "secret-field-payload".into(),
            })),
            "query",
            "rejected",
        ),
        (
            Operation::Work,
            Outcome::from_delivery(DeliveryOutcome::TimedOut),
            "work",
            "timed_out",
        ),
        (
            Operation::Work,
            Outcome::from_delivery(DeliveryOutcome::Panicked),
            "work",
            "panicked",
        ),
    ];
    for (operation, outcome, _, _) in cases {
        observer.record_operation(operation, outcome, Duration::from_millis(125));
        observer.record_operation(operation, outcome, Duration::ZERO);
    }
    let data = collect(&provider, &exporter);
    let counter = metric(&data, "rom.host.operations");
    assert_eq!(counter.unit(), "{operation}");
    let AggregatedMetrics::U64(MetricData::Sum(sum)) = counter.data() else {
        panic!("expected unsigned operation counter")
    };
    assert!(sum.is_monotonic());
    let counts: BTreeMap<_, _> = sum
        .data_points()
        .map(|p| (attributes(p.attributes()), p.value()))
        .collect();
    assert_eq!(counts.len(), cases.len());
    for (_, _, operation, outcome) in cases {
        assert_eq!(
            counts[&format!("operation={operation},outcome={outcome}")],
            2
        );
    }
    let duration = metric(&data, "rom.host.operation.duration");
    assert_eq!(duration.unit(), "s");
    let AggregatedMetrics::F64(MetricData::Histogram(histogram)) = duration.data() else {
        panic!("expected seconds histogram")
    };
    for p in histogram.data_points() {
        assert_eq!(p.count(), 2);
        assert_eq!(p.sum(), 0.125);
        assert!(p.sum().is_finite());
    }
    let debug = format!("{data:?}");
    assert!(!debug.contains("secret-actor"));
    assert!(!debug.contains("secret-field-payload"));
    provider.shutdown().unwrap();
}

#[test]
fn large_snapshot_counts_remain_unsigned_and_exact() {
    let (observer, provider, exporter) = setup();
    let mut sample = status(IntakeState::Open, false);
    sample.owned_work = usize::MAX;
    observer.record_status(&Ok(sample)).unwrap();
    let data = collect(&provider, &exporter);
    assert_eq!(
        gauge(&data, "rom.runtime.owned_work")[""],
        u64::try_from(usize::MAX).unwrap()
    );
    provider.shutdown().unwrap();
}
