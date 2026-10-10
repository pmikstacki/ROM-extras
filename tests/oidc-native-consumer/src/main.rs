use rom_auth::{IdentityProfile, PrincipalKind, oidc::OidcTokenBindings};
use rom_oidc_presets::IssuerPreset;
use serde_json::Value;
use std::{fs, path::Path};

mod resources;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let path = std::env::args_os()
        .nth(1)
        .expect("private input path required");
    let bytes = fs::read(Path::new(&path)).expect("private input readable");
    assert!(bytes.len() <= 131072);
    let input: Value = serde_json::from_slice(&bytes).expect("input JSON");
    let value = |key: &str| input[key].as_str().expect("required input field");
    let now = input["now"].as_u64().expect("trusted clock");
    let preset = IssuerPreset::exact_https(value("issuer")).expect("explicit issuer");
    let mut verifier = preset
        .configure_jwks(
            "fixture-humans",
            value("client_id"),
            value("jwks").as_bytes(),
        )
        .expect("approved public snapshot");
    let bindings = OidcTokenBindings {
        access_token: Some(value("access_token")),
        authorization_code: Some(value("authorization_code")),
    };
    let proof = verifier
        .authenticate(value("id_token"), value("nonce"), bindings, now)
        .expect("native human ID token");
    assert_eq!(proof.principal_kind(), PrincipalKind::Human);
    assert_eq!(proof.profile(), IdentityProfile::OidcRs256Human);
    assert_eq!(proof.authority(), "fixture-humans");
    assert!(!proof.subject().is_empty());
    assert_eq!(proof.valid_until(), now + 30);
    assert!(
        verifier
            .authenticate(value("id_token"), "wrong-nonce", bindings, now)
            .is_err()
    );
    assert!(
        verifier
            .authenticate(value("id_token"), value("nonce"), bindings, now + 3601)
            .is_err()
    );
    let mut wrong_audience = preset
        .configure_jwks("fixture-humans", "wrong-client", value("jwks").as_bytes())
        .unwrap();
    assert!(
        wrong_audience
            .authenticate(value("id_token"), value("nonce"), bindings, now)
            .is_err()
    );
    if let Some(previous) = input["previous_jwks"].as_str() {
        let mut old = preset
            .configure_jwks("fixture-humans", value("client_id"), previous.as_bytes())
            .unwrap();
        assert!(
            old.authenticate(value("id_token"), value("nonce"), bindings, now)
                .is_err(),
            "old snapshot accepted native rotated key"
        );
    }
    if let Some(revoked) = input["revoked"].as_object() {
        let old = |key: &str| revoked[key].as_str().unwrap();
        let mut fresh = preset
            .configure_jwks(
                "fixture-humans",
                value("client_id"),
                value("jwks").as_bytes(),
            )
            .unwrap();
        assert_eq!(
            fresh
                .authenticate(
                    old("id_token"),
                    old("nonce"),
                    OidcTokenBindings {
                        access_token: Some(old("access_token")),
                        authorization_code: Some(old("authorization_code"))
                    },
                    revoked["now"].as_u64().unwrap()
                )
                .err(),
            Some(rom_auth::AuthError::UnknownKey)
        );
    }
    let root = std::env::args_os()
        .nth(2)
        .expect("private retained database directory required");
    resources::qualify(&input, Path::new(&root)).await;
    println!("Native RS256 human token, nonce, audience and expiry verified through public API.");
}
