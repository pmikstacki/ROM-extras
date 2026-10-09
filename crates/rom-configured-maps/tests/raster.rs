//! Explicit host TileJSON admission and cancelled descriptor access.
use rom_configured_maps::ConfiguredRaster;
use rom_map_core::{Attribution, BrowserPolicy, Cancellation, Error, RasterTiles, RequestContext};
use std::time::Duration;
fn policy() -> BrowserPolicy {
    BrowserPolicy::new(&["https://tiles.example.test"]).unwrap()
}
fn credit() -> Attribution {
    Attribution::new("Host authored tiles", None).unwrap()
}
const VALID: &[u8] =
    br#"{"tilejson":"3.0.0","tiles":["https://tiles.example.test/{z}/{x}/{y}.png"]}"#;
#[tokio::test]
async fn explicit_configuration_returns_approved_tiles_and_respects_cancellation() {
    let provider = ConfiguredRaster::new(VALID, policy(), credit()).unwrap();
    let cancel = Cancellation::new();
    let context = RequestContext::new(Duration::from_secs(1), cancel.clone()).unwrap();
    let approved = provider.raster_tiles(&context).await.unwrap();
    assert_eq!(
        approved.source().to_browser_json()["tiles"][0],
        "https://tiles.example.test/{z}/{x}/{y}.png"
    );
    cancel.cancel();
    assert!(matches!(
        provider.raster_tiles(&context).await,
        Err(Error::Cancelled)
    ));
}
#[test]
fn unapproved_origin_and_oversized_configuration_fail_before_activation() {
    assert!(
        ConfiguredRaster::new(
            br#"{"tilejson":"3.0.0","tiles":["https://evil.test/{z}/{x}/{y}.png"]}"#,
            policy(),
            credit()
        )
        .is_err()
    );
    assert!(matches!(
        ConfiguredRaster::new(&vec![b' '; 1048577], policy(), credit()),
        Err(Error::TooLarge)
    ));
}
