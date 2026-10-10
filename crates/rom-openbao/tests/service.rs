//! Real OpenBao KV and derived Transit checks; no missing-backend skips.
#![cfg(feature = "service-fixture")]
use rom_kms::{Binding, KeyRef, Kms};
use rom_openbao::{Config, KeyLocation, Limits, OpenBao, SecretLocation};
use rom_secrets::{Error, SecretBytes, SecretRef, SecretResolver, Version};

fn config() -> Config {
    let endpoint = std::env::var("ROM_EXTRAS_OPENBAO_ENDPOINT").expect("fixture endpoint required");
    assert_eq!(endpoint, "https://127.0.0.1:55459");
    let token = std::env::var("ROM_EXTRAS_OPENBAO_TOKEN").expect("limited token required");
    let ca = std::fs::read(std::env::var("ROM_EXTRAS_OPENBAO_CA").expect("private CA required"))
        .unwrap();
    Config {
        endpoint,
        token: SecretBytes::new(token.into_bytes()).unwrap(),
        ca_pem: Some(ca),
        profile: SecretRef::new("openbao-fixture").unwrap(),
        secrets: vec![
            (
                SecretRef::new("oidc-client").unwrap(),
                SecretLocation::new("secret", "rom-extras/client", "credential").unwrap(),
            ),
            (
                SecretRef::new("denied").unwrap(),
                SecretLocation::new("secret", "rom-extras/denied", "credential").unwrap(),
            ),
        ],
        keys: vec![(
            KeyRef::new("application-key").unwrap(),
            KeyLocation::new("transit", "rom-extras").unwrap(),
        )],
    }
}
fn adapter(limits: Limits) -> OpenBao {
    OpenBao::new(config(), limits).unwrap()
}
#[tokio::test]
async fn actual_kv_versions_and_denied_reference_do_not_fall_back() {
    let adapter = adapter(Limits::default());
    let reference = SecretRef::new("oidc-client").unwrap();
    let pinned = adapter
        .resolve(&reference, Version::pinned(1).unwrap())
        .await
        .unwrap();
    let latest = adapter.resolve(&reference, Version::Latest).await.unwrap();
    assert_eq!(pinned.version(), 1);
    assert_eq!(latest.version(), 2);
    let expected_path =
        std::env::var("ROM_EXTRAS_OPENBAO_MARKERS").expect("protected fixture markers required");
    let expected: serde_json::Value =
        serde_json::from_slice(&std::fs::read(expected_path).unwrap()).unwrap();
    assert!(
        pinned.bytes().expose() == expected["v1"].as_str().unwrap().as_bytes(),
        "pinned material mismatch"
    );
    assert!(
        latest.bytes().expose() == expected["v2"].as_str().unwrap().as_bytes(),
        "latest material mismatch"
    );

    assert!(
        pinned.bytes().expose() != latest.bytes().expose(),
        "rotation must change material"
    );
    assert!(matches!(
        adapter
            .resolve(&SecretRef::new("unapproved").unwrap(), Version::Latest)
            .await,
        Err(Error::Invalid)
    ));
    assert!(matches!(
        adapter
            .resolve(&SecretRef::new("denied").unwrap(), Version::Latest)
            .await,
        Err(Error::Denied)
    ));
}
#[tokio::test]
async fn actual_transit_authenticates_context_and_aad_before_exposure() {
    let adapter = adapter(Limits::default());
    let key = KeyRef::new("application-key").unwrap();
    let plain = SecretBytes::new(vec![0, 1, 0, 255]).unwrap();
    let binding = Binding::new(b"tenant-one".to_vec(), b"resource-one".to_vec()).unwrap();
    let encrypted = adapter.encrypt(&key, &plain, &binding).await.unwrap();
    assert_eq!(encrypted.version(), 1);
    let decrypted = adapter.decrypt(&encrypted, &binding).await.unwrap();
    assert!(
        decrypted.expose() == plain.expose(),
        "authenticated plaintext mismatch"
    );
    for bad in [
        Binding::new(b"tenant-two".to_vec(), b"resource-one".to_vec()).unwrap(),
        Binding::new(b"tenant-one".to_vec(), b"resource-two".to_vec()).unwrap(),
    ] {
        assert!(matches!(
            adapter.decrypt(&encrypted, &bad).await,
            Err(Error::Rejected)
        ));
    }
}

