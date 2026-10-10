//! Bounded native PGwire lifecycle; provider session and ownership policy remain separate.
mod connection;
mod error;
mod metadata;

pub use connection::Connection;
pub use error::Error;
pub use metadata::MetadataLimits;
pub use tokio_postgres::{Config, Row, Socket, error::SqlState, types::ToSql};
