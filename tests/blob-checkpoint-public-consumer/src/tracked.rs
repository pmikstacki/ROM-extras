use rom_blob::{BlobStore, Metadata, ObjectKey, StoreFuture};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
/// Counter-only host decorator. No physical keys or create receipts are retained.
pub struct Tracked {
    pub inner: Arc<dyn BlobStore>,
    pub gets: AtomicUsize,
    pub writes: AtomicUsize,
    pub deletes: AtomicUsize,
}
impl Tracked {
    pub fn new(inner: Arc<dyn BlobStore>) -> Self {
        Self {
            inner,
            gets: AtomicUsize::new(0),
            writes: AtomicUsize::new(0),
            deletes: AtomicUsize::new(0),
        }
    }
}
impl BlobStore for Tracked {
    fn create<'a>(&'a self, key: &'a ObjectKey, b: Vec<u8>) -> StoreFuture<'a, ()> {
        self.writes.fetch_add(1, Ordering::Relaxed);
        self.inner.create(key, b)
    }
    fn get<'a>(&'a self, key: &'a ObjectKey, max: usize) -> StoreFuture<'a, Vec<u8>> {
        self.gets.fetch_add(1, Ordering::Relaxed);
        self.inner.get(key, max)
    }
    fn head<'a>(&'a self, key: &'a ObjectKey) -> StoreFuture<'a, Metadata> {
        self.inner.head(key)
    }
    fn delete<'a>(&'a self, key: &'a ObjectKey) -> StoreFuture<'a, ()> {
        self.deletes.fetch_add(1, Ordering::Relaxed);
        self.inner.delete(key)
    }
}
