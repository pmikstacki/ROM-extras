use std::fmt;

/// Configuration failure without destination, payload, or secret disclosure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WebhookError {
    /// Destination syntax is outside the HTTPS profile.
    InvalidDestination,
    /// Resolution is empty, excessive, or outside the address profile.
    ForbiddenAddress,
    /// Resource limits are zero, inconsistent, or exceed the profile.
    InvalidLimits,
    /// The TLS/HTTP client could not be constructed.
    ClientInitialization,
}
impl fmt::Display for WebhookError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidDestination => "invalid webhook destination",
            Self::ForbiddenAddress => "forbidden webhook resolution",
            Self::InvalidLimits => "invalid webhook limits",
            Self::ClientInitialization => "webhook client initialization failed",
        })
    }
}
impl std::error::Error for WebhookError {}
