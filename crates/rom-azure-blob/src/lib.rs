//! Host-owned Azure Blob integration through ROM's published blob boundary.
//!
//! Credentials stay in host configuration. This adapter supplies create-only
//! publication, bounded ETag-conditioned reads and honest uncertain writes.
//! Emulator evidence does not establish live Azure support.
mod adapter;
mod configuration;
mod errors;

pub use adapter::AzureBlob;
pub use configuration::{AzureConfig, Credentials, EndpointPolicy, Limits};
