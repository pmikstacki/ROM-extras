//! Bounded OpenSearch projection target with strict external revisions and persistent tombstones.
mod adapter;
mod config;
mod mapping;
mod search;
mod search_response;
mod transport;
mod writes;
pub use adapter::OpenSearch;
pub use config::TlsConfig;
pub use writes::PreparedPage;

#[cfg(test)]
mod search_response_tests;

#[cfg(test)]
extern crate self as rom_opensearch;
#[cfg(all(test, feature = "service-fixture"))]
#[path = "native_partial_tests.rs"]
mod partial_bulk;
#[cfg(all(test, feature = "service-fixture"))]
mod transport_tests;
