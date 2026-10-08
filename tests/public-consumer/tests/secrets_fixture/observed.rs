//! Count actual resolution; optionally change the provider after it returns.
use rom::{Actor, Command, Runtime};
use rom_identity::IdentityProvider;
use rom_openbao::OpenBao;
use rom_secrets::{Error, ResolvedSecret, SecretRef, SecretResolver, Version};
use std::sync::atomic::{AtomicUsize, Ordering};
pub(crate) struct Observed<'a> {
    pub(crate) inner: &'a OpenBao,
    pub(crate) runtime: &'a Runtime,
    pub(crate) actor: &'a Actor,
    pub(crate) disable_provider: bool,
    pub(crate) calls: AtomicUsize,
}
impl SecretResolver for Observed<'_> {
    async fn resolve(
        &self,
        reference: &SecretRef,
        version: Version,
    ) -> Result<ResolvedSecret, Error> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let secret = self.inner.resolve(reference, version).await?;
        if self.disable_provider {
            let mut provider = self
                .runtime
                .read::<IdentityProvider>(self.actor, "provider")
                .await
                .unwrap()
                .value
                .unwrap();
            provider.enabled = false;
            self.runtime
                .execute(
                    self.actor,
                    Command::replace("provider", provider)
                        .at_revision(1)
                        .idempotency("disable-after-resolution"),
                )
                .await
                .unwrap();
        }
        Ok(secret)
    }
}
