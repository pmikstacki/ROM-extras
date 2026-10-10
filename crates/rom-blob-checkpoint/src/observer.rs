use crate::Error;
use rom_blob::{Blob, BlobStore, Digest, Metadata, ObjectKey, StoreFuture};
use rom_blob_recovery::{Cause, Context};
use std::sync::{Arc, Mutex};
pub(crate) struct Expected {
    digest: Digest,
    bytes: u64,
    key: Option<ObjectKey>,
}
pub(crate) struct Observer {
    inner: Arc<dyn BlobStore>,
    context: Arc<Context>,
    pending: Mutex<Option<Expected>>,
}
impl Observer {
    pub(crate) fn new(inner: Arc<dyn BlobStore>, context: Arc<Context>) -> Self {
        Self {
            inner,
            context,
            pending: Mutex::new(None),
        }
    }
    pub(crate) fn begin(&self, blob: &Blob) -> Result<(), Error> {
        let mut pending = self.pending.lock().map_err(|_| Error::Unavailable)?;
        if pending.is_some() {
            return Err(Error::Conflict);
        }
        *pending = Some(Expected {
            digest: blob.digest.clone(),
            bytes: blob.bytes,
            key: None,
        });
        Ok(())
    }
    pub(crate) fn finish(&self) -> Result<ObjectKey, Error> {
        self.pending
            .lock()
            .map_err(|_| Error::Unavailable)?
            .take()
            .and_then(|p| p.key)
            .ok_or(Error::Integrity)
    }
}
impl BlobStore for Observer {
    fn create<'a>(&'a self, _: &'a ObjectKey, _: Vec<u8>) -> StoreFuture<'a, ()> {
        Box::pin(async { Err(rom_blob::Error::Unsupported) })
    }
    fn head<'a>(&'a self, _: &'a ObjectKey) -> StoreFuture<'a, Metadata> {
        Box::pin(async { Err(rom_blob::Error::Unsupported) })
    }
    fn delete<'a>(&'a self, _: &'a ObjectKey) -> StoreFuture<'a, ()> {
        Box::pin(async { Err(rom_blob::Error::Unsupported) })
    }
    fn get<'a>(&'a self, key: &'a ObjectKey, max: usize) -> StoreFuture<'a, Vec<u8>> {
        Box::pin(async move {
            let (digest, bytes) = {
                let mut slot = self.pending.lock().map_err(|_| rom_blob::Error::Backend)?;
                let p = slot.as_mut().ok_or(rom_blob::Error::Invalid)?;
                if p.key.is_some() {
                    return Err(rom_blob::Error::Invalid);
                }
                p.key = Some(key.clone());
                (p.digest.clone(), p.bytes)
            };
            self.context.check().map_err(wait_error)?;
            let bound = max.min(usize::try_from(bytes).map_err(|_| rom_blob::Error::TooLarge)?);
            let b = self
                .context
                .wait_result(self.inner.get(key, bound))
                .await
                .map_err(wait_error)??;
            self.context.check().map_err(wait_error)?;
            if b.len() > bound {
                return Err(rom_blob::Error::TooLarge);
            }
            if b.len() as u64 != bytes || Digest::of(&b) != digest {
                return Err(rom_blob::Error::Conflict);
            }
            Ok(b)
        })
    }
}
fn wait_error(c: Cause) -> rom_blob::Error {
    match c {
        Cause::Timeout => rom_blob::Error::Timeout,
        Cause::Canceled => rom_blob::Error::Closed,
        _ => rom_blob::Error::Backend,
    }
}
