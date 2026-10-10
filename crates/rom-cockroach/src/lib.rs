//! Maintained CockroachDB native ownership over shared bounded PGwire.
//! Requires explicit26.3 SERIALIZABLE durable-lock settings; no automatic retry or provisioning.
//! Local ownership qualification does not establish full ROM Storage or distributed failure support.
mod config;
mod connection;
mod transaction;
pub use config::{ControlTable, Deadlines};
pub use connection::Connection;
pub use tokio_postgres::{Config, Socket, types::ToSql};
pub use transaction::Transaction;
