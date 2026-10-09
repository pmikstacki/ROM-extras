//! Native PostgreSQL ownership foundation, not a ROM Storage implementation.
//! Use a worker-owned Connection through rom_sql_core::Executor. Provision separately.
//! No retry/reconnect or application SQL hook is added; host TLS configuration stays explicit.
mod config;
mod connection;
mod transaction;
pub use config::{ControlTable, Deadlines};
pub use connection::Connection;
/// Host native configuration and parameter types for trusted driver integration.
pub use tokio_postgres::{Config, Socket, types::ToSql};
pub use transaction::Transaction;
