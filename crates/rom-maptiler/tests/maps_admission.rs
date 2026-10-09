//! Authored metadata admission, without actual account or renderer qualification.
use rom_map_core::{Attribution, Error, RasterTileSize, TileKind};
use rom_maptiler::{AttributionProfile, MapBrowserGrant, MapMetadata, TileJsonVersion};
use serde_json::{Value, json};
fn metadata() -> MapMetadata {
    MapMetadata::new(
        "server-only-fixture-key".into(),
        MapBrowserGrant::new(
            &["https://maps.example.test"],
            "browser-public-token".into(),
        )
        .unwrap(),
        AttributionProfile::new(
            "<a>Fixture native</a>",
            Attribution::new("Fixture approved", None).unwrap(),
        )
        .unwrap(),
    )
    .unwrap()
}
fn tile() -> Value {
    json!({"tilejson":"2.0.0","attribution":"<a>Fixture native</a>","tiles":["https://maps.example.test/tiles/{z}/{x}/{y}.pbf?key=server-only-fixture-key"],"vector_layers":[{"id":"Roads/0001","fields":{}}]})
}
#[test]
fn exact_browser_grant_replaces_only_resource_credentials_and_retains_templates() {
    let prepared = metadata()
        .tilejson(
            &serde_json::to_vec(&tile()).unwrap(),
            TileJsonVersion::V2_0,
            TileKind::Vector,
            None,
        )
        .unwrap()
        .to_browser_json();
    assert_eq!(
        prepared["tiles"][0],
        "https://maps.example.test/tiles/{z}/{x}/{y}.pbf?key=browser-public-token"
    );
    assert_eq!(prepared["vector_layers"][0]["id"], "Roads/0001");
    assert_eq!(prepared["maxzoom"], 22);
    assert_eq!(prepared["credits"][0]["text"], "Fixture approved");
    assert!(!prepared.to_string().contains("server-only-fixture-key"));
}
#[test]
fn unknown_keys_origins_and_native_credits_never_become_browser_metadata() {
    for url in [
        "https://maps.example.test/x?key=unknown",
        "https://maps.example.test/x?key=server-only-fixture-key&x=1",
        "https://maps.example.test/x?key=server-only-fixture-key&key=server-only-fixture-key",
        "https://external.test/x?key=server-only-fixture-key",
        "https://user@maps.example.test/x?key=server-only-fixture-key",
        "https://maps.example.test/x?key=server-only-fixture-key#fragment",
    ] {
        let mut native = tile();
        native["tiles"] = json!([url]);
        assert!(
            metadata()
                .tilejson(
                    &serde_json::to_vec(&native).unwrap(),
                    TileJsonVersion::V2_0,
                    TileKind::Vector,
                    None
                )
                .is_err()
        );
    }
    let mut native = tile();
    native["attribution"] = json!("<script>unknown credit</script>");
    assert!(matches!(
        metadata().tilejson(
            &serde_json::to_vec(&native).unwrap(),
            TileJsonVersion::V2_0,
            TileKind::Vector,
            None
        ),
        Err(Error::InvalidResponse)
    ));
}
#[test]
fn selected_version_defaults_and_raster_size_are_not_inferred() {
    for (version, selected, max) in [
        ("2.0.0", TileJsonVersion::V2_0, 22),
        ("2.1.0", TileJsonVersion::V2_1, 22),
        ("2.2.0", TileJsonVersion::V2_2, 30),
        ("3.0.0", TileJsonVersion::V3_0, 30),
    ] {
        let mut native = tile();
        native["tilejson"] = json!(version);
        let prepared = metadata()
            .tilejson(
                &serde_json::to_vec(&native).unwrap(),
                selected,
                TileKind::Raster,
                Some(RasterTileSize::Pixels256),
            )
            .unwrap()
            .to_browser_json();
        assert_eq!(prepared["maxzoom"], max);
        assert_eq!(prepared["tileSize"], 256);
    }
    let mut native = tile();
    native["maxzoom"] = json!(23);
    assert!(
        metadata()
            .tilejson(
                &serde_json::to_vec(&native).unwrap(),
                TileJsonVersion::V2_0,
                TileKind::Vector,
                None
            )
            .is_err()
    );
    assert!(
        metadata()
            .tilejson(
                &serde_json::to_vec(&tile()).unwrap(),
                TileJsonVersion::V3_0,
                TileKind::Vector,
                None
            )
            .is_err()
    );
}
#[test]
fn styles_resolve_exact_source_ids_and_keep_credits_without_server_key() {
    let metadata = metadata();
    let tile = metadata
        .tilejson(
            &serde_json::to_vec(&tile()).unwrap(),
            TileJsonVersion::V2_0,
            TileKind::Vector,
            None,
        )
        .unwrap();
    let native = json!({"version":8,"sources":{"Base/0001":{"type":"vector","url":"https://maps.example.test/tiles.json?key=server-only-fixture-key"}},"glyphs":"https://maps.example.test/fonts/{fontstack}/{range}.pbf?key=server-only-fixture-key","sprite":[{"id":"main","url":"https://maps.example.test/sprites?key=server-only-fixture-key"}],"layers":[{"id":"道路","type":"line","source":"Base/0001","source-layer":"Roads/0001"}]});
    let bytes = serde_json::to_vec(&native).unwrap();
    assert!(matches!(
        metadata.style(&bytes, &[]),
        Err(Error::Unsupported)
    ));
    let prepared = metadata
        .style(&bytes, &[("Base/0001", &tile)])
        .unwrap()
        .to_browser_json();
    assert_eq!(prepared["style"]["layers"][0]["id"], "道路");
    assert_eq!(
        prepared["style"]["glyphs"],
        "https://maps.example.test/fonts/{fontstack}/{range}.pbf?key=browser-public-token"
    );
    assert_eq!(
        prepared["style"]["sprite"][0]["url"],
        "https://maps.example.test/sprites?key=browser-public-token"
    );
    assert!(!prepared.to_string().contains("server-only-fixture-key"));
}
#[test]
fn server_key_cannot_be_reused_as_browser_token_or_disclosed_in_style_names() {
    assert!(
        MapMetadata::new(
            "server-only-fixture-key".into(),
            MapBrowserGrant::new(
                &["https://maps.example.test"],
                "server-only-fixture-key".into()
            )
            .unwrap(),
            AttributionProfile::new("native", Attribution::new("Fixture", None).unwrap()).unwrap()
        )
        .is_err()
    );
    let native = json!({"version":8,"name":"server-only-fixture-key","sources":{},"layers":[]});
    assert!(matches!(
        metadata().style(&serde_json::to_vec(&native).unwrap(), &[]),
        Err(Error::Rejected)
    ));
}

