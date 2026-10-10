// Host observes exact publication identities through the public BlobStore port.
use rom_blob::{BlobStore, Digest, Metadata, ObjectKey, ObjectReceipt, StoreFuture};
use std::sync::Mutex;
pub(crate) struct Capture<B> {
    pub inner: B,
    pub receipts: Mutex<Vec<ObjectReceipt>>,
}
impl<B> Capture<B> {
    pub fn new(inner: B) -> Self {
        Self {
            inner,
            receipts: Mutex::new(Vec::new()),
        }
    }
}
impl<B: BlobStore> BlobStore for Capture<B> {
    fn create<'a>(&'a self, k: &'a ObjectKey, b: Vec<u8>) -> StoreFuture<'a, ()> {
        Box::pin(async move {
            let receipt = ObjectReceipt {
                key: k.clone(),
                digest: Digest::of(&b),
                bytes: b.len() as u64,
            };
            self.inner.create(k, b).await?;
            self.receipts.lock().unwrap().push(receipt);
            Ok(())
        })
    }
    fn get<'a>(&'a self, k: &'a ObjectKey, max: usize) -> StoreFuture<'a, Vec<u8>> {
        self.inner.get(k, max)
    }
    fn head<'a>(&'a self, k: &'a ObjectKey) -> StoreFuture<'a, Metadata> {
        self.inner.head(k)
    }
    fn delete<'a>(&'a self, k: &'a ObjectKey) -> StoreFuture<'a, ()> {
        self.inner.delete(k)
    }
}
