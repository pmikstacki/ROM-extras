use crate::{context, fixtures, tracked::Tracked};
use rom::{Actor, Command, Resource, Runtime, Storage};
use rom_blob::{Blob, BlobService, BlobStore, Digest};
use rom_blob_checkpoint::{Inspection, Inventory, Limits};
use rom_blob_recovery::{copy, verify};
use rom_extras_maintenance::{Backend, BackupSource, backup, restore};
use std::{
    collections::BTreeMap,
    fs::File,
    path::Path,
    process::{Command as Process, Stdio},
    sync::{Arc, atomic::Ordering},
};
const AZURE: &str = "azure-source";
const S3: &str = "s3-source";
#[derive(Clone, Resource)]
#[resource(name = "checkpoint_notes")]
struct Note {
    value: String,
}
fn runtime(storage: Arc<dyn Storage>, notes: bool) -> Runtime {
    let mut b = Runtime::builder().resource(rom_blob::definition());
    if notes {
        b = b.resource(
            Note::definition()
                .policy(|_, _, _| true)
                .field_policy(|_, _, _, _| true),
        );
    }
    b.build(storage, Runtime::shared_cpu_pool(2).unwrap())
        .unwrap()
}
fn sources() -> BTreeMap<String, Arc<dyn BlobStore>> {
    BTreeMap::from([
        (
            AZURE.into(),
            Arc::new(fixtures::emulator(1024)) as Arc<dyn BlobStore>,
        ),
        (
            S3.into(),
            Arc::new(fixtures::adapter(1024, false)) as Arc<dyn BlobStore>,
        ),
    ])
}
fn destinations() -> BTreeMap<String, Arc<dyn BlobStore>> {
    BTreeMap::from([
        (
            AZURE.into(),
            Arc::new(fixtures::adapter(1024, false)) as Arc<dyn BlobStore>,
        ),
        (
            S3.into(),
            Arc::new(fixtures::emulator(1024)) as Arc<dyn BlobStore>,
        ),
    ])
}
fn service(rt: &Runtime, stores: &BTreeMap<String, Arc<dyn BlobStore>>) -> BlobService {
    let mut b = BlobService::builder(rt.clone());
    for (alias, store) in stores {
        b = b.store(alias, store.clone());
    }
    b.build().unwrap()
}
async fn upload(svc: &BlobService, actor: &Actor, id: &str, store: &str, bytes: &[u8]) {
    svc.reserve(
        actor,
        id,
        store,
        Digest::of(bytes),
        bytes.len() as u64,
        &format!("reserve-{id}"),
    )
    .await
    .unwrap();
    svc.upload(
        actor,
        id,
        Box::pin(futures_util::stream::iter([Ok(bytes.to_vec())])),
    )
    .await
    .unwrap();
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
    let stores: BTreeMap<_, _> = sources()
        .into_iter()
        .map(|(alias, store)| (alias, Arc::new(Tracked::new(store))))
        .collect();
    let exposed: BTreeMap<_, _> = stores
        .iter()
        .map(|(alias, store)| (alias.clone(), store.clone() as Arc<dyn BlobStore>))
        .collect();
    let rt = runtime(storage.clone(), true);
    let svc = service(&rt, &exposed);
    let a = Actor::trusted("checkpoint-consumer", "one");
    let b = Actor::trusted("checkpoint-consumer", "two");
    let prefix = format!(
        "{label}:{}.Exact/ß",
        root.file_name().unwrap().to_str().unwrap()
    );
    let mut expected = BTreeMap::new();
    for n in 0..5 {
        let id = format!("{prefix}:{n}");
        let actor = if n % 2 == 0 { &a } else { &b };
        let alias = if n % 2 == 0 { AZURE } else { S3 };
        let bytes = if n == 4 {
            b"".as_slice()
        } else {
            b"equal native checkpoint".as_slice()
        };
        upload(&svc, actor, &id, alias, bytes).await;
        let row = rt.read::<Blob>(actor, &id).await.unwrap();
        expected.insert(id, (row.revision, row.value.unwrap()));
    }
    svc.reserve(
        &a,
        &format!("{prefix}:pending"),
        AZURE,
        Digest::of(b"pending"),
        7,
        "pending",
    )
    .await
    .unwrap();
    for state in ["detached", "tombstone"] {
        let id = format!("{prefix}:{state}");
        upload(&svc, &a, &id, AZURE, b"excluded bytes").await;
        svc.detach(&a, &id).await.unwrap();
        if state == "tombstone" {
            rt.execute(
                &rom_blob::worker_actor(),
                Command::<Blob>::delete(&id)
                    .at_revision(3)
                    .idempotency("tombstone"),
            )
            .await
            .unwrap();
        }
    }
    rt.execute(
        &a,
        Command::create(
            "unrelated-note",
            Note {
                value: "private-other-resource".into(),
            },
        )
        .idempotency("note"),
    )
    .await
    .unwrap();
    svc.shutdown().await.unwrap();
    let archive = root.join(format!("{label}-archive.rombk"));
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
    let before: BTreeMap<_, _> = stores
        .iter()
        .map(|(alias, s)| {
            (
                alias.clone(),
                (
                    s.gets.load(Ordering::Relaxed),
                    s.writes.load(Ordering::Relaxed),
                ),
            )
        })
        .collect();
    let inspection = root.join(format!("{label}-inspection.db"));
    let inventory = Inspection::prepare(
        backend,
        &archive,
        &inspection,
        exposed.into_iter().collect(),
        Limits::default(),
    )
    .unwrap()
    .collect(context())
    .await
    .unwrap();
    assert_eq!(inventory.counts().ready, 5);
    assert_eq!(inventory.counts().pending, 1);
    assert_eq!(inventory.counts().detached, 1);
    assert_eq!(inventory.counts().tombstones, 1);
    assert_eq!(inventory.counts().other_rows, 1);
    for binding in inventory.bindings() {
        let (revision, blob) = &expected[&binding.resource_key().id];
        assert_eq!(binding.revision(), *revision);
        assert_eq!(binding.upload_revision(), blob.upload_revision);
        assert_eq!(binding.owner(), blob.owner);
        assert_eq!(binding.store(), blob.store);
        assert_eq!(binding.entry().digest(), &blob.digest);
        assert_eq!(binding.entry().bytes(), blob.bytes);
    }
    for (alias, s) in &stores {
        assert_eq!(s.writes.load(Ordering::Relaxed), before[alias].1);
        assert_eq!(s.deletes.load(Ordering::Relaxed), 0);
        assert_eq!(
            s.gets.load(Ordering::Relaxed) - before[alias].0,
            if alias == AZURE { 3 } else { 2 }
        );
    }
    let file = root.join(format!("{label}-inventory.json"));
    inventory.publish(&file).unwrap();
    assert_eq!(
        inventory.publish(&file),
        Err(rom_blob_checkpoint::Error::Conflict)
    );
    assert!(
        !rom_backup::read(&archive, backend, rom_backup::BackupLimits::default())
            .unwrap()
            .0
            .external_blobs_included
    );
    assert!(inspection.exists());
    for phase in ["--restore", "--reopen"] {
        let log = File::create(root.join(format!("{label}{phase}.log"))).unwrap();
        let result = Process::new(std::env::current_exe().unwrap())
            .arg(phase)
            .arg(label)
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log))
            .status()
            .unwrap();
        assert!(
            result.success(),
            "Fresh public consumer process failed; retained private child log"
        );
    }
    println!(
        "{label}: actual mixed archive, five Ready bindings, equal-content owners, two native stores, exclusions, private durable publication and fresh-process restore/reopen passed"
    );
}
pub async fn child(root: &Path, phase: &str, label: &str) {
    let backend = match label {
        "sqlite" => Backend::Sqlite,
        "redb" => Backend::Redb,
        _ => panic!("invalid controlled backend"),
    };
    let archive = root.join(format!("{label}-archive.rombk"));
    let inventory = Inventory::read(
        &root.join(format!("{label}-inventory.json")),
        Limits::default(),
    )
    .unwrap();
    assert_eq!(
        inventory.checkpoint(),
        &Digest::of(&std::fs::read(&archive).unwrap())
    );
    let target = destinations();
    if phase == "--restore" {
        let source = sources();
        let mut created = 0;
        for (alias, destination) in &target {
            let manifest = inventory.for_store(alias).unwrap();
            created += copy(
                source[alias].as_ref(),
                destination.as_ref(),
                &manifest,
                &context(),
            )
            .await
            .unwrap()
            .created;
        }
        assert_eq!(created, 5);
    } else {
        assert_eq!(phase, "--reopen");
        for (alias, destination) in &target {
            assert_eq!(
                verify(
                    destination.as_ref(),
                    &inventory.for_store(alias).unwrap(),
                    &context()
                )
                .await
                .unwrap()
                .verified,
                if alias == AZURE { 3 } else { 2 }
            );
        }
    }
    let path = root.join(format!("{label}-restored.db"));
    let storage: Arc<dyn Storage> = if phase == "--restore" {
        restore(
            backend,
            &archive,
            &path,
            rom_extras_maintenance::Limits::default(),
        )
        .unwrap()
    } else if backend == Backend::Sqlite {
        Arc::new(rom_sqlite::Sqlite::open(&path).unwrap())
    } else {
        Arc::new(rom_redb::Redb::open(&path).unwrap())
    };
    let rt = runtime(storage, false);
    let svc = service(&rt, &target);
    let actor = rom_blob::worker_actor();
    let denied = Actor::trusted("checkpoint-consumer", "denied");
    for binding in inventory.bindings() {
        let id = binding.resource_key().id;
        let row = rt.read::<Blob>(&actor, &id).await.unwrap();
        assert_eq!(row.revision, binding.revision());
        let blob = row.value.unwrap();
        assert_eq!(blob.owner, binding.owner());
        assert_eq!(blob.store, binding.store());
        assert_eq!(blob.upload_revision, binding.upload_revision());
        assert_eq!(blob.digest, *binding.entry().digest());
        assert_eq!(blob.bytes, binding.entry().bytes());
        let bytes = svc.read(&actor, &id).await.unwrap();
        assert_eq!(Digest::of(&bytes), *binding.entry().digest());
        assert_eq!(
            svc.read(&denied, &id).await,
            Err(rom_blob::Error::Core(rom::Error::Denied))
        );
    }
    svc.shutdown().await.unwrap();
    rt.shutdown().await.unwrap();
    println!(
        "fresh-process inventory, exact Resource rows, native content and denied caller verified"
    );
}
