//! MapTiler geocoding normalization outside Resource authorization and browser rendering.
mod decoding;
pub use decoding::{AttributionProfile, geocode_response};
mod adapter;
pub use adapter::{Config, MapTiler};
mod geocoding;
