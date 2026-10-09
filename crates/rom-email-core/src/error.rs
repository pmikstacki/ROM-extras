use std::fmt;
/// Safe categories without message content, addresses or provider diagnostics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmailError {
    /// Input is outside this bounded plain-text message profile.
    InvalidPayload,
    /// Host policy, domain or byte limit is invalid.
    InvalidProfile,
    /// Runtime delivery identity cannot be carried safely.
    InvalidIdentity,
    /// Recipient does not exactly match the host's approved set.
    RecipientDenied,
    /// Input or final MIME bytes exceed the profile limit.
    TooLarge,
}
impl fmt::Display for EmailError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidPayload => "invalid email payload",
            Self::InvalidProfile => "invalid email profile",
            Self::InvalidIdentity => "invalid delivery identity",
            Self::RecipientDenied => "email recipient denied",
            Self::TooLarge => "email exceeds byte limit",
        })
    }
}
impl std::error::Error for EmailError {}
