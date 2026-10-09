//! Descriptor admission only; no tile rendering or native service qualification.
use rom_map_core::{Attribution, BrowserPolicy, Error, MapStyle, TileKind, TileSource};
use serde_json::{Value, json};

fn policy() -> BrowserPolicy {
    BrowserPolicy::new(&["https://maps.example.test"]).unwrap()
}
fn source(kind: TileKind, size: Option<Value>) -> rom_map_core::Result<TileSource> {
    let mut native = json!({"tilejson":"3.0.0","tiles":["https://maps.example.test/{z}/{x}/{y}.png"],"vector_layers":[{"id":"Roads/0001","fields":{}}]});
    if let Some(size) = size {
        native["tileSize"] = size;
    }
    TileSource::from_json(
        &serde_json::to_vec(&native).unwrap(),
        kind,
        &policy(),
        Attribution::new("Fixture", None).unwrap(),
    )
}

#[test]
fn standalone_raster_size_is_preserved_in_pixels() {
    for pixels in [128, 256, 512, 1024] {
        assert_eq!(
            source(TileKind::Raster, Some(json!(pixels)))
                .unwrap()
                .to_browser_json()["tileSize"],
            pixels
        );
    }
    assert!(
        source(TileKind::Raster, None)
            .unwrap()
            .to_browser_json()
            .get("tileSize")
            .is_none()
    );
}

#[test]
fn malformed_raster_size_is_rejected_instead_of_silently_defaulted() {
    for size in [
        json!(null),
        json!(true),
        json!("256"),
        json!(0),
        json!(-256),
        json!(255),
        json!(256.5),
        json!(2048),
    ] {
        assert!(matches!(
            source(TileKind::Raster, Some(size)),
            Err(Error::InvalidResponse)
        ));
    }
}

#[test]
fn vector_manifest_cannot_acquire_raster_size() {
    assert!(matches!(
        source(TileKind::Vector, Some(json!(256))),
        Err(Error::InvalidResponse)
    ));
}

#[test]
fn resolved_raster_size_survives_inlining_and_explicit_override() {
    let tile = source(TileKind::Raster, Some(json!(256))).unwrap();
    for override_size in [None, Some(512)] {
        let mut native = json!({"version":8,"sources":{"Base/0001":{"type":"raster","url":"https://maps.example.test/tiles.json"}},"layers":[{"id":"Base layer","type":"raster","source":"Base/0001"}]});
        if let Some(size) = override_size {
            native["sources"]["Base/0001"]["tileSize"] = json!(size);
        }
        let prepared = MapStyle::from_json(
            &serde_json::to_vec(&native).unwrap(),
            &policy(),
            &[Attribution::new("Host", None).unwrap()],
            &[("Base/0001", &tile)],
        )
        .unwrap()
        .to_browser_json();
        assert_eq!(
            prepared["style"]["sources"]["Base/0001"]["tileSize"],
            override_size.unwrap_or(256)
        );
        assert_eq!(prepared["style"]["layers"][0]["source"], "Base/0001");
    }
}
