//! Shared bounded transport; native response semantics remain provider responsibilities.
mod config;
pub use config::TlsConfig;
mod transport;
pub use transport::{Http, WIRE_LIMIT};
