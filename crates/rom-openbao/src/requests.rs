//! Borrowed wire fields avoid constructing a second plaintext-bearing JSON value.
use base64::{Engine, engine::general_purpose::STANDARD};
use rom_kms::{Binding, Envelope};
use rom_secrets::{Error, SecretBytes};
use serde::Serialize;
use zeroize::Zeroizing;
#[derive(Serialize)]
struct Request<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    key_version: Option<u64>,
    context: &'a str,
    associated_data: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    plaintext: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ciphertext: Option<&'a str>,
}
fn encode(
    binding: &Binding,
    plaintext: Option<&str>,
    ciphertext: Option<&str>,
    key_version: Option<u64>,
) -> Result<Vec<u8>, Error> {
    let context = STANDARD.encode(binding.context());
    let associated_data = STANDARD.encode(binding.aad());
    serde_json::to_vec(&Request {
        key_version,
        context: &context,
        associated_data: &associated_data,
        plaintext,
        ciphertext,
    })
    .map_err(|_| Error::Invalid)
}
pub(crate) fn encrypt(
    plaintext: &SecretBytes,
    binding: &Binding,
    key_version: Option<u64>,
) -> Result<Vec<u8>, Error> {
    let encoded = Zeroizing::new(STANDARD.encode(plaintext.expose()));
    encode(binding, Some(&encoded), None, key_version)
}
pub(crate) fn decrypt(envelope: &Envelope, binding: &Binding) -> Result<Vec<u8>, Error> {
    encode(binding, None, Some(envelope.ciphertext()), None)
}
