//! Bounded map-service reads without projection write semantics.
mod config;
pub use config::Config;
mod transport;
pub use transport::Http;
mod request_url;
mod response;

mod native_response;
pub use native_response::{NativeResponse, ReadStatus};
