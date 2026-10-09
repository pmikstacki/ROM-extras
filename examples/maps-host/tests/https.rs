use rom_map_core::{
    Accuracy, Attribution, Cancellation, Capabilities, GeocodeQuery, RequestContext,
};
use rom_maps_host_example::query_suggestions;
use rom_maptiler::{AttributionProfile, Config, MapTiler};
use std::time::Duration;
#[tokio::test]
#[ignore = "explicit controlled HTTPS endpoint and CA required"]
async fn host_query_over_controlled_https() {
    let endpoint = std::env::var("ROM_MAP_HOST_ENDPOINT").unwrap();
    let ca = std::fs::read(std::env::var("ROM_MAP_HOST_CA").unwrap()).unwrap();
    let provider = MapTiler::new(
        Config::new(
            &endpoint,
            "ROMHostFixture/1",
            "fixture-maptiler",
            Duration::ZERO,
            AttributionProfile::new("fixture credit", Attribution::new("Fixture", None).unwrap())
                .unwrap(),
        )
        .unwrap()
        .with_server_key("synthetic-secret".into())
        .unwrap()
        .with_ca(ca)
        .unwrap(),
    )
    .unwrap();
    let capabilities = Capabilities::new().with_geocoding(&provider);
    let context = RequestContext::new(Duration::from_secs(2), Cancellation::new()).unwrap();
    let result = query_suggestions(
        &capabilities,
        &GeocodeQuery::new("Warsaw & test", 1).unwrap(),
        &context,
        || true,
    )
    .await
    .unwrap();
    let suggestion = &result.results()[0];
    assert_eq!(suggestion.provenance().source_id(), "municipality.00001");
    assert!(matches!(suggestion.accuracy(), Accuracy::Unknown));
    let dto = rom::json!({"label":suggestion.label(),"longitude":suggestion.coordinate().longitude_degrees(),"latitude":suggestion.coordinate().latitude_degrees(),"provider":suggestion.provenance().provider(),"sourceId":suggestion.provenance().source_id(),"attribution":suggestion.provenance().attribution().text(),"accuracy":"unknown"});
    let encoded = dto.to_string();
    assert!(!encoded.contains("synthetic-secret"));
    assert!(!encoded.contains("key="));
    assert!(!dto.as_object().unwrap().contains_key("resourceId"));
    if let Some(path) = std::env::var_os("ROM_MAP_HOST_SUGGESTION_SNAPSHOT") {
        std::fs::write(path, encoded).unwrap();
    }
    let mut checks = 0;
    assert!(
        query_suggestions(
            &capabilities,
            &GeocodeQuery::new("Warsaw & test", 1).unwrap(),
            &context,
            || {
                checks += 1;
                checks == 1
            }
        )
        .await
        .is_err()
    );
}
