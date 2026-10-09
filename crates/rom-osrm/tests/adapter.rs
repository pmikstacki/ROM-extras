//! Explicit endpoint and prepared-profile configuration boundaries.
use rom_map_core::{Attribution, Provenance, TravelMode};
use rom_osrm::Config;
use std::time::Duration;
fn graph() -> Provenance {
    Provenance::new(
        "private-osrm",
        "operator-region.osrm",
        Attribution::new("© OpenStreetMap contributors", None).unwrap(),
    )
    .unwrap()
}
#[test]
fn host_service_and_static_profile_are_explicit() {
    assert!(
        Config::new(
            "https://router.example.test/operator/",
            "Host/1",
            "car",
            TravelMode::Driving,
            graph(),
            Duration::ZERO
        )
        .is_ok()
    );
}
#[test]
fn demo_service_and_injected_profile_are_rejected() {
    for (endpoint, profile) in [
        ("https://router.project-osrm.org/", "driving"),
        ("http://router.example.test/", "car"),
        ("https://router.example.test/", "../car"),
        ("https://router.example.test/", ""),
    ] {
        assert!(
            Config::new(
                endpoint,
                "Host/1",
                profile,
                TravelMode::Driving,
                graph(),
                Duration::ZERO
            )
            .is_err()
        );
    }
}
#[test]
fn absent_snap_radius_never_activates_implicit_unlimited() {
    let config = Config::new(
        "https://router.example.test/",
        "Host/1",
        "car",
        TravelMode::Driving,
        graph(),
        Duration::ZERO,
    )
    .unwrap();
    assert!(matches!(
        rom_osrm::Osrm::new(config),
        Err(rom_map_core::Error::InvalidQuery)
    ));
}
#[test]
fn explicit_snap_radius_is_finite_typed_and_url_bounded() {
    let config = || {
        Config::new(
            "https://router.example.test/",
            "Host/1",
            "car",
            TravelMode::Driving,
            graph(),
            Duration::ZERO,
        )
        .unwrap()
    };
    assert!(
        config()
            .with_snap_radius(rom_map_core::Metres::new(0.0).unwrap())
            .is_ok()
    );
    assert!(
        config()
            .with_snap_radius(rom_map_core::Metres::new(5.0).unwrap())
            .is_ok()
    );
    assert!(matches!(
        config().with_snap_radius(rom_map_core::Metres::new(1e308).unwrap()),
        Err(rom_map_core::Error::TooLarge)
    ));
}
