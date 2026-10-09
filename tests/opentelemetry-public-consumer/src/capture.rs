//! Host-owned bounded SDK capture; no exporter transport or global provider.
use opentelemetry_sdk::{
    error::OTelSdkResult,
    metrics::{
        Temporality,
        data::{AggregatedMetrics, MetricData, ResourceMetrics},
        exporter::PushMetricExporter,
    },
};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

#[derive(Clone, Default, Debug)]
pub struct Capture(Arc<Mutex<usize>>);
impl Capture {
    pub fn collections(&self) -> usize {
        *self.0.lock().unwrap()
    }
}
impl PushMetricExporter for Capture {
    async fn export(&self, data: &ResourceMetrics) -> OTelSdkResult {
        let mut count = self.0.lock().unwrap();
        assert!(*count < 8, "fixture collection bound exceeded");
        validate(data);
        *count += 1;
        Ok(())
    }
    fn force_flush(&self) -> OTelSdkResult {
        Ok(())
    }
    fn shutdown_with_timeout(&self, _: std::time::Duration) -> OTelSdkResult {
        Ok(())
    }
    fn temporality(&self) -> Temporality {
        Temporality::Cumulative
    }
}
fn validate(data: &ResourceMetrics) {
    let debug = format!("{data:?}");
    assert!(!debug.contains(crate::host::CANARY));
    let instruments: BTreeMap<_, _> = data
        .scope_metrics()
        .flat_map(|s| s.metrics())
        .map(|m| (m.name(), m))
        .collect();
    let AggregatedMetrics::U64(MetricData::Sum(sum)) = instruments["rom.host.operations"].data()
    else {
        panic!("unsigned counter required")
    };
    assert!(sum.is_monotonic());
    let counts: BTreeMap<_, _> = sum
        .data_points()
        .map(|p| {
            let outcome = p
                .attributes()
                .find(|a| a.key.as_str() == "outcome")
                .unwrap()
                .value
                .to_string();
            assert_eq!(p.attributes().count(), 2);
            (outcome, p.value())
        })
        .collect();
    assert_eq!(
        counts,
        BTreeMap::from([
            ("success".into(), 3),
            ("rejected".into(), 1),
            ("denied".into(), 1)
        ])
    );
    let AggregatedMetrics::F64(MetricData::Histogram(histogram)) =
        instruments["rom.host.operation.duration"].data()
    else {
        panic!("seconds histogram required")
    };
    assert_eq!(instruments["rom.host.operation.duration"].unit(), "s");
    assert_eq!(histogram.data_points().map(|p| p.count()).sum::<u64>(), 5);
    assert!(
        histogram
            .data_points()
            .all(|p| p.sum().is_finite() && p.sum() >= 0.0)
    );
    let AggregatedMetrics::U64(MetricData::Gauge(ready)) = instruments["rom.runtime.ready"].data()
    else {
        panic!("unsigned readiness gauge required")
    };
    assert_eq!(ready.data_points().next().unwrap().value(), 0);
    let AggregatedMetrics::U64(MetricData::Gauge(intake)) =
        instruments["rom.runtime.intake"].data()
    else {
        panic!("unsigned intake gauge required")
    };
    for p in intake.data_points() {
        let state = p.attributes().next().unwrap().value.to_string();
        assert_eq!(p.value(), u64::from(state == "stopped"));
    }
}
