//! Host cancellation and total-operation budget, including provider admission waits.
use crate::{Error, Result};
use std::{future::Future, time::Duration};
/// Shared monotonic asynchronous cancellation request.
#[derive(Clone)]
pub struct Cancellation {
    sender: tokio::sync::watch::Sender<bool>,
}
impl Cancellation {
    /// Create an unrequested token without network or runtime activation.
    pub fn new() -> Self {
        let (sender, _) = tokio::sync::watch::channel(false);
        Self { sender }
    }
    /// Request irreversible cancellation for every clone and pending waiter.
    pub fn cancel(&self) {
        let _ = self.sender.send_replace(true);
    }
    /// Inspect current cancellation state.
    pub fn is_cancelled(&self) -> bool {
        *self.sender.borrow()
    }
    /// Wait without polling; a request before subscription is retained.
    pub async fn cancelled(&self) {
        let mut receiver = self.sender.subscribe();
        let _ = receiver.wait_for(|requested| *requested).await;
    }
}
impl Default for Cancellation {
    fn default() -> Self {
        Self::new()
    }
}
/// Finite total deadline and host cancellation for one logical provider operation.
pub struct RequestContext {
    deadline: Duration,
    cancellation: Cancellation,
}
impl RequestContext {
    /// Admit a 1-ms through 60-s total budget; the normal host profile uses 5 seconds.
    pub fn new(deadline: Duration, cancellation: Cancellation) -> Result<Self> {
        if !(Duration::from_millis(1)..=Duration::from_secs(60)).contains(&deadline) {
            return Err(Error::InvalidQuery);
        }
        Ok(Self {
            deadline,
            cancellation,
        })
    }
    /// Execute one complete adapter operation, including rate waits, under its total budget.
    pub async fn run<T>(&self, operation: impl Future<Output = Result<T>>) -> Result<T> {
        if self.cancellation.is_cancelled() {
            return Err(Error::Cancelled);
        }
        tokio::select! {
            biased;
            ()=self.cancellation.cancelled()=>Err(Error::Cancelled),
            result=tokio::time::timeout(self.deadline,operation)=>result.map_err(|_|Error::Timeout)?,
        }
    }
    /// Host cancellation token shared with this operation.
    pub fn cancellation(&self) -> &Cancellation {
        &self.cancellation
    }
    /// Admitted total budget.
    pub fn deadline(&self) -> Duration {
        self.deadline
    }
}
