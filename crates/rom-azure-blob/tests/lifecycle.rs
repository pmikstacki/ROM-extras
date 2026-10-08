//! Public BlobService integration with real Azure protocol and reopened native ROM stores.
#[path = "../../../tests/common/blob_lifecycle/mod.rs"]
mod blob_lifecycle;
#[path = "../../../tests/common/azure_fixture.rs"]
mod fixture;

#[tokio::test]
async fn staged_content_authorization_and_detachment_survive_native_reopen() {
    let artifacts = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.superpowers");
    blob_lifecycle::staged_content_authorization_and_detachment("azure", &artifacts, || {
        fixture::emulator(1024)
    })
    .await;
}

#[tokio::test]
async fn deleted_reservation_returns_real_unattached_object_without_implicit_cleanup() {
    let artifacts = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.superpowers");
    blob_lifecycle::deleted_reservation_returns_unattached("azure", &artifacts, || {
        fixture::emulator(1024)
    })
    .await;
}
