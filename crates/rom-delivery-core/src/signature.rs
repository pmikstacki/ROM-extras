use crate::PreparedDelivery;
use base64::{Engine, engine::general_purpose::STANDARD};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use zeroize::Zeroizing;

/// Host-owned HMAC-SHA256 signer for the Standard Webhooks wire profile.
///
/// It has no Debug or serialization API. Stored 32-byte key buffers zeroize
/// on drop. The host supplies random key material through secret configuration.
/// This does not promise erasure of every intermediate MAC state or caller copy.
pub struct WebhookSigner {
    current: Zeroizing<[u8; 32]>,
    previous: Option<Zeroizing<[u8; 32]>>,
}
impl WebhookSigner {
    /// Configure one host-owned 32-byte signing key.
    pub fn new(current: [u8; 32]) -> Self {
        Self {
            current: Zeroizing::new(current),
            previous: None,
        }
    }
    /// Also sign with one previous key during a host-controlled rotation period.
    ///
    /// This replaces any previously configured old key, clearing its stored buffer.
    pub fn with_previous_key(mut self, previous: [u8; 32]) -> Self {
        self.previous = Some(Zeroizing::new(previous));
        self
    }
    /// Sign exact prepared bytes using a trusted host Unix timestamp in seconds.
    ///
    /// The timestamp is refreshed per attempt; the Runtime identity stays stable.
    /// Receivers must verify freshness and deduplicate the ID. This signer does
    /// not verify incoming signatures or claim an exactly-once external effect.
    pub fn sign(&self, message: &PreparedDelivery, unix_seconds: u64) -> SignedHeaders {
        let timestamp = unix_seconds.to_string();
        let mut signature = Self::signature(&self.current, message, &timestamp);
        if let Some(previous) = &self.previous {
            signature.push(' ');
            signature.push_str(&Self::signature(previous, message, &timestamp));
        }
        SignedHeaders {
            id: message.id().into(),
            timestamp,
            signature,
        }
    }
    fn signature(key: &[u8; 32], message: &PreparedDelivery, timestamp: &str) -> String {
        let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("fixed HMAC key length is valid");
        mac.update(message.id().as_bytes());
        mac.update(b".");
        mac.update(timestamp.as_bytes());
        mac.update(b".");
        mac.update(message.body());
        format!("v1,{}", STANDARD.encode(mac.finalize().into_bytes()))
    }
}

/// Standard Webhooks metadata bound to the exact prepared request body.
pub struct SignedHeaders {
    id: String,
    timestamp: String,
    signature: String,
}
impl SignedHeaders {
    /// Value for the `webhook-id` header.
    pub fn id(&self) -> &str {
        &self.id
    }
    /// Value for the `webhook-timestamp` header.
    pub fn timestamp(&self) -> &str {
        &self.timestamp
    }
    /// Value for the `webhook-signature` header, with one or two `v1` signatures.
    pub fn signature(&self) -> &str {
        &self.signature
    }
}
