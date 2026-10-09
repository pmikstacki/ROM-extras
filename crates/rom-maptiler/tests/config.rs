//! Configuration admission tests make no network requests.
use rom_map_core::{Attribution, Error};
use rom_maptiler::{AttributionProfile, Config, MapTiler};
use std::time::Duration;
fn profile() -> AttributionProfile {
    AttributionProfile::new("fixture credit", Attribution::new("Fixture", None).unwrap()).unwrap()
}
#[test]
fn explicit_private_credentials_are_required_before_transport_construction() {
    let config = Config::new(
        "https://127.0.0.1:55464/",
        "ROM fixture",
        "fixture",
        Duration::ZERO,
        profile(),
    )
    .unwrap();
    assert!(matches!(MapTiler::new(config), Err(Error::InvalidQuery)));
    let config = Config::new(
        "https://127.0.0.1:55464/",
        "ROM fixture",
        "fixture",
        Duration::ZERO,
        profile(),
    )
    .unwrap()
    .with_server_key("synthetic-private-key".into())
    .unwrap();
    assert!(MapTiler::new(config).is_ok());
}
#[test]
fn endpoint_and_provider_validation_fail_closed() {
    for endpoint in [
        "http://localhost/",
        "https://user:password@localhost/",
        "https://localhost/?key=secret",
        "https://localhost/geocoding",
    ] {
        assert!(
            Config::new(
                endpoint,
                "ROM fixture",
                "fixture",
                Duration::ZERO,
                profile()
            )
            .is_err()
        );
    }
    for provider in ["", "   ", "bad\nprovider"] {
        assert!(
            Config::new(
                "https://localhost/",
                "ROM fixture",
                provider,
                Duration::ZERO,
                profile()
            )
            .is_err()
        );
    }
}
