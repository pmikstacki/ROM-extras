use crate::{capture::Capture, ctx, fixtures, unique};
use rom::{Actor, Resource, Runtime, Storage};
use rom_blob::{Blob, BlobService, BlobState, Digest, UploadOutcome};
use rom_blob_recovery::{Entry, Limits, Manifest, copy};
use rom_extras_maintenance::{Backend, BackupSource, backup, restore};
use std::{collections::BTreeMap, path::Path, sync::Arc};
fn runtime(s: Arc<dyn Storage>) -> Runtime {
    Runtime::builder()
        .resource(rom_blob::definition())
        .build(s, Runtime::shared_cpu_pool(2).unwrap())
        .unwrap()
}
pub async fn run(root: &Path) {
    qualify(
        root,
        Backend::Sqlite,
        Arc::new(rom_sqlite::Sqlite::open(root.join("sqlite-source.db")).unwrap()),
    )
    .await;
    qualify(
        root,
        Backend::Redb,
        Arc::new(rom_redb::Redb::open(root.join("redb-source.db")).unwrap()),
    )
    .await;
}
async fn qualify<S: Storage + BackupSource>(root: &Path, backend: Backend, storage: Arc<S>) {
    let label = if backend == Backend::Sqlite {
        "sqlite"
    } else {
        "redb"
    };
    let id = format!("{label}.Exact/ID:ß");
    let bytes = unique(label);
    let actor = Actor::trusted("recovery-fixture", "owner");
    let denied = Actor::trusted("recovery-fixture", "other");
    let source = Arc::new(Capture::new(fixtures::emulator(1024)));
    let rt = runtime(storage.clone());
    let svc = BlobService::builder(rt.clone())
        .store("objects", source.clone())
        .build()
        .unwrap();
    svc.reserve(
        &actor,
        &id,
        "objects",
        Digest::of(&bytes),
        bytes.len() as u64,
        "reserve-exact",
    )
    .await
    .unwrap();
    let input = Box::pin(futures_util::stream::iter([Ok(bytes.clone())]));
    assert!(matches!(
        svc.upload(&actor, &id, input).await.unwrap(),
        UploadOutcome::Attached(_)
    ));
    let original = rt.read::<Blob>(&actor, &id).await.unwrap();
    assert_eq!(original.revision, 2);
    svc.shutdown().await.unwrap();
    drop(svc);
    let archive = root.join(format!("{label}-archive.rombk"));
    backup(
        storage.as_ref(),
        &archive,
        rom_extras_maintenance::Limits::default(),
    )
    .unwrap();
    rt.shutdown().await.unwrap();
    drop(rt);
    drop(storage);
    // This controlled host has one approved store and only Blob resources.
    // Derive inventory from the coherent archive, not live Resource JSON.
    let (_, snapshot) =
        rom_backup::read(&archive, backend, rom_backup::BackupLimits::default()).unwrap();
    let mut objects = BTreeMap::<String, Entry>::new();
    for row in snapshot.rows {
        assert_eq!(row.key.kind, Blob::KIND);
        assert_eq!(row.key.id, id);
        let blob = Blob::decode(row.value.unwrap()).unwrap();
        assert_eq!(blob.store, "objects");
        assert_eq!(blob.state, BlobState::Ready);
        let receipts = source.receipts.lock().unwrap();
        assert_eq!(receipts.len(), 1);
        let receipt = &receipts[0];
        assert_eq!(receipt.digest, blob.digest);
        assert_eq!(receipt.bytes, blob.bytes);
        assert_ne!(receipt.key.as_str(), blob.digest.as_str());
        assert!(
            objects
                .insert(
                    receipt.key.as_str().into(),
                    Entry::new(receipt.key.clone(), blob.digest, blob.bytes)
                )
                .is_none()
        );
    }
    let entries = objects.into_values().collect();
    let checkpoint = Digest::of(&std::fs::read(&archive).unwrap());
    let manifest = Manifest::new(checkpoint.clone(), entries, Limits::default()).unwrap();
    let encoded = manifest.encode().unwrap();
    std::fs::write(root.join(format!("{label}-objects.json")), &encoded).unwrap();
    let manifest = Manifest::decode(&encoded, Limits::default()).unwrap();
    assert_eq!(manifest.checkpoint(), &checkpoint);
    let destination = Arc::new(fixtures::adapter(1024, false));
    assert_eq!(
        copy(source.as_ref(), destination.as_ref(), &manifest, &ctx())
            .await
            .unwrap()
            .created,
        1
    );
    let restored_path = root.join(format!("{label}-restored.db"));
    let restored = restore(
        backend,
        &archive,
        &restored_path,
        rom_extras_maintenance::Limits::default(),
    )
    .unwrap();
    let restored_rt = runtime(restored.clone());
    let svc = BlobService::builder(restored_rt.clone())
        .store("objects", destination.clone())
        .build()
        .unwrap();
    let actual = restored_rt.read::<Blob>(&actor, &id).await.unwrap();
    assert_eq!(actual.revision, original.revision);
    let blob = actual.value.unwrap();
    let old = original.value.unwrap();
    assert_eq!(blob.digest, old.digest);
    assert_eq!(blob.store, old.store);
    assert_eq!(blob.upload_revision, old.upload_revision);
    assert_eq!(svc.read(&actor, &id).await.unwrap(), bytes);
    assert_eq!(
        svc.read(&denied, &id).await,
        Err(rom_blob::Error::Core(rom::Error::Denied))
    );
    svc.shutdown().await.unwrap();
    drop(svc);
    restored_rt.shutdown().await.unwrap();
    drop(restored_rt);
    drop(restored);
    let reopened: Arc<dyn Storage> = if backend == Backend::Sqlite {
        Arc::new(rom_sqlite::Sqlite::open(&restored_path).unwrap())
    } else {
        Arc::new(rom_redb::Redb::open(&restored_path).unwrap())
    };
    let reopened_rt = runtime(reopened);
    let svc = BlobService::builder(reopened_rt.clone())
        .store("objects", destination)
        .build()
        .unwrap();
    assert_eq!(svc.read(&actor, &id).await.unwrap(), bytes);
    svc.shutdown().await.unwrap();
    reopened_rt.shutdown().await.unwrap();
    println!(
        "{label}: actual BlobService upload, coherent archive inventory, external copy, restore, exact ID/revision, authorized read, denied caller and native reopen passed"
    );
}
