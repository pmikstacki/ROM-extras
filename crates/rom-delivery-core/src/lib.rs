//! Bounded preparation of ROM delivery payloads.
//!
//! Runtime owns stable identities, committed intents, retry, and authorization.
//! This package prepares bytes without network effects or a second Work ledger.
//! It does not establish any external provider's support profile.
//!
//! ```
//! use rom::{Delivery, json};
//! use rom_delivery_core::{PayloadLimit, PreparedDelivery, WebhookSigner};
//! let message = PreparedDelivery::prepare(
//!     Delivery { id: "work-17".into(), attempt: 1, payload: false },
//!     PayloadLimit::default(),
//! )?;
//! // Synthetic example key only. The host supplies entropy and trusted time.
//! let headers = WebhookSigner::new([7; 32]).sign(&message, 1_800_000_000);
//! assert_eq!(headers.id(), "work-17");
//! assert_eq!(message.body(), b"false");
//! # Ok::<(), rom_delivery_core::DeliveryError>(())
//! ```

mod error;
mod limits;
mod message;
mod signature;

pub use error::DeliveryError;
pub use limits::PayloadLimit;
pub use message::PreparedDelivery;
pub use signature::{SignedHeaders, WebhookSigner};
