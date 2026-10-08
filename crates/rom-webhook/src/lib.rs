//! HTTPS delivery transport. Runtime owns retry and durable intents.
mod destination;
mod error;
mod limits;
mod transport;

pub use destination::Destination;
pub use error::WebhookError;
pub use limits::TransportLimits;
pub use transport::Webhook;
