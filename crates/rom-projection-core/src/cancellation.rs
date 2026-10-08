//! Monotonic shared cancellation request for bounded startup work.
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
/// Cooperative cancellation. Clones share one irreversible request.
#[derive(Clone, Default)]
pub struct Cancellation {
    requested: Arc<AtomicBool>,
}
impl Cancellation {
    /// Create an unrequested token.
    pub fn new() -> Self {
        Self::default()
    }
    /// Request cancellation from any thread. This does not interrupt native I/O.
    pub fn cancel(&self) {
        self.requested.store(true, Ordering::Release);
    }
    /// Inspect the shared cancellation request.
    pub fn is_cancelled(&self) -> bool {
        self.requested.load(Ordering::Acquire)
    }
    pub(crate) fn check(&self) -> crate::Result<()> {
        if self.is_cancelled() {
            Err(crate::Error::Cancelled)
        } else {
            Ok(())
        }
    }
}
impl std::fmt::Debug for Cancellation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Cancellation")
    }
}
