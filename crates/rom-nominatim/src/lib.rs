//! Nominatim native JSONv2 normalization.
mod decoding;
pub use decoding::{reverse_results, search_results};
mod adapter;
pub use adapter::{Config, Nominatim};
