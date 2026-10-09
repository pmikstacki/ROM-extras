//! Shared public-API child interruption and persistent-service recovery scenario.
use crate::{loss_proxy, native_fixture};
use rom::{Actor, Command, JournalCursor, Resource, Runtime, Storage};
use rom_projection_core::{
    Cancellation, Checkpoint, ProjectionHistory, ProjectionTarget, RuntimeHistory,
    StorageLifecycle, TargetFailure, Worker, WorkerFailure,
};
use std::{os::unix::fs::DirBuilderExt, path::Path, sync::Arc, time::Duration};
#[derive(Clone, Resource)]
#[resource(name = "native_process_docs")]
struct Document {
    title: String,
}
fn runtime(path: &Path, redb: bool) -> Runtime {
    let storage: Arc<dyn Storage> = if redb {
        Arc::new(rom_redb::Redb::open(path.join("resources")).unwrap())
    } else {
        Arc::new(rom_sqlite::Sqlite::open(path.join("resources")).unwrap())
    };
    Runtime::builder()
        .resource(
            Document::definition()
                .policy(|a, _, _| a.authority == "native-crash")
                .field_policy(|a, _, _, _| a.authority == "native-crash"),
        )
        .build(storage, Runtime::shared_cpu_pool(2).unwrap())
        .unwrap()
}
fn executor() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}
/// Intentionally bypass all Rust destructors after proving actual remote acceptance and pending local work.
pub fn child() -> ! {
    let directory = std::path::PathBuf::from(std::env::var("ROM_CRASH_DIRECTORY").unwrap());
    let redb = std::env::var("ROM_CRASH_REDB").unwrap() == "true";
    let physical = std::env::var("ROM_CRASH_PHYSICAL").unwrap();
    let endpoint = std::env::var("ROM_CRASH_ENDPOINT").unwrap();
    let rt = executor();
    let host = StorageLifecycle::new().unwrap();
    let runtime = runtime(&directory, redb);
    let actor = Actor::trusted("native-crash", "export");
    rt.block_on(async {
        let origin = runtime.journal_head(&actor, Document::KIND).await.unwrap();
        for id in ["live", "tomb"] {
            runtime
                .execute(
                    &actor,
                    Command::create(
                        id,
                        Document {
                            title: "old".into(),
                        },
                    )
                    .idempotency(&format!("create-{id}")),
                )
                .await
                .unwrap();
        }
        runtime
            .execute(
                &actor,
                Command::replace(
                    "live",
                    Document {
                        title: "newer".into(),
                    },
                )
                .at_revision(1)
                .idempotency("replace-live"),
            )
            .await
            .unwrap();
        runtime
            .execute(
                &actor,
                Command::<Document>::delete("tomb")
                    .at_revision(1)
                    .idempotency("delete-tomb"),
            )
            .await
            .unwrap();
        let (_, mapping) = native_fixture::fixture();
        let profile = mapping.profile().clone();
        let target =
            native_fixture::target_at(&endpoint, profile.clone(), &physical, vec!["title".into()]);
        let mut source =
            RuntimeHistory::new(runtime.clone(), actor.clone(), Document::KIND, mapping).unwrap();
        let batch = source.fetch(&origin).await.unwrap();
        assert_eq!(
            batch.cursor,
            runtime.journal_head(&actor, Document::KIND).await.unwrap()
        );
        let documents = batch
            .events
            .iter()
            .map(|event| source.document(event).unwrap())
            .collect();
        let local = host
            .create(
                directory.join("checkpoint"),
                profile.clone(),
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
                .apply_page(batch.cursor.clone(), documents, &Cancellation::new())
                .await,
            Err(WorkerFailure::Target(TargetFailure::Unknown))
        );
        let snapshot = worker.snapshot().await.unwrap();
        assert!(snapshot.pending_page().is_some());
        assert_eq!(snapshot.checkpoint().cursor(Document::KIND), Some(&origin));
        // This GET-only inspection proves acknowledgement loss followed actual acceptance.
        let (_, mapping) = native_fixture::fixture();
        let mut direct = native_fixture::target_at(
            "https://127.0.0.1:55460",
            profile,
            &physical,
            vec!["title".into()],
        );
        let expected: Vec<_> = batch
            .events
            .iter()
            .filter(|e| e.view.revision == 2)
            .map(|e| mapping.document(e, None).unwrap())
            .collect();
        let prepared = direct
            .prepare(&expected.iter().collect::<Vec<_>>())
            .unwrap();
        assert_eq!(direct.inspect_prepared(&prepared).await.unwrap().len(), 2);
        std::fs::write(
            directory.join("accepted.json"),
            serde_json::to_vec(&(origin, batch.cursor)).unwrap(),
        )
        .unwrap();
        std::process::exit(86);
    })
}
pub fn run(redb: bool, child_args: &[&str]) {
    let directory =
        std::env::temp_dir().join(format!("rom-os-process-{}-{redb}", std::process::id()));
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&directory)
        .unwrap();
    let rt = executor();
    let (mut direct, mapping) = native_fixture::fixture();
    rt.block_on(native_fixture::create_generation(&mut direct));
    let physical = direct.physical_target().to_owned();
    let profile = mapping.profile().clone();
    // Parent owns the relay; process exit cannot orphan a child-owned fixture server.
    let proxy = loss_proxy::LossProxy::start();
    let status = std::process::Command::new("timeout")
        .args(["--kill-after=2s", "60s"])
        .arg(std::env::current_exe().unwrap())
        .args(child_args)
        .env("ROM_CRASH_DIRECTORY", &directory)
        .env("ROM_CRASH_REDB", redb.to_string())
        .env("ROM_CRASH_PHYSICAL", &physical)
        .env("ROM_CRASH_ENDPOINT", &proxy.endpoint)
        .status()
        .unwrap();
    assert_eq!(
        status.code(),
        Some(86),
        "child must exit at the proven remote-acceptance barrier"
    );
    drop(proxy);
    let (origin, endpoint): (JournalCursor, JournalCursor) =
        serde_json::from_slice(&std::fs::read(directory.join("accepted.json")).unwrap()).unwrap();
    let status = std::process::Command::new("timeout")
        .args([
            "--kill-after=2s",
            "30s",
            "docker",
            "restart",
            "--time",
            "10",
        ])
        .arg(native_fixture::container_name())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap();
    assert!(status.success());
    let runtime = runtime(&directory, redb);
    let actor = Actor::trusted("native-crash", "export");
    let host = StorageLifecycle::new().unwrap();
    rt.block_on(async {
        native_fixture::generation_readiness::wait("*").await;
        let mut source =
            RuntimeHistory::new(runtime.clone(), actor.clone(), Document::KIND, mapping).unwrap();
        let batch = source.fetch(&origin).await.unwrap();
        assert_eq!(batch.cursor, endpoint);
        let expected: Vec<_> = batch
            .events
            .iter()
            .filter(|e| e.view.revision == 2)
            .map(|e| source.document(e).unwrap())
            .collect();
        assert_eq!(expected.len(), 2);
        let prepared = direct
            .prepare(&expected.iter().collect::<Vec<_>>())
            .unwrap();
        // Inspect before recovery: no replay may recreate state lost during backend restart.
        tokio::time::timeout(Duration::from_secs(45), async {
            loop {
                if let Ok(observed) = direct.inspect_prepared(&prepared).await {
                    assert_eq!(observed.len(), 2);
                    break;
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        .expect("exact newer/tombstone state absent after backend restart");
        let local = host
            .open(
                directory.join("checkpoint"),
                profile.clone(),
                Cancellation::new(),
            )
            .unwrap()
            .receive()
            .await
            .unwrap();
        let mut worker = Worker::new(local, direct).unwrap();
        let state = worker.snapshot().await.unwrap();
        assert!(state.pending_page().is_some());
        assert_eq!(state.checkpoint().cursor(Document::KIND), Some(&origin));
        assert!(matches!(
            runtime.read_projected(&actor, Document::KIND, "tomb").await,
            Err(rom::Error::Denied)
        ));
        assert_eq!(
            runtime
                .read_projected(&actor, Document::KIND, "live")
                .await
                .unwrap()
                .revision,
            2
        );
        worker
            .recover(&mut source, &Cancellation::new())
            .await
            .unwrap()
            .unwrap();
        let state = worker.snapshot().await.unwrap();
        assert!(state.pending_page().is_none());
        assert_eq!(state.checkpoint().cursor(Document::KIND), Some(&endpoint));
        worker.shutdown_async().await.unwrap();
        drop(worker);
        let reopened = host
            .open(directory.join("checkpoint"), profile, Cancellation::new())
            .unwrap()
            .receive()
            .await
            .unwrap();
        let state = reopened.load().unwrap().receive().await.unwrap();
        assert!(state.pending_page().is_none());
        assert_eq!(state.checkpoint().cursor(Document::KIND), Some(&endpoint));
        for document in &expected {
            let metadata = document.metadata();
            let stored = reopened
                .key_state(metadata.key().clone())
                .unwrap()
                .receive()
                .await
                .unwrap()
                .unwrap();
            assert_eq!(stored.revision(), metadata.revision());
            assert_eq!(stored.is_tombstone(), metadata.is_tombstone());
            assert_eq!(stored.digest(), metadata.digest());
        }
        reopened.shutdown_async().await.unwrap();
        drop(reopened);
        drop(source);
        runtime.shutdown().await.unwrap();
    });
    drop(runtime);
    host.shutdown().unwrap();
    drop(host);
    std::fs::remove_dir_all(directory).unwrap();
}
