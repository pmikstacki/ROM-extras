use rom::{Actor, Resource, Runtime, Storage};
use rom_blob::{BlobService, BlobStore, Metadata, ObjectKey, StoreFuture};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};
#[derive(Default)]
pub struct Memory {
    pub values: Mutex<BTreeMap<String, Vec<u8>>>,
    pub gets: AtomicUsize,
    pub writes: AtomicUsize,
    pub deletes: AtomicUsize,
    pub mode: AtomicUsize,
    pub active: AtomicUsize,
}
impl BlobStore for Memory {
    fn create<'a>(&'a self, key: &'a ObjectKey, bytes: Vec<u8>) -> StoreFuture<'a, ()> {
        Box::pin(async move {
            self.writes.fetch_add(1, Ordering::Relaxed);
            let mut values = self.values.lock().unwrap();
            if values.contains_key(key.as_str()) {
                return Err(rom_blob::Error::Conflict);
            }
            values.insert(key.as_str().into(), bytes);
            Ok(())
        })
    }
    fn get<'a>(&'a self, key: &'a ObjectKey, max: usize) -> StoreFuture<'a, Vec<u8>> {
        Box::pin(async move {
            self.gets.fetch_add(1, Ordering::Relaxed);
            self.active.fetch_add(1, Ordering::Relaxed);
            struct Active<'a>(&'a AtomicUsize);
            impl Drop for Active<'_> {
                fn drop(&mut self) {
                    self.0.fetch_sub(1, Ordering::Relaxed);
                }
            }
            let _active = Active(&self.active);
            match self.mode.load(Ordering::Relaxed) {
                1 => return Err(rom_blob::Error::Missing),
                2 => return Err(rom_blob::Error::Denied),
                3 => std::future::pending::<()>().await,
                _ => {}
            }
            let mut b = self
                .values
                .lock()
                .unwrap()
                .get(key.as_str())
                .cloned()
                .ok_or(rom_blob::Error::Missing)?;
            if self.mode.load(Ordering::Relaxed) == 4 && !b.is_empty() {
                b[0] ^= 1;
            }
            if b.len() > max {
                return Err(rom_blob::Error::TooLarge);
            }
            Ok(b)
        })
    }
    fn head<'a>(&'a self, _: &'a ObjectKey) -> StoreFuture<'a, Metadata> {
        Box::pin(async { Err(rom_blob::Error::Unsupported) })
    }
    fn delete<'a>(&'a self, _: &'a ObjectKey) -> StoreFuture<'a, ()> {
        Box::pin(async move {
            self.deletes.fetch_add(1, Ordering::Relaxed);
            Err(rom_blob::Error::Unsupported)
        })
    }
}
#[derive(Clone, Resource)]
#[resource(name = "checkpoint_notes")]
pub struct Note {
    pub text: String,
}
pub fn path() -> PathBuf {
    static SEQ: AtomicUsize = AtomicUsize::new(0);
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../.superpowers/blob-checkpoint-unit-runs");
    std::fs::create_dir_all(&root).unwrap();
    let p = root.join(format!(
        "run-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&p).unwrap();
    p
}
pub fn runtime(storage: Arc<dyn Storage>) -> Runtime {
    Runtime::builder()
        .resource(rom_blob::definition())
        .resource(
            Note::definition()
                .policy(|_, _, _| true)
                .field_policy(|_, _, _, _| true),
        )
        .build(storage, Runtime::shared_cpu_pool(2).unwrap())
        .unwrap()
}
pub async fn upload(service: &BlobService, actor: &Actor, id: &str, store: &str, bytes: &[u8]) {
    service
        .reserve(
            actor,
            id,
            store,
            rom_blob::Digest::of(bytes),
            bytes.len() as u64,
            &format!("reserve-{id}"),
        )
        .await
        .unwrap();
    let data = bytes.to_vec();
    let input = Box::pin(futures_util::stream::iter([Ok(data)]));
    service.upload(actor, id, input).await.unwrap();
}
pub struct Fixture {
    pub root: PathBuf,
    pub archive: PathBuf,
    pub backend: rom_extras_maintenance::Backend,
    pub objects: Arc<Memory>,
}
pub async fn checkpoint() -> Fixture {
    let root = path();
    #[cfg(feature = "sqlite")]
    let (backend, storage) = (
        rom_extras_maintenance::Backend::Sqlite,
        Arc::new(rom_sqlite::Sqlite::open(root.join("source.db")).unwrap()),
    );
    #[cfg(all(not(feature = "sqlite"), feature = "redb"))]
    let (backend, storage) = (
        rom_extras_maintenance::Backend::Redb,
        Arc::new(rom_redb::Redb::open(root.join("source.db")).unwrap()),
    );
    let rt = runtime(storage.clone());
    let objects = Arc::new(Memory::default());
    let svc = BlobService::builder(rt.clone())
        .store("objects", objects.clone())
        .build()
        .unwrap();
    upload(
        &svc,
        &Actor::trusted("fixture", "owner"),
        "checkpoint.secret/ß",
        "objects",
        b"checkpoint payload",
    )
    .await;
    svc.shutdown().await.unwrap();
    let archive = root.join("source.rombk");
    rom_extras_maintenance::backup(
        storage.as_ref(),
        &archive,
        rom_extras_maintenance::Limits::default(),
    )
    .unwrap();
    rt.shutdown().await.unwrap();
    drop(svc);
    drop(rt);
    drop(storage);
    objects.gets.store(0, Ordering::Relaxed);
    Fixture {
        root,
        archive,
        backend,
        objects,
    }
}
