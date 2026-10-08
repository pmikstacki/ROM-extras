//! Configuration rejects unsafe endpoints and ambiguous locations before I/O.
use rom_openbao::{Config, Limits, OpenBao, SecretLocation};
use rom_secrets::{Error, SecretBytes, SecretRef};
fn config(endpoint: &str) -> Config {
    Config {
        endpoint: endpoint.into(),
        token: SecretBytes::new(b"fixture-token".to_vec()).unwrap(),
        ca_pem: None,
        profile: SecretRef::new("fixture").unwrap(),
        secrets: Vec::new(),
        keys: Vec::new(),
    }
}
#[test]
fn endpoint_cannot_redirect_credential_authority_or_embed_private_data() {
    for endpoint in [
        "http://127.0.0.1:55459",
        "https://user:password@localhost",
        "https://localhost/private",
        "https://localhost/?token=value",
        "https://localhost/#fragment",
    ] {
        assert!(matches!(
            OpenBao::new(config(endpoint), Limits::default()),
            Err(Error::Invalid)
        ));
    }
}
#[test]
fn locations_reject_path_injection_and_duplicate_aliases() {
    for path in [
        "../client",
        "a//b",
        "/client",
        "a%2Fb",
        "a?version=1",
        "a#x",
    ] {
        assert!(SecretLocation::new("secret", path, "credential").is_err());
    }
    let mut config = config("https://localhost:55459");
    for _ in 0..2 {
        config.secrets.push((
            SecretRef::new("client").unwrap(),
            SecretLocation::new("secret", "approved/client", "credential").unwrap(),
        ));
    }
    assert!(matches!(
        OpenBao::new(config, Limits::default()),
        Err(Error::Invalid)
    ));
}
