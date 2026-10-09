use crate::tile_document::TileDocument;
use rom_map_core::{
    Attribution, BrowserPolicy, ProviderFuture, RasterSource, RasterTiles, RequestContext, Result,
    TileKind,
};
/// Host-approved raster TileJSON; no implicit network activity.
pub struct ConfiguredRaster(TileDocument);
impl ConfiguredRaster {
    /// Validate the host document, browser origins and attribution before activation.
    pub fn new(bytes: &[u8], policy: BrowserPolicy, credit: Attribution) -> Result<Self> {
        Ok(Self(TileDocument::new(
            bytes,
            policy,
            credit,
            TileKind::Raster,
        )?))
    }
}
impl RasterTiles for ConfiguredRaster {
    fn raster_tiles<'a>(&'a self, context: &'a RequestContext) -> ProviderFuture<'a, RasterSource> {
        Box::pin(async move {
            context
                .run(async { RasterSource::new(self.0.source()?) })
                .await
        })
    }
}
