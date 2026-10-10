use crate::Cause;
use std::future::Future;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;
/// One absolute deadline and cancellation token for a whole invocation.
/// Bounds cooperative caller waiting, not hidden provider work after a future is dropped.
pub struct Context {
    deadline: Instant,
    cancel: CancellationToken,
}
impl Context {
    /// No provider connection or operation occurs at construction.
    pub fn new(deadline: Instant, cancel: CancellationToken) -> Self {
        Self { deadline, cancel }
    }
    /// Check cancellation and the absolute deadline without admitting provider work.
    pub fn check(&self) -> Result<(), Cause> {
        if self.cancel.is_cancelled() {
            Err(Cause::Canceled)
        } else if Instant::now() >= self.deadline {
            Err(Cause::Timeout)
        } else {
            Ok(())
        }
    }
    /// Bound cooperative waiting for a public BlobStore or BlobService operation.
    /// Dropping its future does not establish termination of accepted hidden work.
    pub async fn wait<T, F>(&self, future: F) -> Result<T, Cause>
    where
        F: Future<Output = rom_blob::Result<T>>,
    {
        self.wait_result(future).await?.map_err(Cause::from)
    }
    /// Preserve a public provider/service result separately from caller timeout or cancellation.
    /// Accepted supervised operations can continue after this waiter returns.
    pub async fn wait_result<T, F>(&self, future: F) -> Result<rom_blob::Result<T>, Cause>
    where
        F: Future<Output = rom_blob::Result<T>>,
    {
        tokio::select! {biased;
         _=self.cancel.cancelled()=>Err(Cause::Canceled),
         _=tokio::time::sleep_until(self.deadline)=>Err(Cause::Timeout),
         r=future=>Ok(r),
        }
    }
    pub(crate) async fn create<F>(&self, future: F) -> Result<(), rom_blob::Error>
    where
        F: Future<Output = rom_blob::Result<()>>,
    {
        tokio::select! {biased;
         _=self.cancel.cancelled()=>Err(rom_blob::Error::Unknown),
         _=tokio::time::sleep_until(self.deadline)=>Err(rom_blob::Error::Unknown),
         r=future=>r,
        }
    }
}
