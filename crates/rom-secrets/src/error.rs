/// Fixed host-integration failures without upstream text or secret payloads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// Invalid reference, configuration or input.
    Invalid,
    /// Authentication or permission failed.
    Denied,
    /// Selected value is absent or hidden by provider policy.
    NotFound,
    /// Service cannot currently answer the request.
    Unavailable,
    /// Waiting deadline elapsed; remote cancellation is not established.
    Timeout,
    /// Concurrent operation limit refused admission.
    Busy,
    /// Input or response exceeded the configured bound.
    Limit,
    /// Provider response violates the selected contract.
    Protocol,
    /// Ciphertext or authenticated context was rejected.
    Rejected,
    /// Requested capability is unsupported by the selected profile.
    Unsupported,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}
impl std::error::Error for Error {}
