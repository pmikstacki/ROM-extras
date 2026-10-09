/// Finite acquisition errors without paths, URLs, credentials or response bodies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// The pure core refused the exact fetched representation.
    Admission(rom_import::Error),
    /// A host configuration or validator exceeds the supported contract.
    InvalidConfig,
    /// A supplied file handle does not refer to a regular file.
    UnsupportedSource,
    /// Declared, streamed or retained metadata exceeds admission.
    TooLarge,
    /// The file, HTTP peer, TLS identity or framing is unavailable.
    Unavailable,
    /// HTTP status, encoding or content type is refused.
    Rejected,
    /// HTTP404 has no document; it must not imply Resource deletion.
    Missing,
    /// The conditional representation did not meet the host's exact validator.
    PreconditionFailed,
    /// HTTP429; the hint does not cause automatic retries.
    RateLimited {
        /// Optional integer seconds, bounded to at most3600.
        retry_after_seconds: Option<u32>,
    },
    /// The total HTTP operation budget expired.
    Timeout,
    /// The host requested cancellation or closed its cancellation channel.
    Cancelled,
    /// The finite concurrent operation slots are occupied.
    Overloaded,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}
