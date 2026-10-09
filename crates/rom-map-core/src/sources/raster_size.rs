//! Finite raster decoder profile, with explicit pixel units.
use crate::{Error, Result, TileKind};
use serde_json::Value;

/// Supported logical raster tile sizes in pixels; independent of device pixel ratio.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RasterTileSize {
    /// 128 logical pixels.
    Pixels128,
    /// 256 logical pixels.
    Pixels256,
    /// 512 logical pixels.
    Pixels512,
    /// 1024 logical pixels.
    Pixels1024,
}
impl RasterTileSize {
    /// Explicit pixel count for a browser source descriptor.
    pub fn pixels(self) -> u16 {
        match self {
            Self::Pixels128 => 128,
            Self::Pixels256 => 256,
            Self::Pixels512 => 512,
            Self::Pixels1024 => 1024,
        }
    }
}
impl TryFrom<u16> for RasterTileSize {
    type Error = Error;
    fn try_from(pixels: u16) -> Result<Self> {
        match pixels {
            128 => Ok(Self::Pixels128),
            256 => Ok(Self::Pixels256),
            512 => Ok(Self::Pixels512),
            1024 => Ok(Self::Pixels1024),
            _ => Err(Error::InvalidResponse),
        }
    }
}
pub(crate) fn prepare(native: Option<&Value>, kind: TileKind) -> Result<Option<RasterTileSize>> {
    native
        .map(|value| {
            if !matches!(kind, TileKind::Raster) {
                return Err(Error::InvalidResponse);
            }
            let pixels = value
                .as_u64()
                .and_then(|pixels| u16::try_from(pixels).ok())
                .ok_or(Error::InvalidResponse)?;
            RasterTileSize::try_from(pixels)
        })
        .transpose()
}
