//! Provider-neutral native BlobService lifecycle acceptance.
use super::{
    helpers::{close, directory, input, reserve, runtime, service},
    observed::Observed,
};
use rom::{Actor, Command};
use rom_blob::{Blob, BlobStore, Digest, UploadOutcome};
use std::{
    path::Path,
    sync::{Arc, atomic::Ordering},
    time::Duration,
};

pub(crate) async fn deleted_reservation_returns_unattached<B: BlobStore + 'static>(
    provider: &str,
    artifacts: &Path,
    factory: impl Fn() -> B,
) {
    for redb in [false, true] {
        let dir = directory(artifacts, provider, redb, "unattached");
        let db = dir.join("database");
        let rt = runtime(redb, &db);
        let store = Arc::new(Observed::new(factory()));
        let svc = service(&rt, provider, store.clone());
        let actor = Actor::trusted("fixture", "alice");
        let id = dir.file_name().unwrap().to_str().unwrap().to_owned();
        reserve(&svc, &actor, provider, &id, b"false").await;
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
        let svc = service(&reopened, provider, store.clone());
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
