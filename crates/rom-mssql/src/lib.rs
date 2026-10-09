//! Native SQL Server ownership foundation, not a ROM Storage implementation.
//!
//! Use a worker-owned [`Connection`] through `rom_sql_core::Executor`.
//! Provision the documented control table separately. All arbitration uses
//! `rom_sql_core` ownership operations. No automatic DDL, retry or reconnect occurs.
//! Prepared SQL belongs to trusted persistence drivers, never Resource action hooks.
mod config;
mod connection;
#[cfg(test)]
mod tests;
mod transaction;
pub use config::{ControlTable, Deadlines};
pub use connection::Connection;
/// Native configuration and parameter types for trusted driver integration.
pub use tiberius::{AuthMethod, Config, ToSql};
pub use transaction::Transaction;
