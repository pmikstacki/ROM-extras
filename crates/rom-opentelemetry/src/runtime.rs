use crate::{Operation, Outcome, TelemetryError};
use opentelemetry::{
    KeyValue,
    metrics::{Counter, Gauge, Histogram, Meter},
};
use rom::{IntakeState, RuntimeStatus};
use std::{fmt, time::Duration};

/// Fixed instruments using an explicit host-owned Meter.
/// No globals, exporter, Runtime retention, background sampler or arbitrary labels.
/// Measurements describe host observations, not a diagnostic or durable commit ledger.
pub struct RuntimeMetrics {
    available: Gauge<u64>,
    ready: Gauge<u64>,
    failed: Gauge<u64>,
    intake: Gauge<u64>,
    owned: Gauge<u64>,
    permits: Gauge<u64>,
    registered: Gauge<u64>,
    status_failures: Counter<u64>,
    operations: Counter<u64>,
    duration: Histogram<f64>,
}
impl fmt::Debug for RuntimeMetrics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RuntimeMetrics").finish_non_exhaustive()
    }
}
impl RuntimeMetrics {
    /// Create and reuse fixed instruments. The host owns scope, providers and export policy.
    pub fn new(meter: Meter) -> Self {
        Self {
            available: meter
                .u64_gauge("rom.runtime.status_available")
                .with_unit("1")
                .build(),
            ready: meter.u64_gauge("rom.runtime.ready").with_unit("1").build(),
            failed: meter.u64_gauge("rom.runtime.failed").with_unit("1").build(),
            intake: meter.u64_gauge("rom.runtime.intake").with_unit("1").build(),
            owned: meter
                .u64_gauge("rom.runtime.owned_work")
                .with_unit("{work}")
                .build(),
            permits: meter
                .u64_gauge("rom.runtime.available_permits")
                .with_unit("{permit}")
                .build(),
            registered: meter
                .u64_gauge("rom.runtime.registered")
                .with_unit("{definition}")
                .build(),
            status_failures: meter
                .u64_counter("rom.runtime.status_failures")
                .with_unit("{failure}")
                .build(),
            operations: meter
                .u64_counter("rom.host.operations")
                .with_unit("{operation}")
                .build(),
            duration: meter
                .f64_histogram("rom.host.operation.duration")
                .with_unit("s")
                .build(),
        }
    }
    /// Record one public host sample without retaining the result or its errors.
    /// Failed samples mark unavailable and count failures; other gauges remain last-known.
    /// Counts convert before recording. SDK collection can race these individual calls;
    /// this does not promise an atomic multi-instrument telemetry snapshot.
    pub fn record_status(&self, sample: &rom::Result<RuntimeStatus>) -> Result<(), TelemetryError> {
        let Ok(status) = sample else {
            self.status_failure();
            return Ok(());
        };
        let counts = [
            status.owned_work,
            status.available_action_permits,
            status.available_io_permits,
            status.available_subscription_permits,
            status.registered_resources,
            status.registered_reactions,
            status.registered_channels,
        ]
        .map(u64::try_from);
        let [
            Ok(owned),
            Ok(action),
            Ok(io),
            Ok(subscription),
            Ok(resource),
            Ok(reaction),
            Ok(channel),
        ] = counts
        else {
            self.status_failure();
            return Err(TelemetryError::CountOverflow);
        };
        self.ready.record(u64::from(status.is_ready()), &[]);
        self.failed.record(u64::from(status.failed), &[]);
        for (state, name) in [
            (IntakeState::Open, "open"),
            (IntakeState::Draining, "draining"),
            (IntakeState::Stopped, "stopped"),
        ] {
            self.intake.record(
                u64::from(status.intake == state),
                &[KeyValue::new("state", name)],
            );
        }
        self.owned.record(owned, &[]);
        for (name, value) in [
            ("action", action),
            ("io", io),
            ("subscription", subscription),
        ] {
            self.permits.record(value, &[KeyValue::new("pool", name)]);
        }
        for (name, value) in [
            ("resource", resource),
            ("reaction", reaction),
            ("channel", channel),
        ] {
            self.registered
                .record(value, &[KeyValue::new("definition", name)]);
        }
        self.available.record(1, &[]);
        Ok(())
    }
    fn status_failure(&self) {
        self.available.record(0, &[]);
        self.status_failures.add(1, &[]);
    }
    /// Record a host-observed operation and duration in seconds with finite attributes.
    /// Success/replay, relay acceptance and unknown effects retain distinct meanings.
    /// Typed Duration cannot carry a negative or nonfinite floating-point input.
    pub fn record_operation(&self, operation: Operation, outcome: Outcome, duration: Duration) {
        let attributes = [
            KeyValue::new("operation", operation.label()),
            KeyValue::new("outcome", outcome.label()),
        ];
        self.operations.add(1, &attributes);
        self.duration.record(duration.as_secs_f64(), &attributes);
    }
}
