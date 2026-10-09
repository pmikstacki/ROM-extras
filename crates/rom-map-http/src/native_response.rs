//! Explicit native response admission; raw bodies are never safe error messages.
/// The only native response classes explicitly admitted by the map reader.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadStatus {
    /// Native HTTP success, requiring adapter validation.
    Success,
    /// Native HTTP400, requiring adapter-specific protocol normalization.
    BadRequest,
}
/// Bounded native data for adapter decoding; never expose it directly to the browser.
pub struct NativeResponse {
    pub(crate) status: ReadStatus,
    pub(crate) body: Vec<u8>,
}
impl NativeResponse {
    /// Native status class; no URI, credential or message is included.
    pub fn status(&self) -> ReadStatus {
        self.status
    }
    /// Bounded native bytes, still untrusted and possibly containing confidential service data.
    pub fn body(&self) -> &[u8] {
        &self.body
    }
}
