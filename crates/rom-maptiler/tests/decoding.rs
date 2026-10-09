//! Authored native-shape tests, not MapTiler service qualification.
use rom_map_core::{Accuracy, Attribution, Error};
use rom_maptiler::{AttributionProfile, geocode_response};
fn profile() -> AttributionProfile {
    AttributionProfile::new(
        "authored approved native credit",
        Attribution::new("Authored fixture dataset", None).unwrap(),
    )
    .unwrap()
}
const VALID:&[u8]=br#"{"type":"FeatureCollection","attribution":"authored approved native credit","features":[{"type":"Feature","id":"municipality.00001","place_name":"Authored Place","geometry":{"type":"Point","coordinates":[21,52]},"center":[21,52],"relevance":1}]}"#;
#[test]
fn native_id_position_and_unknown_accuracy_are_retained() {
    let result = geocode_response(VALID, 1, "maptiler-fixture", &profile()).unwrap();
    let value = &result.results()[0];
    assert_eq!(value.provenance().source_id(), "municipality.00001");
    assert_eq!(value.coordinate().longitude_latitude(), [21.0, 52.0]);
    assert!(matches!(value.accuracy(), Accuracy::Unknown));
}
#[test]
fn empty_results_are_valid_but_unapproved_credit_and_excess_results_fail() {
    assert!(geocode_response(br#"{"type":"FeatureCollection","attribution":"authored approved native credit","features":[]}"#,1,"fixture",&profile()).unwrap().results().is_empty());
    assert!(geocode_response(br#"{"type":"FeatureCollection","attribution":"unreviewed extra license","features":[]}"#,1,"fixture",&profile()).is_err());
    assert!(matches!(
        geocode_response(VALID, 11, "fixture", &profile()),
        Err(Error::InvalidQuery)
    ));
    assert!(matches!(
        geocode_response(&vec![b' '; 1048577], 1, "fixture", &profile()),
        Err(Error::TooLarge)
    ));
}
#[test]
fn malformed_geometry_and_identity_never_become_approved_results() {
    for replacement in ["[181,52]", "[21,91]", "[21,52,0]", "[]"] {
        let input = std::str::from_utf8(VALID)
            .unwrap()
            .replace("[21,52]", replacement);
        assert!(geocode_response(input.as_bytes(), 1, "fixture", &profile()).is_err());
    }
    let input = std::str::from_utf8(VALID)
        .unwrap()
        .replace("municipality.00001", "");
    assert!(geocode_response(input.as_bytes(), 1, "fixture", &profile()).is_err());
}
#[test]
fn service_result_limit_is_enforced_without_truncating_identity() {
    let mut document: serde_json::Value = serde_json::from_slice(VALID).unwrap();
    let feature = document["features"][0].clone();
    document["features"].as_array_mut().unwrap().push(feature);
    let bytes = serde_json::to_vec(&document).unwrap();
    assert!(matches!(
        geocode_response(&bytes, 1, "fixture", &profile()),
        Err(Error::TooLarge)
    ));
    assert_eq!(
        geocode_response(&bytes, 2, "fixture", &profile())
            .unwrap()
            .results()
            .len(),
        2
    );
}
#[test]
fn wgs84_edges_are_valid_and_invalid_native_bbox_is_rejected() {
    for position in ["[-180,-90]", "[180,90]"] {
        let bytes = std::str::from_utf8(VALID)
            .unwrap()
            .replace("[21,52]", position);
        assert!(geocode_response(bytes.as_bytes(), 1, "fixture", &profile()).is_ok());
    }
    for bbox in [
        serde_json::json!([-181, -90, 180, 90]),
        serde_json::json!([-180, 50, 180, 40]),
        serde_json::json!([0, 0, 1]),
    ] {
        let mut document: serde_json::Value = serde_json::from_slice(VALID).unwrap();
        document["features"][0]["bbox"] = bbox;
        assert!(
            geocode_response(
                &serde_json::to_vec(&document).unwrap(),
                1,
                "fixture",
                &profile()
            )
            .is_err()
        );
    }
}
