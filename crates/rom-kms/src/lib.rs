//! Host-owned authenticated encryption contracts, separate from ROM mutation.
mod binding;
mod encryption_version;
mod envelope;
mod migration;
mod provider;
pub use binding::Binding;
pub use encryption_version::EncryptionVersion;
pub use envelope::{Algorithm, Envelope, KeyRef};
pub use migration::{MigrationLimits, Migrator};
pub use provider::Kms;
