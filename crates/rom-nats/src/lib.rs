//! Acknowledged NATS JetStream delivery. Runtime owns durable intents and retry.
mod delivery;
mod error;

pub use delivery::JetStreamDelivery;
pub use error::NatsDeliveryError;
