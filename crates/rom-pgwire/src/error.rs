use crate::SqlState;

/// Finite transport failures. Native messages, queries and credentials are discarded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// Invalid endpoint, operation bound or blocking call under an active Tokio runtime.
    Invalid,
    /// Connection establishment failed, or the transport is no longer available.
    Unavailable,
    /// The server rejected an operation with this SQLSTATE; no message is retained.
    Server(SqlState),
    /// Bounded metadata admission failed; the pending transport was retired.
    ResponseLimit,
    /// A complete native response was not established; the transport was retired.
    Unknown,
}
impl Error {
    /// The server's finite protocol code, without native diagnostic strings.
    pub fn sqlstate(&self) -> Option<&str> {
        match self {
            Self::Server(code) => Some(code.code()),
            _ => None,
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}
