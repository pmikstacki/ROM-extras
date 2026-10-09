//! Native response boundary tests.
use rom_map_core::{Accuracy, Error};
use rom_nominatim::search_results;
use serde_json::{Value, json};
fn place() -> Value {
    json!({"osm_type":"way","osm_id":"00042","lat":"52.1","lon":"21.2","boundingbox":["52","53","21","22"],"display_name":"A place","licence":"Data © OpenStreetMap contributors, ODbL 1.0.","importance":0.99})
}
fn decode(value: Value, limit: usize) -> rom_map_core::Result<rom_map_core::GeocodeResults> {
    search_results(
        &serde_json::to_vec(&value).unwrap(),
        limit,
        "host-nominatim",
    )
}
#[test]
fn native_positions_bounds_ids_and_credit_remain_exact() {
    let results = decode(json!([place()]), 1).unwrap();
    let result = &results.results()[0];
    assert_eq!(result.coordinate().longitude_latitude(), [21.2, 52.1]);
    assert_eq!(
        result.bounds().unwrap().west_south_east_north(),
        [21.0, 52.0, 22.0, 53.0]
    );
    assert_eq!(result.provenance().source_id(), "way/00042");
    assert_eq!(result.provenance().provider(), "host-nominatim");
    assert_eq!(
        result.provenance().attribution().text(),
        "Data © OpenStreetMap contributors, ODbL 1.0."
    );
    assert!(matches!(result.accuracy(), Accuracy::Unknown));
}
#[test]
fn integer_native_identity_and_empty_search_are_supported() {
    let mut p = place();
    p["osm_id"] = json!(18446744073709551615u64);
    assert_eq!(
        decode(json!([p]), 1).unwrap().results()[0]
            .provenance()
            .source_id(),
        "way/18446744073709551615"
    );
    assert!(decode(json!([]), 1).unwrap().results().is_empty());
}
#[test]
fn malformed_native_fields_are_not_approved() {
    for (field, value) in [
        ("lat", json!("NaN")),
        ("lon", json!("181")),
        ("osm_id", json!(1.5)),
        ("osm_type", json!("other")),
        ("licence", json!("<script>")),
        ("boundingbox", json!(["53", "52", "21", "22"])),
    ] {
        let mut p = place();
        p[field] = value;
        assert!(decode(json!([p]), 1).is_err(), "{field}");
    }
    assert!(decode(json!({"error":"untrusted secret"}), 1).is_err());
    assert!(matches!(
        decode(json!([place(), place()]), 1),
        Err(Error::TooLarge)
    ));
}
#[test]
fn response_and_query_limits_reject_before_decoding() {
    assert!(matches!(
        search_results(&vec![b' '; 1048577], 1, "host"),
        Err(Error::TooLarge)
    ));
    assert!(matches!(decode(json!([]), 0), Err(Error::InvalidQuery)));
}
#[test]
fn reverse_empty_is_only_the_exact_native_no_coverage_response() {
    use rom_nominatim::reverse_results;
    let empty = reverse_results(br#"{"error":"Unable to geocode"}"#, "operator").unwrap();
    assert!(empty.results().is_empty());
    let result = reverse_results(&serde_json::to_vec(&place()).unwrap(), "operator").unwrap();
    assert_eq!(result.results()[0].provenance().source_id(), "way/00042");
    for value in [
        json!([]),
        json!({"error":"private backend secret"}),
        json!({"error":"Unable to geocode","lon":"21"}),
    ] {
        assert!(reverse_results(&serde_json::to_vec(&value).unwrap(), "operator").is_err());
    }
}
#[test]
fn service_scoped_place_identity_supports_non_osm_results_without_normalization() {
    let mut p = place();
    let object = p.as_object_mut().unwrap();
    object.remove("osm_type");
    object.remove("osm_id");
    object.insert("place_id".into(), json!("00099"));
    let result = decode(json!([p.clone()]), 1).unwrap();
    assert_eq!(
        result.results()[0].provenance().source_id(),
        "nominatim-place/00099"
    );
    p["osm_type"] = json!("node");
    assert!(decode(json!([p]), 1).is_err());
}
