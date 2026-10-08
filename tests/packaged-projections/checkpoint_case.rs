use rom::{JournalCursor, Key};
use rom_projection_core::{
    Cancellation, Checkpoint, CheckpointStore, CommitStatus, Error, OperationMetadata, PageIntent,
    ProjectionProfile, RemoteObservation, StorageWorker, TransactionId,
};
use std::os::unix::fs::DirBuilderExt;

struct Directory(std::path::PathBuf);
impl Drop for Directory {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

pub fn run() {
    let directory = Directory(std::env::temp_dir().join(format!(
        "rom-extras-consumer-checkpoint-{}",
        std::process::id()
    )));
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&directory.0)
        .unwrap();
    let path = directory.0.join("projection.redb");
    let profile =
        ProjectionProfile::new("consumer-deployment", "qdrant", "mapping-v1", None).unwrap();
    let cursor = JournalCursor {
        generation: "consumer-journal".into(),
        kind: "document".into(),
        position: 0,
    };
    let initial = Checkpoint::new("consumer-target", vec![cursor.clone()]).unwrap();
    let mut store = CheckpointStore::create(&path, &profile, &initial).unwrap();
    let operation = OperationMetadata::new(
        Key {
            kind: "document".into(),
            id: "consumer-a".into(),
        },
        3,
        7,
        false,
        [9; 32],
    )
    .unwrap();
    let page = PageIntent::new(
        &initial,
        JournalCursor {
            position: 3,
            ..cursor
        },
        vec![operation.clone()],
    )
    .unwrap();
    let prepared = store.prepare_page(&page).unwrap();
    drop(store);
    let cancel = Cancellation::new();
    cancel.cancel();
    assert!(matches!(
        CheckpointStore::open_cancellable(&path, &profile, &cancel),
        Err(Error::Cancelled)
    ));
    let mut store =
        CheckpointStore::open_cancellable(&path, &profile, &Cancellation::new()).unwrap();
    let prepared = TransactionId::decode(&prepared.encode()).unwrap();
    assert_eq!(store.reconcile(&prepared).unwrap(), CommitStatus::Applied);
    assert_eq!(
        store
            .load()
            .unwrap()
            .checkpoint()
            .cursor("document")
            .unwrap()
            .position,
        0
    );
    let observed = RemoteObservation::new(&profile, "consumer-target", operation.clone()).unwrap();
    let reconciled = page.reconcile(&profile, vec![observed]).unwrap();
    let completed = store.complete_page(&page, &reconciled).unwrap();
    drop(store);
    let store = CheckpointStore::open(&path, &profile).unwrap();
    assert_eq!(store.reconcile(&completed).unwrap(), CommitStatus::Applied);
    assert_eq!(
        store
            .load()
            .unwrap()
            .checkpoint()
            .cursor("document")
            .unwrap()
            .position,
        3
    );
    assert_eq!(
        store
            .key_state(operation.key())
            .unwrap()
            .unwrap()
            .revision(),
        7
    );
    drop(store);
    let worker = StorageWorker::open(path.clone(), profile.clone(), Cancellation::new()).unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let snapshot = runtime.block_on(worker.load().unwrap().receive()).unwrap();
    assert_eq!(
        snapshot.checkpoint().cursor("document").unwrap().position,
        3
    );
    worker.shutdown().unwrap();
    drop(CheckpointStore::open(&path, &profile).unwrap());
}