#[tokio::test]
async fn actual_transit_preserves_empty_and_maximum_owned_plaintext() {
    let adapter = adapter(Limits::default());
    let key = KeyRef::new("application-key").unwrap();
    let binding = Binding::new(vec![42; 4096], vec![7; 4096]).unwrap();
    for bytes in [Vec::new(), vec![255; 4096]] {
        let plain = SecretBytes::new(bytes).unwrap();
        let envelope = adapter.encrypt(&key, &plain, &binding).await.unwrap();
        let decoded = adapter.decrypt(&envelope, &binding).await.unwrap();
        assert!(
            decoded.expose() == plain.expose(),
            "bounded plaintext mismatch"
        );
    }
}

#[tokio::test]
async fn response_limit_applies_to_success_and_permission_error_bodies() {
    let adapter = adapter(Limits {
        max_response_bytes: 1,
        ..Limits::default()
    });
    for alias in ["oidc-client", "denied"] {
        assert!(matches!(
            adapter
                .resolve(&SecretRef::new(alias).unwrap(), Version::Latest)
                .await,
            Err(Error::Limit)
        ));
    }
}

#[tokio::test]
async fn fixture_rejects_unknown_ca_and_unmatched_ip_identity() {
    let reference = SecretRef::new("oidc-client").unwrap();
    let mut wrong_ca = config();
    wrong_ca.ca_pem = None;
    let client = OpenBao::new(wrong_ca, Limits::default()).unwrap();
    assert!(matches!(
        client.resolve(&reference, Version::Latest).await,
        Err(Error::Unavailable)
    ));
    let mut wrong_name = config();
    let probe = std::process::Command::new("timeout")
        .env("LC_ALL", "C")
        .args([
            "--kill-after=2s",
            "8s",
            "curl",
            "--noproxy",
            "*",
            "--silent",
            "--show-error",
            "--max-time",
            "5",
            "--cacert",
        ])
        .arg(std::env::var("ROM_EXTRAS_OPENBAO_CA").unwrap())
        .args(["https://[::ffff:127.0.0.1]:55459/v1/sys/seal-status"])
        .output()
        .unwrap();
    assert_eq!(
        probe.status.code(),
        Some(60),
        "independent certificate rejection required"
    );
    assert!(
        String::from_utf8_lossy(&probe.stderr)
            .contains("no alternative certificate subject name matches"),
        "independent name rejection required"
    );
    wrong_name.endpoint = "https://[::ffff:127.0.0.1]:55459".into();
    let client = OpenBao::new(wrong_name, Limits::default()).unwrap();
    assert!(matches!(
        client.resolve(&reference, Version::Latest).await,
        Err(Error::Unavailable)
    ));
}

