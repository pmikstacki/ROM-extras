//! Native store ownership, staging, and explicit drain helpers.
use super::observed::Observed;
use rom::{Actor, Runtime, Storage};
use rom_blob::{BlobService, BlobState, BlobStore, Digest, Upload};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

pub(crate) fn directory(root: &Path, provider: &str, redb: bool, scenario: &str) -> PathBuf {
    let id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = root.to_path_buf().join(format!(
        "{provider}-lifecycle-{scenario}-{redb}-{}-{id}",
        std::process::id()
    ));
    std::fs::create_dir(&path).unwrap();
    path
}
pub(crate) fn runtime(redb: bool, path: &Path) -> Runtime {
    let storage: Arc<dyn Storage> = if redb {
        Arc::new(rom_redb::Redb::open(path).unwrap())
    } else {
        Arc::new(rom_sqlite::Sqlite::open(path).unwrap())
    };
    Runtime::builder()
        .resource(rom_blob::definition())
        .build(storage, Runtime::shared_cpu_pool(2).unwrap())
        .unwrap()
}
pub(crate) fn service<B: BlobStore + 'static>(
    runtime: &Runtime,
    provider: &str,
    store: Arc<Observed<B>>,
) -> BlobService {
    BlobService::builder(runtime.clone())
        .store(provider, store)
        .build()
        .unwrap()
}
pub(crate) fn input(bytes: &[u8]) -> Upload {
    Box::pin(futures_util::stream::iter([Ok(bytes.to_vec())]))
}
pub(crate) async fn reserve(
    service: &BlobService,
    actor: &Actor,
    provider: &str,
    id: &str,
    bytes: &[u8],
) {
    let pending = service
        .reserve(
            actor,
            id,
            provider,
            Digest::of(bytes),
            bytes.len() as u64,
            &format!("reserve-{id}"),
        )
        .await
        .unwrap();
    assert_eq!(pending.revision, 1);
    assert_eq!(pending.value.unwrap().state, BlobState::Pending);
}
pub(crate) async fn close(service: BlobService, runtime: Runtime) {
    service.shutdown().await.unwrap();
    runtime.shutdown().await.unwrap();
    drop(service);
    drop(runtime);
}
