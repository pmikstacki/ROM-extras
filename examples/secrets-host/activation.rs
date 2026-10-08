//! Prepare host-owned material from a currently authorized provider Resource.
use rom::{Actor, Runtime};
use rom_identity::ProviderActivation;
use rom_secrets::{Error, ResolvedSecret, SecretRef, SecretResolver, Version};

/// Read, resolve and recheck the captured configuration revision.
/// The host must next configure its verifier from this activation's exact values.
/// This function does not authenticate a caller or cache a proof.
pub async fn prepare<R: SecretResolver>(
    runtime: &Runtime,
    host: &Actor,
    authority: &str,
    resolver: &R,
) -> Result<(ProviderActivation, ResolvedSecret), Error> {
    let activation = ProviderActivation::read(runtime, host, authority)
        .await
        .map_err(core_error)?;
    let alias = SecretRef::new(
        activation
            .config()
            .credential_ref
            .as_deref()
            .ok_or(Error::Invalid)?,
    )?;
    let secret = resolver.resolve(&alias, Version::Latest).await?;
    let current = ProviderActivation::read(runtime, host, authority)
        .await
        .map_err(core_error)?;
    if current.revision() != activation.revision() {
        return Err(Error::Denied);
    }
    Ok((activation, secret))
}
fn core_error(error: rom::Error) -> Error {
    match error {
        rom::Error::Denied => Error::Denied,
        _ => Error::Unavailable,
    }
}
