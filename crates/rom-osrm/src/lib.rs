//! OSRM route normalization with explicit graph provenance and units.
mod decoding;
pub use decoding::route_response;

mod adapter;
pub use adapter::{Config, Osrm};
