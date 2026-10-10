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
    pub(crate) fn check(&self) -> Result<(), Cause> {
        if self.cancel.is_cancelled() {
            Err(Cause::Canceled)
        } else if Instant::now() >= self.deadline {
            Err(Cause::Timeout)
        } else {
            Ok(())
        }
    }
    pub(crate) async fn wait<T, F>(&self, future: F) -> Result<T, Cause>
    where
        F: Future<Output = rom_blob::Result<T>>,
    {
        tokio::select! {biased;
         _=self.cancel.cancelled()=>Err(Cause::Canceled),
         _=tokio::time::sleep_until(self.deadline)=>Err(Cause::Timeout),
         r=future=>r.map_err(Cause::from),
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
