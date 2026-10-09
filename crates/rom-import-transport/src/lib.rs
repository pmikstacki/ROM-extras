//! Host-opened regular files and optional explicit conditional HTTPS imports.
//! Acquisition feeds the unchanged pure sourced-action core; hosts own execution.
mod error;
mod file;
pub use error::Error;
pub use file::prepare_file;
#[cfg(feature = "http")]
mod config;
#[cfg(feature = "http")]
mod context;
#[cfg(feature = "http")]
mod etag;
#[cfg(feature = "http")]
mod http;
#[cfg(feature = "http")]
mod response;
#[cfg(feature = "http")]
pub use config::HttpConfig;
#[cfg(feature = "http")]
pub use context::RequestContext;
#[cfg(feature = "http")]
pub use etag::StrongEtag;
#[cfg(feature = "http")]
pub use http::HttpSource;
