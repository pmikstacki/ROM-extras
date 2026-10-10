//! Public checkpoint inventory behavior; in-memory object fixtures are not native-service qualification.
#![cfg(any(feature = "sqlite", feature = "redb"))]
mod support;
use rom::{Actor, Command, Resource};
use rom_blob::{Blob, BlobService, Digest};
use rom_blob_checkpoint::{Inspection, Inventory, Limits};
use rom_blob_recovery::Context;
use rom_extras_maintenance::{Backend, backup};
use std::{
    sync::{Arc, atomic::Ordering},
    time::Duration,
};
use tokio_util::sync::CancellationToken;
fn context() -> Context {
    Context::new(
        tokio::time::Instant::now() + Duration::from_secs(10),
        CancellationToken::new(),
    )
}
#[tokio::test]
async fn malformed_archive_blob_and_zero_upload_revision_fail_before_inspection_or_reads() {
    let fixture = support::checkpoint().await;
    for (label, field, value) in [
        ("zero-upload", "upload_revision", rom::json!(0)),
        ("bad-digest", "digest", rom::json!("invalid-content-digest")),
    ] {
        let (_, mut snapshot) = rom_backup::read(
            &fixture.archive,
            fixture.backend,
            rom_backup::BackupLimits::default(),
        )
        .unwrap();
        let row = snapshot
            .rows
            .iter_mut()
            .find(|r| r.key.kind == Blob::KIND)
            .unwrap();
        row.value.as_mut().unwrap()[field] = value;
        let changed = row.clone();
        // Keep public snapshot receipt/event consistency while authoring invalid Blob fields.
        for receipt in &mut snapshot.receipts {
            if receipt.row.key == changed.key && receipt.row.revision == changed.revision {
                receipt.row = changed.clone();
            }
        }
        for (_, event) in &mut snapshot.events {
            if event.key == changed.key && event.revision == changed.revision {
                *event = changed.clone();
            }
        }
        let mut state = serde_json::to_value(&snapshot.state).unwrap();
        for event in state["events"].as_array_mut().unwrap() {
            let row: rom::Row = serde_json::from_value(event["row"].clone()).unwrap();
            if row.key == changed.key && row.revision == changed.revision {
                event["row"] = serde_json::to_value(&changed).unwrap();
            }
        }
        snapshot.state = serde_json::from_value(state).unwrap();
        let archive = fixture.root.join(format!("{label}.rombk"));
        rom_backup::write(
            &archive,
            fixture.backend,
            &snapshot,
            rom_backup::BackupLimits::default(),
        )
        .unwrap();
        let inspection = fixture.root.join(format!("{label}.db"));
        let result = Inspection::prepare(
            fixture.backend,
            &archive,
            &inspection,
            vec![("objects".into(), fixture.objects.clone())],
            Limits::default(),
        );
        assert_eq!(result.err(), Some(rom_blob_checkpoint::Error::Invalid));
        assert!(!inspection.exists());
        assert_eq!(fixture.objects.gets.load(Ordering::Relaxed), 0);
    }
}
#[tokio::test]
#[cfg(any(feature = "sqlite", feature = "redb"))]
async fn exact_equal_content_mixed_archive_persists_without_create_inventory() {
    let root = support::path();
    #[cfg(feature = "sqlite")]
    let (backend, storage) = (
        Backend::Sqlite,
        Arc::new(rom_sqlite::Sqlite::open(root.join("source.db")).unwrap()),
    );
    #[cfg(all(not(feature = "sqlite"), feature = "redb"))]
    let (backend, storage) = (
        Backend::Redb,
        Arc::new(rom_redb::Redb::open(root.join("source.db")).unwrap()),
    );
    let rt = support::runtime(storage.clone());
    let objects = Arc::new(support::Memory::default());
    let svc = BlobService::builder(rt.clone())
        .store("exact-alias", objects.clone())
        .build()
        .unwrap();
    let a = Actor::trusted("fixture", "one");
    let b = Actor::trusted("fixture", "two");
    for (actor, id) in [(&a, "Exact/A:ß"), (&b, "exact/a:ß")] {
        support::upload(&svc, actor, id, "exact-alias", b"same bytes").await;
    }
    svc.reserve(
        &a,
        "pending",
        "exact-alias",
        Digest::of(b"pending"),
        7,
        "reserve-pending",
    )
    .await
    .unwrap();
    support::upload(&svc, &a, "detached", "exact-alias", b"old bytes").await;
    svc.detach(&a, "detached").await.unwrap();
    rt.execute(
        &a,
        Command::create(
            "note",
            support::Note {
                text: "protected-other-value".into(),
            },
        )
        .idempotency("note"),
    )
    .await
    .unwrap();
    svc.shutdown().await.unwrap();
    let archive = root.join("source.rombk");
    backup(
        storage.as_ref(),
        &archive,
        rom_extras_maintenance::Limits::default(),
    )
    .unwrap();
    rt.shutdown().await.unwrap();
    drop(svc);
    drop(rt);
    drop(storage);
    let writes = objects.writes.load(Ordering::Relaxed);
    objects.gets.store(0, Ordering::Relaxed);
    let inventory = Inspection::prepare(
        backend,
        &archive,
        &root.join("inspection.db"),
        vec![("exact-alias".into(), objects.clone())],
        Limits::default(),
    )
    .unwrap()
    .collect(context())
    .await
    .unwrap();
    assert_eq!(inventory.counts().ready, 2);
    assert_eq!(inventory.counts().pending, 1);
    assert_eq!(inventory.counts().detached, 1);
    assert_eq!(inventory.counts().other_rows, 1);
    assert_eq!(inventory.bindings()[0].resource_key().kind, Blob::KIND);
    assert_ne!(
        inventory.bindings()[0].entry().key(),
        inventory.bindings()[1].entry().key()
    );
    assert_eq!(
        inventory.bindings()[0].entry().digest(),
        inventory.bindings()[1].entry().digest()
    );
    assert_eq!(objects.gets.load(Ordering::Relaxed), 2);
    assert_eq!(objects.writes.load(Ordering::Relaxed), writes);
    assert_eq!(objects.deletes.load(Ordering::Relaxed), 0);
    let manifest = inventory.for_store("exact-alias").unwrap();
    assert_eq!(manifest.entries().len(), 2);
    assert_eq!(
        inventory.checkpoint(),
        &Digest::of(&std::fs::read(&archive).unwrap())
    );
    let file = root.join("inventory.json");
    inventory.publish(&file).unwrap();
    assert!(inventory.publish(&file).is_err());
    let decoded = Inventory::read(&file, Limits::default()).unwrap();
    assert_eq!(decoded.checkpoint(), inventory.checkpoint());
    assert_eq!(decoded.bindings().len(), 2);
    assert!(root.join("inspection.db").exists());
    let diagnostic = format!("{inventory:?}");
    for secret in [
        "Exact/A:ß",
        "exact/a:ß",
        "protected-other-value",
        "exact-alias",
    ] {
        assert!(!diagnostic.contains(secret));
    }
}

