//! Real response loss and public BlobService recovery on both native stores.
use super::{adapter, profile::proxy_adapter, wire::Proxy};
use crate::blob_lifecycle::{
    helpers::{close, directory, input, reserve, runtime, service},
    observed::Observed,
};
use rom::Actor;
use rom_blob::{Blob, BlobState, BlobStore, Error, UploadOutcome};
use std::{path::Path, sync::Arc};

pub(crate) async fn recover(artifacts: &Path) {
    for redb in [false, true] {
        let dir = directory(artifacts, "s3", redb, "wire-ack-loss");
        let db = dir.join("database");
        let proxy = Proxy::start();
        let store = Arc::new(Observed::new(proxy_adapter(proxy.endpoint())));
        let rt = runtime(redb, &db);
        let svc = service(&rt, "s3", store.clone());
        let alice = Actor::trusted("fixture", "alice");
        let id = dir.file_name().unwrap().to_str().unwrap();
        reserve(&svc, &alice, "s3", id, b"false").await;
        let outcome = svc.upload(&alice, id, input(b"false")).await;
        let first = proxy.counts();
        assert_eq!(
            (first.puts, first.successes),
            (1, 1),
            "baseline must reach a real backend success"
        );
        assert!(
            matches!(outcome, Err(Error::Unknown)),
            "lost response must report Unknown"
        );
        assert_eq!((first.puts, first.successes, first.dropped), (1, 1, 1));
        let key = first.key.unwrap();
        let pending = rt.read::<Blob>(&alice, id).await.unwrap();
        assert_eq!(pending.revision, 1);
        assert_eq!(pending.value.unwrap().state, BlobState::Pending);
        assert_eq!(adapter(1024, false).get(&key, 5).await.unwrap(), b"false");
        let recovery = svc.upload(&alice, id, input(b"false")).await;
        if recovery.is_err() {
            let c = proxy.counts();
            eprintln!(
                "recovery diagnostic puts={} successes={} conflicts={} gets={} dropped={}",
                c.puts, c.successes, c.conflicts, c.gets, c.dropped
            );
        }
        assert!(
            matches!(recovery, Ok(UploadOutcome::Attached(_))),
            "explicit recovery must attach"
        );
        let recovered = proxy.counts();
        assert_eq!(
            (
                recovered.puts,
                recovered.successes,
                recovered.conflicts,
                recovered.dropped
            ),
            (2, 1, 1, 1)
        );
        assert!(recovered.gets >= 1, "conflict must verify stored bytes");
        let ready = rt.read::<Blob>(&alice, id).await.unwrap();
        assert_eq!(ready.revision, 2);
        let ready = ready.value.unwrap();
        assert_eq!(ready.state, BlobState::Ready);
        assert_eq!(ready.upload_revision, 1);
        assert!(matches!(
            svc.upload(&alice, id, input(b"false")).await.unwrap(),
            UploadOutcome::Attached(_)
        ));
        assert_eq!(proxy.counts().puts, 2);
        assert_eq!(svc.read(&alice, id).await.unwrap(), b"false");
        close(svc, rt).await;
        drop(store);
        let counts = proxy.counts();
        proxy.drain().await;
        let reopened = runtime(redb, &db);
        let store = Arc::new(Observed::new(adapter(1024, false)));
        let svc = service(&reopened, "s3", store);
        let persisted = reopened.read::<Blob>(&alice, id).await.unwrap();
        assert_eq!(persisted.revision, 2);
        assert_eq!(persisted.value.unwrap().state, BlobState::Ready);
        assert_eq!(svc.read(&alice, id).await.unwrap(), b"false");
        close(svc, reopened).await;
        println!(
            "wire_ack_loss native={} puts={} backend_success={} conflicts={} suppressed={} verified_gets={}; Unknown/Pending -> explicit retry/Ready -> reopen; object retained",
            if redb { "redb" } else { "sqlite" },
            counts.puts,
            counts.successes,
            counts.conflicts,
            counts.dropped,
            counts.gets
        );
    }
}
