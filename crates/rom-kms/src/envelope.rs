use rom_secrets::{Error, SecretRef};
/// Approved opaque key alias. Provider paths are host-owned configuration.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct KeyRef(SecretRef);
impl KeyRef {
    /// Validate the same opaque alias grammar used for host secret references.
    pub fn new(alias: impl Into<String>) -> Result<Self, Error> {
        SecretRef::new(alias).map(Self)
    }
    /// Borrow the non-secret key alias.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}
/// Bounded ciphertext with profile, key identity, and actual key version.
/// This envelope contains no plaintext, credentials, context, or AAD.
/// The initial contract selects a derived AEAD profile; other algorithms need
/// separate capability review before an implementation advertises support.
pub struct Envelope {
    profile: SecretRef,
    key: KeyRef,
    version: u64,
    ciphertext: String,
}
impl Envelope {
    /// Reconstruct a stored envelope. Provider authentication still occurs on decrypt.
    pub fn new(
        profile: SecretRef,
        key: KeyRef,
        version: u64,
        ciphertext: String,
    ) -> Result<Self, Error> {
        if ciphertext.len() > 16384 {
            return Err(Error::Limit);
        }
        if version == 0 || ciphertext.is_empty() {
            return Err(Error::Invalid);
        }
        Ok(Self {
            profile,
            key,
            version,
            ciphertext,
        })
    }
    /// Borrow the selected host provider-profile identity.
    pub fn profile(&self) -> &SecretRef {
        &self.profile
    }
    /// Borrow the immutable approved key alias.
    pub fn key(&self) -> &KeyRef {
        &self.key
    }
    /// Return the actual encryption key version.
    pub fn version(&self) -> u64 {
        self.version
    }
    /// Borrow ciphertext for persistence. This method never exposes plaintext.
    pub fn ciphertext(&self) -> &str {
        &self.ciphertext
    }
}

/// Algorithm capability of the initial authenticated-encryption contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Algorithm {
    /// Derived AES-256-GCM with a 96-bit provider-generated nonce.
    DerivedAes256Gcm96,
}
impl Envelope {
    /// Identify the envelope's authenticated-encryption profile.
    pub fn algorithm(&self) -> Algorithm {
        Algorithm::DerivedAes256Gcm96
    }
}
