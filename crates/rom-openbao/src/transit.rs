use crate::OpenBao;
use crate::requests;
use base64::{Engine, engine::general_purpose::STANDARD};
use reqwest::Method;
use rom_kms::{Binding, EncryptionVersion, Envelope, KeyRef, Kms};
use rom_secrets::{Error, SecretBytes, SecretRef};
use zeroize::Zeroizing;
fn version(ciphertext: &str) -> Result<u64, Error> {
    let mut parts = ciphertext.splitn(3, ':');
    if parts.next() != Some("vault") {
        return Err(Error::Protocol);
    }
    let version = parts
        .next()
        .and_then(|v| v.strip_prefix('v'))
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|v| *v > 0)
        .ok_or(Error::Protocol)?;
    if parts.next().is_none_or(str::is_empty) {
        return Err(Error::Protocol);
    }
    Ok(version)
}
pub(crate) fn encrypted(
    profile: &SecretRef,
    key: &KeyRef,
    selection: EncryptionVersion,
    response: &mut serde_json::Value,
) -> Result<Envelope, Error> {
    let actual = response
        .pointer("/data/key_version")
        .and_then(serde_json::Value::as_u64)
        .filter(|v| (1..=i32::MAX as u64).contains(v))
        .ok_or(Error::Protocol)?;
    let serde_json::Value::String(ciphertext) = response
        .pointer_mut("/data/ciphertext")
        .ok_or(Error::Protocol)?
        .take()
    else {
        return Err(Error::Protocol);
    };
    if version(&ciphertext)? != actual
        || matches!(selection,EncryptionVersion::Pinned(v) if v.get()!=actual)
    {
        return Err(Error::Protocol);
    }
    Envelope::new(profile.clone(), key.clone(), actual, ciphertext)
}
impl Kms for OpenBao {
    async fn encrypt(
        &self,
        key: &KeyRef,
        plaintext: &SecretBytes,
        binding: &Binding,
    ) -> Result<Envelope, Error> {
        self.encrypt_at(key, plaintext, binding, EncryptionVersion::Latest)
            .await
    }
    fn validate_encryption(&self, key: &KeyRef, selection: EncryptionVersion) -> Result<(), Error> {
        if !self.keys.contains_key(key) {
            return Err(Error::Invalid);
        }
        if matches!(selection,EncryptionVersion::Pinned(v) if v.get()>i32::MAX as u64) {
            return Err(Error::Invalid);
        }
        Ok(())
    }
    async fn encrypt_at(
        &self,
        key: &KeyRef,
        plaintext: &SecretBytes,
        binding: &Binding,
        selection: EncryptionVersion,
    ) -> Result<Envelope, Error> {
        self.validate_encryption(key, selection)?;
        let location = self.keys.get(key).ok_or(Error::Invalid)?;
        let _permit = self.transport.admit()?;
        let url = self
            .transport
            .url(&[&location.mount, "encrypt", &location.key])?;
        let version = match selection {
            EncryptionVersion::Latest => None,
            EncryptionVersion::Pinned(v) => Some(v.get()),
        };
        let body = requests::encrypt(plaintext, binding, version)?;
        let mut response = self
            .transport
            .request(Method::POST, url, Some(body), Error::Rejected)
            .await?;
        encrypted(&self.profile, key, selection, &mut response)
    }
    async fn decrypt(&self, envelope: &Envelope, binding: &Binding) -> Result<SecretBytes, Error> {
        if envelope.profile() != &self.profile
            || version(envelope.ciphertext())? != envelope.version()
        {
            return Err(Error::Invalid);
        }
        let location = self.keys.get(envelope.key()).ok_or(Error::Invalid)?;
        let _permit = self.transport.admit()?;
        let url = self
            .transport
            .url(&[&location.mount, "decrypt", &location.key])?;
        let body = requests::decrypt(envelope, binding)?;
        let mut response = self
            .transport
            .request(Method::POST, url, Some(body), Error::Rejected)
            .await?;
        let serde_json::Value::String(encoded) = response
            .pointer_mut("/data/plaintext")
            .ok_or(Error::Protocol)?
            .take()
        else {
            return Err(Error::Protocol);
        };
        let encoded = Zeroizing::new(encoded);
        if encoded.len() > 5464 {
            return Err(Error::Limit);
        }
        let plaintext = STANDARD
            .decode(encoded.as_bytes())
            .map_err(|_| Error::Protocol)?;
        SecretBytes::new(plaintext)
    }
}
