//! Public configuration admission must reject descriptor injection before any native connection.
use rom_oracle::{Config, Deadlines, TlsWallet, Value};
use std::time::Duration;
#[test]
fn unspecified_and_multicast_ip_must_not_fall_back_to_dns() {
    for host in ["0.0.0.0", "::", "224.0.0.1", "ff02::1"] {
        assert!(
            Config::new(host, 1521, "service", "host-user", "secret-canary").is_err(),
            "special IP must be rejected"
        );
    }
    for host in ["127.0.0.1", "::1", "database.example.invalid"] {
        assert!(Config::new(host, 1521, "service", "host-user", "secret-canary").is_ok());
    }
}
#[test]
fn descriptor_delimiters_and_native_defaults_are_not_admitted() {
    for host in [
        "",
        "host)(PROTOCOL=TCP",
        "host\n",
        "host/path",
        "-host",
        "host-",
    ] {
        assert!(Config::new(host, 1521, "service", "user", "secret").is_err());
    }
    assert!(Config::new("localhost", 0, "service", "user", "secret").is_err());
    assert!(Config::new("localhost", 1521, "svc)(HOST=other)", "user", "secret").is_err());
    assert!(Config::new("localhost", 1521, "service", "user", "").is_err());
    assert!(TlsWallet::new("relative", "CN=server").is_err());
    assert!(TlsWallet::new("/tmp/../wallet", "CN=server").is_err());
    assert!(TlsWallet::new("/tmp/wallet", "CN=server)(SSL_SERVER_DN_MATCH=NO)").is_err());
}
#[test]
fn roundtrip_and_lock_units_are_explicit() {
    let s = Duration::from_secs;
    assert!(Deadlines::new(s(10), s(3), s(3), s(1)).is_ok());
    for bounds in [
        (s(0), s(1), s(3), s(1)),
        (s(3), s(3), s(3), s(1)),
        (s(10), s(3), s(3), s(3)),
        (s(10), s(3), s(3), Duration::from_millis(500)),
        (s(61), s(3), s(3), s(1)),
        (s(10), s(3), Duration::from_nanos(1), s(1)),
    ] {
        assert!(Deadlines::new(bounds.0, bounds.1, bounds.2, bounds.3).is_err());
    }
}
#[test]
fn debug_hides_host_credentials_wallet_and_bound_material() {
    let config = Config::new(
        "database-canary.example.invalid",
        1521,
        "service",
        "user-canary",
        "password-canary",
    )
    .unwrap();
    let wallet = TlsWallet::new("/tmp/wallet-canary", "CN=certificate-canary").unwrap();
    let value = Value::Raw(Some(b"payload-canary".to_vec()));
    let text = format!("{config:?}{wallet:?}{value:?}");
    assert!(!text.contains("canary"));
}
