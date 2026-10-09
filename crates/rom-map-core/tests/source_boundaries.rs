//! Synthetic metadata; no deployed map service or license qualification claim.
use rom_map_core::{Attribution, Error};
use rom_map_core::{BrowserPolicy, TileKind, TileSource};
use serde_json::{Value, json};
fn credit() -> Attribution {
    Attribution::new("Fixture dataset", Some("https://maps.example.test/credits")).unwrap()
}
fn manifest() -> Value {
    json!({"tilejson":"3.0.0","tiles":["https://maps.example.test/tiles/{z}/{x}/{y}.png"],"scheme":"tms","minzoom":0,"maxzoom":16,"bounds":[-180,-80,180,80]})
}
fn prepare(value: Value, kind: TileKind) -> rom_map_core::Result<TileSource> {
    let policy = BrowserPolicy::new(&["https://maps.example.test"]).unwrap();
    TileSource::from_json(
        &serde_json::to_vec(&value).unwrap(),
        kind,
        &policy,
        credit(),
    )
}
#[test]
fn explicit_origin_is_required_and_exact() {
    let policy = BrowserPolicy::new(&["https://maps.example.test"]).unwrap();
    assert_eq!(
        policy
            .approve("https://maps.example.test/tiles/{z}/{x}/{y}.png")
            .unwrap(),
        "https://maps.example.test/tiles/{z}/{x}/{y}.png"
    );
    for url in [
        "http://maps.example.test/x",
        "https://maps.example.test.attacker.test/x",
        "https://maps.example.test:444/x",
        "https://user:password@maps.example.test/x",
        "https://maps.example.test/x?backend_secret=private",
        "https://maps.example.test/x#fragment",
        "https://maps.example.test/tiles/{unknown}",
    ] {
        assert!(policy.approve(url).is_err());
    }
    for origins in [
        vec![],
        vec!["https://maps.example.test/path"],
        vec!["https://maps.example.test?key=private"],
        vec!["http://maps.example.test"],
    ] {
        assert!(BrowserPolicy::new(&origins).is_err());
    }
}
#[test]
fn raster_descriptor_retains_tms_zoom_bounds_and_plain_credit() {
    let source = prepare(manifest(), TileKind::Raster).unwrap();
    let browser = source.to_browser_json();
    assert_eq!(browser["type"], "raster");
    assert_eq!(browser["scheme"], "tms");
    assert_eq!(browser["maxzoom"], 16);
    assert_eq!(browser["bounds"], json!([-180.0, -80.0, 180.0, 80.0]));
    assert_eq!(browser["credits"][0]["text"], "Fixture dataset");
}
#[test]
fn malformed_or_excessive_native_metadata_is_rejected_without_truncation() {
    for (field, value) in [
        ("tilejson", json!("2.2.0")),
        ("tiles", json!([])),
        ("tiles", json!(["https://external.test/{z}/{x}/{y}"])),
        ("scheme", json!("flip")),
        ("minzoom", json!(-1)),
        ("maxzoom", json!(31)),
        ("bounds", json!([170, -10, -170, 10])),
    ] {
        let mut m = manifest();
        m[field] = value;
        assert!(prepare(m, TileKind::Raster).is_err());
    }
    let mut m = manifest();
    m["tiles"] = json!(vec!["https://maps.example.test/{z}/{x}/{y}"; 17]);
    assert!(matches!(prepare(m, TileKind::Raster), Err(Error::TooLarge)));
    let policy = BrowserPolicy::new(&["https://maps.example.test"]).unwrap();
    assert!(matches!(
        TileSource::from_json(&vec![b' '; 1048577], TileKind::Raster, &policy, credit()),
        Err(Error::TooLarge)
    ));
}
#[test]
fn vector_layers_require_original_nonempty_identity_and_string_fields() {
    assert!(prepare(manifest(), TileKind::Vector).is_err());
    let mut m = manifest();
    m["vector_layers"] = json!([{"id":"Roads/0001","fields":{"name":"original label"}}]);
    assert_eq!(
        prepare(m.clone(), TileKind::Vector)
            .unwrap()
            .to_browser_json()["vector_layers"][0]["id"],
        "Roads/0001"
    );
    m["vector_layers"][0]["fields"]["name"] = json!(3);
    assert!(prepare(m, TileKind::Vector).is_err());
}
#[test]
fn unsafe_native_html_is_not_silently_lost_or_disclosed() {
    let mut m = manifest();
    m["attribution"] = json!("<img src='https://tracking.test/x'>credit");
    assert!(prepare(m, TileKind::Raster).is_err());
    let mut m = manifest();
    m["attribution"] = json!("Additional native dataset credit");
    let browser = prepare(m, TileKind::Raster).unwrap().to_browser_json();
    assert_eq!(
        browser["credits"][1]["text"],
        "Additional native dataset credit"
    );
    assert!(!browser.to_string().contains("backend_secret"));
}
#[test]
fn unused_network_metadata_does_not_escape_to_browser() {
    let mut m = manifest();
    m["data"] = json!(["https://external.test/overlay.json"]);
    m["metadata"] = json!({"secret":"private"});
    let browser = prepare(m, TileKind::Raster).unwrap().to_browser_json();
    assert!(browser.get("data").is_none());
    assert!(browser.get("metadata").is_none());
}
#[test]
fn placeholders_cannot_change_the_approved_authority() {
    let policy =
        BrowserPolicy::new(&["https://0.example.test", "https://maps.example.test:10000"]).unwrap();
    for template in [
        "https://{x}.example.test/{z}/{x}/{y}.png",
        "https://maps.example.test:1{x}{x}{x}{x}/{z}/{x}/{y}.png",
    ] {
        assert!(policy.approve(template).is_err());
    }
}
