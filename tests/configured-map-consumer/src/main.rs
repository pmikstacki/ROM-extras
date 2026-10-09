use rom_configured_maps::{ConfiguredRaster, ConfiguredStyle, ConfiguredVector};
use rom_map_core::{
    Attribution, BrowserPolicy, Cancellation, Capabilities, Error, RequestContext, Styles,
};
use std::time::Duration;
fn policy() -> BrowserPolicy {
    BrowserPolicy::new(&["https://tiles.operator.test"]).unwrap()
}
fn credit() -> Attribution {
    Attribution::new(
        "Operator authored dataset",
        Some("https://tiles.operator.test/license"),
    )
    .unwrap()
}
#[tokio::main(flavor = "current_thread")]
async fn main() {
    if let Some(endpoint) = std::env::args().nth(1) {
        native(
            &endpoint,
            &std::env::args().nth(2).expect("explicit CA file required"),
        )
        .await;
        return;
    }
    let raster=ConfiguredRaster::new(br#"{"tilejson":"3.0.0","tiles":["https://tiles.operator.test/{z}/{x}/{y}.png"],"scheme":"tms","maxzoom":12}"#,policy(),credit()).unwrap();
    let vector=ConfiguredVector::new(br#"{"tilejson":"3.0.0","tiles":["https://tiles.operator.test/{z}/{x}/{y}.pbf"],"vector_layers":[{"id":"Roads/00001","fields":{}}]}"#,policy(),credit()).unwrap();
    let context = RequestContext::new(Duration::from_secs(1), Cancellation::new()).unwrap();
    let selected = Capabilities::new()
        .with_raster_tiles(&raster)
        .with_vector_tiles(&vector);
    assert!(matches!(selected.routing(), Err(Error::Unsupported)));
    assert!(matches!(selected.styles(), Err(Error::Unsupported)));
    let r = selected
        .raster_tiles()
        .unwrap()
        .raster_tiles(&context)
        .await
        .unwrap();
    assert_eq!(r.source().to_browser_json()["scheme"], "tms");
    let v = selected
        .vector_tiles()
        .unwrap()
        .vector_tiles(&context)
        .await
        .unwrap();
    assert_eq!(
        v.source().to_browser_json()["vector_layers"][0]["id"],
        "Roads/00001"
    );
    let style = ConfiguredStyle::new(
        br#"{"version":8,"sources":{},"layers":[{"id":"Background/00001","type":"background"}]}"#,
        policy(),
        vec![credit()],
        vec![],
    )
    .unwrap();
    assert_eq!(
        style.style(&context).await.unwrap().to_browser_json()["credits"][0]["text"],
        "Operator authored dataset"
    );
    assert!(ConfiguredRaster::new(br#"{"tilejson":"3.0.0","tiles":["https://tiles.operator.test/{z}/{x}/{y}.png?key=backend-fixture-secret"]}"#,policy(),credit()).is_err());
    println!(
        "Independent configured-map consumer: optional capabilities, layer IDs, credits and secret-bearing URL rejection passed; no network qualification"
    );
}

async fn native(endpoint: &str, ca_path: &str) {
    use rom_configured_maps::{HostedRaster, HostedStyle, HostedVector};
    use rom_map_core::{TileKind, TileSource};
    let ca = std::fs::read(ca_path).unwrap();
    let http = || {
        rom_map_http::Config::new(
            endpoint,
            "ROM independent metadata consumer/1",
            1048576,
            Duration::ZERO,
            1,
        )
        .unwrap()
        .with_ca(ca.clone())
        .unwrap()
    };
    let policy = || BrowserPolicy::new(&[endpoint.trim_end_matches('/')]).unwrap();
    let credit = || Attribution::new("Authored MIT fixture", None).unwrap();
    let context = RequestContext::new(Duration::from_secs(2), Cancellation::new()).unwrap();
    let raster = HostedRaster::new(http(), "raster.json", policy(), credit()).unwrap();
    let vector = HostedVector::new(http(), "vector.json", policy(), credit()).unwrap();
    let selected = Capabilities::new()
        .with_raster_tiles(&raster)
        .with_vector_tiles(&vector);
    assert_eq!(
        selected
            .raster_tiles()
            .unwrap()
            .raster_tiles(&context)
            .await
            .unwrap()
            .source()
            .to_browser_json()["maxzoom"],
        0
    );
    assert_eq!(
        selected
            .vector_tiles()
            .unwrap()
            .vector_tiles(&context)
            .await
            .unwrap()
            .source()
            .to_browser_json()["vector_layers"][0]["id"],
        "Roads/00001"
    );
    let bytes = rom_map_http::Http::new(http())
        .unwrap()
        .get(&["raster.json"], &[], &context)
        .await
        .unwrap();
    let dependency = TileSource::from_json(&bytes, TileKind::Raster, &policy(), credit()).unwrap();
    let style = HostedStyle::new(
        http(),
        "style.json",
        policy(),
        vec![credit()],
        vec![("Base/00001".into(), dependency)],
    )
    .unwrap();
    let browser = style.style(&context).await.unwrap().to_browser_json();
    assert!(
        browser["style"]["sources"]["Base/00001"]
            .get("url")
            .is_none()
    );
    assert_eq!(browser["style"]["layers"][0]["id"], "Base layer/00001");
    use rom_map_core::RasterTiles;
    let limited = HostedRaster::new(http(), "limited.json", policy(), credit()).unwrap();
    assert!(matches!(
        limited.raster_tiles(&context).await,
        Err(Error::RateLimited { .. })
    ));
    let redirect = HostedRaster::new(http(), "redirect.json", policy(), credit()).unwrap();
    assert!(matches!(
        redirect.raster_tiles(&context).await,
        Err(Error::Rejected)
    ));
    println!(
        "Independent native HTTPS metadata consumer: raster/vector/explicit style resolution, native IDs, 429 and rejected redirect passed; no renderer or tile-data qualification"
    );
}
