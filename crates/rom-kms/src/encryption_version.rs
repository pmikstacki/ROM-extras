use rom_secrets::Error;
use std::num::NonZeroU64;
/// Explicit encryption selection. Latest is not a stable migration destination.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EncryptionVersion {
    /// Use the provider's current encryption version and return its actual version.
    Latest,
    /// Request exactly this positive version; never substitute latest.
    Pinned(NonZeroU64),
}
impl EncryptionVersion {
    /// Validate a positive version. Providers can impose smaller wire-profile bounds.
    pub fn pinned(version: u64) -> Result<Self, Error> {
        NonZeroU64::new(version)
            .map(Self::Pinned)
            .ok_or(Error::Invalid)
    }
}
