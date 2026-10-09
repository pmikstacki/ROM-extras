//! Synthetic style boundary evidence; no renderer or remote-service qualification.
use rom_map_core::{Attribution, Error};
use rom_map_core::{BrowserPolicy, MapStyle, TileKind, TileSource};
use serde_json::{Value, json};
fn policy() -> BrowserPolicy {
    BrowserPolicy::new(&["https://maps.example.test"]).unwrap()
}
fn credits() -> Vec<Attribution> {
    vec![
        Attribution::new("Dataset A", Some("https://maps.example.test/credit")).unwrap(),
        Attribution::new("Dataset B", None).unwrap(),
    ]
}
fn style() -> Value {
    json!({"version":8,"sources":{"Base/0001":{"type":"raster","tiles":["https://maps.example.test/{z}/{x}/{y}.png"],"tileSize":256}},"layers":[{"id":"Base layer","type":"raster","source":"Base/0001"}]})
}
fn prepare(value: Value) -> rom_map_core::Result<MapStyle> {
    MapStyle::from_json(
        &serde_json::to_vec(&value).unwrap(),
        &policy(),
        &credits(),
        &[],
    )
}
#[test]
fn inline_sources_and_multiple_credits_are_approved_without_activation() {
    let browser = prepare(style()).unwrap().to_browser_json();
    assert_eq!(browser["style"]["sources"]["Base/0001"]["tileSize"], 256);
    assert_eq!(browser["style"]["layers"][0]["source"], "Base/0001");
    assert_eq!(browser["credits"][0]["text"], "Dataset A");
    assert_eq!(browser["credits"][1]["text"], "Dataset B");
    assert_eq!(
        browser["approvedOrigins"],
        json!(["https://maps.example.test"])
    );
}
#[test]
fn every_supported_network_reference_requires_approval() {
    for (field, value) in [
        (
            "glyphs",
            json!("https://external.test/{fontstack}/{range}.pbf"),
        ),
        ("sprite", json!("https://external.test/sprites")),
        (
            "sprite",
            json!([{"id":"remote","url":"https://external.test/sprites"}]),
        ),
        (
            "font-faces",
            json!({"font":"https://external.test/font.ttf"}),
        ),
    ] {
        let mut s = style();
        s[field] = value;
        assert!(prepare(s).is_err());
    }
    let mut s = style();
    s["glyphs"] = json!("https://maps.example.test/{fontstack}/{range}.pbf");
    s["sprite"] = json!([{"id":"main","url":"https://maps.example.test/sprites"}]);
    assert!(prepare(s).is_ok());
}
#[test]
fn opaque_tilejson_sources_must_be_resolved_and_inlined() {
    let mut s = style();
    s["sources"]["Base/0001"] =
        json!({"type":"raster","url":"https://maps.example.test/tiles.json"});
    assert!(prepare(s.clone()).is_err());
    let tile=TileSource::from_json(&serde_json::to_vec(&json!({"tilejson":"3.0.0","tiles":["https://maps.example.test/{z}/{x}/{y}.png"],"attribution":"Native dataset"})).unwrap(),TileKind::Raster,&policy(),Attribution::new("Source dataset",None).unwrap()).unwrap();
    let browser = MapStyle::from_json(
        &serde_json::to_vec(&s).unwrap(),
        &policy(),
        &credits(),
        &[("Base/0001", &tile)],
    )
    .unwrap()
    .to_browser_json();
    assert!(
        browser["style"]["sources"]["Base/0001"]
            .get("url")
            .is_none()
    );
    assert_eq!(
        browser["style"]["sources"]["Base/0001"]["tiles"][0],
        "https://maps.example.test/{z}/{x}/{y}.png"
    );
    assert!(
        browser["credits"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["text"] == "Native dataset")
    );
}
#[test]
fn malformed_styles_and_missing_layer_sources_are_rejected() {
    for (field, value) in [
        ("version", json!(9)),
        ("sources", json!([])),
        ("layers", json!({})),
    ] {
        let mut s = style();
        s[field] = value;
        assert!(prepare(s).is_err());
    }
    let mut s = style();
    s["layers"][0]["source"] = json!("missing");
    assert!(prepare(s).is_err());
    let mut s = style();
    s["sources"]["Base/0001"]["tiles"] =
        json!(["https://maps.example.test/{z}/{x}/{y}.png?backend_key=private"]);
    assert!(prepare(s).is_err());
}
#[test]
fn arbitrary_metadata_does_not_escape_to_browser() {
    let mut s = style();
    s["metadata"] = json!({"backend_key":"private"});
    s["sources"]["Base/0001"]["metadata"] = json!({"secret":"private"});
    s["layers"][0]["metadata"] = json!({"secret":"private"});
    let browser = prepare(s).unwrap().to_browser_json();
    assert!(!browser.to_string().contains("private"));
    assert!(browser["style"].get("metadata").is_none());
}
#[test]
fn native_credit_equal_to_old_placeholder_is_still_preserved() {
    let mut s = style();
    s["sources"]["Base/0001"]["attribution"] = json!("Host-configured source");
    let browser = prepare(s).unwrap().to_browser_json();
    assert!(
        browser["credits"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["text"] == "Host-configured source")
    );
}
#[test]
fn vector_layer_references_bind_native_metadata() {
    let mut s = style();
    s["sources"]["Base/0001"] = json!({"type":"vector","tiles":["https://maps.example.test/{z}/{x}/{y}.pbf"],"vector_layers":[{"id":"Roads/0001","fields":{}}]});
    s["layers"][0]["type"] = json!("line");
    assert!(prepare(s.clone()).is_err());
    s["layers"][0]["source-layer"] = json!("missing");
    assert!(prepare(s.clone()).is_err());
    s["layers"][0]["source-layer"] = json!("Roads/0001");
    let browser = prepare(s).unwrap().to_browser_json();
    assert_eq!(browser["style"]["layers"][0]["source-layer"], "Roads/0001");
    assert!(
        browser["style"]["sources"]["Base/0001"]
            .get("vector_layers")
            .is_none()
    );
}
#[test]
fn inline_maplibre_defaults_do_not_acquire_tilejson_defaults() {
    let browser = prepare(style()).unwrap().to_browser_json();
    assert_eq!(browser["style"]["sources"]["Base/0001"]["maxzoom"], 22);
}
#[test]
fn native_source_semantics_are_preserved_instead_of_silently_lost() {
    for (field, value) in [
        ("encoding", json!("mlt")),
        ("promoteId", json!("original-id")),
    ] {
        let mut s = style();
        s["sources"]["Base/0001"] = json!({"type":"vector","tiles":["https://maps.example.test/{z}/{x}/{y}.pbf"],"vector_layers":[{"id":"roads","fields":{}}]});
        s["sources"]["Base/0001"][field] = value;
        s["layers"][0]["type"] = json!("line");
        s["layers"][0]["source-layer"] = json!("roads");
        let browser = prepare(s.clone()).unwrap().to_browser_json();
        assert_eq!(
            browser["style"]["sources"]["Base/0001"][field],
            s["sources"]["Base/0001"][field]
        );
    }
}
#[test]
fn unsupported_rendering_semantics_are_explicit_not_silently_removed() {
    let policy = BrowserPolicy::new(&["https://maps.example.test"]).unwrap();
    let credits = [Attribution::new("Fixture", None).unwrap()];
    for field in [
        "light",
        "sky",
        "projection",
        "state",
        "transition",
        "centerAltitude",
    ] {
        let mut native = serde_json::json!({"version":8,"sources":{},"layers":[]});
        native[field] = serde_json::json!({"fixture":true});
        assert!(
            matches!(
                MapStyle::from_json(
                    &serde_json::to_vec(&native).unwrap(),
                    &policy,
                    &credits,
                    &[]
                ),
                Err(Error::Unsupported)
            ),
            "silently removed rendering property: {field}"
        );
    }
}
#[test]
fn resolved_source_overrides_are_validated_and_preserve_all_required_credits() {
    let tile = TileSource::from_json(
        &serde_json::to_vec(&json!({"tilejson":"3.0.0","tiles":["https://maps.example.test/{z}/{x}/{y}.png"],"maxzoom":18})).unwrap(),
        TileKind::Raster, &policy(), Attribution::new("Native credit", None).unwrap(),
    ).unwrap();
    for (field, value) in [
        ("minzoom", json!(4)),
        ("maxzoom", json!(12)),
        ("bounds", json!([-20.0, -10.0, 20.0, 10.0])),
        ("scheme", json!("tms")),
        ("attribution", json!("Additional required credit")),
    ] {
        let mut s = style();
        s["sources"]["Base/0001"] =
            json!({"type":"raster","url":"https://maps.example.test/tiles.json"});
        s["sources"]["Base/0001"][field] = value.clone();
        let browser = MapStyle::from_json(
            &serde_json::to_vec(&s).unwrap(),
            &policy(),
            &credits(),
            &[("Base/0001", &tile)],
        )
        .unwrap()
        .to_browser_json();
        if field == "attribution" {
            assert!(
                browser["credits"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|c| c["text"] == value)
            );
        } else {
            assert_eq!(browser["style"]["sources"]["Base/0001"][field], value);
        }
        assert!(
            browser["credits"]
                .as_array()
                .unwrap()
                .iter()
                .any(|c| c["text"] == "Native credit")
        );
    }
    let mut s = style();
    s["sources"]["Base/0001"] = json!({"type":"raster","url":"https://maps.example.test/tiles.json","minzoom":20,"maxzoom":12});
    assert!(
        MapStyle::from_json(
            &serde_json::to_vec(&s).unwrap(),
            &policy(),
            &credits(),
            &[("Base/0001", &tile)]
        )
        .is_err()
    );
}
#[test]
fn resolved_manifest_cannot_change_declared_source_kind_or_admit_html_credit() {
    let tile = TileSource::from_json(
        &serde_json::to_vec(
            &json!({"tilejson":"3.0.0","tiles":["https://maps.example.test/{z}/{x}/{y}.png"]}),
        )
        .unwrap(),
        TileKind::Raster,
        &policy(),
        Attribution::new("Dataset", None).unwrap(),
    )
    .unwrap();
    for source in [
        json!({"type":"vector","url":"https://maps.example.test/tiles.json"}),
        json!({"type":"raster","url":"https://maps.example.test/tiles.json","attribution":"<img src=https://external.test/tracker>"}),
    ] {
        let mut s = style();
        s["sources"]["Base/0001"] = source;
        assert!(
            MapStyle::from_json(
                &serde_json::to_vec(&s).unwrap(),
                &policy(),
                &credits(),
                &[("Base/0001", &tile)]
            )
            .is_err()
        );
    }
}
