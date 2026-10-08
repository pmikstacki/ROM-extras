//! Public BlobService integration with real Azure protocol and reopened native ROM stores.
#[path = "../../../tests/common/azure_fixture.rs"]
mod fixture;
#[path = "common/lifecycle_store.rs"]
mod lifecycle_store;
use lifecycle_store::Observed;
use rom::{Actor, Command, Runtime, Storage};
use rom_blob::{Blob, BlobService, BlobState, BlobStore, Digest, Error, Upload, UploadOutcome};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, atomic::Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

fn directory(redb: bool, scenario: &str) -> PathBuf {
    let id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../.superpowers")
        .join(format!(
            "azure-lifecycle-{scenario}-{redb}-{}-{id}",
            std::process::id()
        ));
    std::fs::create_dir(&path).unwrap();
    path
}
fn runtime(redb: bool, path: &Path) -> Runtime {
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
fn service(runtime: &Runtime, store: Arc<Observed>) -> BlobService {
    BlobService::builder(runtime.clone())
        .store("azure", store)
        .build()
        .unwrap()
}
fn input(bytes: &[u8]) -> Upload {
    Box::pin(futures_util::stream::iter([Ok(bytes.to_vec())]))
}
async fn reserve(service: &BlobService, actor: &Actor, id: &str, bytes: &[u8]) {
    let pending = service
        .reserve(
            actor,
            id,
            "azure",
            Digest::of(bytes),
            bytes.len() as u64,
            &format!("reserve-{id}"),
        )
        .await
        .unwrap();
    assert_eq!(pending.revision, 1);
    assert_eq!(pending.value.unwrap().state, BlobState::Pending);
}
async fn close(service: BlobService, runtime: Runtime) {
    service.shutdown().await.unwrap();
    runtime.shutdown().await.unwrap();
    drop(service);
    drop(runtime);
}
#[tokio::test]
async fn staged_content_authorization_and_detachment_survive_native_reopen() {
    for redb in [false, true] {
        let dir = directory(redb, "attached");
        let db = dir.join("database");
        let rt = runtime(redb, &db);
        let store = Arc::new(Observed::new(fixture::emulator(1024)));
        let svc = service(&rt, store.clone());
        let alice = Actor::trusted("fixture", "alice");
        let mallory = Actor::trusted("fixture", "mallory");
        let id = dir.file_name().unwrap().to_str().unwrap().to_owned();
        reserve(&svc, &alice, &id, b"false").await;
        assert!(matches!(
            svc.upload(&alice, &id, input(b"False")).await,
            Err(Error::Conflict)
        ));
        assert_eq!(store.creates.load(Ordering::SeqCst), 0);
        let failed_input: Upload = Box::pin(futures_util::stream::iter([
            Ok(b"fa".to_vec()),
            Err(Error::Backend),
        ]));
        assert!(matches!(
            svc.upload(&alice, &id, failed_input).await,
            Err(Error::Backend)
        ));
        assert_eq!(store.creates.load(Ordering::SeqCst), 0);
        assert_eq!(rt.read::<Blob>(&alice, &id).await.unwrap().revision, 1);
        assert!(matches!(
            svc.upload(&alice, &id, input(b"false")).await.unwrap(),
            UploadOutcome::Attached(_)
        ));
        assert_eq!(store.creates.load(Ordering::SeqCst), 1);
        assert_eq!(svc.read(&alice, &id).await.unwrap(), b"false");
        let reads = store.reads.load(Ordering::SeqCst);
        assert_eq!(
            svc.read(&mallory, &id).await,
            Err(Error::Core(rom::Error::Denied))
        );
        assert_eq!(store.reads.load(Ordering::SeqCst), reads);
        assert!(matches!(
            svc.upload(&alice, &id, input(b"false")).await.unwrap(),
            UploadOutcome::Attached(_)
        ));
        assert_eq!(store.creates.load(Ordering::SeqCst), 1);
        close(svc, rt).await;
        let reopened = runtime(redb, &db);
        let svc = service(&reopened, store.clone());
        assert_eq!(svc.read(&alice, &id).await.unwrap(), b"false");
        let snapshot = reopened.read::<Blob>(&alice, &id).await.unwrap();
        assert_eq!(snapshot.revision, 2);
        assert_eq!(snapshot.value.unwrap().upload_revision, 1);
        let receipt = svc.detach(&alice, &id).await.unwrap();
        assert_eq!(svc.detach(&alice, &id).await.unwrap().key, receipt.key);
        assert_eq!(svc.read(&alice, &id).await, Err(Error::Missing));
        assert_eq!(store.inner.get(&receipt.key, 5).await.unwrap(), b"false");
        close(svc, reopened).await;
        let reopened = runtime(redb, &db);
        let svc = service(&reopened, store.clone());
        assert_eq!(
            reopened
                .read::<Blob>(&alice, &id)
                .await
                .unwrap()
                .value
                .unwrap()
                .state,
            BlobState::Detached
        );
        assert_eq!(svc.read(&alice, &id).await, Err(Error::Missing));
        assert!(matches!(
            svc.upload(&alice, &id, input(b"false")).await,
            Err(Error::Conflict)
        ));
        assert_eq!(store.creates.load(Ordering::SeqCst), 1);
        assert_eq!(store.inner.get(&receipt.key, 5).await.unwrap(), b"false");
        // Keep detached bytes: no grace/quiescence cleanup authorization is inferred.
        close(svc, reopened).await;
    }
}
#[tokio::test]
async fn deleted_reservation_returns_real_unattached_object_without_implicit_cleanup() {
    for redb in [false, true] {
        let dir = directory(redb, "unattached");
        let db = dir.join("database");
        let rt = runtime(redb, &db);
        let store = Arc::new(Observed::new(fixture::emulator(1024)));
        let svc = service(&rt, store.clone());
        let actor = Actor::trusted("fixture", "alice");
        let id = dir.file_name().unwrap().to_str().unwrap().to_owned();
        reserve(&svc, &actor, &id, b"false").await;
        store.pause.store(true, Ordering::SeqCst);
        let (writer_service, writer_actor, writer_id) = (svc.clone(), actor.clone(), id.clone());
        let writer = tokio::spawn(async move {
            writer_service
                .upload(&writer_actor, &writer_id, input(b"false"))
                .await
        });
        tokio::time::timeout(Duration::from_secs(10), store.published.notified())
            .await
            .unwrap();
        let key = store.last_key.lock().unwrap().clone().unwrap();
        assert_eq!(store.inner.get(&key, 5).await.unwrap(), b"false");
        rt.execute(
            &actor,
            Command::<Blob>::delete(&id)
                .at_revision(1)
                .idempotency("cancel"),
        )
        .await
        .unwrap();
        store.release.notify_one();
        let outcome = tokio::time::timeout(Duration::from_secs(10), writer)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let UploadOutcome::Unattached { object, cause } = outcome else {
            panic!("deleted reservation cannot attach")
        };
        assert_eq!(cause, rom::Error::Denied);
        assert_eq!(object.key, key);
        assert_eq!(object.digest, Digest::of(b"false"));
        assert_eq!(object.bytes, 5);
        assert!(matches!(
            rt.read::<Blob>(&actor, &id).await,
            Err(rom::Error::Denied)
        ));
        assert!(
            rt.query(&actor, &rom::Query::<Blob>::all())
                .await
                .unwrap()
                .is_empty()
        );
        close(svc, rt).await;
        let reopened = runtime(redb, &db);
        let svc = service(&reopened, store.clone());
        assert!(matches!(
            reopened.read::<Blob>(&actor, &id).await,
            Err(rom::Error::Denied)
        ));
        assert!(
            reopened
                .query(&actor, &rom::Query::<Blob>::all())
                .await
                .unwrap()
                .is_empty()
        );
        assert_eq!(store.inner.get(&object.key, 5).await.unwrap(), b"false");
        assert_eq!(store.creates.load(Ordering::SeqCst), 1);
        // This is retained orphan evidence, not automatic deletion or a wire ACK-loss test.
        close(svc, reopened).await;
    }
}
