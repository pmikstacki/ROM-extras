use std::fmt;

/// Preparation failures occur before an adapter performs any external effect.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeliveryError {
    /// Host-configured payload limit is outside the accepted profile.
    InvalidLimit,
    /// Delivery identity cannot be safely carried in this wire profile.
    InvalidIdentity,
    /// Encoded JSON exceeds the configured payload-byte limit.
    TooLarge,
    /// Payload serialization failed without exceeding the byte limit.
    InvalidPayload,
}
impl fmt::Display for DeliveryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidLimit => "invalid delivery payload limit",
            Self::InvalidIdentity => "invalid delivery identity",
            Self::TooLarge => "delivery payload exceeds byte limit",
            Self::InvalidPayload => "delivery payload serialization failed",
        })
    }
}
impl std::error::Error for DeliveryError {}
