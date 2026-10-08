use std::fmt;

/// Binding failures without connection credentials or payload disclosure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NatsDeliveryError {
    /// Name, subject, admission, or deadline is outside the supported profile.
    InvalidConfiguration,
    /// The stream lookup failed or exceeded its deadline.
    Unavailable,
    /// The stream is not file-backed, acknowledged, and explicitly bound.
    UnsupportedStream,
}
impl fmt::Display for NatsDeliveryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidConfiguration => "invalid JetStream delivery configuration",
            Self::Unavailable => "JetStream binding unavailable",
            Self::UnsupportedStream => "unsupported JetStream stream profile",
        })
    }
}
impl std::error::Error for NatsDeliveryError {}
