//! Real service smoke test against extracted Cargo archives only.
use rom_kms::{Binding, KeyRef, Kms};
use rom_openbao::{Config, KeyLocation, Limits, OpenBao, SecretLocation};
use rom_secrets::{SecretBytes, SecretRef, SecretResolver, Version};
#[tokio::main(flavor = "current_thread")]
async fn main() {
    let endpoint = std::env::var("ROM_EXTRAS_OPENBAO_ENDPOINT").unwrap();
    assert_eq!(endpoint, "https://127.0.0.1:55459");
    let adapter = OpenBao::new(
        Config {
            endpoint,
            token: SecretBytes::new(
                std::env::var("ROM_EXTRAS_OPENBAO_TOKEN")
                    .unwrap()
                    .into_bytes(),
            )
            .unwrap(),
            ca_pem: Some(std::fs::read(std::env::var("ROM_EXTRAS_OPENBAO_CA").unwrap()).unwrap()),
            profile: SecretRef::new("packaged-openbao").unwrap(),
            secrets: vec![(
                SecretRef::new("oidc-client").unwrap(),
                SecretLocation::new("secret", "rom-extras/client", "credential").unwrap(),
            )],
            keys: vec![(
                KeyRef::new("application-key").unwrap(),
                KeyLocation::new("transit", "rom-extras").unwrap(),
            )],
        },
        Limits::default(),
    )
    .unwrap();
    let secret = adapter
        .resolve(&SecretRef::new("oidc-client").unwrap(), Version::Latest)
        .await
        .unwrap();
    let markers: serde_json::Value = serde_json::from_slice(
        &std::fs::read(std::env::var("ROM_EXTRAS_OPENBAO_MARKERS").unwrap()).unwrap(),
    )
    .unwrap();
    assert_eq!(secret.version(), 2);
    assert!(
        secret.bytes().expose() == markers["v2"].as_str().unwrap().as_bytes(),
        "packaged material mismatch"
    );
    let plain = SecretBytes::new(vec![0, 255, 0, 1]).unwrap();
    let binding = Binding::new(b"packaged-tenant".to_vec(), b"packaged-resource".to_vec()).unwrap();
    let envelope = adapter
        .encrypt(&KeyRef::new("application-key").unwrap(), &plain, &binding)
        .await
        .unwrap();
    let decoded = adapter.decrypt(&envelope, &binding).await.unwrap();
    assert!(
        decoded.expose() == plain.expose(),
        "packaged ciphertext mismatch"
    );
    println!("Packaged KV and derived Transit smoke passed.");
}
