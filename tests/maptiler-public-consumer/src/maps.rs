//! Actual authored HTTPS, independent from workspace feature unification.
use rom_map_core::{
    Attribution, Cancellation, Error, RasterTiles, RequestContext, Styles, VectorTiles,
};
use rom_maptiler::{
    AttributionProfile, MapBrowserGrant, MapMetadata, MapTilerMaps, MapTilerRaster, MapTilerStyle,
    MapTilerVector, MapsConfig, TileJsonVersion,
};
use std::{sync::Arc, time::Duration};
fn context() -> RequestContext {
    RequestContext::new(Duration::from_secs(2), Cancellation::new()).unwrap()
}
pub async fn run(endpoint: &str, ca: &str) {
    let metadata = MapMetadata::new(
        "synthetic-secret".into(),
        MapBrowserGrant::new(
            &["https://maps.example.test"],
            "browser-public-token".into(),
        )
        .unwrap(),
        AttributionProfile::new("fixture credit", Attribution::new("Fixture", None).unwrap())
            .unwrap(),
    )
    .unwrap();
    let service = Arc::new(
        MapTilerMaps::new(
            MapsConfig::new(endpoint, "IndependentMaps/1", Duration::ZERO, metadata)
                .unwrap()
                .with_backend_metadata_permission()
                .with_ca(std::fs::read(ca).unwrap())
                .unwrap(),
        )
        .unwrap(),
    );
    let vector =
        MapTilerVector::new(service.clone(), "vector-0001", TileJsonVersion::V2_0).unwrap();
    let tile = vector.vector_tiles(&context()).await.unwrap();
    let first = tile.source().to_browser_json();
    assert_eq!(first["vector_layers"][0]["id"], "Roads/0001");
    assert_eq!(first["maxzoom"], 22);
    assert_eq!(
        first["tiles"][0],
        "https://maps.example.test/tiles/{z}/{x}/{y}.pbf?key=browser-public-token"
    );
    let second = vector
        .vector_tiles(&context())
        .await
        .unwrap()
        .source()
        .to_browser_json();
    assert_ne!(
        first["vector_layers"][0]["fields"]["request"],
        second["vector_layers"][0]["fields"]["request"],
        "each explicit request must reach fixture; no adapter cache"
    );
    let raster = MapTilerRaster::new(service.clone(), "map-0001", TileJsonVersion::V2_0).unwrap();
    let raster = raster
        .raster_tiles(&context())
        .await
        .unwrap()
        .source()
        .to_browser_json();
    assert_eq!(raster["tileSize"], 256);
    let style = MapTilerStyle::new(
        service.clone(),
        "map-0001",
        vec![(
            "Base/0001".into(),
            vector.vector_tiles(&context()).await.unwrap().into_source(),
        )],
    )
    .unwrap();
    let style = style.style(&context()).await.unwrap().to_browser_json();
    assert_eq!(style["style"]["layers"][0]["source-layer"], "Roads/0001");
    assert_eq!(
        style["style"]["glyphs"],
        "https://maps.example.test/fonts/{fontstack}/{range}.pbf?key=browser-public-token"
    );
    assert_eq!(style["credits"][0]["text"], "Fixture");
    for descriptor in [first, raster, style] {
        assert!(!descriptor.to_string().contains("synthetic-secret"));
    }
    for (id, expected) in [
        (
            "rate",
            Error::RateLimited {
                retry_after_seconds: Some(2),
            },
        ),
        ("rate", Error::Unavailable),
        ("redirect", Error::Rejected),
        ("huge", Error::TooLarge),
        ("bad-credit", Error::InvalidResponse),
        ("bad-geometry", Error::InvalidResponse),
        ("empty", Error::InvalidResponse),
    ] {
        let adapter = MapTilerVector::new(service.clone(), id, TileJsonVersion::V2_0).unwrap();
        let error = match adapter.vector_tiles(&context()).await {
            Err(e) => e,
            Ok(_) => panic!("invalid fixture metadata was admitted"),
        };
        assert_eq!(error, expected);
        assert!(!error.to_string().contains("synthetic-secret"));
    }
    let slow = MapTilerVector::new(service.clone(), "slow", TileJsonVersion::V2_0).unwrap();
    let short = RequestContext::new(Duration::from_millis(40), Cancellation::new()).unwrap();
    assert!(matches!(
        slow.vector_tiles(&short).await,
        Err(Error::Timeout)
    ));
    let cancel = Cancellation::new();
    let in_flight = RequestContext::new(Duration::from_secs(2), cancel.clone()).unwrap();
    let (result, ()) = tokio::join!(slow.vector_tiles(&in_flight), async {
        tokio::time::sleep(Duration::from_millis(30)).await;
        cancel.cancel();
    });
    assert!(matches!(result, Err(Error::Cancelled)));
    let held_context = RequestContext::new(Duration::from_millis(40), Cancellation::new()).unwrap();
    let other_context = context();
    let raster = MapTilerRaster::new(service.clone(), "map-0001", TileJsonVersion::V2_0).unwrap();
    let (held, other) = tokio::join!(
        slow.vector_tiles(&held_context),
        raster.raster_tiles(&other_context)
    );
    assert!(matches!(held, Err(Error::Timeout)));
    assert!(
        matches!(other, Err(Error::Unavailable)),
        "shared service must bound admission across capabilities"
    );
    let cancel = Cancellation::new();
    cancel.cancel();
    let cancelled = RequestContext::new(Duration::from_secs(1), cancel).unwrap();
    assert!(matches!(
        vector.vector_tiles(&cancelled).await,
        Err(Error::Cancelled)
    ));
    assert!(vector.vector_tiles(&context()).await.is_ok());
    println!(
        "Independent MapTiler maps: actual controlled HTTPS styles/raster/vector, errors, no cache/retry, deadlines and cancellation passed; not native account or rendering qualification"
    );
}
