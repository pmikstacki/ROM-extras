//! Invalid requests and cancellation must terminate before any connection attempt.
use rom_map_core::{
    Attribution, BoundingBox, Cancellation, Coordinate, Error, GeocodeQuery, Geocoding,
    RequestContext, ReverseGeocoding,
};
use rom_maptiler::{AttributionProfile, Config, MapTiler};
use std::time::Duration;
fn adapter() -> MapTiler {
    let attribution =
        AttributionProfile::new("fixture", Attribution::new("Fixture", None).unwrap()).unwrap();
    MapTiler::new(
        Config::new(
            "https://127.0.0.1:1/",
            "ROM fixture",
            "fixture",
            Duration::ZERO,
            attribution,
        )
        .unwrap()
        .with_server_key("synthetic-private-key".into())
        .unwrap(),
    )
    .unwrap()
}
fn context() -> RequestContext {
    RequestContext::new(Duration::from_secs(1), Cancellation::new()).unwrap()
}
#[tokio::test]
async fn native_dispatch_and_service_limits_do_not_change_text_capability() {
    let service = adapter();
    for (text, limit) in [("21,52", 1), ("Paris;Rome", 1), ("Paris", 11)] {
        let query = GeocodeQuery::new(text, limit).unwrap();
        assert!(matches!(
            service.geocode(&query, &context()).await,
            Err(Error::InvalidQuery)
        ));
    }
    let query = GeocodeQuery::new("Paris", 1)
        .unwrap()
        .with_bounds(BoundingBox::new(170.0, -10.0, -170.0, 10.0).unwrap());
    assert!(matches!(
        service.geocode(&query, &context()).await,
        Err(Error::Unsupported)
    ));
}
#[tokio::test]
async fn already_cancelled_host_context_prevents_both_capabilities() {
    let service = adapter();
    let cancellation = Cancellation::new();
    cancellation.cancel();
    let context = RequestContext::new(Duration::from_secs(1), cancellation).unwrap();
    assert!(matches!(
        service
            .geocode(&GeocodeQuery::new("Paris", 1).unwrap(), &context)
            .await,
        Err(Error::Cancelled)
    ));
    assert!(matches!(
        service
            .reverse(Coordinate::new(21.0, 52.0).unwrap(), &context)
            .await,
        Err(Error::Cancelled)
    ));
}
