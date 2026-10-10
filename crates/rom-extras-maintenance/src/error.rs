/// Fixed errors omit paths, native diagnostics, Resource values and converter error text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// Invalid host input or invalid migrated representation.
    Invalid,
    /// A requested budget or checked archive exceeds its bound.
    TooLarge,
    /// Disabled capability or unsupported native/archive format.
    Unsupported,
    /// Existing destination or another owner prevents admission.
    Conflict,
    /// Publication outcome is uncertain; preserve the original destination and do not retry automatically.
    Unknown,
    /// Native/archive access or validation failed; no private diagnostic is retained.
    Unavailable,
}
impl From<rom::Error> for Error {
    fn from(e: rom::Error) -> Self {
        match e {
            rom::Error::Unknown => Self::Unknown,
            rom::Error::TooLarge => Self::TooLarge,
            rom::Error::Unsupported(_) => Self::Unsupported,
            rom::Error::Conflict => Self::Conflict,
            rom::Error::Invalid { .. } => Self::Invalid,
            _ => Self::Unavailable,
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}
