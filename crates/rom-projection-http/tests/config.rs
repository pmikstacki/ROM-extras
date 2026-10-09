//! Catch unsafe API-key admission and missing fixed-origin bounds.
use rom_projection_core::Error;
use rom_projection_http::TlsConfig;
use std::time::Duration;
#[test]
fn valid_api_key_configuration_is_admitted_without_client_identity() {
    assert!(
        TlsConfig::api_key(
            "https://127.0.0.1:55461",
            vec![1],
            b"opaque-fixture-key".to_vec(),
            Duration::from_secs(5)
        )
        .is_ok()
    );
}
#[test]
fn api_keys_cannot_inject_headers_and_unbounded_material_is_rejected() {
    for key in [
        b"".as_slice(),
        b"secret\r\nInjected: yes".as_slice(),
        b"secret\0hidden".as_slice(),
    ] {
        assert!(matches!(
            TlsConfig::api_key(
                "https://127.0.0.1",
                vec![1],
                key.to_vec(),
                Duration::from_secs(5)
            ),
            Err(Error::Invalid)
        ));
    }
    assert!(matches!(
        TlsConfig::api_key(
            "https://127.0.0.1",
            vec![1],
            vec![b'x'; 4097],
            Duration::from_secs(5)
        ),
        Err(Error::TooLarge)
    ));
}
#[test]
fn api_key_authentication_keeps_fixed_https_origin_and_deadline_rules() {
    for url in [
        "http://127.0.0.1",
        "https://user:secret@127.0.0.1",
        "https://127.0.0.1/private",
        "https://127.0.0.1/?key=secret",
        "https://127.0.0.1/#secret",
    ] {
        assert!(matches!(
            TlsConfig::api_key(url, vec![1], vec![b'x'], Duration::from_secs(5)),
            Err(Error::Invalid)
        ));
    }
    assert!(matches!(
        TlsConfig::api_key("https://127.0.0.1", vec![1], vec![b'x'], Duration::ZERO),
        Err(Error::Invalid)
    ));
}
