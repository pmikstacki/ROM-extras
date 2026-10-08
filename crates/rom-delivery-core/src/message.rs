use crate::{DeliveryError, PayloadLimit};
use rom::{Delivery, Input};
use std::{
    fmt,
    io::{self, Write},
};

struct BoundedBody {
    bytes: Vec<u8>,
    limit: usize,
    overflow: bool,
}
impl Write for BoundedBody {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.limit - self.bytes.len() {
            self.overflow = true;
            return Err(io::Error::other("delivery JSON exceeds byte limit"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Stable delivery identity and exact JSON bytes prepared before external I/O.
///
/// Debug omits payload values. There is no payload mutation API: sign and send
/// these same bytes. This is not a durable receipt or an external acceptance.
pub struct PreparedDelivery {
    id: String,
    attempt: u32,
    body: Vec<u8>,
}
impl PreparedDelivery {
    /// Prepare a typed Runtime delivery within its encoded-body limit.
    ///
    /// Identity is 1..=2048 ASCII letters, digits, hyphens, or underscores.
    /// Invalid identity rejects before invoking the payload encoder. Custom
    /// `Input::encode` allocations, computation, or panics are not bounded here.
    pub fn prepare<P: Input>(
        delivery: Delivery<P>,
        limit: PayloadLimit,
    ) -> Result<Self, DeliveryError> {
        if delivery.id.is_empty()
            || delivery.id.len() > 2048
            || !delivery
                .id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
        {
            return Err(DeliveryError::InvalidIdentity);
        }
        let mut output = BoundedBody {
            bytes: Vec::new(),
            limit: limit.bytes(),
            overflow: false,
        };
        if serde_json::to_writer(&mut output, &delivery.payload.encode()).is_err() {
            return Err(if output.overflow {
                DeliveryError::TooLarge
            } else {
                DeliveryError::InvalidPayload
            });
        }
        Ok(Self {
            id: delivery.id,
            attempt: delivery.attempt,
            body: output.bytes,
        })
    }
    /// Unchanged Runtime delivery identity, also suitable for receiver deduplication.
    pub fn id(&self) -> &str {
        &self.id
    }
    /// Runtime attempt number; it never changes the stable identity.
    pub fn attempt(&self) -> u32 {
        self.attempt
    }
    /// Exact body bytes to sign and send without reserialization.
    pub fn body(&self) -> &[u8] {
        &self.body
    }
    /// Transfer the exact prepared bytes into a transport request.
    pub fn into_body(self) -> Vec<u8> {
        self.body
    }
}
impl fmt::Debug for PreparedDelivery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PreparedDelivery")
            .field("attempt", &self.attempt)
            .field("body_bytes", &self.body.len())
            .finish_non_exhaustive()
    }
}
