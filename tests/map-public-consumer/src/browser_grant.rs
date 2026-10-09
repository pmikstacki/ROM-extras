//! Independent browser-public token contract checks; no provider-account qualification.
use rom_map_core::{Attribution, BrowserPolicy, MapStyle, TileKind, TileSource};
pub fn run() {
    let policy = BrowserPolicy::new(&["https://api.operator.test"])
        .unwrap()
        .with_public_query_token(
            "https://api.operator.test",
            "key",
            "browser-fixture-token".into(),
        )
        .unwrap();
    let tile=TileSource::from_json(br#"{"tilejson":"3.0.0","tiles":["https://api.operator.test/{z}/{x}/{y}.png?key=browser-fixture-token"]}"#,TileKind::Raster,&policy,Attribution::new("Authored fixture",None).unwrap()).unwrap();
    let style=MapStyle::from_json(br#"{"version":8,"sources":{"Base/00001":{"type":"raster","url":"https://api.operator.test/metadata.json?key=browser-fixture-token"}},"layers":[{"id":"Base/00001","type":"raster","source":"Base/00001"}]}"#,&policy,&[Attribution::new("Authored fixture",None).unwrap()],&[("Base/00001",&tile)]).unwrap().to_browser_json();
    assert!(style["style"]["sources"]["Base/00001"].get("url").is_none());
    assert_eq!(
        style["style"]["sources"]["Base/00001"]["tiles"][0],
        "https://api.operator.test/{z}/{x}/{y}.png?key=browser-fixture-token"
    );
    assert_eq!(style["style"]["layers"][0]["source"], "Base/00001");
    assert!(
        policy
            .approve("https://api.operator.test/{z}/{x}/{y}.png?key=backend-fixture-token")
            .is_err()
    );
    println!(
        "Independent browser grant consumer: explicit token, inlined source identity and other-token rejection passed; no network or provider-account qualification"
    );
}
