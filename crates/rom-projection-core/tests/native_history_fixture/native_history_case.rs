//! Actual public ROM journals; the target remains synthetic, not backend qualification.
use rom::{Access, Actor, Command, JournalCursor, Resource, Runtime, Storage};
use rom_projection_core::{
    ApprovedDocument, Cancellation, Checkpoint, DocumentMapping, Error, HostVectors,
    OperationMetadata, ProjectionHistory, ProjectionProfile, ProjectionTarget, RemoteObservation,
    RuntimeHistory, StorageLifecycle, TargetFailure, VectorInput, Worker, WorkerFailure,
};
use std::{
    os::unix::fs::DirBuilderExt,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
#[derive(Clone, Resource)]
#[resource(name = "native_documents")]
struct Doc {
    title: String,
    secret: String,
    readable: bool,
}
struct Directory(std::path::PathBuf);
impl Directory {
    fn new(redb: bool, case: usize) -> Self {
        let path = std::env::temp_dir().join(format!(
            "rom-extras-native-history-{}-{redb}-{case}",
            std::process::id()
        ));
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&path)
            .unwrap();
        Self(path)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
fn writer() -> Actor {
    Actor::trusted("fixture", "operator")
}
fn export() -> Actor {
    Actor::trusted("fixture", "export").with_kind(rom::PrincipalKind::Service)
}
fn runtime(redb: bool, path: &Path) -> Runtime {
    let storage: Arc<dyn Storage> = if redb {
        Arc::new(rom_redb::Redb::open(path).unwrap())
    } else {
        Arc::new(rom_sqlite::Sqlite::open(path).unwrap())
    };
    Runtime::builder()
        .resource(
            Doc::definition()
                .policy(|a, access, d| {
                    a.authority == "fixture"
                        && (a.subject == "operator"
                            || (matches!(access, Access::Read)
                                && a.subject == "export"
                                && d.readable))
                })
                .field_policy(|a, _, f, d| {
                    a.subject == "operator"
                        || (a.subject == "export"
                            && (f == "readable" || (f == "title" && d.secret != "revoke-title")))
                }),
        )
        .limits(rom::Limits {
            snapshot_rows: 2,
            command_bytes: 32 * 1024, // Fixture admits values larger than the projection selection bound.
            ..Default::default()
        })
        .build(storage, Runtime::shared_cpu_pool(2).unwrap())
        .unwrap()
}
fn mapping(vector: bool) -> DocumentMapping {
    DocumentMapping::new(
        ProjectionProfile::new(
            "native-fixture",
            "qdrant",
            "mapping",
            vector.then_some("fixture-model-v1"),
        )
        .unwrap(),
        vec!["title".into()],
        vector.then_some(2),
    )
    .unwrap()
}
fn doc(title: &str) -> Doc {
    Doc {
        title: title.into(),
        secret: "unexported-private-value".into(),
        readable: true,
    }
}
struct Target {
    profile: ProjectionProfile,
    fail: bool,
    calls: Arc<AtomicUsize>,
}
impl ProjectionTarget for Target {
    type Request = Vec<OperationMetadata>;
    fn profile(&self) -> &ProjectionProfile {
        &self.profile
    }
    fn physical_target(&self) -> &str {
        "native-target"
    }
    fn prepare(
        &self,
        documents: &[&ApprovedDocument],
    ) -> rom_projection_core::Result<Self::Request> {
        for d in documents {
            if let Some(fields) = d.fields() {
                assert!(!fields.contains_key("secret"));
                assert!(!fields.contains_key("readable"));
            }
        }
        Ok(documents.iter().map(|d| d.metadata().clone()).collect())
    }
    async fn apply(
        &mut self,
        request: Self::Request,
    ) -> Result<Vec<RemoteObservation>, TargetFailure> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.fail {
            return Err(TargetFailure::Unknown);
        }
        Ok(request
            .into_iter()
            .map(|op| RemoteObservation::new(&self.profile, "native-target", op).unwrap())
            .collect())
    }
}
struct Vectors {
    model: String,
    flip: bool,
    calls: Arc<AtomicUsize>,
}
impl HostVectors for Vectors {
    fn model_id(&self) -> &str {
        &self.model
    }
    fn vector(&mut self, input: VectorInput<'_>) -> rom_projection_core::Result<Vec<f32>> {
        assert_eq!(input.key().kind, Doc::KIND);
        assert!(input.revision() > 0);
        assert_eq!(input.fields().len(), 1);
        assert_eq!(input.fields()[0].0, "title");
        assert!(input.fields()[0].1.is_string());
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.flip {
            self.model = "changed-model".into();
        }
        Ok(vec![1.0, 2.0])
    }
}
fn worker(
    host: &StorageLifecycle,
    rt: &tokio::runtime::Runtime,
    path: &Path,
    seed: (&ProjectionProfile, &JournalCursor),
    fail: bool,
    calls: Arc<AtomicUsize>,
    create: bool,
) -> Worker<Target> {
    let (profile, cursor) = seed;
    let storage = if create {
        rt.block_on(
            host.create(
                path.into(),
                profile.clone(),
                Checkpoint::new("native-target", vec![cursor.clone()]).unwrap(),
                Cancellation::new(),
            )
            .unwrap()
            .receive(),
        )
        .unwrap()
    } else {
        rt.block_on(
            host.open(path.into(), profile.clone(), Cancellation::new())
                .unwrap()
                .receive(),
        )
        .unwrap()
    };
    Worker::new(
        storage,
        Target {
            profile: profile.clone(),
            fail,
            calls,
        },
    )
    .unwrap()
}
pub fn run(redb: bool, case: usize) {
    let directory = Directory::new(redb, case);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let native_path = directory.0.join("resources");
    let checkpoint_path = directory.0.join("checkpoint.redb");
    let runtime = self::runtime(redb, &native_path);
    let origin = rt
        .block_on(runtime.journal_head(&export(), Doc::KIND))
        .unwrap();
    let host = StorageLifecycle::new().unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    if case == 0 {
        rt.block_on(runtime.execute(
            &writer(),
            Command::create("a", doc("old")).idempotency("create-a"),
        ))
        .unwrap();
        rt.block_on(
            runtime.execute(
                &writer(),
                Command::replace("a", doc("new"))
                    .at_revision(1)
                    .idempotency("replace-a"),
            ),
        )
        .unwrap();
        rt.block_on(
            runtime.execute(
                &writer(),
                Command::<Doc>::delete("a")
                    .at_revision(2)
                    .idempotency("delete-a"),
            ),
        )
        .unwrap();
        let endpoint = rt
            .block_on(runtime.journal_head(&export(), Doc::KIND))
            .unwrap();
        let m = mapping(false);
        let profile = m.profile().clone();
        let mut source = RuntimeHistory::new(runtime.clone(), export(), Doc::KIND, m).unwrap();
        let first = rt.block_on(source.fetch(&origin)).unwrap();
        assert!(first.events.is_empty()); // Current tombstone denies old live disclosure.
        assert_eq!(first.cursor.position, 2);
        let batch = rt.block_on(source.fetch(&first.cursor)).unwrap();
        assert_eq!(batch.events.len(), 1);
        let approved = source.document(&batch.events[0]).unwrap();
        let key = approved.metadata().key().clone();
        assert!(approved.metadata().is_tombstone());
        let mut w = worker(
            &host,
            &rt,
            &checkpoint_path,
            (&profile, &origin),
            true,
            calls.clone(),
            true,
        );
        assert_eq!(
            rt.block_on(w.apply_page(endpoint.clone(), vec![approved], &Cancellation::new())),
            Err(WorkerFailure::Target(TargetFailure::Unknown))
        );
        rt.block_on(w.shutdown_async()).unwrap();
        drop(w);
        drop(source);
        rt.block_on(runtime.shutdown()).unwrap();
        drop(runtime);
        let reopened = self::runtime(redb, &native_path);
        rt.block_on(reopened.execute(
            &writer(),
            Command::create("later", doc("overrun")).idempotency("create-later"),
        ))
        .unwrap();
        let mut source =
            RuntimeHistory::new(reopened.clone(), export(), Doc::KIND, mapping(false)).unwrap();
        let mut w = worker(
            &host,
            &rt,
            &checkpoint_path,
            (&profile, &origin),
            false,
            calls.clone(),
            false,
        );
        rt.block_on(w.recover(&mut source, &Cancellation::new()))
            .unwrap()
            .unwrap();
        let state = rt.block_on(w.snapshot()).unwrap();
        assert_eq!(state.checkpoint().cursor(Doc::KIND), Some(&endpoint));
        assert!(state.pending_page().is_none());
        rt.block_on(w.shutdown_async()).unwrap();
        drop(w);
        let store = rom_projection_core::CheckpointStore::open(&checkpoint_path, &profile).unwrap();
        let state = store.key_state(&key).unwrap().unwrap();
        assert!(state.is_tombstone());
        assert_eq!(state.revision(), 3);
        drop(store);
        drop(source);
        rt.block_on(reopened.shutdown()).unwrap();
        return;
    }
    if case == 4 || case == 6 {
        let title = if case == 6 {
            "x".repeat(18000)
        } else {
            "approved".into()
        };
        rt.block_on(runtime.execute(
            &writer(),
            Command::create("a", doc(&title)).idempotency("create-a"),
        ))
        .unwrap();
        let vector_calls = Arc::new(AtomicUsize::new(0));
        let mut source = RuntimeHistory::with_vectors(
            runtime.clone(),
            export(),
            Doc::KIND,
            mapping(true),
            Vectors {
                model: "fixture-model-v1".into(),
                flip: false,
                calls: vector_calls.clone(),
            },
        )
        .unwrap();
        let batch = rt.block_on(source.fetch(&origin)).unwrap();
        if case == 6 {
            assert!(matches!(
                source.document(&batch.events[0]),
                Err(Error::TooLarge)
            ));
            assert_eq!(vector_calls.load(Ordering::SeqCst), 0);
        } else {
            let approved = source.document(&batch.events[0]).unwrap();
            assert_eq!(approved.vector(), Some(&[1.0, 2.0][..]));
            assert_eq!(vector_calls.load(Ordering::SeqCst), 1);
            let mut changed = RuntimeHistory::with_vectors(
                runtime.clone(),
                export(),
                Doc::KIND,
                mapping(true),
                Vectors {
                    model: "fixture-model-v1".into(),
                    flip: true,
                    calls: vector_calls.clone(),
                },
            )
            .unwrap();
            assert!(matches!(
                changed.document(&batch.events[0]),
                Err(Error::RebuildRequired)
            ));
            rt.block_on(
                runtime.execute(
                    &writer(),
                    Command::<Doc>::delete("a")
                        .at_revision(1)
                        .idempotency("delete-a"),
                ),
            )
            .unwrap();
            let after = rt.block_on(source.fetch(&batch.cursor)).unwrap();
            assert!(
                source
                    .document(&after.events[0])
                    .unwrap()
                    .metadata()
                    .is_tombstone()
            );
            assert_eq!(vector_calls.load(Ordering::SeqCst), 2); // One success and one rejected model mutation; tombstone skips vector.
        }
        rt.block_on(runtime.shutdown()).unwrap();
        return;
    }
    let mut value = doc("approved");
    if case == 5 {
        value.readable = false;
    }
    rt.block_on(runtime.execute(
        &writer(),
        Command::create("a", value).idempotency("create-a"),
    ))
    .unwrap();
    let endpoint = rt
        .block_on(runtime.journal_head(&export(), Doc::KIND))
        .unwrap();
    let m = mapping(false);
    let profile = m.profile().clone();
    let mut source = RuntimeHistory::new(runtime.clone(), export(), Doc::KIND, m).unwrap();
    let batch = rt.block_on(source.fetch(&origin)).unwrap();
    if case == 3 {
        let wrong = JournalCursor {
            generation: "wrong-generation".into(),
            ..origin.clone()
        };
        assert!(matches!(
            rt.block_on(source.fetch(&wrong)),
            Err(Error::HistoryGap)
        ));
        let mut expired = RuntimeHistory::new(
            runtime.clone(),
            export().expires_at(0),
            Doc::KIND,
            mapping(false),
        )
        .unwrap();
        assert!(matches!(
            rt.block_on(expired.fetch(&origin)),
            Err(Error::RebuildRequired)
        ));
        rt.block_on(runtime.shutdown()).unwrap();
        assert!(matches!(
            rt.block_on(source.fetch(&origin)),
            Err(Error::SourceUnavailable)
        ));
        return;
    }
    let documents = batch
        .events
        .iter()
        .map(|event| source.document(event).unwrap())
        .collect();
    let mut w = worker(
        &host,
        &rt,
        &checkpoint_path,
        (&profile, &origin),
        case != 5,
        calls.clone(),
        true,
    );
    if case == 5 {
        assert!(batch.events.is_empty());
        assert_eq!(batch.cursor, endpoint);
        rt.block_on(w.apply_page(endpoint.clone(), documents, &Cancellation::new()))
            .unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(
            rt.block_on(w.snapshot())
                .unwrap()
                .checkpoint()
                .cursor(Doc::KIND),
            Some(&endpoint)
        );
    } else {
        assert!(matches!(
            rt.block_on(w.apply_page(endpoint.clone(), documents, &Cancellation::new())),
            Err(WorkerFailure::Target(TargetFailure::Unknown))
        ));
        let mut revoked = doc("new");
        if case == 1 {
            revoked.readable = false;
        } else {
            revoked.secret = "revoke-title".into();
        }
        rt.block_on(
            runtime.execute(
                &writer(),
                Command::replace("a", revoked)
                    .at_revision(1)
                    .idempotency("revoke-a"),
            ),
        )
        .unwrap();
        assert_eq!(
            rt.block_on(w.recover(&mut source, &Cancellation::new())),
            Err(WorkerFailure::Core(Error::RebuildRequired))
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        let state = rt.block_on(w.snapshot()).unwrap();
        assert!(state.pending_page().is_some());
        assert_eq!(state.checkpoint().cursor(Doc::KIND), Some(&origin));
    }
    rt.block_on(w.shutdown_async()).unwrap();
    rt.block_on(runtime.shutdown()).unwrap();
}
