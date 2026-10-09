//! Fixed host-owned Qdrant writes with bounded preparation and exact native stored-point reconciliation.
mod writes;
pub use writes::PreparedWrite;
mod generation;
#[cfg(test)]
mod writes_tests;
pub use generation::{Distance, Generation};
mod adapter;
mod page;
mod reconcile;
pub use adapter::Qdrant;
pub use page::PreparedPage;
pub use rom_projection_http::TlsConfig;
#[cfg(test)]
mod reconcile_tests;
