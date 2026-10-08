use crate::{AzureConfig, Limits, configuration, errors};
use futures_util::StreamExt;
use object_store::{
    GetOptions, ObjectStore, ObjectStoreExt, PutMode, PutOptions, azure::MicrosoftAzure, path::Path,
};
use rom_blob::{BlobStore, Error, Metadata, ObjectKey, Result, StoreFuture};
use std::future::Future;
use tokio::sync::Semaphore;

/// Azure implementation of ROM's public blob port; construct and use inside Tokio.
///
/// A cancelled or timed-out write may have published an object. The adapter does
/// not retry or maintain its own attachment ledger. Reuse ROM's BlobService.
pub struct AzureBlob {
    inner: MicrosoftAzure,
    limits: Limits,
    admission: Semaphore,
}
impl AzureBlob {
    /// Validate explicit host configuration and construct a client without network requests.
    pub fn connect(config: AzureConfig<'_>, limits: Limits) -> Result<Self> {
        Ok(Self {
            inner: configuration::build(config, limits)?,
            admission: Semaphore::new(limits.max_in_flight),
            limits,
        })
    }
    fn run<'a, T: Send + 'a>(
        &'a self,
        operation: impl Future<Output = Result<T>> + Send + 'a,
        timeout: Error,
    ) -> StoreFuture<'a, T> {
        Box::pin(async move {
            let _permit = self
                .admission
                .try_acquire()
                .map_err(|_| Error::Overloaded)?;
            tokio::time::timeout(self.limits.deadline, operation)
                .await
                .unwrap_or(Err(timeout))
        })
    }
}
impl BlobStore for AzureBlob {
    fn create<'a>(&'a self, key: &'a ObjectKey, bytes: Vec<u8>) -> StoreFuture<'a, ()> {
        self.run(
            async move {
                if bytes.len() > self.limits.max_bytes {
                    return Err(Error::TooLarge);
                }
                self.inner
                    .put_opts(
                        &Path::from(key.as_str()),
                        bytes.into(),
                        PutOptions {
                            mode: PutMode::Create,
                            ..Default::default()
                        },
                    )
                    .await
                    .map_err(errors::write)?;
                Ok(())
            },
            Error::Unknown,
        )
    }
    fn get<'a>(&'a self, key: &'a ObjectKey, max_bytes: usize) -> StoreFuture<'a, Vec<u8>> {
        self.run(
            async move {
                let path = Path::from(key.as_str());
                let meta = self.inner.head(&path).await.map_err(errors::read)?;
                let limit = max_bytes.min(self.limits.max_bytes);
                if meta.size > limit as u64 {
                    return Err(Error::TooLarge);
                }
                if meta.size == 0 {
                    return Ok(Vec::new());
                }
                let etag = meta.e_tag.ok_or(Error::Unsupported)?;
                let result = self
                    .inner
                    .get_opts(
                        &path,
                        GetOptions {
                            if_match: Some(etag),
                            ..Default::default()
                        },
                    )
                    .await
                    .map_err(errors::read)?;
                if result.meta.size != meta.size {
                    return Err(Error::Backend);
                }
                let mut stream = result.into_stream();
                let mut bytes = Vec::new();
                let mut chunks = 0usize;
                while let Some(chunk) = stream.next().await {
                    chunks += 1;
                    let chunk = chunk.map_err(errors::read)?;
                    if chunks > 65_536 || chunk.len() > limit - bytes.len() {
                        return Err(Error::TooLarge);
                    }
                    bytes.extend_from_slice(&chunk);
                }
                if bytes.len() as u64 != meta.size {
                    return Err(Error::Backend);
                }
                Ok(bytes)
            },
            Error::Timeout,
        )
    }
    fn head<'a>(&'a self, key: &'a ObjectKey) -> StoreFuture<'a, Metadata> {
        self.run(
            async move {
                let meta = self
                    .inner
                    .head(&Path::from(key.as_str()))
                    .await
                    .map_err(errors::read)?;
                Ok(Metadata { bytes: meta.size })
            },
            Error::Timeout,
        )
    }
    fn delete<'a>(&'a self, key: &'a ObjectKey) -> StoreFuture<'a, ()> {
        self.run(
            async move {
                match self.inner.delete(&Path::from(key.as_str())).await {
                    Ok(()) | Err(object_store::Error::NotFound { .. }) => Ok(()),
                    Err(error) => Err(errors::write(error)),
                }
            },
            Error::Unknown,
        )
    }
}
