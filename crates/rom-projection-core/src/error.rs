//! Fixed diagnostic classifications.
/// A sanitized projection failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// Cooperative startup cancellation; no commit outcome is inferred.
    Cancelled,
    /// Invalid caller metadata.
    Invalid,
    /// Admission limit exceeded.
    TooLarge,
    /// Inconsistent expected state or content.
    Conflict,
    /// Journal continuity cannot be established.
    HistoryGap,
    /// Retained authorized content cannot reproduce the immutable intent.
    RebuildRequired,
    /// Another cooperating owner holds the file.
    Busy,
    /// Unqualified format, platform, or capability.
    Unsupported,
    /// Invalid durable records.
    Corrupt,
    /// Native I/O failed before a confirmed commit.
    Storage,
    /// Native commit has an uncertain outcome; reconcile this token after reopening.
    Unknown {
        /// Immutable transition identity.
        transaction: crate::TransactionId,
    },
    /// Fresh initialization could have committed; inspect complete initial state after reopening.
    UnknownInitialization,
}
/// Fixed-error operation result.
pub type Result<T> = std::result::Result<T, Error>;

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "projection checkpoint {}",
            match self {
                Self::Cancelled => "cancelled",
                Self::Invalid => "invalid",
                Self::TooLarge => "too large",
                Self::Conflict => "conflict",
                Self::HistoryGap => "history gap",
                Self::RebuildRequired => "rebuild required",
                Self::Busy => "busy",
                Self::Unsupported => "unsupported",
                Self::Corrupt => "corrupt",
                Self::Storage => "storage failure",
                Self::Unknown { .. } => "unknown",
                Self::UnknownInitialization => "unknown initialization",
            }
        )
    }
}
impl std::error::Error for Error {}
