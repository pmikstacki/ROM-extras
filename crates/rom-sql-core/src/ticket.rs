use crate::ExecutorError;
use std::{
    cell::Cell,
    sync::mpsc::{Receiver, RecvTimeoutError},
    thread::{self, ThreadId},
    time::Duration,
};

/// One admitted operation's response. Dropping this handle does not cancel work.
///
/// A waiting timeout leaves the ticket usable. A terminal response can be read
/// once; database errors returned by the operation are its own result type.
pub struct Ticket<R> {
    pub(crate) response: Receiver<Result<R, ExecutorError>>,
    pub(crate) worker: ThreadId,
    pub(crate) consumed: Cell<bool>,
}

impl<R> Ticket<R> {
    /// Wait for a terminal response without a deadline.
    ///
    /// Call this from a blocking execution context. It blocks the calling thread.
    pub fn wait(&self) -> Result<R, ExecutorError> {
        self.check_wait()?;
        let result = self.response.recv().unwrap_or(Err(ExecutorError::Closed));
        self.consumed.set(true);
        result
    }

    /// Wait up to `timeout` without cancelling the operation.
    ///
    /// `Unknown` on timeout permits another wait. It never proves rollback.
    pub fn wait_timeout(&self, timeout: Duration) -> Result<R, ExecutorError> {
        self.check_wait()?;
        match self.response.recv_timeout(timeout) {
            Ok(result) => {
                self.consumed.set(true);
                result
            }
            Err(RecvTimeoutError::Timeout) => Err(ExecutorError::Unknown),
            Err(RecvTimeoutError::Disconnected) => {
                self.consumed.set(true);
                Err(ExecutorError::Closed)
            }
        }
    }

    fn check_wait(&self) -> Result<(), ExecutorError> {
        if thread::current().id() == self.worker {
            return Err(ExecutorError::Reentrant);
        }
        if self.consumed.get() {
            return Err(ExecutorError::OutcomeConsumed);
        }
        Ok(())
    }
}
