use crate::{jwk_wire::Set, rsa_components};
use rom_auth::{
    AuthError,
    jwt::{DecodingKey, TrustedKeys},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};
/// Immutable host-approved public keys. Constructed through IssuerPreset::configure_jwks.
/// Fetch clones this snapshot without I/O. It does not discover rotation or establish provider revocation.
#[derive(Clone)]
pub struct JwksKeys {
    keys: BTreeMap<String, DecodingKey>,
}
impl TrustedKeys for JwksKeys {
    fn fetch(&mut self) -> Result<BTreeMap<String, DecodingKey>, AuthError> {
        Ok(self.keys.clone())
    }
}
impl fmt::Debug for JwksKeys {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JwksKeys")
            .field("count", &self.keys.len())
            .finish_non_exhaustive()
    }
}
pub(crate) fn parse(bytes: &[u8]) -> Result<JwksKeys, AuthError> {
    if bytes.len() > 65536 {
        return Err(AuthError::TooLarge);
    }
    let set: Set = serde_json::from_slice(bytes).map_err(|_| AuthError::Invalid)?;
    if set.keys.len() > 32 {
        return Err(AuthError::TooLarge);
    }
    let mut ids = BTreeSet::new();
    for key in &set.keys {
        if key.kid.len() > 64 || key.kty.len() > 32 {
            return Err(AuthError::TooLarge);
        }
        if key.kid.is_empty()
            || key.kid.chars().any(char::is_control)
            || key.kty.is_empty()
            || !ids.insert(key.kid.as_str())
            || ["d", "p", "q", "dp", "dq", "qi", "oth", "k"]
                .iter()
                .any(|p| key.extensions.contains_key(*p))
        {
            return Err(AuthError::Invalid);
        }
    }
    let mut keys = BTreeMap::new();
    for key in set.keys {
        if key.kty != "RSA"
            || key.alg.as_deref() != Some("RS256")
            || key.usage.as_deref().is_some_and(|u| u != "sig")
        {
            continue;
        }
        if key.key_ops.as_ref().is_some_and(|ops| ops != &["verify"]) {
            return Err(AuthError::Invalid);
        }
        if keys.len() == 8 {
            return Err(AuthError::TooLarge);
        }
        let n = key.n.ok_or(AuthError::Invalid)?;
        let e = key.e.ok_or(AuthError::Invalid)?;
        rsa_components::validate(&n, &e)?;
        let decoding = DecodingKey::from_rsa_components(&n, &e).map_err(|_| AuthError::Invalid)?;
        keys.insert(key.kid, decoding);
    }
    if keys.is_empty() {
        return Err(AuthError::WrongProfile);
    }
    Ok(JwksKeys { keys })
}
