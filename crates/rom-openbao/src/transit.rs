use crate::OpenBao;
use crate::requests;
use base64::{Engine, engine::general_purpose::STANDARD};
use reqwest::Method;
use rom_kms::{Binding, Envelope, KeyRef, Kms};
use rom_secrets::{Error, SecretBytes};
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
impl Kms for OpenBao {
    async fn encrypt(
        &self,
        key: &KeyRef,
        plaintext: &SecretBytes,
        binding: &Binding,
    ) -> Result<Envelope, Error> {
        let location = self.keys.get(key).ok_or(Error::Invalid)?;
        let _permit = self.transport.admit()?;
        let url = self
            .transport
            .url(&[&location.mount, "encrypt", &location.key])?;
        let body = requests::encrypt(plaintext, binding)?;
        let mut response = self
            .transport
            .request(Method::POST, url, Some(body), Error::Rejected)
            .await?;
        let serde_json::Value::String(ciphertext) = response
            .pointer_mut("/data/ciphertext")
            .ok_or(Error::Protocol)?
            .take()
        else {
            return Err(Error::Protocol);
        };
        let version = version(&ciphertext)?;
        Envelope::new(self.profile.clone(), key.clone(), version, ciphertext)
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
