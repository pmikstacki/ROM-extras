//! Host-defined provider using only exported map contracts.
use rom_map_core::{
    Attribution, BrowserPolicy, Cancellation, Capabilities, Error, ProviderFuture, RasterSource,
    RasterTiles, RequestContext, TileKind, TileSource,
};
use std::time::Duration;
struct CustomHostSource;
impl RasterTiles for CustomHostSource {
    fn raster_tiles<'a>(&'a self, context: &'a RequestContext) -> ProviderFuture<'a, RasterSource> {
        Box::pin(async move {
            context.run(async {
                RasterSource::new(TileSource::from_json(
                    br#"{"tilejson":"3.0.0","tiles":["https://host.example.test/{z}/{x}/{y}.png"],"scheme":"tms","maxzoom":12}"#,
                    TileKind::Raster, &BrowserPolicy::new(&["https://host.example.test"])?,
                    Attribution::new("Independent host dataset", None)?,
                )?)
            }).await
        })
    }
}
pub async fn run() {
    let provider = CustomHostSource;
    let selected = Capabilities::new().with_raster_tiles(&provider);
    let context = RequestContext::new(Duration::from_secs(1), Cancellation::new()).unwrap();
    let raster = selected
        .raster_tiles()
        .unwrap()
        .raster_tiles(&context)
        .await
        .unwrap();
    let descriptor = raster.source().to_browser_json();
    assert_eq!(descriptor["type"], "raster");
    assert_eq!(descriptor["scheme"], "tms");
    assert_eq!(descriptor["credits"][0]["text"], "Independent host dataset");
    assert!(matches!(selected.styles(), Err(Error::Unsupported)));
    assert!(matches!(selected.vector_tiles(), Err(Error::Unsupported)));
    assert!(matches!(selected.routing(), Err(Error::Unsupported)));
}
