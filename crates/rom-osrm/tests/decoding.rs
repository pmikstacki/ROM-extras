//! Native OSRM output boundary cases; no live OSRM qualification.
use rom_map_core::{Attribution, Error, Provenance};
use rom_osrm::route_response;
use serde_json::{Value, json};
fn graph() -> Provenance {
    Provenance::new(
        "private-osrm",
        "operator-region.osrm",
        Attribution::new(
            "© OpenStreetMap contributors",
            Some("https://www.openstreetmap.org/copyright"),
        )
        .unwrap(),
    )
    .unwrap()
}
fn native() -> Value {
    json!({"code":"Ok","routes":[{"distance":1234.5,"duration":67.25,"geometry":{"type":"LineString","coordinates":[[21.0,52.0],[21.1,52.1]]}}]})
}
fn decode(value: Value, success: bool) -> rom_map_core::Result<Option<rom_map_core::Route>> {
    route_response(&serde_json::to_vec(&value).unwrap(), success, graph())
}
#[test]
fn valid_native_route_keeps_units_geometry_and_graph_provenance() {
    let route = decode(native(), true).unwrap().unwrap();
    assert_eq!(route.distance().as_metres(), 1234.5);
    assert_eq!(route.duration().as_seconds(), 67.25);
    assert_eq!(
        route.geometry().to_geojson(),
        native()["routes"][0]["geometry"]
    );
    assert_eq!(route.provenance().source_id(), "operator-region.osrm");
}
#[test]
fn native_absence_is_distinct_from_server_protocol_errors() {
    for code in ["NoRoute", "NoSegment"] {
        assert!(
            decode(json!({"code":code,"message":"synthetic-secret"}), false)
                .unwrap()
                .is_none()
        );
        assert!(decode(json!({"code":code}), true).is_err());
    }
    assert!(matches!(
        decode(
            json!({"code":"InvalidQuery","message":"synthetic-secret"}),
            false
        ),
        Err(Error::Rejected)
    ));
    assert!(matches!(
        decode(json!({"code":"TooBig"}), false),
        Err(Error::TooLarge)
    ));
    assert!(decode(native(), false).is_err());
}
#[test]
fn malformed_geometry_and_quantities_never_become_a_route() {
    for geometry in [
        json!({"type":"Point","coordinates":[21,52]}),
        json!({"type":"LineString","coordinates":[[21,52]]}),
        json!({"type":"LineString","coordinates":[[181,52],[21,52]]}),
        json!({"type":"LineString","coordinates":[[21,52,0],[21,52]]}),
    ] {
        let mut value = native();
        value["routes"][0]["geometry"] = geometry;
        assert!(decode(value, true).is_err());
    }
    for field in ["distance", "duration"] {
        let mut value = native();
        value["routes"][0][field] = json!(-1);
        assert!(decode(value, true).is_err());
    }
}
#[test]
fn excess_routes_vertices_and_payload_are_rejected() {
    let mut value = native();
    value["routes"] = json!([value["routes"][0].clone(), value["routes"][0].clone()]);
    assert!(matches!(decode(value, true), Err(Error::TooLarge)));
    let mut value = native();
    value["routes"][0]["geometry"]["coordinates"] = json!(vec![[21.0, 52.0]; 8193]);
    assert!(matches!(decode(value, true), Err(Error::TooLarge)));
    assert!(matches!(
        route_response(&vec![b' '; 1048577], true, graph()),
        Err(Error::TooLarge)
    ));
}
#[test]
fn empty_routes_and_unexpected_status_are_not_approved() {
    assert!(decode(json!({"code":"Ok","routes":[]}), true).is_err());
    assert!(decode(json!({"code":"mystery"}), true).is_err());
    assert!(matches!(
        decode(
            json!({"code":"mystery","message":"synthetic-secret"}),
            false
        ),
        Err(Error::Rejected)
    ));
}
