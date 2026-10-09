//! Adapter admission tests; native service qualification is separate.
use rom_nominatim::Config;
use std::time::Duration;
#[test]
fn host_selected_private_service_is_admitted_without_network() {
    assert!(
        Config::new(
            "https://geo.example.test/operator/",
            "Host/1 (operator@example.test)",
            "operator-nominatim",
            Duration::from_secs(1)
        )
        .is_ok()
    );
}
#[test]
fn donated_public_service_and_invalid_configuration_are_rejected() {
    for endpoint in [
        "https://nominatim.openstreetmap.org/",
        "http://geo.example.test/",
        "https://geo.example.test/?key=secret",
    ] {
        assert!(Config::new(endpoint, "Host/1", "operator", Duration::ZERO).is_err());
    }
    assert!(Config::new("https://geo.example.test/", "Host/1", "", Duration::ZERO).is_err());
}
#[test]
fn whitespace_provider_is_rejected_before_any_network() {
    assert!(Config::new("https://geo.example.test/", "Host/1", "   ", Duration::ZERO).is_err());
}