#[tokio::test]
async fn output_json_limit_is_rejected_before_inspection_or_provider_reads() {
    let f = support::checkpoint().await;
    let limit = Limits {
        max_json_bytes: 32,
        ..Limits::default()
    };
    let result = Inspection::prepare(
        f.backend,
        &f.archive,
        &f.root.join("inspection.db"),
        vec![("objects".into(), f.objects.clone())],
        limit,
    );
    assert_eq!(result.err(), Some(rom_blob_checkpoint::Error::TooLarge));
    assert_eq!(f.objects.gets.load(Ordering::Relaxed), 0);
    assert!(!f.root.join("inspection.db").exists());
}

#[tokio::test]
async fn missing_denied_corrupt_and_timed_out_sources_return_no_inventory_or_writes() {
    use rom_blob_checkpoint::Error;
    for (mode, expected) in [
        (1, Error::Missing),
        (2, Error::Denied),
        (4, Error::Integrity),
        (3, Error::Timeout),
    ] {
        let f = support::checkpoint().await;
        let writes = f.objects.writes.load(Ordering::Relaxed);
        let inspected = f.root.join("inspection.db");
        let inspection = Inspection::prepare(
            f.backend,
            &f.archive,
            &inspected,
            vec![("objects".into(), f.objects.clone())],
            Limits::default(),
        )
        .unwrap();
        f.objects.mode.store(mode, Ordering::Relaxed);
        let ctx = Context::new(
            tokio::time::Instant::now() + Duration::from_millis(100),
            CancellationToken::new(),
        );
        let failure = inspection.collect(ctx).await.err().unwrap();
        assert_eq!(failure.cause, expected);
        assert_eq!(failure.verified, 0);
        assert!(failure.cleanup.is_none());
        assert!(!format!("{failure:?} {failure}").contains("checkpoint.secret/ß"));
        assert!(inspected.exists());
        assert_eq!(f.objects.gets.load(Ordering::Relaxed), 1);
        assert_eq!(f.objects.active.load(Ordering::Relaxed), 0);
        assert_eq!(f.objects.writes.load(Ordering::Relaxed), writes);
        assert_eq!(f.objects.deletes.load(Ordering::Relaxed), 0);
    }
}

