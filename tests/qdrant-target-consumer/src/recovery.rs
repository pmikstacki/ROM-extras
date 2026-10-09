//! Actual public authorized Runtime history and process crash with durable pending work.
use crate::native::Host;
use rom::{Actor, Command, Resource, Runtime, Storage};
use rom_projection_core::{
    Cancellation, Checkpoint, HostVectors, ProjectionHistory, ProjectionTarget, RuntimeHistory,
    StorageLifecycle, TargetFailure, VectorInput, Worker, WorkerFailure,
};
use std::{path::Path, sync::Arc};
#[derive(Clone, Resource)]
#[resource(name = "native_qdrant_documents")]
struct Document {
    value: u64,
}
struct Vectors;
impl HostVectors for Vectors {
    fn model_id(&self) -> &str {
        "host-model-v1"
    }
    fn vector(&mut self, input: VectorInput<'_>) -> rom_projection_core::Result<Vec<f32>> {
        assert_eq!(input.fields().len(), 1);
        Ok(vec![3., 4., 0.])
    }
}
pub async fn run(host: Host, path: &Path, redb: bool, write: bool) {
    let lifecycle = tokio::task::spawn_blocking(StorageLifecycle::new)
        .await
        .unwrap()
        .unwrap();
    let source_path = path.join(if redb {
        "resources.redb"
    } else {
        "resources.sqlite"
    });
    let storage: Arc<dyn Storage> = tokio::task::spawn_blocking(move || {
        if redb {
            Arc::new(rom_redb::Redb::open(source_path).unwrap()) as Arc<dyn Storage>
        } else {
            Arc::new(rom_sqlite::Sqlite::open(source_path).unwrap()) as Arc<dyn Storage>
        }
    })
    .await
    .unwrap();
    let runtime = Runtime::builder()
        .resource(
            Document::definition()
                .policy(|a, _, _| a.authority == "native-fixture")
                .field_policy(|a, _, _, _| a.authority == "native-fixture"),
        )
        .build(storage, Runtime::shared_cpu_pool(2).unwrap())
        .unwrap();
    let actor = Actor::trusted("native-fixture", "export");
    let mapping = host.mapping();
    let profile = mapping.profile().clone();
    let target = host.target(0);
    let physical = target.physical_target().to_owned();
    let mut source = RuntimeHistory::with_vectors(
        runtime.clone(),
        actor.clone(),
        Document::KIND,
        mapping,
        Vectors,
    )
    .unwrap();
    if write {
        let origin = runtime.journal_head(&actor, Document::KIND).await.unwrap();
        for suffix in ["a", "b"] {
            runtime
                .execute(
                    &actor,
                    Command::create(
                        &format!("{}-{suffix}", if redb { "redb" } else { "sqlite" }),
                        Document { value: u64::MAX },
                    )
                    .idempotency(&format!("native-create-{suffix}")),
                )
                .await
                .unwrap();
        }
        let batch = source.fetch(&origin).await.unwrap();
        let documents = batch
            .events
            .iter()
            .map(|event| source.document(event).unwrap())
            .collect();
        let local = lifecycle
            .create(
                path.join("checkpoint"),
                profile,
                Checkpoint::new(&physical, vec![origin.clone()]).unwrap(),
                Cancellation::new(),
            )
            .unwrap()
            .receive()
            .await
            .unwrap();
        let mut worker = Worker::new(local, target).unwrap();
        assert_eq!(
            worker
                .apply_page(batch.cursor, documents, &Cancellation::new())
                .await,
            Err(WorkerFailure::Target(TargetFailure::Unknown))
        );
        let snapshot = worker.snapshot().await.unwrap();
        assert!(snapshot.pending_page().is_some());
        assert_eq!(snapshot.checkpoint().cursor(Document::KIND), Some(&origin));
        // Exit the entire owned process without clean shutdown after native write acknowledgement loss.
        std::process::exit(70);
    }
    let local = lifecycle
        .open(
            path.join("checkpoint"),
            profile.clone(),
            Cancellation::new(),
        )
        .unwrap()
        .receive()
        .await
        .unwrap();
    let pending = local.load().unwrap().receive().await.unwrap();
    assert!(pending.pending_page().is_some());
    let batch = source
        .fetch(pending.checkpoint().cursor(Document::KIND).unwrap())
        .await
        .unwrap();
    assert_eq!(batch.events.len(), 2);
    let documents: Vec<_> = batch
        .events
        .iter()
        .map(|event| source.document(event).unwrap())
        .collect();
    let references: Vec<_> = documents.iter().collect();
    // Inspect before replay: otherwise an upsert could conceal point loss during server restart.
    if target
        .inspect_prepared(&target.prepare(&references).unwrap())
        .await
        .is_err()
    {
        assert!(
            local
                .load()
                .unwrap()
                .receive()
                .await
                .unwrap()
                .pending_page()
                .is_some()
        );
        std::process::exit(71);
    }
    let mut worker = Worker::new(local, target).unwrap();
    assert!(worker.snapshot().await.unwrap().pending_page().is_some());
    assert!(
        worker
            .recover(&mut source, &Cancellation::new())
            .await
            .unwrap()
            .is_some()
    );
    let snapshot = worker.snapshot().await.unwrap();
    assert!(snapshot.pending_page().is_none());
    assert_eq!(
        snapshot
            .checkpoint()
            .cursor(Document::KIND)
            .unwrap()
            .position,
        2
    );
    worker.shutdown_async().await.unwrap();
    drop(worker);
    let reopened = lifecycle
        .open(path.join("checkpoint"), profile, Cancellation::new())
        .unwrap()
        .receive()
        .await
        .unwrap();
    let state = reopened.load().unwrap().receive().await.unwrap();
    assert!(state.pending_page().is_none());
    assert_eq!(
        state.checkpoint().cursor(Document::KIND).unwrap().position,
        2
    );
    for document in &documents {
        let metadata = document.metadata();
        let key = reopened
            .key_state(metadata.key().clone())
            .unwrap()
            .receive()
            .await
            .unwrap()
            .unwrap();
        assert_eq!(key.revision(), metadata.revision());
        assert_eq!(key.digest(), metadata.digest());
        assert!(!key.is_tombstone());
    }
    reopened.shutdown_async().await.unwrap();
    drop(reopened);
    drop(source);
    runtime.shutdown().await.unwrap();
    drop(runtime);
    tokio::task::spawn_blocking(move || lifecycle.shutdown())
        .await
        .unwrap()
        .unwrap();
}
