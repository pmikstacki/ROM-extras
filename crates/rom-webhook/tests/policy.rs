//! Destination and admission policy regressions.
use rom_webhook::{Destination, TransportLimits};
use std::{net::IpAddr, time::Duration};

// Removing rejection lets externally selected destinations reach local services.
#[test]
fn refuses_non_public_resolutions_and_ambiguous_urls() {
    for ip in [
        "127.0.0.1",
        "10.0.0.1",
        "100.64.0.1",
        "169.254.169.254",
        "192.0.2.1",
        "224.0.0.1",
        "::1",
        "::ffff:127.0.0.1",
        "64:ff9b::7f00:1",
        "2001:db8::1",
        "2002:7f00:1::",
        "3fff::1",
        "fc00::1",
        "fe80::1",
    ] {
        assert!(
            Destination::public_resolved(
                "https://receiver.example/hook",
                &[ip.parse::<IpAddr>().unwrap()]
            )
            .is_err(),
            "{ip}"
        );
    }
    for url in [
        "http://receiver.example/",
        "https://user:pass@receiver.example/",
        "https://receiver.example/#fragment",
        " https://receiver.example/",
        "https://receiver.example/\\path",
    ] {
        assert!(Destination::public_resolved(url, &["8.8.8.8".parse().unwrap()]).is_err());
    }
    assert!(Destination::public_resolved("https://receiver.example/", &[]).is_err());
    assert!(
        Destination::public_resolved(
            "https://receiver.example/",
            &["8.8.8.8".parse().unwrap(), "127.0.0.1".parse().unwrap()]
        )
        .is_err()
    );
    assert!(
        Destination::public_resolved("https://127.0.0.1/", &["8.8.8.8".parse().unwrap()]).is_err()
    );
    assert!(
        Destination::public_resolved("https://8.8.8.8/", &["1.1.1.1".parse().unwrap()]).is_err()
    );
    assert!(
        Destination::public_resolved(
            "https://receiver.example:8443/hook",
            &[
                "8.8.8.8".parse().unwrap(),
                "2606:4700::1111".parse().unwrap()
            ]
        )
        .is_ok()
    );
}

#[test]
fn refuses_unbounded_or_inconsistent_transport_limits() {
    assert!(TransportLimits::new(0, Duration::from_secs(1), Duration::from_secs(2)).is_err());
    assert!(TransportLimits::new(1, Duration::ZERO, Duration::from_secs(2)).is_err());
    assert!(TransportLimits::new(1, Duration::from_secs(3), Duration::from_secs(2)).is_err());
    assert!(TransportLimits::new(1, Duration::from_secs(1), Duration::from_secs(301)).is_err());
    assert!(TransportLimits::new(1, Duration::from_secs(1), Duration::from_secs(2)).is_ok());
}
