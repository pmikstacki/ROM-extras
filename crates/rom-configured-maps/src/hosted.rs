use crate::hosted_document::HostedDocument;
use rom_map_core::{
    Attribution, BrowserPolicy, ProviderFuture, RasterSource, RasterTiles, RequestContext, Result,
    TileKind, TileSource, VectorSource, VectorTiles,
};
struct HostedTiles {
    document: HostedDocument,
    policy: BrowserPolicy,
    credit: Attribution,
    kind: TileKind,
}
impl HostedTiles {
    fn new(
        http: rom_map_http::Config,
        document: &str,
        policy: BrowserPolicy,
        credit: Attribution,
        kind: TileKind,
    ) -> Result<Self> {
        Ok(Self {
            document: HostedDocument::new(http, document)?,
            policy,
            credit,
            kind,
        })
    }
    async fn source(&self, context: &RequestContext) -> Result<TileSource> {
        context
            .run(async {
                let bytes = self.document.bytes(context).await?;
                TileSource::from_json(
                    &bytes,
                    self.kind,
                    &self.policy,
                    Attribution::new(self.credit.text(), self.credit.link())?,
                )
            })
            .await
    }
}
/// Explicit HTTPS raster TileJSON with private backend and browser policies kept separate.
pub struct HostedRaster(HostedTiles);
impl HostedRaster {
    /// Prepare one explicit host filename without connecting or following nested URLs.
    pub fn new(
        http: rom_map_http::Config,
        document: &str,
        policy: BrowserPolicy,
        credit: Attribution,
    ) -> Result<Self> {
        Ok(Self(HostedTiles::new(
            http,
            document,
            policy,
            credit,
            TileKind::Raster,
        )?))
    }
}
impl RasterTiles for HostedRaster {
    fn raster_tiles<'a>(&'a self, context: &'a RequestContext) -> ProviderFuture<'a, RasterSource> {
        Box::pin(async move { RasterSource::new(self.0.source(context).await?) })
    }
}
/// Explicit HTTPS vector TileJSON preserving native layer identifiers.
pub struct HostedVector(HostedTiles);
impl HostedVector {
    /// Prepare one explicit host filename without connecting or following nested URLs.
    pub fn new(
        http: rom_map_http::Config,
        document: &str,
        policy: BrowserPolicy,
        credit: Attribution,
    ) -> Result<Self> {
        Ok(Self(HostedTiles::new(
            http,
            document,
            policy,
            credit,
            TileKind::Vector,
        )?))
    }
}
impl VectorTiles for HostedVector {
    fn vector_tiles<'a>(&'a self, context: &'a RequestContext) -> ProviderFuture<'a, VectorSource> {
        Box::pin(async move { VectorSource::new(self.0.source(context).await?) })
    }
}
