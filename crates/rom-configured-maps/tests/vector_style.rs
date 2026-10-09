//! Host-configured vector layers and styles preserve native identifiers and credits.
use rom_configured_maps::{ConfiguredStyle, ConfiguredVector};
use rom_map_core::{
    Attribution, BrowserPolicy, Cancellation, Error, RequestContext, Styles, VectorTiles,
};
use std::time::Duration;
fn policy() -> BrowserPolicy {
    BrowserPolicy::new(&["https://tiles.example.test"]).unwrap()
}
fn credit() -> Attribution {
    Attribution::new("Authored fixture tiles", None).unwrap()
}
fn context() -> RequestContext {
    RequestContext::new(Duration::from_secs(1), Cancellation::new()).unwrap()
}
const VECTOR:&[u8]=br#"{"tilejson":"3.0.0","tiles":["https://tiles.example.test/{z}/{x}/{y}.pbf"],"vector_layers":[{"id":"roads/0001","fields":{}}]}"#;
#[tokio::test]
async fn native_layer_ids_and_explicit_style_are_retained() {
    let vector = ConfiguredVector::new(VECTOR, policy(), credit()).unwrap();
    let approved = vector.vector_tiles(&context()).await.unwrap();
    assert_eq!(
        approved.source().to_browser_json()["vector_layers"][0]["id"],
        "roads/0001"
    );
    let style = ConfiguredStyle::new(
        br#"{"version":8,"sources":{},"layers":[{"id":"Background/0001","type":"background"}]}"#,
        policy(),
        vec![credit()],
        vec![],
    )
    .unwrap();
    assert_eq!(
        style.style(&context()).await.unwrap().to_browser_json()["style"]["layers"][0]["id"],
        "Background/0001"
    );
}
#[test]
fn missing_attribution_and_unapproved_style_loader_are_rejected() {
    assert!(
        ConfiguredStyle::new(
            br#"{"version":8,"sources":{},"layers":[]}"#,
            policy(),
            vec![],
            vec![]
        )
        .is_err()
    );
    assert!(
        ConfiguredStyle::new(
            br#"{"version":8,"sprite":"https://evil.test/sprite","sources":{},"layers":[]}"#,
            policy(),
            vec![credit()],
            vec![]
        )
        .is_err()
    );
    assert!(matches!(
        ConfiguredVector::new(&vec![b' '; 1048577], policy(), credit()),
        Err(Error::TooLarge)
    ));
}

#[tokio::test]
async fn explicit_tilejson_resolution_inlines_tiles_without_loader_url() {
    let tiles = rom_map_core::TileSource::from_json(
        VECTOR,
        rom_map_core::TileKind::Vector,
        &policy(),
        credit(),
    )
    .unwrap();
    let style=ConfiguredStyle::new(br#"{"version":8,"sources":{"native/0001":{"type":"vector","url":"https://tiles.example.test/source.json"}},"layers":[{"id":"roads","type":"line","source":"native/0001","source-layer":"roads/0001"}]}"#,policy(),vec![credit()],vec![("native/0001".into(),tiles)]).unwrap();
    let approved = style.style(&context()).await.unwrap().to_browser_json();
    assert!(
        approved["style"]["sources"]["native/0001"]
            .get("url")
            .is_none()
    );
    assert_eq!(
        approved["style"]["sources"]["native/0001"]["tiles"][0],
        "https://tiles.example.test/{z}/{x}/{y}.pbf"
    );
    assert_eq!(approved["style"]["layers"][0]["source-layer"], "roads/0001");
}
