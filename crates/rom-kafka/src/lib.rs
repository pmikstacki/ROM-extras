//! Bounded Kafka publication through a privately configured acknowledged producer.
mod config;
mod delivery;
mod error;
pub use delivery::KafkaDelivery;
pub use error::KafkaDeliveryError;
#[cfg(test)]
mod config_tests;