mod fixture;
#[tokio::test]
async fn paused_actual_service_exceeds_deadline_then_same_client_recovers() {
    let adapter = adapter(Limits {
        deadline: std::time::Duration::from_millis(100),
        ..Limits::default()
    });
    let reference = SecretRef::new("oidc-client").unwrap();
    assert_eq!(
        adapter
            .resolve(&reference, Version::Latest)
            .await
            .unwrap()
            .version(),
        2
    );
    let paused = fixture::Paused::new();
    let started = std::time::Instant::now();
    let outcome = adapter.resolve(&reference, Version::Latest).await;
    let elapsed = started.elapsed();
    paused.resume();
    assert!(matches!(outcome, Err(Error::Timeout)));
    assert!(
        elapsed < std::time::Duration::from_secs(2),
        "operation did not respect bounded deadline"
    );
    assert_eq!(
        adapter
            .resolve(&reference, Version::Latest)
            .await
            .unwrap()
            .version(),
        2
    );
}
#[tokio::test]
async fn expired_token_and_denied_key_do_not_use_previous_material() {
    use serde_json::json;
    let admin = fixture::Admin::new();
    let name = fixture::unique();
    admin.post(&format!("sys/policies/acl/{name}"), json!({"policy":"path \"secret/data/rom-extras/client\" { capabilities = [\"read\"] }"})).await.unwrap();
    let issued = admin.post("auth/token/create", json!({"policies":[name],"no_default_policy":true,"ttl":"3s","explicit_max_ttl":"3s","renewable":false})).await.unwrap();
    assert_eq!(issued["auth"]["lease_duration"], 3);
    assert_eq!(issued["auth"]["renewable"], false);
    let mut config = config();
    config.token = SecretBytes::new(
        issued["auth"]["client_token"]
            .as_str()
            .unwrap()
            .as_bytes()
            .to_vec(),
    )
    .unwrap();
    let adapter = OpenBao::new(config, Limits::default()).unwrap();
    let reference = SecretRef::new("oidc-client").unwrap();
    let first = adapter.resolve(&reference, Version::Latest).await.unwrap();
    assert_eq!(first.version(), 2);
    let plain = SecretBytes::new(vec![1, 0, 255]).unwrap();
    let binding = Binding::new(b"denied-tenant".to_vec(), b"denied-resource".to_vec()).unwrap();
    assert!(matches!(
        adapter
            .encrypt(&KeyRef::new("application-key").unwrap(), &plain, &binding)
            .await,
        Err(Error::Denied)
    ));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        match adapter.resolve(&reference, Version::Latest).await {
            Err(Error::Denied) => break,
            Ok(_) if std::time::Instant::now() < deadline => {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await
            }
            _ => panic!("token expiry was not enforced"),
        }
    }
}

#[tokio::test]
async fn update_only_token_cannot_create_a_missing_transit_key() {
    let admin = fixture::Admin::new();
    let name = fixture::unique();
    let token = admin
        .token(&name, &format!("rom-extras/{name}"), &name)
        .await;
    let key = KeyRef::new("missing-key").unwrap();
    let mut config = config();
    config.token = SecretBytes::new(token.into_bytes()).unwrap();
    config.keys = vec![(key.clone(), KeyLocation::new("transit", &name).unwrap())];
    let adapter = OpenBao::new(config, Limits::default()).unwrap();
    let path = format!("transit/keys/{name}");
    assert_eq!(admin.status(&path).await, 404);
    let binding = Binding::new(b"missing-tenant".to_vec(), b"missing-resource".to_vec()).unwrap();
    let outcome = adapter
        .encrypt(&key, &SecretBytes::new(vec![0, 1]).unwrap(), &binding)
        .await;
    let error = match outcome {
        Err(error) => error,
        Ok(_) => panic!("missing key was created"),
    };
    assert_eq!(error, Error::Denied);
    assert_eq!(admin.status(&path).await, 404);
}

