use rom_map_core::{
    Attribution, Cancellation, Capabilities, Coordinate, Provenance, RequestContext, RouteQuery,
    TravelMode,
};
use rom_osrm::{Config, Osrm};
use std::time::Duration;
#[tokio::main(flavor = "current_thread")]
async fn main() {
    let mut args = std::env::args().skip(1);
    let endpoint = args.next().expect("explicit controlled endpoint required");
    let ca = args.next().expect("explicit controlled CA file required");
    assert!(args.next().is_none());
    let graph = Provenance::new(
        "controlled-osrm-26.10",
        "fixture.osrm",
        Attribution::new("ROM-extras authored synthetic graph (MIT)", None).unwrap(),
    )
    .unwrap();
    let provider = Osrm::new(
        Config::new(
            &endpoint,
            "ROM-extras independent consumer/1",
            "car",
            TravelMode::Driving,
            graph,
            Duration::ZERO,
        )
        .unwrap()
        .with_snap_radius(rom_map_core::Metres::new(5.0).unwrap())
        .unwrap()
        .with_ca(std::fs::read(ca).unwrap())
        .unwrap(),
    )
    .unwrap();
    let ports = Capabilities::new().with_routing(&provider);
    assert!(ports.geocoding().is_err());
    assert!(ports.styles().is_err());
    let context = RequestContext::new(Duration::from_secs(2), Cancellation::new()).unwrap();
    let query = RouteQuery::new(
        vec![
            Coordinate::new(21.0001, 52.0).unwrap(),
            Coordinate::new(21.001, 52.0009).unwrap(),
        ],
        TravelMode::Driving,
    )
    .unwrap();
    let route = ports
        .routing()
        .unwrap()
        .route(&query, &context)
        .await
        .unwrap()
        .unwrap();
    assert!(route.distance().as_metres() > 140.0 && route.distance().as_metres() < 200.0);
    assert!(route.duration().as_seconds() > 0.0 && route.duration().as_seconds() < 60.0);
    assert_eq!(route.provenance().source_id(), "fixture.osrm");
    assert_eq!(route.geometry().to_geojson()["type"], "LineString");
    let disconnected = RouteQuery::new(
        vec![
            Coordinate::new(21.0001, 52.0).unwrap(),
            Coordinate::new(21.0205, 52.02).unwrap(),
        ],
        TravelMode::Driving,
    )
    .unwrap();
    assert!(
        ports
            .routing()
            .unwrap()
            .route(&disconnected, &context)
            .await
            .unwrap()
            .is_none()
    );
    for first in [
        Coordinate::new(0.0, 0.0).unwrap(),
        Coordinate::new(21.0001, 52.0001).unwrap(),
    ] {
        let outside = RouteQuery::new(
            vec![first, Coordinate::new(21.001, 52.0009).unwrap()],
            TravelMode::Driving,
        )
        .unwrap();
        assert!(
            ports
                .routing()
                .unwrap()
                .route(&outside, &context)
                .await
                .unwrap()
                .is_none()
        );
    }
    println!(
        "Independent public OSRM consumer: native route units/geometry/graph identity, disconnected absence and finite-radius NoSegment passed"
    );
}
