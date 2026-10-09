//! Public worker composition with a synthetic target, not backend qualification.
use rom::{JournalBatch, JournalCursor, JournalView, Key, ProjectedView};
use rom_projection_core::{
    ApprovedDocument, Cancellation, Checkpoint, DocumentMapping, OperationMetadata,
    ProjectionHistory, ProjectionProfile, ProjectionTarget, RemoteObservation, StorageWorker,
    TargetFailure, Worker, WorkerFailure,
};
use std::os::unix::fs::DirBuilderExt;
struct Target {
    profile: ProjectionProfile,
    fail_once: bool,
}
impl ProjectionTarget for Target {
    type Request = Vec<OperationMetadata>;
    fn profile(&self) -> &ProjectionProfile {
        &self.profile
    }
    fn physical_target(&self) -> &str {
        "consumer-target"
    }
    fn prepare(
        &self,
        documents: &[&ApprovedDocument],
    ) -> rom_projection_core::Result<Self::Request> {
        Ok(documents.iter().map(|d| d.metadata().clone()).collect())
    }
    async fn apply(
        &mut self,
        request: Self::Request,
    ) -> Result<Vec<RemoteObservation>, TargetFailure> {
        if std::mem::take(&mut self.fail_once) {
            return Err(TargetFailure::Unknown);
        }
        Ok(request
            .into_iter()
            .map(|op| RemoteObservation::new(&self.profile, "consumer-target", op).unwrap())
            .collect())
    }
}
struct History {
    mapping: DocumentMapping,
    event: JournalView,
}
impl ProjectionHistory for History {
    fn profile(&self) -> &ProjectionProfile {
        self.mapping.profile()
    }
    async fn fetch(&mut self, after: &JournalCursor) -> rom_projection_core::Result<JournalBatch> {
        assert_eq!(after.position, 0);
        Ok(JournalBatch {
            events: vec![self.event.clone()],
            cursor: JournalCursor {
                position: 5,
                ..after.clone()
            },
        })
    }
    fn document(&mut self, event: &JournalView) -> rom_projection_core::Result<ApprovedDocument> {
        self.mapping.document(event, None)
    }
}
struct Directory(std::path::PathBuf);
impl Drop for Directory {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
pub fn run() {
    let mapping = DocumentMapping::new(
        ProjectionProfile::new("consumer", "qdrant", "worker", None).unwrap(),
        vec!["a".into()],
        None,
    )
    .unwrap();
    let event = JournalView {
        position: 3,
        view: ProjectedView {
            key: Key {
                kind: "document".into(),
                id: "a".into(),
            },
            revision: 7,
            value: Some(serde_json::from_str(r#"{"a":42}"#).unwrap()),
        },
    };
    let document = mapping.document(&event, None).unwrap();
    let operation = document.metadata().clone();
    let directory = Directory(
        std::env::temp_dir().join(format!("rom-extras-consumer-worker-{}", std::process::id())),
    );
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&directory.0)
        .unwrap();
    let path = directory.0.join("worker.redb");
    let cursor = JournalCursor {
        generation: "consumer-journal".into(),
        kind: "document".into(),
        position: 0,
    };
    let initial = Checkpoint::new("consumer-target", vec![cursor.clone()]).unwrap();
    let storage = StorageWorker::create(path.clone(), mapping.profile().clone(), initial).unwrap();
    let mut worker = Worker::new(
        storage,
        Target {
            profile: mapping.profile().clone(),
            fail_once: true,
        },
    )
    .unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let cancel = Cancellation::new();
    let next = JournalCursor {
        position: 3,
        ..cursor
    };
    assert_eq!(
        runtime.block_on(worker.apply_page(next.clone(), vec![document], &cancel)),
        Err(WorkerFailure::Target(TargetFailure::Unknown))
    );
    let snapshot = runtime.block_on(worker.snapshot()).unwrap();
    assert_eq!(
        snapshot.checkpoint().cursor("document").unwrap().position,
        0
    );
    assert!(snapshot.pending_page().is_some());
    assert_eq!(
        runtime.block_on(worker.apply_page(next, Vec::new(), &cancel)),
        Err(WorkerFailure::RecoveryRequired)
    );
    worker.shutdown().unwrap();
    drop(worker);
    let storage =
        StorageWorker::open(path.clone(), mapping.profile().clone(), Cancellation::new()).unwrap();
    let mut worker = Worker::new(
        storage,
        Target {
            profile: mapping.profile().clone(),
            fail_once: false,
        },
    )
    .unwrap();
    let mut history = History { mapping, event };
    assert!(
        runtime
            .block_on(worker.recover(&mut history, &cancel))
            .unwrap()
            .is_some()
    );
    let snapshot = runtime.block_on(worker.snapshot()).unwrap();
    assert_eq!(
        snapshot.checkpoint().cursor("document").unwrap().position,
        3
    );
    assert!(snapshot.pending_page().is_none());
    worker.shutdown().unwrap();
    drop(worker);
    let store =
        rom_projection_core::CheckpointStore::open(&path, history.mapping.profile()).unwrap();
    assert_eq!(
        store
            .key_state(operation.key())
            .unwrap()
            .unwrap()
            .revision(),
        7
    );
    drop(store);
}
