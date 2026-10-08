use crate::DeliveryError;

/// Encoded JSON byte limit, including quotes, escapes, and UTF-8 representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PayloadLimit(pub(crate) usize);
impl PayloadLimit {
    /// Select a positive serialized-body limit of at most 1 MiB.
    pub fn new(bytes: usize) -> Result<Self, DeliveryError> {
        if !(1..=1_048_576).contains(&bytes) {
            return Err(DeliveryError::InvalidLimit);
        }
        Ok(Self(bytes))
    }
    /// Maximum accepted encoded body length in bytes.
    pub fn bytes(self) -> usize {
        self.0
    }
}
impl Default for PayloadLimit {
    fn default() -> Self {
        Self(65_536)
    }
}
