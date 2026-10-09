//! Pure wire and fixed-origin admission boundaries; no external fixture is required.
use rom_opensearch::TlsConfig;
use rom_projection_core::{Error, ProjectionProfile};
use std::time::Duration;
#[test]
fn tls_origin_rejects_credential_paths_redirect_inputs_and_unbounded_material() {
    for url in [
        "http://127.0.0.1/",
        "https://user@127.0.0.1/",
        "https://127.0.0.1/private",
        "https://127.0.0.1/?token=a",
        "https://127.0.0.1/#secret",
    ] {
        assert!(matches!(
            TlsConfig::new(url, vec![1], vec![1], Duration::from_secs(5)),
            Err(Error::Invalid)
        ));
    }
    assert!(matches!(
        TlsConfig::new("https://127.0.0.1/", vec![1], vec![1], Duration::ZERO),
        Err(Error::Invalid)
    ));
    assert!(matches!(
        TlsConfig::new(
            "https://127.0.0.1/",
            vec![1],
            vec![1; 32769],
            Duration::from_secs(5)
        ),
        Err(Error::TooLarge)
    ));
}
#[test]
fn profile_fingerprint_includes_model_mapping_provider_and_deployment() {
    let make = |a, b, c, d| ProjectionProfile::new(a, b, c, d).unwrap().fingerprint();
    let original = make("a", "b", "c", None);
    assert_eq!(original, make("a", "b", "c", None));
    for changed in [
        make("x", "b", "c", None),
        make("a", "x", "c", None),
        make("a", "b", "x", None),
        make("a", "b", "c", Some("model")),
    ] {
        assert_ne!(original, changed);
    }
}
