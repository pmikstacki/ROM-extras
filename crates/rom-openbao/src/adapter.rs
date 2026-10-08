use crate::{Config, KeyLocation, Limits, SecretLocation, transport::Transport};
use rom_kms::KeyRef;
use rom_secrets::{Error, SecretRef};
use std::collections::BTreeMap;
/// Host-owned KV resolver and derived Transit AEAD provider.
/// No plaintext cache, administrative operations, or automatic retries are provided.
pub struct OpenBao {
    pub(crate) transport: Transport,
    pub(crate) profile: SecretRef,
    pub(crate) secrets: BTreeMap<SecretRef, SecretLocation>,
    pub(crate) keys: BTreeMap<KeyRef, KeyLocation>,
}
impl OpenBao {
    /// Validate configuration without network access. Duplicate mappings are invalid.
    pub fn new(config: Config, limits: Limits) -> Result<Self, Error> {
        if config.secrets.len() > 256 || config.keys.len() > 256 {
            return Err(Error::Limit);
        }
        let mut secrets = BTreeMap::new();
        for (alias, location) in config.secrets {
            if secrets.insert(alias, location).is_some() {
                return Err(Error::Invalid);
            }
        }
        let mut keys = BTreeMap::new();
        for (alias, location) in config.keys {
            if keys.insert(alias, location).is_some() {
                return Err(Error::Invalid);
            }
        }
        Ok(Self {
            transport: Transport::new(&config.endpoint, config.token, config.ca_pem, limits)?,
            profile: config.profile,
            secrets,
            keys,
        })
    }
}