#[test]
fn percent_encoded_server_key_in_resource_paths_cannot_escape() {
    for encoded in [
        "%73erver-only-fixture-key",
        "%2573erver-only-fixture-key",
        "server-only-fixture-%6bey",
    ] {
        for suffix in ["", "?key=server-only-fixture-key"] {
            let url = format!("https://maps.example.test/{encoded}/{{z}}/{{x}}/{{y}}.pbf{suffix}");
            let mut native = tile();
            native["tiles"] = json!([url]);
            assert!(matches!(
                metadata().tilejson(
                    &serde_json::to_vec(&native).unwrap(),
                    TileJsonVersion::V2_0,
                    TileKind::Vector,
                    None
                ),
                Err(Error::Rejected)
            ));
            for field in ["glyphs", "sprite"] {
                let mut native = json!({"version":8,"sources":{},"layers":[]});
                native[field] = json!(if field == "glyphs" {
                    format!(
                        "https://maps.example.test/{encoded}/{{fontstack}}/{{range}}.pbf{suffix}"
                    )
                } else {
                    format!("https://maps.example.test/{encoded}/sprites{suffix}")
                });
                assert!(matches!(
                    metadata().style(&serde_json::to_vec(&native).unwrap(), &[]),
                    Err(Error::Rejected)
                ));
            }
        }
    }
}
