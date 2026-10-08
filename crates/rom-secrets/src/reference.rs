use crate::Error;
use std::num::NonZeroU64;
/// Host-approved opaque alias. It never contains a provider URL or path.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SecretRef(String);
impl SecretRef {
    /// Validate an ASCII alias of at most 64 bytes, beginning with a letter.
    pub fn new(alias: impl Into<String>) -> Result<Self, Error> {
        let alias = alias.into();
        if alias.is_empty()
            || alias.len() > 64
            || !alias.as_bytes()[0].is_ascii_alphabetic()
            || !alias
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        {
            return Err(Error::Invalid);
        }
        Ok(Self(alias))
    }
    /// Borrow the validated non-secret alias.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
/// Explicit provider version selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Version {
    /// Resolve the latest available version without a plaintext cache.
    Latest,
    /// Resolve this positive version; never silently substitute latest.
    Pinned(NonZeroU64),
}
impl Version {
    /// Validate a positive pinned version.
    pub fn pinned(version: u64) -> Result<Self, Error> {
        NonZeroU64::new(version)
            .map(Self::Pinned)
            .ok_or(Error::Invalid)
    }
}
