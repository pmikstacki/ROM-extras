use crate::{KeyLocation, SecretLocation};
use rom_kms::KeyRef;
use rom_secrets::{SecretBytes, SecretRef};
use std::time::Duration;
/// Host-approved profile configuration. No secret-bearing Debug implementation.
pub struct Config {
    /// Explicit HTTPS origin, without path, credentials, query or fragment.
    pub endpoint: String,
    /// Existing limited provider token; never use a runtime root token.
    pub token: SecretBytes,
    /// Optional additional PEM trust CA; certificate/name verification remains enabled.
    pub ca_pem: Option<Vec<u8>>,
    /// Stable provider profile identity for ciphertext envelopes.
    pub profile: SecretRef,
    /// Approved KV aliases. Duplicate aliases are rejected.
    pub secrets: Vec<(SecretRef, SecretLocation)>,
    /// Approved existing derived aes256-gcm96 keys. Duplicate aliases are rejected.
    pub keys: Vec<(KeyRef, KeyLocation)>,
}
/// Per-client limits. Refusal does not create an unbounded admission queue.
#[derive(Clone, Copy)]
pub struct Limits {
    /// Complete HTTP operation deadline, in 1 ms..=60 seconds.
    pub deadline: Duration,
    /// Maximum concurrent calls, in 1..=64.
    pub max_in_flight: usize,
    /// Streamed response bound, in 1..=32 KiB, including error responses.
    pub max_response_bytes: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            deadline: Duration::from_secs(5),
            max_in_flight: 4,
            max_response_bytes: 32768,
        }
    }
}
