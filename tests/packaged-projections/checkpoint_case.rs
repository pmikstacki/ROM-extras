use rom::{JournalBatch, JournalCursor, JournalView, Key, ProjectedView};
use rom_projection_core::{
    Cancellation, Checkpoint, CheckpointStore, CommitStatus, Error, OperationMetadata, PageIntent,
    PendingHistory, ProjectionProfile, RemoteObservation, StorageWorker, TransactionId,
};
use std::os::unix::fs::DirBuilderExt;

struct Directory(std::path::PathBuf);
impl Drop for Directory {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

pub fn run() {
    let mapping = rom_projection_core::DocumentMapping::new(
        ProjectionProfile::new("consumer", "qdrant", "mapping", None).unwrap(),
        vec!["a".into()],
        None,
    )
    .unwrap();
    for input in [
        "18446744073709551616",
        "18446744073709551617",
        "0.100000000000000000000000000000001",
        "1e-999",
    ] {
        let value = serde_json::from_str(input).unwrap();
        let event = JournalView {
            position: 1,
            view: ProjectedView {
                key: Key {
                    kind: "document".into(),
                    id: "numeric-case".into(),
                },
                revision: 1,
                value: Some([("a".into(), value)].into_iter().collect()),
            },
        };
        assert!(matches!(
            mapping.document(&event, None),
            Err(Error::Invalid)
        ));
    }
    let make_event = |value| JournalView {
        position: 1,
        view: ProjectedView {
            key: Key {
                kind: "document".into(),
                id: "accepted-numeric".into(),
            },
            revision: 1,
            value: Some(serde_json::from_str(value).unwrap()),
        },
    };
    let a = mapping
        .document(
            &make_event(r#"{"a":{"z":0.1,"b":18446744073709551615}}"#),
            None,
        )
        .unwrap();
    let b = mapping
        .document(
            &make_event(r#"{"a":{"b":18446744073709551615,"z":0.1}}"#),
            None,
        )
        .unwrap();
    assert_eq!(a.metadata().digest(), b.metadata().digest());
    assert_eq!(a.fields().unwrap()["a"]["b"].as_u64(), Some(u64::MAX));
    let huge = format!("{{\"a\":{}}}", "1".repeat(1000));
    assert!(matches!(
        mapping.document(&make_event(&huge), None),
        Err(Error::TooLarge)
    ));
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
    let mut history =
        PendingHistory::new(store.load().unwrap().pending_page().unwrap().clone()).unwrap();
    history
        .push(
            &JournalBatch {
                events: vec![JournalView {
                    position: 3,
                    view: ProjectedView {
                        key: operation.key().clone(),
                        revision: 7,
                        value: Some(Default::default()),
                    },
                }],
                cursor: JournalCursor {
                    position: 5,
                    ..page.next_cursor().clone()
                },
            },
            &Cancellation::new(),
            |_| Ok(operation.clone()),
        )
        .unwrap();
    assert_eq!(history.finish().unwrap(), page);
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
