//! Local endpoint/admission bounds; not service qualification.
use rom_map_http::Config;
use std::time::Duration;
fn config(endpoint: &str) -> rom_map_core::Result<Config> {
    Config::new(
        endpoint,
        "ROM fixture/1 (operator@example.test)",
        1048576,
        Duration::from_secs(1),
        1,
    )
}
#[test]
fn explicit_verified_https_origin_and_base_path_are_admitted() {
    assert!(config("https://maps.example.test").is_ok());
    assert!(config("https://maps.example.test/operator/nominatim/").is_ok());
}
#[test]
fn endpoint_credentials_and_ambiguous_paths_are_rejected() {
    for endpoint in [
        "http://maps.example.test",
        "https://secret@maps.example.test",
        "https://maps.example.test?key=secret",
        "https://maps.example.test/#secret",
        "https://maps.example.test/operator",
        "https://maps.example.test/a/../",
        "https://maps.example.test/a\\b/",
        "https://maps.example.test/%2f/",
    ] {
        assert!(config(endpoint).is_err());
    }
}
#[test]
fn response_and_admission_limits_are_finite_and_explicit() {
    for limit in [0, 1048577] {
        assert!(
            Config::new(
                "https://maps.example.test/",
                "Host/1",
                limit,
                Duration::ZERO,
                1
            )
            .is_err()
        );
    }
    for n in [0, 9] {
        assert!(
            Config::new(
                "https://maps.example.test/",
                "Host/1",
                1024,
                Duration::ZERO,
                n
            )
            .is_err()
        );
    }
    assert!(
        Config::new(
            "https://maps.example.test/",
            "Host/1",
            1024,
            Duration::from_secs(61),
            1
        )
        .is_err()
    );
    assert!(Config::new("https://maps.example.test/", "", 1024, Duration::ZERO, 1).is_err());
    assert!(
        Config::new(
            "https://maps.example.test/",
            "Host\r\nInjected: value",
            1024,
            Duration::ZERO,
            1
        )
        .is_err()
    );
    assert!(
        Config::new(
            "https://maps.example.test/",
            "Host/1",
            1024,
            Duration::ZERO,
            8
        )
        .is_ok()
    );
}
