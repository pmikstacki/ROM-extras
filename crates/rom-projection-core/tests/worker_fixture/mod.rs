use super::support::{Directory, initial};
use rom::{JournalBatch, JournalCursor, JournalView, Key, ProjectedView, json};
use rom_projection_core::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
pub fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
}
pub fn mapping() -> DocumentMapping {
    DocumentMapping::new(super::support::profile(), vec!["a".into()], None).unwrap()
}
pub fn event(position: u64, revision: u64) -> JournalView {
    JournalView {
        position,
        view: ProjectedView {
            key: Key {
                kind: "document".into(),
                id: "a".into(),
            },
            revision,
            value: Some(json!({"a":revision}).as_object().unwrap().clone()),
        },
    }
}
pub fn document(position: u64, revision: u64) -> ApprovedDocument {
    mapping()
        .document(&event(position, revision), None)
        .unwrap()
}
#[derive(Clone, Copy)]
pub enum Mode {
    Exact,
    Unknown,
    Higher,
    TooLarge,
    Cancel,
    Wait,
}
pub struct Target {
    profile: ProjectionProfile,
    path: std::path::PathBuf,
    calls: Arc<AtomicUsize>,
    mode: Mode,
    cancel: Cancellation,
    entered: Option<tokio::sync::oneshot::Sender<()>>,
    release: Option<tokio::sync::oneshot::Receiver<()>>,
}
impl ProjectionTarget for Target {
    type Request = Vec<OperationMetadata>;
    fn profile(&self) -> &ProjectionProfile {
        &self.profile
    }
    fn physical_target(&self) -> &str {
        "physical-a"
    }
    fn prepare(&self, docs: &[&ApprovedDocument]) -> Result<Self::Request> {
        if matches!(self.mode, Mode::TooLarge) {
            return Err(Error::TooLarge);
        }
        Ok(docs.iter().map(|d| d.metadata().clone()).collect())
    }
    async fn apply(
        &mut self,
        request: Self::Request,
    ) -> std::result::Result<Vec<RemoteObservation>, TargetFailure> {
        // The owner is idle after preparation; inspect a copied actual durable file under its own reservation.
        let probe = Directory::new();
        std::fs::copy(&self.path, probe.file()).unwrap();
        let store = CheckpointStore::open(&probe.file(), &self.profile).unwrap();
        let state = store.load().unwrap();
        assert!(state.pending_page().is_some());
        assert_eq!(state.checkpoint().cursor("document").unwrap().position, 0);
        self.calls.fetch_add(1, Ordering::SeqCst);
        if matches!(self.mode, Mode::Cancel) {
            self.cancel.cancel();
        }
        if matches!(self.mode, Mode::Wait) {
            self.entered.take().unwrap().send(()).unwrap();
            self.release.take().unwrap().await.unwrap();
        }
        if matches!(self.mode, Mode::Unknown) {
            return Err(TargetFailure::Unknown);
        }
        Ok(request
            .into_iter()
            .map(|op| {
                let op = if matches!(self.mode, Mode::Higher) {
                    OperationMetadata::new(
                        op.key().clone(),
                        1,
                        op.revision() + 1,
                        op.is_tombstone(),
                        *op.digest(),
                    )
                    .unwrap()
                } else {
                    op
                };
                RemoteObservation::new(&self.profile, "physical-a", op).unwrap()
            })
            .collect())
    }
}
pub struct Fixture {
    pub worker: Worker<Target>,
    pub calls: Arc<AtomicUsize>,
    pub cancel: Cancellation,
    pub entered: Option<tokio::sync::oneshot::Receiver<()>>,
    pub release: Option<tokio::sync::oneshot::Sender<()>>,
    dir: Directory,
}
impl Fixture {
    pub fn new(mode: Mode) -> Self {
        let dir = Directory::new();
        let profile = mapping().profile().clone();
        let storage = StorageWorker::create(dir.file(), profile.clone(), initial()).unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let cancel = Cancellation::new();
        let (entered_tx, entered) = tokio::sync::oneshot::channel();
        let (release, release_rx) = tokio::sync::oneshot::channel();
        let target = Target {
            profile,
            path: dir.file(),
            calls: calls.clone(),
            mode,
            cancel: cancel.clone(),
            entered: Some(entered_tx),
            release: Some(release_rx),
        };
        Self {
            worker: Worker::new(storage, target).unwrap(),
            calls,
            cancel,
            entered: Some(entered),
            release: Some(release),
            dir,
        }
    }
    pub fn restart(&mut self, mode: Mode) {
        self.worker.shutdown().unwrap();
        let profile = mapping().profile().clone();
        let storage =
            StorageWorker::open(self.dir.file(), profile.clone(), Cancellation::new()).unwrap();
        self.worker = Worker::new(
            storage,
            Target {
                profile,
                path: self.dir.file(),
                calls: self.calls.clone(),
                mode,
                cancel: self.cancel.clone(),
                entered: None,
                release: None,
            },
        )
        .unwrap();
    }
}
pub struct History {
    mapping: DocumentMapping,
    events: Option<Vec<JournalView>>,
    end: u64,
    pub mapped: usize,
}
impl History {
    pub fn new(events: Vec<JournalView>, end: u64) -> Self {
        Self {
            mapping: mapping(),
            events: Some(events),
            end,
            mapped: 0,
        }
    }
}
impl ProjectionHistory for History {
    fn profile(&self) -> &ProjectionProfile {
        self.mapping.profile()
    }
    async fn fetch(&mut self, _after: &JournalCursor) -> Result<JournalBatch> {
        Ok(JournalBatch {
            events: self.events.take().unwrap(),
            cursor: super::support::cursor(self.end),
        })
    }
    fn document(&mut self, e: &JournalView) -> Result<ApprovedDocument> {
        self.mapped += 1;
        self.mapping.document(e, None)
    }
}
