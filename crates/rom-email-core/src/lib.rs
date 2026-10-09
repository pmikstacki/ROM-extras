//! Pure, bounded, host-approved RFC email preparation for public ROM channels.
//! No transport, credentials, retry or alternative Work ledger is provided.
mod address;
mod error;
mod payload;
mod prepared;
mod profile;
pub use error::EmailError;
pub use payload::EmailNotification;
pub use prepared::PreparedEmail;
pub use profile::EmailProfile;
