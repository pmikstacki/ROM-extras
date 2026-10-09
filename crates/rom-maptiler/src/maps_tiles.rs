//! Independent map-raster and vector-tileset capabilities.
use crate::{MapTilerMaps, TileJsonVersion};
use rom_map_core::{
    ProviderFuture, RasterSource, RasterTileSize, RasterTiles, RequestContext, Result, TileKind,
    VectorSource, VectorTiles,
};
use std::sync::Arc;
/// Raster metadata for the Maps API's explicit256 logical pixel variant.
pub struct MapTilerRaster {
    service: Arc<MapTilerMaps>,
    id: String,
    version: TileJsonVersion,
}
impl MapTilerRaster {
    /// Bind a literal map ID and expected native TileJSON version without I/O.
    pub fn new(service: Arc<MapTilerMaps>, map_id: &str, version: TileJsonVersion) -> Result<Self> {
        Ok(Self {
            service,
            id: super::maps_service::service_id(map_id)?,
            version,
        })
    }
}
impl RasterTiles for MapTilerRaster {
    fn raster_tiles<'a>(&'a self, context: &'a RequestContext) -> ProviderFuture<'a, RasterSource> {
        Box::pin(async move {
            context
                .run(async {
                    let bytes = self
                        .service
                        .http
                        .get(&["maps", &self.id, "256", "tiles.json"], &[], context)
                        .await?;
                    RasterSource::new(self.service.metadata.tilejson(
                        &bytes,
                        self.version,
                        TileKind::Raster,
                        Some(RasterTileSize::Pixels256),
                    )?)
                })
                .await
        })
    }
}
/// Vector metadata for an independently selected Tiles API tileset.
pub struct MapTilerVector {
    service: Arc<MapTilerMaps>,
    id: String,
    version: TileJsonVersion,
}
impl MapTilerVector {
    /// Bind a literal tileset ID and expected native version without guessing a map-to-tileset mapping.
    pub fn new(
        service: Arc<MapTilerMaps>,
        tileset_id: &str,
        version: TileJsonVersion,
    ) -> Result<Self> {
        Ok(Self {
            service,
            id: super::maps_service::service_id(tileset_id)?,
            version,
        })
    }
}
impl VectorTiles for MapTilerVector {
    fn vector_tiles<'a>(&'a self, context: &'a RequestContext) -> ProviderFuture<'a, VectorSource> {
        Box::pin(async move {
            context
                .run(async {
                    let bytes = self
                        .service
                        .http
                        .get(&["tiles", &self.id, "tiles.json"], &[], context)
                        .await?;
                    VectorSource::new(self.service.metadata.tilejson(
                        &bytes,
                        self.version,
                        TileKind::Vector,
                        None,
                    )?)
                })
                .await
        })
    }
}
