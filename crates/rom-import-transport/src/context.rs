use crate::Error;
use std::{future::Future, time::Duration};
use tokio::sync::watch;
/// One total HTTP budget and explicit host watch-channel cancellation.
/// Sending true or closing the channel cancels; the host should latch true once requested.
pub struct RequestContext {
    budget: Duration,
    cancel: watch::Receiver<bool>,
}
impl RequestContext {
    /// Admit a 1ms–60s total budget without starting work or selecting a source.
    pub fn new(budget: Duration, cancel: watch::Receiver<bool>) -> Result<Self, Error> {
        if !(Duration::from_millis(1)..=Duration::from_secs(60)).contains(&budget) {
            return Err(Error::InvalidConfig);
        }
        Ok(Self { budget, cancel })
    }
    fn cancelled(&self) -> bool {
        *self.cancel.borrow() || self.cancel.has_changed().is_err()
    }
    pub(crate) async fn run<T>(
        &self,
        future: impl Future<Output = Result<T, Error>>,
    ) -> Result<T, Error> {
        if self.cancelled() {
            return Err(Error::Cancelled);
        }
        let start = tokio::time::Instant::now();
        let mut cancel = self.cancel.clone();
        let result = tokio::select! {
            biased;
            _=cancel.wait_for(|requested|*requested)=>Err(Error::Cancelled),
            result=tokio::time::timeout(self.budget,future)=>result.map_err(|_|Error::Timeout)?,
        };
        if self.cancelled() {
            return Err(Error::Cancelled);
        }
        if start.elapsed() >= self.budget {
            return Err(Error::Timeout);
        }
        result
    }
}
