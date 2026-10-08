use crate::{Binding, Envelope, KeyRef};
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
    /// Decrypt only after provider authentication of the supplied binding.
    fn decrypt<'a>(
        &'a self,
        envelope: &'a Envelope,
        binding: &'a Binding,
    ) -> impl std::future::Future<Output = Result<SecretBytes, Error>> + Send + 'a;
}
