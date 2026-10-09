//! Public adapter selection without service requests.
use rom_map_core::{Attribution, Capabilities, Error};
use rom_maptiler::{
    AttributionProfile, MapBrowserGrant, MapMetadata, MapTilerMaps, MapTilerRaster, MapTilerStyle,
    MapTilerVector, MapsConfig, TileJsonVersion,
};
use std::{sync::Arc, time::Duration};
fn config() -> MapsConfig {
    MapsConfig::new(
        "https://backend.example.test/operator/",
        "Host/1",
        Duration::ZERO,
        MapMetadata::new(
            "server-fixture-key".into(),
            MapBrowserGrant::new(
                &["https://maps.example.test"],
                "browser-public-token".into(),
            )
            .unwrap(),
            AttributionProfile::new("native", Attribution::new("Fixture", None).unwrap()).unwrap(),
        )
        .unwrap(),
    )
    .unwrap()
}
#[test]
fn backend_metadata_permission_is_required_before_service_preparation() {
    assert!(matches!(MapTilerMaps::new(config()), Err(Error::Rejected)));
    assert!(MapBrowserGrant::new(&[], "browser-public-token".into()).is_err());
}
#[test]
fn each_adapter_selects_only_its_capability_and_literal_service_ids() {
    let service = Arc::new(MapTilerMaps::new(config().with_backend_metadata_permission()).unwrap());
    let raster = MapTilerRaster::new(service.clone(), "streets-v4", TileJsonVersion::V2_0).unwrap();
    let vector =
        MapTilerVector::new(service.clone(), "tileset-0001", TileJsonVersion::V2_0).unwrap();
    let style = MapTilerStyle::new(service.clone(), "streets-v4", vec![]).unwrap();
    let selected = Capabilities::new()
        .with_styles(&style)
        .with_raster_tiles(&raster)
        .with_vector_tiles(&vector);
    assert!(selected.styles().is_ok());
    assert!(selected.raster_tiles().is_ok());
    assert!(selected.vector_tiles().is_ok());
    assert!(matches!(selected.geocoding(), Err(Error::Unsupported)));
    assert!(matches!(selected.routing(), Err(Error::Unsupported)));
    for id in ["", "..", "a/b", "a?key=x", "a%2fb", "a#x", " a", "a\\b"] {
        assert!(MapTilerRaster::new(service.clone(), id, TileJsonVersion::V2_0).is_err());
        assert!(MapTilerVector::new(service.clone(), id, TileJsonVersion::V2_0).is_err());
        assert!(MapTilerStyle::new(service.clone(), id, vec![]).is_err());
    }
}
