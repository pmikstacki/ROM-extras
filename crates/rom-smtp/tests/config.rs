//! Safe declaration and diagnostic contracts without network I/O.
use rom_email_core::EmailProfile;
use rom_smtp::{Credentials, Endpoint, Limits, Smtp};
use std::{net::SocketAddr, time::Duration};
#[test]
fn endpoints_limits_and_credentials_reject_ambiguous_or_unbounded_inputs() {
    let address: SocketAddr = "127.0.0.1:1025".parse().unwrap();
    for name in ["", "https://smtp.example.invalid", "a\r\nX", "a:25"] {
        assert!(Endpoint::new(address, name, "host.example.invalid").is_err());
    }
    for address in ["0.0.0.0:25", "224.0.0.1:25", "127.0.0.1:0"] {
        assert!(
            Endpoint::new(
                address.parse().unwrap(),
                "smtp.example.invalid",
                "host.example.invalid"
            )
            .is_err()
        );
    }
    for value in ["", "a\0b", "a\nb"] {
        assert!(Credentials::new("fixture".into(), value.into()).is_err());
    }
    assert!(Credentials::new("a".repeat(1025), "secret".into()).is_err());
    for timeout in [Duration::ZERO, Duration::from_secs(61)] {
        assert!(Limits::new(timeout, 65536, Duration::ZERO).is_err());
    }
    for size in [0, 1023, 65537] {
        assert!(Limits::new(Duration::from_secs(1), size, Duration::ZERO).is_err());
    }
    assert!(Limits::new(Duration::from_secs(1), 65536, Duration::from_secs(86401)).is_err());
}
#[test]
fn private_credentials_and_configuration_diagnostics_are_opaque() {
    let credentials = Credentials::new("private-user".into(), "private-password".into()).unwrap();
    let debug = format!("{credentials:?}");
    assert!(!debug.contains("private-user"));
    assert!(!debug.contains("private-password"));
    let endpoint = Endpoint::new(
        "127.0.0.1:1025".parse().unwrap(),
        "smtp.fixture.test",
        "host.example.invalid",
    )
    .unwrap();
    let profile = EmailProfile::new(
        "sender@example.invalid",
        &["to@example.invalid"],
        "host.example.invalid",
        1048576,
    )
    .unwrap();
    let limits = Limits::new(Duration::from_secs(1), 65536, Duration::ZERO).unwrap();
    let result =
        Smtp::with_private_root(endpoint, credentials, profile, limits, b"not a certificate");
    assert!(result.is_err());
}
