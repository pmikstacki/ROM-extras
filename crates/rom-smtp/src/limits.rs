use crate::SmtpError;
use std::time::Duration;
/// Host-selected whole-attempt deadline, total decrypted reply budget and dispatch interval.
/// A shared transport admits one attempt and never queues or retries internally.
pub struct Limits {
    pub(crate) timeout: Duration,
    pub(crate) response_bytes: usize,
    pub(crate) interval: Duration,
}
impl Limits {
    /// Require 1ms–60s timeout, 1024–65536 reply bytes and 0–86400s dispatch interval.
    pub fn new(
        timeout: Duration,
        response_bytes: usize,
        interval: Duration,
    ) -> Result<Self, SmtpError> {
        if timeout < Duration::from_millis(1)
            || timeout > Duration::from_secs(60)
            || !(1024..=65536).contains(&response_bytes)
            || interval > Duration::from_secs(86400)
        {
            return Err(SmtpError::InvalidLimits);
        }
        Ok(Self {
            timeout,
            response_bytes,
            interval,
        })
    }
}
