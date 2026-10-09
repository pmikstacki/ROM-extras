use std::fmt;
/// Frozen RFC message bytes and one host-approved SMTP envelope; not an acceptance receipt.
/// Explicit access exposes personal data. Debug deliberately omits it.
pub struct PreparedEmail {
    from: String,
    to: String,
    message_id: String,
    body: Vec<u8>,
}
impl PreparedEmail {
    pub(crate) fn new(from: String, to: String, message_id: String, body: Vec<u8>) -> Self {
        Self {
            from,
            to,
            message_id,
            body,
        }
    }
    /// Exact approved envelope sender.
    pub fn from(&self) -> &str {
        &self.from
    }
    /// Exact approved single envelope recipient.
    pub fn to(&self) -> &str {
        &self.to
    }
    /// Stable RFC Message-ID, independent of attempt; no receiver deduplication guarantee.
    pub fn message_id(&self) -> &str {
        &self.message_id
    }
    /// Exact frozen RFC message bytes, including headers and transfer encoding.
    pub fn body(&self) -> &[u8] {
        &self.body
    }
}
impl fmt::Debug for PreparedEmail {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PreparedEmail")
            .field("body_bytes", &self.body.len())
            .finish_non_exhaustive()
    }
}
