//! Exact issuer binding and configuration rejection scenarios.
use rom_auth::{
    AuthError,
    jwt::{DecodingKey, TrustedKeys},
};
use rom_oidc_presets::IssuerPreset;
use std::collections::BTreeMap;

struct NoKeys;
impl TrustedKeys for NoKeys {
    fn fetch(&mut self) -> Result<BTreeMap<String, DecodingKey>, AuthError> {
        panic!("configuration must never fetch keys")
    }
}

#[test]
fn exact_issuer_preserves_the_original_binding() {
    for issuer in [
        "https://login.example",
        "https://login.example/",
        "https://LOGIN.example:443/tenant%2Fa",
    ] {
        assert_eq!(IssuerPreset::exact_https(issuer).unwrap().issuer(), issuer);
    }
}

#[test]
fn invalid_issuer_forms_are_rejected() {
    for issuer in [
        "",
        "http://localhost/realm",
        "https:///example",
        "https://user:secret@example",
        "https://user@example",
        "https://example?x=1",
        "https://example#frag",
        " https://example",
        "https://exam\nple",
        "https://example\\realm",
        "https:example",
        "https://example/a b",
    ] {
        assert!(
            IssuerPreset::exact_https(issuer).is_err(),
            "accepted {issuer:?}"
        );
    }
    assert!(IssuerPreset::exact_https(&format!("https://example/{}", "a".repeat(2048))).is_err());
}

#[test]
fn unicode_whitespace_and_controls_are_rejected_before_url_parsing() {
    for forbidden in ['\u{00a0}', '\u{0085}'] {
        assert!(IssuerPreset::exact_https(&format!("https://example/a{forbidden}b")).is_err());
    }
}

#[test]
fn keycloak_keeps_the_deployment_context_path() {
    for base in ["https://login.example/auth", "https://login.example/auth/"] {
        assert_eq!(
            IssuerPreset::keycloak(base, "staff-eu_1").unwrap().issuer(),
            "https://login.example/auth/realms/staff-eu_1"
        );
    }
    for realm in [
        "",
        ".",
        "..",
        "staff/admin",
        "staff%2Fadmin",
        "staff?other",
        "zażółć",
    ] {
        assert!(IssuerPreset::keycloak("https://login.example", realm).is_err());
    }
}

#[test]
fn entra_binds_one_concrete_tenant() {
    let tenant = "12345678-ABCD-4321-ABCD-1234567890AB";
    assert_eq!(
        IssuerPreset::entra_tenant(tenant).unwrap().issuer(),
        "https://login.microsoftonline.com/12345678-abcd-4321-abcd-1234567890ab/v2.0"
    );
    for tenant in [
        "common",
        "organizations",
        "consumers",
        "example.org",
        "123",
        "12345678-abcd-4321-abcd-1234567890ag",
    ] {
        assert!(IssuerPreset::entra_tenant(tenant).is_err());
    }
}

#[test]
fn configuration_delegates_names_and_does_not_fetch_keys() {
    let preset = IssuerPreset::exact_https("https://login.example").unwrap();
    assert!(preset.configure("staff", "rom-web", NoKeys).is_ok());
    assert!(matches!(
        preset.configure("", "rom-web", NoKeys),
        Err(AuthError::InvalidConfiguration)
    ));
    assert!(matches!(
        preset.configure("staff", " ", NoKeys),
        Err(AuthError::InvalidConfiguration)
    ));
}
