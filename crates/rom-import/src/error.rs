/// Finite admission failures without source content or credential details.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// A limit is zero or exceeds the core's hard ceiling.
    InvalidLimits,
    /// The host supplied an invalid grant or action binding.
    InvalidGrant,
    /// The bytes differ from the host-approved representation.
    DigestMismatch,
    /// The document exceeds an admission limit.
    TooLarge,
    /// The document is malformed, ambiguous, trailing or excessively nested JSON.
    InvalidJson,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}
