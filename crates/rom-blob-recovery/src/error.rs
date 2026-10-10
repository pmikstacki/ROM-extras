use crate::Report;
use std::{error::Error, fmt};
/// Safe fixed error categories. Provider messages and object identities are omitted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cause {
    /// Invalid inventory or provider input.
    Invalid,
    /// Inventory or object exceeds a bound.
    Limit,
    /// Required content is absent.
    Missing,
    /// Provider denied access.
    Denied,
    /// Read-back content differs from the inventory.
    Integrity,
    /// Provider cannot establish publication outcome.
    Unknown,
    /// Caller wait deadline expired.
    Timeout,
    /// Caller explicitly canceled this invocation.
    Canceled,
    /// Provider lacks required operation support.
    Unsupported,
    /// Other provider failure; diagnostic data is omitted.
    Backend,
}
impl fmt::Display for Cause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "blob recovery {self:?}")
    }
}
impl Error for Cause {}
impl From<rom_blob::Error> for Cause {
    fn from(e: rom_blob::Error) -> Self {
        match e {
            rom_blob::Error::Invalid | rom_blob::Error::Input => Self::Invalid,
            rom_blob::Error::TooLarge => Self::Limit,
            rom_blob::Error::Missing => Self::Missing,
            rom_blob::Error::Denied => Self::Denied,
            rom_blob::Error::Unknown => Self::Unknown,
            rom_blob::Error::Timeout => Self::Timeout,
            rom_blob::Error::Unsupported => Self::Unsupported,
            _ => Self::Backend,
        }
    }
}
/// Operation at the failing entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Check existing destination before any write.
    DestinationRead,
    /// Validate source bytes before publication.
    SourceRead,
    /// One admitted create-only call.
    Create,
    /// Verify after acknowledged creation or a concurrent-create conflict.
    DestinationVerify,
}
/// Publication certainty for the failing entry in this invocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Publication {
    /// No create call was admitted by this invocation.
    NotAttempted,
    /// A create call returned success, but verification did not complete.
    Confirmed,
    /// An admitted create call did not establish publication outcome.
    Unknown,
    /// The create call returned Conflict without overwriting an existing object.
    Conflict,
}
/// Safe partial progress. Earlier verified objects remain published; no cleanup occurs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Failure {
    /// Count-only verified prefix; not a durable checkpoint.
    pub progress: Report,
    /// Zero-based entry index, or entries.len() for a final deadline/cancellation check.
    /// Identities stay in the private host manifest.
    pub index: usize,
    /// Failing operation.
    pub phase: Phase,
    /// Safe fixed cause.
    pub cause: Cause,
    /// Publication certainty for this entry.
    pub publication: Publication,
}
impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "blob recovery entry {} {:?}: {} ({:?})",
            self.index, self.phase, self.cause, self.publication
        )
    }
}
impl Error for Failure {}