#[tokio::test]
async fn occupied_admission_rejects_without_queue_and_recovers_after_actual_read() {
    use std::{future::Future, task::Poll};
    let adapter = adapter(Limits {
        max_in_flight: 1,
        ..Limits::default()
    });
    let reference = SecretRef::new("oidc-client").unwrap();
    let mut first = std::pin::pin!(adapter.resolve(&reference, Version::Latest));
    std::future::poll_fn(|cx| {
        assert!(
            matches!(first.as_mut().poll(cx), Poll::Pending),
            "actual network read must remain admitted"
        );
        Poll::Ready(())
    })
    .await;
    assert!(matches!(
        adapter.resolve(&reference, Version::Latest).await,
        Err(Error::Busy)
    ));
    assert_eq!(first.await.unwrap().version(), 2);
    assert_eq!(
        adapter
            .resolve(&reference, Version::Latest)
            .await
            .unwrap()
            .version(),
        2
    );
}
#[tokio::test]
async fn persistent_restart_preserves_versions_and_ciphertext_with_fresh_client() {
    let before = adapter(Limits::default());
    let reference = SecretRef::new("oidc-client").unwrap();
    let original = before.resolve(&reference, Version::Latest).await.unwrap();
    let plain = SecretBytes::new(vec![0, 42, 255]).unwrap();
    let binding = Binding::new(b"restart-tenant".to_vec(), b"restart-resource".to_vec()).unwrap();
    let envelope = before
        .encrypt(&KeyRef::new("application-key").unwrap(), &plain, &binding)
        .await
        .unwrap();
    assert!(fixture::container("stop"), "fixture stop failed");
    assert!(matches!(
        before.resolve(&reference, Version::Latest).await,
        Err(Error::Unavailable)
    ));
    assert!(fixture::container("start"), "fixture start failed");
    fixture::Admin::new().unseal_existing().await;
    let after = adapter(Limits::default());
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    let recovered = loop {
        match after.resolve(&reference, Version::Latest).await {
            Ok(value) => break value,
            Err(Error::Unavailable) if std::time::Instant::now() < deadline => {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await
            }
            Err(error) => panic!("fixture recovery failed: {error}"),
        }
    };
    assert_eq!(recovered.version(), original.version());
    assert!(
        recovered.bytes().expose() == original.bytes().expose(),
        "restart material mismatch"
    );
    let decoded = after.decrypt(&envelope, &binding).await.unwrap();
    assert!(
        decoded.expose() == plain.expose(),
        "restart plaintext mismatch"
    );
}
#[tokio::test]
async fn live_rotation_deleted_versions_and_revocation_never_return_cached_material() {
    use serde_json::json;
    let admin = fixture::Admin::new();
    let name = fixture::unique();
    let path = format!("rom-extras/{name}");
    let token = admin.token(&name, &path, &name).await;
    let mut config = config();
    config.token = SecretBytes::new(token.clone().into_bytes()).unwrap();
    config.secrets = vec![(
        SecretRef::new("rotating").unwrap(),
        SecretLocation::new("secret", &path, "credential").unwrap(),
    )];
    config.keys = Vec::new();
    let adapter = OpenBao::new(config, Limits::default()).unwrap();
    let reference = SecretRef::new("rotating").unwrap();
    let write = format!("secret/data/{path}");
    admin
        .post(
            &write,
            json!({"options":{"cas":0},"data":{"credential":"first-value"}}),
        )
        .await
        .unwrap();
    let first = adapter.resolve(&reference, Version::Latest).await.unwrap();
    assert_eq!(first.version(), 1);
    assert!(first.bytes().expose() == b"first-value");
    admin
        .post(
            &write,
            json!({"options":{"cas":1},"data":{"credential":"second-value"}}),
        )
        .await
        .unwrap();
    let pinned = adapter
        .resolve(&reference, Version::pinned(1).unwrap())
        .await
        .unwrap();
    let latest = adapter.resolve(&reference, Version::Latest).await.unwrap();
    assert!(pinned.bytes().expose() == b"first-value");
    assert!(latest.bytes().expose() == b"second-value");
    assert_eq!(latest.version(), 2);
    admin
        .post(&format!("secret/delete/{path}"), json!({"versions":[1]}))
        .await
        .unwrap();
    assert!(matches!(
        adapter
            .resolve(&reference, Version::pinned(1).unwrap())
            .await,
        Err(Error::NotFound)
    ));
    admin
        .post(&format!("secret/destroy/{path}"), json!({"versions":[2]}))
        .await
        .unwrap();
    assert!(matches!(
        adapter.resolve(&reference, Version::Latest).await,
        Err(Error::NotFound)
    ));
    admin
        .post("auth/token/revoke", json!({"token":token}))
        .await
        .unwrap();
    assert!(matches!(
        adapter.resolve(&reference, Version::Latest).await,
        Err(Error::Denied)
    ));
}

