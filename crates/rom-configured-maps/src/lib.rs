//! Host-configured source capabilities without implicit network activity.
mod raster;
mod style;
mod tile_document;
mod vector;
pub use raster::ConfiguredRaster;
pub use style::ConfiguredStyle;
pub use vector::ConfiguredVector;
mod hosted;
pub use hosted::{HostedRaster, HostedVector};

mod hosted_document;
mod hosted_style;
pub use hosted_style::HostedStyle;
