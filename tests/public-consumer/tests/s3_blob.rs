//! Real S3-compatible qualification through ROM's unchanged public adapter.
use rom_blob::{BlobStore, Digest, Error, ObjectKey};
#[cfg(feature = "s3-lifecycle")]
#[path = "../../common/blob_lifecycle/mod.rs"]
mod blob_lifecycle;
mod s3_fixture;
use s3_fixture::{adapter, restart};
use std::{
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

fn key(name: &str) -> ObjectKey {
    ObjectKey::parse(Digest::of(name.as_bytes()).as_str()).unwrap()
}
static FIXTURE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
#[tokio::test]
async fn published_port_passes_on_configured_s3_fixture() {
    let _exclusive = FIXTURE.lock().await;
    rom_conformance::blob::basic(&adapter(16, false))
        .await
        .unwrap();
}
#[tokio::test]
async fn eight_way_create_has_one_winner_and_seven_confirmed_conflicts() {
    let _exclusive = FIXTURE.lock().await;
    let store = Arc::new(adapter(16, false));
    let unique = format!(
        "s3-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    // Eight competing values, released together, across sixteen fresh keys.
    for round in 0..16 {
        let k = key(&format!("{unique}-race-{round}"));
        let barrier = Arc::new(tokio::sync::Barrier::new(8));
        let mut tasks = tokio::task::JoinSet::new();
        for candidate in 0u8..8 {
            let (store, k, barrier) = (store.clone(), k.clone(), barrier.clone());
            tasks.spawn(async move {
                barrier.wait().await;
                (candidate, store.create(&k, vec![candidate]).await)
            });
        }
        let mut winners = vec![];
        while let Some(result) = tasks.join_next().await {
            let (candidate, outcome) = result.unwrap();
            match outcome {
                Ok(()) => winners.push(candidate),
                Err(Error::Conflict) => {}
                other => panic!("unexpected create outcome: {other:?}"),
            }
        }
        assert_eq!(winners.len(), 1);
        assert_eq!(store.get(&k, 1).await.unwrap(), winners);
        assert_eq!(
            store.create(&k, b"false".to_vec()).await,
            Err(Error::Conflict)
        );
        assert_eq!(store.get(&k, 1).await.unwrap(), winners);
        store.delete(&k).await.unwrap();
    }
}
#[tokio::test]
async fn confirmed_denial_and_restart_retention_pass_on_configured_s3_fixture() {
    let _exclusive = FIXTURE.lock().await;
    let store = Arc::new(adapter(16, false));
    let unique = format!(
        "s3-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let denied = key(&format!("{unique}-denied"));
    let incorrect = adapter(16, true);
    assert_eq!(
        incorrect.create(&denied, b"false".to_vec()).await,
        Err(Error::Denied)
    );
    assert_eq!(incorrect.head(&denied).await, Err(Error::Denied));
    assert_eq!(store.head(&denied).await, Err(Error::Missing));
    let retained = key(&format!("{unique}-retained"));
    store.create(&retained, b"false".to_vec()).await.unwrap();
    assert_eq!(store.head(&retained).await.unwrap().bytes, 5);
    restart().await;
    let rebound = adapter(16, false);
    tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            match rebound.head(&retained).await {
                Ok(meta) => {
                    assert_eq!(meta.bytes, 5);
                    break;
                }
                Err(Error::Backend | Error::Timeout) => {
                    tokio::time::sleep(Duration::from_millis(100)).await
                }
                other => panic!("unexpected restart outcome: {other:?}"),
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(rebound.get(&retained, 5).await.unwrap(), b"false");
    assert_eq!(rebound.get(&retained, 4).await, Err(Error::TooLarge));
    assert_eq!(
        rebound.create(&retained, b"true".to_vec()).await,
        Err(Error::Conflict)
    );
    assert_eq!(rebound.get(&retained, 5).await.unwrap(), b"false");
    // Retain acknowledged bytes for later recovery checks.
}

#[tokio::test]
async fn uncertain_concurrent_creates_reconcile_without_overwrite_or_cleanup() {
    let _exclusive = FIXTURE.lock().await;
    let store = Arc::new(adapter(16, false));
    let k = key(&format!(
        "uncertain-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let barrier = Arc::new(tokio::sync::Barrier::new(8));
    let mut tasks = tokio::task::JoinSet::new();
    for candidate in 0u8..8 {
        let (store, k, barrier) = (store.clone(), k.clone(), barrier.clone());
        tasks.spawn(async move {
            barrier.wait().await;
            (candidate, store.create(&k, vec![candidate]).await)
        });
    }
    let mut accepted = vec![];
    let mut uncertain = vec![];
    let mut conflicted = vec![];
    while let Some(result) = tasks.join_next().await {
        let (candidate, result) = result.unwrap();
        match result {
            Ok(()) => accepted.push(candidate),
            Err(Error::Unknown) => uncertain.push(candidate),
            Err(Error::Conflict) => conflicted.push(candidate),
            other => panic!("unexpected uncertainty outcome: {other:?}"),
        }
    }
    assert!(accepted.len() <= 1);
    assert_eq!(accepted.len() + uncertain.len() + conflicted.len(), 8);
    let bytes = tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            match store.get(&k, 1).await {
                Ok(bytes) => break bytes,
                Err(Error::Missing) => tokio::time::sleep(Duration::from_millis(100)).await,
                other => panic!("unexpected reconciliation read: {other:?}"),
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(bytes.len(), 1);
    assert!(accepted.contains(&bytes[0]) || uncertain.contains(&bytes[0]));
    assert!(!conflicted.contains(&bytes[0]));
    if let Some(winner) = accepted.first() {
        assert_eq!(bytes[0], *winner);
    }
    for candidate in 0u8..8 {
        assert_eq!(
            store.create(&k, vec![candidate]).await,
            Err(Error::Conflict)
        );
        assert_eq!(store.get(&k, 1).await.unwrap(), bytes);
    }
    println!(
        "uncertainty accepted={} unknown={} conflicts={} retry_conflicts=8; object retained",
        accepted.len(),
        uncertain.len(),
        conflicted.len()
    );
    // No deletion: completed waiters do not prove provider quiescence after Unknown.
}

#[cfg(feature = "s3-lifecycle")]
#[tokio::test]
async fn blob_service_attachment_authorization_and_detachment_survive_native_reopen() {
    let _exclusive = FIXTURE.lock().await;
    let artifacts = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.superpowers");
    blob_lifecycle::staged_content_authorization_and_detachment("s3", &artifacts, || {
        adapter(1024, false)
    })
    .await;
}

#[cfg(feature = "s3-lifecycle")]
#[tokio::test]
async fn blob_service_deleted_reservation_retains_real_unattached_object() {
    let _exclusive = FIXTURE.lock().await;
    let artifacts = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.superpowers");
    blob_lifecycle::deleted_reservation_returns_unattached("s3", &artifacts, || {
        adapter(1024, false)
    })
    .await;
}

#[cfg(feature = "s3-lifecycle")]
#[tokio::test]
async fn lost_s3_success_response_recovers_pending_on_explicit_upload() {
    let _exclusive = FIXTURE.lock().await;
    let artifacts = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.superpowers");
    tokio::time::timeout(Duration::from_secs(45), s3_fixture::recover(&artifacts))
        .await
        .expect("bounded S3 acknowledgement recovery");
}
