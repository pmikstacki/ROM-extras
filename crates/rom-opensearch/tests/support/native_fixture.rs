//! Shared real-service fixture construction; no transport simulation.
#[path = "fixture_admin.rs"]
mod fixture_admin;
#[path = "generation_readiness.rs"]
pub(crate) mod generation_readiness;

use rom_opensearch::{OpenSearch, TlsConfig};
use rom_projection_core::{DocumentMapping, ProjectionProfile, ProjectionTarget};
use std::{
    fs,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
pub(crate) fn fixture() -> (OpenSearch, DocumentMapping) {
    fixture_with_fields(vec!["title".into()])
}
pub(crate) fn fixture_with_fields(fields: Vec<String>) -> (OpenSearch, DocumentMapping) {
    let mapping = DocumentMapping::new(
        ProjectionProfile::new("native-test", "opensearch", "text-v1", None).unwrap(),
        vec!["title".into(), "other".into()],
        None,
    )
    .unwrap();
    let name = format!(
        "rom_extras_rust_{}_{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    (
        target_at(
            "https://127.0.0.1:55460",
            mapping.profile().clone(),
            &name,
            fields,
        ),
        mapping,
    )
}
/// Resolve uncertain initialization by read-only inspection; never repeat creation after Unknown.
pub(crate) async fn create_generation(target: &mut OpenSearch) {
    // A preceding case can finish while other retained indices still recover after restart.
    generation_readiness::wait("*").await;
    match target.create_generation().await {
        Ok(()) => {
            generation_readiness::wait(target.physical_target()).await;
            return;
        }
        Err(rom_projection_core::TargetFailure::Unknown) => {}
        Err(error) => panic!(
            "native generation {} rejected: {error:?}",
            target.physical_target()
        ),
    }
    let inspection = async {
        loop {
            if target.verify_generation().await.is_ok() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    };
    tokio::time::timeout(Duration::from_secs(45), inspection)
        .await
        .expect("uncertain native generation did not reconcile");
    generation_readiness::wait(target.physical_target()).await;
}

/// Construct the same fixed TLS policy for direct native traffic or an owned relay.
pub(crate) fn target_at(
    endpoint: &str,
    profile: ProjectionProfile,
    physical: &str,
    fields: Vec<String>,
) -> OpenSearch {
    let root = "/root/ROM-extras/.superpowers/opensearch-fixture";
    let tls = TlsConfig::new(
        endpoint,
        fs::read(format!("{root}/tls/ca.pem")).unwrap(),
        [
            fs::read(format!("{root}/tls/projection-writer.pem")).unwrap(),
            fs::read(format!("{root}/client-private/projection-writer.key")).unwrap(),
        ]
        .concat(),
        Duration::from_secs(5),
    )
    .unwrap();
    OpenSearch::new(tls, profile, physical, fields).unwrap()
}

/// Change only this newly-created test index's schema; retain all historical data and indexes.
#[allow(dead_code)]
pub(crate) async fn change_mapping(physical: &str) {
    assert!(physical.starts_with(&format!("rom_extras_rust_{}_", std::process::id())));
    let response = fixture_admin::client()
        .put(format!("https://127.0.0.1:55460/{physical}/_mapping"))
        .header("content-type", "application/json")
        .body(r#"{"properties":{"fixture_added":{"type":"keyword","index":false}}}"#)
        .send()
        .await
        .unwrap();
    assert!(
        response.status().is_success(),
        "native schema mutation HTTP {}",
        response.status()
    );
}

/// Select only an explicitly named owned fixture; production targets do not use Docker.
#[allow(dead_code)]
pub(crate) fn container_name() -> String {
    let name = std::env::var("ROM_EXTRAS_OPENSEARCH_CONTAINER")
        .unwrap_or_else(|_| "rom-extras-opensearch-20261008".into());
    assert!(
        name.starts_with("rom-extras-opensearch-")
            && name.len() <= 128
            && name
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_'),
        "invalid owned fixture container name"
    );
    name
}
