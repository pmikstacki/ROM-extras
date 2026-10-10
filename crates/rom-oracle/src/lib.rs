//! Native Oracle ownership foundation over shared SQL arbitration and a private OCI worker.
//! OCI timeout bounds each round trip; absolute operation/socket containment and full Storage remain unqualified.
mod config;
mod connection;
mod metadata;
mod transaction;
mod value;
mod worker;
pub use config::{Config, ControlTable, Deadlines, TlsWallet};
pub use connection::Connection;
pub use transaction::Transaction;
pub use value::Value;
