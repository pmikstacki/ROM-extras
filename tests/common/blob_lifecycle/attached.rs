//! Provider-neutral native BlobService lifecycle acceptance.
use super::{
    helpers::{close, directory, input, reserve, runtime, service},
    observed::Observed,
};
use rom::Actor;
use rom_blob::{Blob, BlobState, BlobStore, Error, Upload, UploadOutcome};
use std::{
    path::Path,
    sync::{Arc, atomic::Ordering},
};

pub(crate) async fn staged_content_authorization_and_detachment<B: BlobStore + 'static>(
    provider: &str,
    artifacts: &Path,
    factory: impl Fn() -> B,
) {
    for redb in [false, true] {
        let dir = directory(artifacts, provider, redb, "attached");
        let db = dir.join("database");
        let rt = runtime(redb, &db);
        let store = Arc::new(Observed::new(factory()));
        let svc = service(&rt, provider, store.clone());
        let alice = Actor::trusted("fixture", "alice");
        let mallory = Actor::trusted("fixture", "mallory");
        let id = dir.file_name().unwrap().to_str().unwrap().to_owned();
        reserve(&svc, &alice, provider, &id, b"false").await;
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
        let svc = service(&reopened, provider, store.clone());
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
        let svc = service(&reopened, provider, store.clone());
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