#[tokio::test]
async fn actual_key_rotation_and_minimum_version_preserve_authenticated_outcomes() {
    use rom_kms::Envelope;
    use serde_json::json;
    let admin = fixture::Admin::new();
    let name = fixture::unique();
    admin.post(&format!("transit/keys/{name}"),json!({"type":"aes256-gcm96","derived":true,"exportable":false,"allow_plaintext_backup":false,"convergent_encryption":false})).await.unwrap();
    let token = admin
        .token(&name, &format!("rom-extras/{name}"), &name)
        .await;
    let key = KeyRef::new("rotating-key").unwrap();
    let mut config = config();
    config.profile = SecretRef::new(name.clone()).unwrap();
    config.token = SecretBytes::new(token.into_bytes()).unwrap();
    config.secrets = Vec::new();
    config.keys = vec![(key.clone(), KeyLocation::new("transit", &name).unwrap())];
    let adapter = OpenBao::new(config, Limits::default()).unwrap();
    let plain = SecretBytes::new(vec![0, 0, 255]).unwrap();
    let binding = Binding::new(b"rotation-tenant".to_vec(), b"rotation-resource".to_vec()).unwrap();
    let old = adapter.encrypt(&key, &plain, &binding).await.unwrap();
    assert_eq!(old.version(), 1);
    admin
        .post(&format!("transit/keys/{name}/rotate"), json!({}))
        .await
        .unwrap();
    let new = adapter.encrypt(&key, &plain, &binding).await.unwrap();
    assert_eq!(new.version(), 2);
    for envelope in [&old, &new] {
        let decoded = adapter.decrypt(envelope, &binding).await.unwrap();
        assert!(
            decoded.expose() == plain.expose(),
            "rotation plaintext mismatch"
        );
    }
    let wrong_profile = Envelope::new(
        SecretRef::new("different-profile").unwrap(),
        key.clone(),
        2,
        new.ciphertext().into(),
    )
    .unwrap();
    assert!(matches!(
        adapter.decrypt(&wrong_profile, &binding).await,
        Err(Error::Invalid)
    ));
    let wrong_key = Envelope::new(
        new.profile().clone(),
        KeyRef::new("unapproved-key").unwrap(),
        2,
        new.ciphertext().into(),
    )
    .unwrap();
    assert!(matches!(
        adapter.decrypt(&wrong_key, &binding).await,
        Err(Error::Invalid)
    ));
    let wrong_version = Envelope::new(
        new.profile().clone(),
        key.clone(),
        1,
        new.ciphertext().into(),
    )
    .unwrap();
    assert!(matches!(
        adapter.decrypt(&wrong_version, &binding).await,
        Err(Error::Invalid)
    ));
    let malformed = Envelope::new(
        new.profile().clone(),
        key,
        2,
        "vault:v2:not_valid_base64".into(),
    )
    .unwrap();
    assert!(matches!(
        adapter.decrypt(&malformed, &binding).await,
        Err(Error::Rejected)
    ));
    admin
        .post(
            &format!("transit/keys/{name}/config"),
            json!({"min_decryption_version":2}),
        )
        .await
        .unwrap();
    assert!(matches!(
        adapter.decrypt(&old, &binding).await,
        Err(Error::Rejected)
    ));
    let decoded = adapter.decrypt(&new, &binding).await.unwrap();
    assert!(
        decoded.expose() == plain.expose(),
        "minimum-version plaintext mismatch"
    );
}

#[path = "fixture/migration.rs"]
mod migration;
#[tokio::test]
async fn actual_authenticated_envelope_migration_preserves_binding_versions_and_recovery() {
    migration::run(config()).await;
}
