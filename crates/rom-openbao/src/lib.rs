//! Bounded host-owned OpenBao KV v2 and derived Transit adapter.
//! Credentials and resolved material are never Resource fields or diagnostics.
//! Administrative key creation, rotation, unseal and token renewal stay with hosts.
mod adapter;
mod configuration;
mod locations;
mod requests;
mod secrets;
mod transit;
mod transport;
pub use adapter::OpenBao;
pub use configuration::{Config, Limits};
pub use locations::{KeyLocation, SecretLocation};

#[cfg(test)]
mod transit_tests;
