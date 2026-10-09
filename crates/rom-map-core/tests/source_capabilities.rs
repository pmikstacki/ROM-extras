//! Independent source-port admission without activating a renderer or network.
use rom_map_core::{
    Attribution, BrowserPolicy, Cancellation, Capabilities, Error, ProviderFuture, RasterSource,
    RasterTiles, RequestContext, TileKind, TileSource, VectorSource,
};
use std::time::Duration;

struct HostRaster;
impl RasterTiles for HostRaster {
    fn raster_tiles<'a>(&'a self, context: &'a RequestContext) -> ProviderFuture<'a, RasterSource> {
        Box::pin(async move {
            context.run(async {
                let policy = BrowserPolicy::new(&["https://maps.example.test"])?;
                RasterSource::new(TileSource::from_json(
                    br#"{"tilejson":"3.0.0","tiles":["https://maps.example.test/{z}/{x}/{y}.png"],"scheme":"tms","maxzoom":12}"#,
                    TileKind::Raster, &policy, Attribution::new("Host dataset", None)?,
                )?)
            }).await
        })
    }
}

#[tokio::test(flavor = "current_thread")]
async fn raster_capability_does_not_require_style_vector_or_geocoder() {
    let provider = HostRaster;
    let selected = Capabilities::new().with_raster_tiles(&provider);
    assert!(matches!(selected.styles(), Err(Error::Unsupported)));
    assert!(matches!(selected.vector_tiles(), Err(Error::Unsupported)));
    assert!(matches!(selected.geocoding(), Err(Error::Unsupported)));
    let context = RequestContext::new(Duration::from_secs(1), Cancellation::new()).unwrap();
    let source = selected
        .raster_tiles()
        .unwrap()
        .raster_tiles(&context)
        .await
        .unwrap();
    assert_eq!(source.source().to_browser_json()["scheme"], "tms");
    assert_eq!(source.source().to_browser_json()["maxzoom"], 12);
}

#[test]
fn raster_and_vector_capability_values_reject_wrong_native_kind() {
    let policy = BrowserPolicy::new(&["https://maps.example.test"]).unwrap();
    let vector = TileSource::from_json(br#"{"tilejson":"3.0.0","tiles":["https://maps.example.test/{z}/{x}/{y}.pbf"],"vector_layers":[{"id":"Roads/0001","fields":{}}]}"#, TileKind::Vector, &policy, Attribution::new("Dataset", None).unwrap()).unwrap();
    assert!(matches!(
        RasterSource::new(vector),
        Err(Error::InvalidResponse)
    ));
    let raster = TileSource::from_json(
        br#"{"tilejson":"3.0.0","tiles":["https://maps.example.test/{z}/{x}/{y}.png"]}"#,
        TileKind::Raster,
        &policy,
        Attribution::new("Dataset", None).unwrap(),
    )
    .unwrap();
    assert!(matches!(
        VectorSource::new(raster),
        Err(Error::InvalidResponse)
    ));
}
