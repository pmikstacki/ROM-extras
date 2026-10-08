//! Host-owned authenticated encryption contracts, separate from ROM mutation.
mod binding;
mod envelope;
mod provider;
pub use binding::Binding;
pub use envelope::{Algorithm, Envelope, KeyRef};
pub use provider::Kms;
