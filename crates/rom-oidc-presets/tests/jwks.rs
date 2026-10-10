//! Authored public JWKS profile inputs; not native login qualification.
use rom_auth::AuthError;
use rom_oidc_presets::IssuerPreset;
use serde_json::{Value, json};
fn key() -> Value {
    // Canonical base64url for a 2048-bit odd authored modulus. Signed tests use actual RSA keys.
    let n = format!("gYGB{}", "gYGB".repeat(84));
    json!({"kty":"RSA","alg":"RS256","kid":"exact-key","use":"sig","n":format!("{n}gQ"),"e":"AQAB"})
}
fn result(v: Value) -> Result<(), AuthError> {
    IssuerPreset::exact_https("https://issuer.fixture.test/realm")
        .unwrap()
        .configure_jwks("employees", "rom-web", &serde_json::to_vec(&v).unwrap())
        .map(|_| ())
}
#[test]
fn selected_signing_keys_ignore_encryption_and_extensions() {
    let mut encryption = key();
    encryption["alg"] = json!("RSA-OAEP");
    encryption["use"] = json!("enc");
    encryption["kid"] = json!("encrypt");
    let mut signing = key();
    signing["x5u"] = json!("https://never-fetch.invalid/certificate");
    signing["provider_extension"] = json!({"future":true});
    assert!(result(json!({"keys":[encryption,signing]})).is_ok());
}
#[test]
fn duplicate_ids_private_fields_and_nulls_are_rejected() {
    assert_eq!(
        result(json!({"keys":[key(),key()]})),
        Err(AuthError::Invalid)
    );
    for field in ["d", "p", "q", "dp", "dq", "qi", "oth", "k"] {
        let mut k = key();
        k[field] = json!(null);
        assert_eq!(result(json!({"keys":[k]})), Err(AuthError::Invalid));
    }
    for field in ["use", "alg", "key_ops", "n", "e"] {
        let mut k = key();
        k[field] = json!(null);
        assert_eq!(result(json!({"keys":[k]})), Err(AuthError::Invalid));
    }
}
#[test]
fn operations_and_rsa_integer_encoding_are_explicit() {
    for operations in [
        json!([]),
        json!(["sign"]),
        json!(["verify", "verify"]),
        json!(["verify", "encrypt"]),
    ] {
        let mut k = key();
        k["key_ops"] = operations;
        assert_eq!(result(json!({"keys":[k]})), Err(AuthError::Invalid));
    }
    let mut k = key();
    k["key_ops"] = json!(["verify"]);
    assert!(result(json!({"keys":[k]})).is_ok());
    for e in ["", "AAEAAQ", "Ag", "AQ", "AQAB=", "AQ+/"] {
        let mut k = key();
        k["e"] = json!(e);
        assert_eq!(result(json!({"keys":[k]})), Err(AuthError::Invalid));
    }
}
#[test]
fn empty_oversized_ambiguous_and_wrong_profile_sets_fail() {
    assert_eq!(result(json!({"keys":[]})), Err(AuthError::WrongProfile));
    let mut unsupported = key();
    unsupported["alg"] = json!("PS256");
    assert_eq!(
        result(json!({"keys":[unsupported]})),
        Err(AuthError::WrongProfile)
    );
    let mut keys = Vec::new();
    for i in 0..9 {
        let mut k = key();
        k["kid"] = json!(format!("key-{i}"));
        keys.push(k);
    }
    assert_eq!(result(json!({"keys":keys})), Err(AuthError::TooLarge));
    let p = IssuerPreset::exact_https("https://issuer.fixture.test").unwrap();
    assert_eq!(
        p.configure_jwks("employees", "rom-web", &vec![b' '; 65537])
            .err(),
        Some(AuthError::TooLarge)
    );
    assert_eq!(
        p.configure_jwks("employees", "rom-web", br#"{"keys":[],"keys":[]}"#)
            .err(),
        Some(AuthError::Invalid)
    );
}
#[test]
fn boundary_moduli_counts_key_ids_and_duplicate_names_are_rejected() {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    for bytes in [
        vec![0x81; 255],
        vec![0x81; 1025],
        vec![0x80; 256],
        vec![0; 256],
    ] {
        let mut k = key();
        k["n"] = json!(URL_SAFE_NO_PAD.encode(bytes));
        assert!(matches!(
            result(json!({"keys":[k]})),
            Err(AuthError::Invalid | AuthError::TooLarge)
        ));
    }
    let mut keys = Vec::new();
    for i in 0..33 {
        let mut k = key();
        k["kid"] = json!(format!("key-{i}"));
        k["alg"] = json!("PS256");
        keys.push(k);
    }
    assert_eq!(result(json!({"keys":keys})), Err(AuthError::TooLarge));
    let mut k = key();
    k["kid"] = json!("x".repeat(65));
    assert_eq!(result(json!({"keys":[k]})), Err(AuthError::TooLarge));
    let mut ignored = key();
    ignored["alg"] = json!("RSA-OAEP");
    ignored["use"] = json!("enc");
    assert_eq!(
        result(json!({"keys":[key(),ignored]})),
        Err(AuthError::Invalid)
    );
    let p = IssuerPreset::exact_https("https://issuer.fixture.test").unwrap();
    for body in [
        br#"{"keys":[{"kid":"a","kid":"b","kty":"EC"}]}"#.as_slice(),
        br#"{"keys":[{"kid":"a","kty":"RSA","alg":"RS256","alg":"PS256"}]}"#.as_slice(),
    ] {
        assert_eq!(
            p.configure_jwks("employees", "rom-web", body).err(),
            Some(AuthError::Invalid)
        );
    }
}
