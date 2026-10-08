//! Signed synthetic tokens through the preset's actual ROM verifier.
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, encode};
use rom_auth::{
    AuthError, IdentityProfile, PrincipalKind, jwt::TrustedKeys, oidc::OidcTokenBindings,
};
use rom_oidc_presets::IssuerPreset;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    io::Write,
    process::{Command, Stdio},
    sync::{Arc, Mutex, OnceLock},
};

const NOW: u64 = 1_800_000_000;
const NONCE: &str = "host-retained-nonce";

struct SigningKey {
    private: Vec<u8>,
    public: Vec<u8>,
}
fn generate() -> SigningKey {
    let generated = Command::new("openssl")
        .args([
            "genpkey",
            "-algorithm",
            "RSA",
            "-pkeyopt",
            "rsa_keygen_bits:2048",
        ])
        .stderr(Stdio::null())
        .output()
        .expect("OpenSSL is required for real synthetic signatures");
    assert!(generated.status.success());
    let mut child = Command::new("openssl")
        .args(["pkey", "-pubout"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&generated.stdout)
        .unwrap();
    let public = child.wait_with_output().unwrap();
    assert!(public.status.success());
    SigningKey {
        private: generated.stdout,
        public: public.stdout,
    }
}
fn signing_keys() -> &'static [SigningKey; 2] {
    static KEYS: OnceLock<[SigningKey; 2]> = OnceLock::new();
    KEYS.get_or_init(|| [generate(), generate()])
}
fn token(issuer: &str, kid: &str, index: usize, change: impl FnOnce(&mut Value)) -> String {
    let mut claims = json!({"iss":issuer,"sub":"person-17","aud":"rom-web","exp":NOW+600,"iat":NOW,"nonce":NONCE});
    change(&mut claims);
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(kid.into());
    encode(
        &header,
        &claims,
        &EncodingKey::from_rsa_pem(&signing_keys()[index].private).unwrap(),
    )
    .unwrap()
}
#[derive(Clone)]
struct Keys(Arc<Mutex<(String, usize)>>);
impl TrustedKeys for Keys {
    fn fetch(&mut self) -> Result<BTreeMap<String, DecodingKey>, AuthError> {
        let (kid, index) = self.0.lock().unwrap().clone();
        Ok(BTreeMap::from([(
            kid,
            DecodingKey::from_rsa_pem(&signing_keys()[index].public).unwrap(),
        )]))
    }
}
fn keys() -> Keys {
    Keys(Arc::new(Mutex::new(("original".into(), 0))))
}

#[test]
fn every_preset_preserves_rom_human_evidence_and_expiry() {
    let presets = [
        IssuerPreset::exact_https("https://dex.example/issuer/").unwrap(),
        IssuerPreset::keycloak("https://login.example/auth", "staff").unwrap(),
        IssuerPreset::entra_tenant("12345678-abcd-4321-abcd-1234567890ab").unwrap(),
    ];
    for preset in presets {
        let mut adapter = preset.configure("employees", "rom-web", keys()).unwrap();
        let signed = token(preset.issuer(), "original", 0, |_| {});
        let proof = adapter
            .authenticate(&signed, NONCE, OidcTokenBindings::default(), NOW)
            .unwrap();
        assert_eq!(proof.authority(), "employees");
        assert_eq!(proof.subject(), "person-17");
        assert_eq!(proof.principal_kind(), PrincipalKind::Human);
        assert_eq!(proof.profile(), IdentityProfile::OidcRs256Human);
        assert_eq!(proof.valid_until(), NOW + 30);
    }
}

#[test]
fn binding_expiry_and_signature_failures_cannot_create_evidence() {
    let preset = IssuerPreset::exact_https("https://dex.example/issuer/").unwrap();
    for (claim, invalid) in [
        ("iss", json!("https://dex.example/issuer")),
        ("aud", json!("another-app")),
        ("nonce", json!("unretained")),
        ("exp", json!(NOW)),
    ] {
        let mut adapter = preset.configure("employees", "rom-web", keys()).unwrap();
        let signed = token(preset.issuer(), "original", 0, |claims| {
            claims[claim] = invalid
        });
        assert!(
            adapter
                .authenticate(&signed, NONCE, OidcTokenBindings::default(), NOW)
                .is_err(),
            "accepted invalid {claim}"
        );
    }
    let mut adapter = preset.configure("employees", "rom-web", keys()).unwrap();
    let signed = token(preset.issuer(), "original", 1, |_| {});
    assert!(
        adapter
            .authenticate(&signed, NONCE, OidcTokenBindings::default(), NOW)
            .is_err()
    );
}

#[test]
fn rotation_replaces_keys_without_changing_the_issuer_or_profile() {
    let preset = IssuerPreset::keycloak("https://login.example", "staff").unwrap();
    let source = keys();
    let mut adapter = preset
        .configure("employees", "rom-web", source.clone())
        .unwrap();
    let original = token(preset.issuer(), "original", 0, |_| {});
    assert!(
        adapter
            .authenticate(&original, NONCE, OidcTokenBindings::default(), NOW)
            .is_ok()
    );
    *source.0.lock().unwrap() = ("rotated".into(), 1);
    let rotated = token(preset.issuer(), "rotated", 1, |_| {});
    assert!(matches!(
        adapter.authenticate(&rotated, NONCE, OidcTokenBindings::default(), NOW + 1),
        Err(AuthError::RefreshLimited)
    ));
    let proof = adapter
        .authenticate(&rotated, NONCE, OidcTokenBindings::default(), NOW + 5)
        .unwrap();
    assert_eq!(proof.principal_kind(), PrincipalKind::Human);
    assert!(
        adapter
            .authenticate(&original, NONCE, OidcTokenBindings::default(), NOW + 10)
            .is_err()
    );
}
