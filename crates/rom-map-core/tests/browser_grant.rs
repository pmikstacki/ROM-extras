//! Explicit host browser-token grants cannot authorize backend or other-origin credentials.
use rom_map_core::{BrowserPolicy, Error};
#[test]
fn query_credentials_are_rejected_without_explicit_host_grant() {
    let policy = BrowserPolicy::new(&["https://api.provider.test"]).unwrap();
    assert!(
        policy
            .approve("https://api.provider.test/tiles/{z}/{x}/{y}.pbf?key=public-fixture-token")
            .is_err()
    );
}
#[test]
fn explicit_public_token_is_exact_and_origin_scoped() {
    let policy = BrowserPolicy::new(&["https://api.provider.test", "https://other.test"])
        .unwrap()
        .with_public_query_token(
            "https://api.provider.test",
            "key",
            "public-fixture-token".into(),
        )
        .unwrap();
    let allowed = "https://api.provider.test/tiles/{z}/{x}/{y}.pbf?key=public-fixture-token";
    assert_eq!(policy.approve(allowed).unwrap(), allowed);
    for url in [
        "https://other.test/tile?key=public-fixture-token",
        "https://api.provider.test/tile?key=backend-fixture-secret",
        "https://api.provider.test/tile?key=public-fixture-token&key=backend-fixture-secret",
        "https://api.provider.test/tile?key=public-fixture-token&debug=1",
    ] {
        assert!(policy.approve(url).is_err());
    }
}
#[test]
fn unknown_origin_invalid_field_and_empty_token_cannot_be_granted() {
    assert!(
        BrowserPolicy::new(&["https://api.provider.test"])
            .unwrap()
            .with_public_query_token("https://unknown.test", "key", "public-fixture-token".into())
            .is_err()
    );
    assert!(matches!(
        BrowserPolicy::new(&["https://api.provider.test"])
            .unwrap()
            .with_public_query_token("https://api.provider.test", "key", "".into()),
        Err(Error::InvalidQuery)
    ));
    assert!(
        BrowserPolicy::new(&["https://api.provider.test"])
            .unwrap()
            .with_public_query_token(
                "https://api.provider.test",
                "key&other",
                "public-fixture-token".into()
            )
            .is_err()
    );
}

#[test]
fn approved_token_survives_descriptors_but_backend_queries_and_metadata_do_not() {
    use rom_map_core::{Attribution, MapStyle, TileKind, TileSource};
    let policy = BrowserPolicy::new(&["https://api.provider.test"])
        .unwrap()
        .with_public_query_token(
            "https://api.provider.test",
            "key",
            "public-fixture-token".into(),
        )
        .unwrap();
    let credit = || Attribution::new("Authored fixture", None).unwrap();
    let tile=TileSource::from_json(br#"{"tilejson":"3.0.0","tiles":["https://api.provider.test/{z}/{x}/{y}.png?key=public-fixture-token"],"metadata":{"backend":"backend-fixture-secret"}}"#,TileKind::Raster,&policy,credit()).unwrap();
    let browser = tile.to_browser_json();
    assert!(
        browser["tiles"][0]
            .as_str()
            .unwrap()
            .contains("public-fixture-token")
    );
    assert!(!browser.to_string().contains("backend-fixture-secret"));
    let style=MapStyle::from_json(br#"{"version":8,"sources":{"Base/00001":{"type":"raster","tiles":["https://api.provider.test/{z}/{x}/{y}.png?key=public-fixture-token"]}},"layers":[{"id":"Base/00001","type":"raster","source":"Base/00001"}],"metadata":{"backend":"backend-fixture-secret"}}"#,&policy,&[credit()],&[]).unwrap();
    let browser = style.to_browser_json();
    assert!(browser.to_string().contains("public-fixture-token"));
    assert!(!browser.to_string().contains("backend-fixture-secret"));
    assert!(TileSource::from_json(br#"{"tilejson":"3.0.0","tiles":["https://api.provider.test/{z}/{x}/{y}.png?key=backend-fixture-secret"]}"#,TileKind::Raster,&policy,credit()).is_err());
}
