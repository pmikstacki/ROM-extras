use crate::{Binding, EncryptionVersion, Envelope, KeyRef};
use rom_secrets::{Error, SecretBytes};
/// Host encryption service. Implementations do not create or export provider keys.
pub trait Kms: Send + Sync {
    /// Encrypt bounded plaintext under an approved key and authenticated binding.
    fn encrypt<'a>(
        &'a self,
        key: &'a KeyRef,
        plaintext: &'a SecretBytes,
        binding: &'a Binding,
    ) -> impl std::future::Future<Output = Result<Envelope, Error>> + Send + 'a;
    /// Locally validate an approved destination and supported encryption selection.
    /// Pinned selection defaults to Unsupported, without provider I/O.
    fn validate_encryption(&self, _key: &KeyRef, version: EncryptionVersion) -> Result<(), Error> {
        match version {
            EncryptionVersion::Latest => Ok(()),
            EncryptionVersion::Pinned(_) => Err(Error::Unsupported),
        }
    }
    /// Encrypt with explicit version selection. Existing providers keep latest behavior.
    /// Implement pinned selection together with honest local validate_encryption support.
    fn encrypt_at<'a>(
        &'a self,
        key: &'a KeyRef,
        plaintext: &'a SecretBytes,
        binding: &'a Binding,
        version: EncryptionVersion,
    ) -> impl std::future::Future<Output = Result<Envelope, Error>> + Send + 'a {
        async move {
            self.validate_encryption(key, version)?;
            match version {
                EncryptionVersion::Latest => self.encrypt(key, plaintext, binding).await,
                EncryptionVersion::Pinned(_) => Err(Error::Unsupported),
            }
        }
    }
    /// Decrypt only after provider authentication of the supplied binding.
    fn decrypt<'a>(
        &'a self,
        envelope: &'a Envelope,
        binding: &'a Binding,
    ) -> impl std::future::Future<Output = Result<SecretBytes, Error>> + Send + 'a;
}
