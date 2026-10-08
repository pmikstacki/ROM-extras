//! Shared public SQL executor acceptance over the PostgreSQL wire protocol.
pub(crate) mod cases;
mod connection;
pub(crate) use cases::{deadline, panic_rollback};
pub(crate) use connection::connection;
