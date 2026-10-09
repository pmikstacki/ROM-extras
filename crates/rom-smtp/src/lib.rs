//! One-attempt authenticated implicit-TLS SMTP submission for public ROM channels.
//! Runtime owns retry and reconciliation. SMTP acknowledgment is not inbox delivery.
mod attempt;
mod bounded_stream;
mod credentials;
mod endpoint;
mod error;
mod limits;
mod transport;
pub use credentials::Credentials;
pub use endpoint::Endpoint;
pub use error::SmtpError;
pub use limits::Limits;
pub use transport::Smtp;
