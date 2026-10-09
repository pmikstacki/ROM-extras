//! Catch coordinate swapping, invalid units, silently reordered bounds and malformed geometry.
use rom_map_core::{BoundingBox, Coordinate, Error, Metres, RouteGeometry, Seconds};
use serde_json::json;
#[test]
fn positions_preserve_longitude_latitude_order_at_poles_and_dateline() {
    for (lon, lat) in [
        (180.0, 90.0),
        (-180.0, -90.0),
        (179.9, 45.0),
        (19.94, 50.06),
    ] {
        assert_eq!(
            Coordinate::new(lon, lat).unwrap().longitude_latitude(),
            [lon, lat]
        );
    }
}
#[test]
fn out_of_range_and_nonfinite_positions_are_rejected() {
    for (lon, lat) in [
        (180.1, 0.0),
        (-180.1, 0.0),
        (0.0, 90.1),
        (0.0, -90.1),
        (f64::NAN, 0.0),
        (0.0, f64::INFINITY),
    ] {
        assert!(matches!(
            Coordinate::new(lon, lat),
            Err(Error::InvalidCoordinate)
        ));
    }
}
#[test]
fn antimeridian_boxes_retain_order_and_contain_both_sides() {
    let b = BoundingBox::new(177.0, -20.0, -178.0, -16.0).unwrap();
    assert_eq!(b.west_south_east_north(), [177.0, -20.0, -178.0, -16.0]);
    assert!(b.crosses_antimeridian());
    for lon in [179.0, -179.0, 180.0, -180.0] {
        assert!(b.contains(Coordinate::new(lon, -18.0).unwrap()));
    }
    assert!(!b.contains(Coordinate::new(0.0, -18.0).unwrap()));
    assert!(!b.contains(Coordinate::new(179.0, 0.0).unwrap()));
}
#[test]
fn latitude_order_and_invalid_box_coordinates_fail() {
    for (w, s, e, n) in [
        (0.0, 10.0, 1.0, -10.0),
        (181.0, 0.0, 0.0, 1.0),
        (0.0, f64::NAN, 1.0, 1.0),
    ] {
        assert!(matches!(
            BoundingBox::new(w, s, e, n),
            Err(Error::InvalidBounds)
        ));
    }
}
#[test]
fn finite_nonnegative_units_preserve_metres_and_seconds_without_conversion() {
    assert_eq!(Metres::new(1250.25).unwrap().as_metres(), 1250.25);
    assert_eq!(Seconds::new(60.5).unwrap().as_seconds(), 60.5);
    assert_eq!(Metres::new(0.0).unwrap().as_metres(), 0.0);
    for n in [-1.0, f64::NAN, f64::INFINITY] {
        assert!(matches!(Metres::new(n), Err(Error::InvalidUnit)));
        assert!(matches!(Seconds::new(n), Err(Error::InvalidUnit)));
    }
}
#[test]
fn routes_encode_real_longitude_first_geojson_and_reject_short_lines() {
    let a = Coordinate::new(19.94, 50.06).unwrap();
    let b = Coordinate::new(20.0, 50.1).unwrap();
    let route = RouteGeometry::line_string(vec![a, b]).unwrap();
    assert_eq!(
        route.to_geojson(),
        json!({"type":"LineString","coordinates":[[19.94,50.06],[20.0,50.1]]})
    );
    assert!(matches!(
        RouteGeometry::line_string(vec![a]),
        Err(Error::InvalidGeometry)
    ));
    assert!(matches!(
        RouteGeometry::multi_line_string(vec![]),
        Err(Error::InvalidGeometry)
    ));
    assert!(matches!(
        RouteGeometry::multi_line_string(vec![vec![a, b], vec![a]]),
        Err(Error::InvalidGeometry)
    ));
    assert!(matches!(
        RouteGeometry::line_string(vec![a; 8193]),
        Err(Error::TooLarge)
    ));
}
