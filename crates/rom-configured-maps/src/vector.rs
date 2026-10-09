use crate::tile_document::TileDocument;
use rom_map_core::{
    Attribution, BrowserPolicy, ProviderFuture, RequestContext, Result, TileKind, VectorSource,
    VectorTiles,
};
/// Host-approved vector TileJSON preserving native layer identifiers.
pub struct ConfiguredVector(TileDocument);
impl ConfiguredVector {
    /// Validate the host document, browser origins and attribution before activation.
    pub fn new(bytes: &[u8], policy: BrowserPolicy, credit: Attribution) -> Result<Self> {
        Ok(Self(TileDocument::new(
            bytes,
            policy,
            credit,
            TileKind::Vector,
        )?))
    }
}
impl VectorTiles for ConfiguredVector {
    fn vector_tiles<'a>(&'a self, context: &'a RequestContext) -> ProviderFuture<'a, VectorSource> {
        Box::pin(async move {
            context
                .run(async { VectorSource::new(self.0.source()?) })
                .await
        })
    }
}
