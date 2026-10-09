//! Source-kind admission for independently selectable raster and vector ports.
use super::{TileKind, TileSource};
use crate::{Error, Result};
/// Approved raster metadata; cannot contain a vector source.
pub struct RasterSource(TileSource);
impl RasterSource {
    /// Require the host-admitted raster kind.
    pub fn new(source: TileSource) -> Result<Self> {
        if !matches!(source.kind(), TileKind::Raster) {
            return Err(Error::InvalidResponse);
        }
        Ok(Self(source))
    }
    /// Approved raster source descriptor without automatic connections.
    pub fn source(&self) -> &TileSource {
        &self.0
    }
}
/// Approved vector metadata with original layer identifiers.
pub struct VectorSource(TileSource);
impl VectorSource {
    /// Require the host-admitted vector kind.
    pub fn new(source: TileSource) -> Result<Self> {
        if !matches!(source.kind(), TileKind::Vector) {
            return Err(Error::InvalidResponse);
        }
        Ok(Self(source))
    }
    /// Approved vector source descriptor without automatic connections.
    pub fn source(&self) -> &TileSource {
        &self.0
    }
}
