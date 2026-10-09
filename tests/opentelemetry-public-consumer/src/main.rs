//! Independent public API consumer. SDK metrics feature only; no transport.
mod capture;
mod host;
use opentelemetry::metrics::MeterProvider;
use opentelemetry_sdk::{
    Resource,
    metrics::{PeriodicReader, SdkMeterProvider},
};
use rom_opentelemetry::RuntimeMetrics;
use std::time::Duration;

fn main() {
    let root = std::path::PathBuf::from(std::env::var("ROM_EXTRAS_OTEL_METRICS_PATH").unwrap());
    std::fs::create_dir_all(&root).unwrap();
    for redb in [false, true] {
        let exporter = capture::Capture::default();
        let provider = SdkMeterProvider::builder()
            .with_resource(Resource::builder_empty().build())
            .with_reader(
                PeriodicReader::builder(exporter.clone())
                    .with_interval(Duration::from_secs(3600))
                    .build(),
            )
            .build();
        let metrics = RuntimeMetrics::new(provider.meter("rom-extras-public-consumer"));
        let backend = if redb { "redb" } else { "sqlite" };
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        runtime.block_on(host::qualify(&root.join(backend), redb, &metrics));
        drop(runtime);
        provider.force_flush().unwrap();
        assert_eq!(exporter.collections(), 1);
        provider.shutdown().unwrap();
        println!(
            "{backend}: actual create/action/rejection/denial/replay/shutdown; finite payload-free SDK metrics passed"
        );
    }
}
