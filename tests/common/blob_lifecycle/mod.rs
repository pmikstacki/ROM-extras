//! Shared public BlobService acceptance against real provider and native store ports.
mod attached;
mod helpers;
mod observed;
mod unattached;

pub(crate) use attached::staged_content_authorization_and_detachment;
pub(crate) use unattached::deleted_reservation_returns_unattached;
