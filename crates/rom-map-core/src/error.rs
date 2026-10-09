//! Sanitized map failures without native bodies, queries or credentials.
/// Fixed map contract and service failure categories.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// Nonfinite or out-of-range WGS84 coordinate.
    InvalidCoordinate,
    /// Invalid bounds or latitude ordering.
    InvalidBounds,
    /// Invalid or empty typed query.
    InvalidQuery,
    /// Invalid line geometry.
    InvalidGeometry,
    /// Invalid distance, duration or accuracy unit.
    InvalidUnit,
    /// Malformed native response.
    InvalidResponse,
    /// A declared input or output budget was exceeded.
    TooLarge,
    /// The selected provider does not expose this capability.
    Unsupported,
    /// Provider rate rejection with a bounded seconds hint, without implicit retry.
    RateLimited {
        /// Bounded native Retry-After delay, when available.
        retry_after_seconds: Option<u32>,
    },
    /// Total operation deadline expired.
    Timeout,
    /// The host cancelled the operation.
    Cancelled,
    /// The service outcome is unavailable.
    Unavailable,
    /// The configured service rejected the request.
    Rejected,
}
/// Sanitized map operation result.
pub type Result<T> = std::result::Result<T, Error>;
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("map operation failed")
    }
}
impl std::error::Error for Error {}
