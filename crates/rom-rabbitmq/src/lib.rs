//! Mandatory confirmed publication of bounded ROM channel payloads.
mod delivery;
mod error;
pub use delivery::RabbitMqDelivery;
pub use error::RabbitMqError;
