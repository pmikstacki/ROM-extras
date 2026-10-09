//! Shared real-service fixture construction; no transport simulation.
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
    let root = "/root/ROM-extras/.superpowers/opensearch-fixture";
    let tls = TlsConfig::new(
        "https://127.0.0.1:55460",
        fs::read(format!("{root}/tls/ca.pem")).unwrap(),
        [
            fs::read(format!("{root}/tls/projection-writer.pem")).unwrap(),
            fs::read(format!("{root}/client-private/projection-writer.key")).unwrap(),
        ]
        .concat(),
        Duration::from_secs(5),
    )
    .unwrap();
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
        OpenSearch::new(tls, mapping.profile().clone(), &name, fields).unwrap(),
        mapping,
    )
}
/// Resolve uncertain initialization by read-only inspection; never repeat creation after Unknown.
pub(crate) async fn create_generation(target: &mut OpenSearch) {
    match target.create_generation().await {
        Ok(()) => return,
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
}
