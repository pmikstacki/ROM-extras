//! Observation of real provider operations; not a wire acknowledgement fault.
use rom_blob::{BlobStore, Metadata, ObjectKey, StoreFuture};
use std::sync::{
    Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use tokio::sync::Notify;

pub(super) struct Observed<B> {
    pub inner: B,
    pub creates: AtomicUsize,
    pub reads: AtomicUsize,
    pub pause: AtomicBool,
    pub published: Notify,
    pub release: Notify,
    pub last_key: Mutex<Option<ObjectKey>>,
}
impl<B> Observed<B> {
    pub fn new(inner: B) -> Self {
        Self {
            inner,
            creates: AtomicUsize::new(0),
            reads: AtomicUsize::new(0),
            pause: AtomicBool::new(false),
            published: Notify::new(),
            release: Notify::new(),
            last_key: Mutex::new(None),
        }
    }
}
impl<B: BlobStore> BlobStore for Observed<B> {
    fn create<'a>(&'a self, key: &'a ObjectKey, bytes: Vec<u8>) -> StoreFuture<'a, ()> {
        Box::pin(async move {
            self.creates.fetch_add(1, Ordering::SeqCst);
            self.inner.create(key, bytes).await?;
            *self.last_key.lock().unwrap() = Some(key.clone());
            if self.pause.swap(false, Ordering::SeqCst) {
                self.published.notify_one();
                self.release.notified().await;
            }
            Ok(())
        })
    }
    fn get<'a>(&'a self, key: &'a ObjectKey, max: usize) -> StoreFuture<'a, Vec<u8>> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        self.inner.get(key, max)
    }
    fn head<'a>(&'a self, key: &'a ObjectKey) -> StoreFuture<'a, Metadata> {
        self.inner.head(key)
    }
    fn delete<'a>(&'a self, key: &'a ObjectKey) -> StoreFuture<'a, ()> {
        self.inner.delete(key)
    }
}