#[tokio::test]
async fn canceled_before_admission_performs_no_provider_read() {
    let f = support::checkpoint().await;
    let inspection = Inspection::prepare(
        f.backend,
        &f.archive,
        &f.root.join("inspection.db"),
        vec![("objects".into(), f.objects.clone())],
        Limits::default(),
    )
    .unwrap();
    let cancel = CancellationToken::new();
    cancel.cancel();
    let failure = inspection
        .collect(Context::new(
            tokio::time::Instant::now() + Duration::from_secs(1),
            cancel,
        ))
        .await
        .err()
        .unwrap();
    assert_eq!(failure.cause, rom_blob_checkpoint::Error::Canceled);
    assert_eq!(f.objects.gets.load(Ordering::Relaxed), 0);
}

#[tokio::test]
async fn cancellation_during_accepted_read_drains_isolated_observation() {
    let f = support::checkpoint().await;
    let inspection = Inspection::prepare(
        f.backend,
        &f.archive,
        &f.root.join("inspection.db"),
        vec![("objects".into(), f.objects.clone())],
        Limits::default(),
    )
    .unwrap();
    f.objects.mode.store(3, Ordering::Relaxed);
    let cancel = CancellationToken::new();
    let signal = cancel.clone();
    let objects = f.objects.clone();
    let watcher = tokio::spawn(async move {
        tokio::time::timeout(Duration::from_secs(1), async {
            while objects.gets.load(Ordering::Relaxed) == 0 {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        })
        .await
        .unwrap();
        signal.cancel();
    });
    let failure = inspection
        .collect(Context::new(
            tokio::time::Instant::now() + Duration::from_secs(2),
            cancel,
        ))
        .await
        .err()
        .unwrap();
    watcher.await.unwrap();
    assert_eq!(failure.cause, rom_blob_checkpoint::Error::Canceled);
    assert_eq!(f.objects.gets.load(Ordering::Relaxed), 1);
    assert_eq!(f.objects.active.load(Ordering::Relaxed), 0);
    assert!(failure.cleanup.is_none());
}

#[tokio::test]
async fn dropping_whole_collector_has_no_completion_proof_and_retains_inspection() {
    let f = support::checkpoint().await;
    let path = f.root.join("inspection.db");
    let inspection = Inspection::prepare(
        f.backend,
        &f.archive,
        &path,
        vec![("objects".into(), f.objects.clone())],
        Limits::default(),
    )
    .unwrap();
    f.objects.mode.store(3, Ordering::Relaxed);
    let mut future = Box::pin(inspection.collect(Context::new(
        tokio::time::Instant::now() + Duration::from_millis(200),
        CancellationToken::new(),
    )));
    tokio::time::timeout(Duration::from_secs(1),async{loop{tokio::select!{r=&mut future=>panic!("Unexpected completion before accepted-read observation: {r:?}"),_ = tokio::time::sleep(Duration::from_millis(1))=>{if f.objects.gets.load(Ordering::Relaxed)>0{break}}}}}).await.unwrap();
    drop(future);
    assert_eq!(f.objects.active.load(Ordering::Relaxed), 1);
    tokio::time::timeout(Duration::from_secs(1), async {
        while f.objects.active.load(Ordering::Relaxed) != 0 {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
    assert!(path.exists());
    assert_eq!(f.objects.gets.load(Ordering::Relaxed), 1);
    assert!(!f.root.join("inventory.json").exists());
}

#[tokio::test]
async fn missing_duplicate_alias_and_object_bounds_fail_before_provider_io() {
    use rom_blob_checkpoint::Error;
    let f = support::checkpoint().await;
    for (index, limits) in [
        Limits {
            max_ready: 0,
            ..Limits::default()
        },
        Limits {
            max_object_bytes: 1,
            ..Limits::default()
        },
        Limits {
            max_total_bytes: 1,
            ..Limits::default()
        },
    ]
    .into_iter()
    .enumerate()
    {
        let path = f.root.join(format!("inspection-{index}.db"));
        assert_eq!(
            Inspection::prepare(
                f.backend,
                &f.archive,
                &path,
                vec![("objects".into(), f.objects.clone())],
                limits
            )
            .err(),
            Some(Error::TooLarge)
        );
        assert!(!path.exists());
    }
    assert_eq!(
        Inspection::prepare(
            f.backend,
            &f.archive,
            &f.root.join("wrong-alias.db"),
            vec![("other".into(), f.objects.clone())],
            Limits::default()
        )
        .err(),
        Some(Error::Invalid)
    );
    assert_eq!(
        Inspection::prepare(
            f.backend,
            &f.archive,
            &f.root.join("duplicate.db"),
            vec![
                ("objects".into(), f.objects.clone()),
                ("objects".into(), f.objects.clone())
            ],
            Limits::default()
        )
        .err(),
        Some(Error::Invalid)
    );
    assert_eq!(f.objects.gets.load(Ordering::Relaxed), 0);
}
