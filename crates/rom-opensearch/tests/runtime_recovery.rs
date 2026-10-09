//! Actual public SQLite/redb journal, native OpenSearch acknowledgement loss and checkpoint reopen.
#![cfg(feature = "service-fixture")]
#[path = "support/native_fixture.rs"]
mod native_fixture;
use rom::{Actor, Command, Resource, Runtime, Storage};
use rom_projection_core::{
    Cancellation, Checkpoint, ProjectionHistory, ProjectionTarget, RuntimeHistory,
    StorageLifecycle, TargetFailure, Worker, WorkerFailure,
};
use std::{os::unix::fs::DirBuilderExt, sync::Arc};
#[derive(Clone, Resource)]
#[resource(name = "rust_search_documents")]
struct Document {
    title: String,
}
fn case(redb: bool) {
    let path = std::env::temp_dir().join(format!(
        "rom-opensearch-checkpoint-{}-{redb}",
        std::process::id()
    ));
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&path)
        .unwrap();
    let host = StorageLifecycle::new().unwrap();
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let storage: Arc<dyn Storage> = if redb {
        Arc::new(rom_redb::Redb::open(path.join("resources")).unwrap())
    } else {
        Arc::new(rom_sqlite::Sqlite::open(path.join("resources")).unwrap())
    };
    let runtime = Runtime::builder()
        .resource(
            Document::definition()
                .policy(|a, _, _| a.authority == "native-fixture")
                .field_policy(|a, _, _, _| a.authority == "native-fixture"),
        )
        .build(storage, Runtime::shared_cpu_pool(2).unwrap())
        .unwrap();
    let actor = Actor::trusted("native-fixture", "export");
    let origin = rt
        .block_on(runtime.journal_head(&actor, Document::KIND))
        .unwrap();
    rt.block_on(
        runtime.execute(
            &actor,
            Command::create(
                "a",
                Document {
                    title: "native lost response".into(),
                },
            )
            .idempotency("create-a"),
        ),
    )
    .unwrap();
    let (mut direct, mapping) = native_fixture::fixture();
    rt.block_on(native_fixture::create_generation(&mut direct));
    let profile = mapping.profile().clone();
    let physical = direct.physical_target().to_owned();
    let proxy = loss_proxy::LossProxy::start();
    let lossy = loss_proxy::lossy_target(&proxy, &mapping, &physical);
    let mut source =
        RuntimeHistory::new(runtime.clone(), actor.clone(), Document::KIND, mapping).unwrap();
    let batch = rt.block_on(source.fetch(&origin)).unwrap();
    let endpoint = batch.cursor.clone();
    let documents = batch
        .events
        .iter()
        .map(|e| source.document(e).unwrap())
        .collect();
    let local = rt
        .block_on(
            host.create(
                path.join("checkpoint"),
                profile.clone(),
                Checkpoint::new(&physical, vec![origin.clone()]).unwrap(),
                Cancellation::new(),
            )
            .unwrap()
            .receive(),
        )
        .unwrap();
    let mut worker = Worker::new(local, lossy).unwrap();
    assert_eq!(
        rt.block_on(worker.apply_page(endpoint.clone(), documents, &Cancellation::new())),
        Err(WorkerFailure::Target(TargetFailure::Unknown))
    );
    let state = rt.block_on(worker.snapshot()).unwrap();
    assert!(state.pending_page().is_some());
    assert_eq!(state.checkpoint().cursor(Document::KIND), Some(&origin));
    rt.block_on(worker.shutdown_async()).unwrap();
    drop(worker);
    drop(proxy);
    let local = rt
        .block_on(
            host.open(path.join("checkpoint"), profile, Cancellation::new())
                .unwrap()
                .receive(),
        )
        .unwrap();
    let mut worker = Worker::new(local, direct).unwrap();
    rt.block_on(worker.recover(&mut source, &Cancellation::new()))
        .unwrap()
        .unwrap();
    let state = rt.block_on(worker.snapshot()).unwrap();
    assert!(state.pending_page().is_none());
    assert_eq!(state.checkpoint().cursor(Document::KIND), Some(&endpoint));
    rt.block_on(worker.shutdown_async()).unwrap();
    drop(worker);
    drop(source);
    rt.block_on(runtime.shutdown()).unwrap();
    drop(runtime);
    host.shutdown().unwrap();
    drop(host);
    std::fs::remove_dir_all(path).unwrap();
}
#[test]
fn sqlite_native_recovery_after_actual_acknowledgement_loss() {
    case(false);
}
#[test]
fn redb_native_recovery_after_actual_acknowledgement_loss() {
    case(true);
}

#[path = "support/loss_proxy.rs"]
mod loss_proxy;
