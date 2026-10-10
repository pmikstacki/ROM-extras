use crate::{Binding, EncryptionVersion, Envelope, KeyRef, Kms};
use rom_secrets::Error;
use std::time::Duration;
use tokio::{sync::Semaphore, time::Instant};
/// Admission and total deadline for one shared migrator, separate from HTTP limits.
#[derive(Clone, Copy)]
pub struct MigrationLimits {
    /// Cooperative complete migration deadline, in1ms..=60s.
    pub deadline: Duration,
    /// Maximum retained migration plaintext buffers, in1..=64. No admission queue.
    pub max_in_flight: usize,
}
impl Default for MigrationLimits {
    fn default() -> Self {
        Self {
            deadline: Duration::from_secs(5),
            max_in_flight: 4,
        }
    }
}
/// Host-owned authenticated re-encryption. No persistence, retries or detached tasks.
/// Share one instance to apply its concurrency bound. Owned plaintext drops on cancellation.
/// Provider/HTTP/host copies and remote cancellation are outside that erasure guarantee.
pub struct Migrator<K> {
    provider: K,
    admission: Semaphore,
    deadline: Duration,
}
impl<K: Kms> Migrator<K> {
    /// Validate limits without contacting the provider.
    pub fn new(provider: K, limits: MigrationLimits) -> Result<Self, Error> {
        if !(1..=64).contains(&limits.max_in_flight)
            || !(Duration::from_millis(1)..=Duration::from_secs(60)).contains(&limits.deadline)
        {
            return Err(Error::Invalid);
        }
        Ok(Self {
            provider,
            admission: Semaphore::new(limits.max_in_flight),
            deadline: limits.deadline,
        })
    }
    /// Borrow the configured provider for explicit host operations.
    pub fn provider(&self) -> &K {
        &self.provider
    }
    /// Authenticate the original, then encrypt under an explicitly pinned destination.
    /// Context/AAD stay identical. Return a replacement without changing the original.
    /// The host authorizes selection and conditionally persists the returned envelope.
    /// Requires an active Tokio time driver; does not create a hidden runtime.
    /// Timeout and future drop do not establish rollback or remote cancellation.
    pub async fn migrate(
        &self,
        source: &Envelope,
        binding: &Binding,
        destination: &KeyRef,
        version: EncryptionVersion,
    ) -> Result<Envelope, Error> {
        let EncryptionVersion::Pinned(pinned) = version else {
            return Err(Error::Invalid);
        };
        self.provider.validate_encryption(destination, version)?;
        let _permit = self.admission.try_acquire().map_err(|_| Error::Busy)?;
        let expires = Instant::now() + self.deadline;
        let operation = async {
            let decrypted = self.provider.decrypt(source, binding).await;
            if Instant::now() >= expires {
                return Err(Error::Timeout);
            }
            let plaintext = decrypted?;
            let encrypted = self
                .provider
                .encrypt_at(destination, &plaintext, binding, version)
                .await;
            if Instant::now() >= expires {
                return Err(Error::Timeout);
            }
            let replacement = encrypted?;
            if replacement.key() != destination
                || replacement.version() != pinned.get()
                || replacement.profile() != source.profile()
            {
                return Err(Error::Protocol);
            }
            Ok(replacement)
        };
        tokio::time::timeout_at(expires, operation)
            .await
            .map_err(|_| Error::Timeout)?
    }
}
