use crate::WebhookError;
use std::time::Duration;

/// Finite per-transport admission and network limits.
#[derive(Clone, Copy, Debug)]
pub struct TransportLimits {
    pub(crate) concurrency: usize,
    pub(crate) connect: Duration,
    pub(crate) request: Duration,
}
impl TransportLimits {
    /// Set 1..=1024 in-flight attempts and positive deadlines up to 300 seconds.
    /// Connect timeout must not exceed the total request deadline.
    pub fn new(
        concurrency: usize,
        connect: Duration,
        request: Duration,
    ) -> Result<Self, WebhookError> {
        if !(1..=1024).contains(&concurrency)
            || connect.is_zero()
            || request.is_zero()
            || connect > request
            || request > Duration::from_secs(300)
        {
            return Err(WebhookError::InvalidLimits);
        }
        Ok(Self {
            concurrency,
            connect,
            request,
        })
    }
}
impl Default for TransportLimits {
    fn default() -> Self {
        Self {
            concurrency: 8,
            connect: Duration::from_secs(2),
            request: Duration::from_secs(4),
        }
    }
}
