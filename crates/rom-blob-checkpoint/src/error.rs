/// Fixed maintenance failures omit paths, private identities and provider diagnostics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// Invalid host configuration or private artifact representation.
    Invalid,
    /// An input or response exceeds an explicit bound.
    TooLarge,
    /// Disabled backend or unavailable native capability.
    Unsupported,
    /// An existing file, incompatible row or competing owner prevents admission.
    Conflict,
    /// Publication may have occurred. Preserve the artifact and reconcile explicitly.
    Unknown,
    /// Native or provider access failed without a public safe classification.
    Unavailable,
    /// Required source content is absent.
    Missing,
    /// The provider or public Resource read denied access.
    Denied,
    /// The cooperative operation deadline expired.
    Timeout,
    /// The host canceled this invocation.
    Canceled,
    /// Source content or a public row differs from the expected checkpoint.
    Integrity,
}
impl From<rom::Error> for Error {
    fn from(e: rom::Error) -> Self {
        match e {
            rom::Error::Unknown => Self::Unknown,
            rom::Error::TooLarge => Self::TooLarge,
            rom::Error::Unsupported(_) => Self::Unsupported,
            rom::Error::Conflict => Self::Conflict,
            rom::Error::Denied => Self::Denied,
            rom::Error::Invalid { .. } => Self::Invalid,
            _ => Self::Unavailable,
        }
    }
}
impl From<rom_extras_maintenance::Error> for Error {
    fn from(e: rom_extras_maintenance::Error) -> Self {
        use rom_extras_maintenance::Error as E;
        match e {
            E::Invalid => Self::Invalid,
            E::TooLarge => Self::TooLarge,
            E::Unsupported => Self::Unsupported,
            E::Conflict => Self::Conflict,
            E::Unknown => Self::Unknown,
            E::Unavailable => Self::Unavailable,
        }
    }
}
impl From<rom_blob_recovery::Cause> for Error {
    fn from(e: rom_blob_recovery::Cause) -> Self {
        use rom_blob_recovery::Cause as C;
        match e {
            C::Invalid => Self::Invalid,
            C::Limit => Self::TooLarge,
            C::Missing => Self::Missing,
            C::Denied => Self::Denied,
            C::Integrity => Self::Integrity,
            C::Unknown => Self::Unknown,
            C::Timeout => Self::Timeout,
            C::Canceled => Self::Canceled,
            C::Unsupported => Self::Unsupported,
            C::Backend => Self::Unavailable,
        }
    }
}
impl From<rom_blob::Error> for Error {
    fn from(e: rom_blob::Error) -> Self {
        match e {
            rom_blob::Error::Core(e) => e.into(),
            rom_blob::Error::Conflict => Self::Integrity,
            e => rom_blob_recovery::Cause::from(e).into(),
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}
/// Operation at the failing Ready-row index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Public current-row comparison.
    Row,
    /// Verified public service/provider read.
    Read,
    /// Exact observed physical-key binding.
    Observe,
    /// Service draining and Runtime/native cleanup.
    Shutdown,
}
/// Count-only failure. No partial inventory can be published through this result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Failure {
    /// Fully verified preceding Ready rows.
    pub verified: usize,
    /// Failing Ready-row index, or total Ready count for cleanup failure.
    pub index: usize,
    /// Failed operation.
    pub phase: Phase,
    /// Fixed safe cause.
    pub cause: Error,
    /// A separately classified cleanup failure, when applicable.
    pub cleanup: Option<Error>,
}
impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "blob checkpoint row {} {:?}: {}",
            self.index, self.phase, self.cause
        )
    }
}
impl std::error::Error for Failure {}
