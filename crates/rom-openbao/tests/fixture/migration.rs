//! Public KMS migration exercised against newly provisioned retained native keys.
use crate::fixture::{Admin, Paused, container, unique};
use rom_kms::{Binding, EncryptionVersion, KeyRef, Kms, MigrationLimits, Migrator};
use rom_openbao::{Config, KeyLocation, Limits, OpenBao};
use rom_secrets::{Error, SecretBytes, SecretRef};
use serde_json::json;
use std::time::Duration;
fn runtime_config(base: &Config, token: &str, name: &str) -> Config {
    Config {
        endpoint: base.endpoint.clone(),
        ca_pem: base.ca_pem.clone(),
        token: SecretBytes::new(token.as_bytes().to_vec()).unwrap(),
        profile: SecretRef::new(name).unwrap(),
        secrets: Vec::new(),
        keys: [
            ("source", name.to_owned()),
            ("destination", format!("{name}_destination")),
            ("denied", format!("{name}_denied")),
            ("missing", format!("{name}_missing")),
        ]
        .into_iter()
        .map(|(alias, path)| {
            (
                KeyRef::new(alias).unwrap(),
                KeyLocation::new("transit", &path).unwrap(),
            )
        })
        .collect(),
    }
}
/// Run actual native migration and recovery using public contracts only.
pub async fn run(base: Config) {
    let admin = Admin::new();
    let name = unique();
    let second = format!("{name}_destination");
    let denied = format!("{name}_denied");
    for key in [&name, &second, &denied] {
        admin.post(&format!("transit/keys/{key}"),json!({"type":"aes256-gcm96","derived":true,"exportable":false,"allow_plaintext_backup":false,"convergent_encryption":false})).await.unwrap();
    }
    let policy=[name.clone(),second.clone(),format!("{name}_missing")].iter().map(|key|format!("path \"transit/encrypt/{key}\" {{ capabilities=[\"update\"] }}\npath \"transit/decrypt/{key}\" {{ capabilities=[\"update\"] }}\n")).collect::<String>();
    admin
        .post(
            &format!("sys/policies/acl/{name}"),
            json!({"policy":policy}),
        )
        .await
        .unwrap();
    let issued = admin
        .post(
            "auth/token/create",
            json!({"policies":[name],"no_default_policy":true,"ttl":"1h","renewable":false}),
        )
        .await
        .unwrap();
    let token = issued["auth"]["client_token"].as_str().unwrap();
    let markers: serde_json::Value = serde_json::from_slice(
        &std::fs::read(std::env::var("ROM_EXTRAS_OPENBAO_MARKERS").unwrap()).unwrap(),
    )
    .unwrap();
    let mut bytes = markers["v2"].as_str().unwrap().as_bytes().to_vec();
    bytes.extend_from_slice(&[0, 255, 0, 42]);
    let plaintext = SecretBytes::new(bytes).unwrap();
    let binding = Binding::new(
        b"migration-tenant".to_vec(),
        b"migration-resource-nonempty-aad".to_vec(),
    )
    .unwrap();
    let source = KeyRef::new("source").unwrap();
    let destination = KeyRef::new("destination").unwrap();
    let v2 = EncryptionVersion::pinned(2).unwrap();
    let migrator = Migrator::new(
        OpenBao::new(runtime_config(&base, token, &name), Limits::default()).unwrap(),
        MigrationLimits::default(),
    )
    .unwrap();
    let original = migrator
        .provider()
        .encrypt(&source, &plaintext, &binding)
        .await
        .unwrap();
    assert_eq!(original.version(), 1);
    let original_ciphertext = original.ciphertext().to_owned();
    for key in [&name, &second] {
        admin
            .post(&format!("transit/keys/{key}/rotate"), json!({}))
            .await
            .unwrap();
    }
    admin
        .post(&format!("transit/keys/{name}/rotate"), json!({}))
        .await
        .unwrap();
    admin
        .post(&format!("transit/keys/{second}/rotate"), json!({}))
        .await
        .unwrap();
    let latest = migrator
        .provider()
        .encrypt(&source, &plaintext, &binding)
        .await
        .unwrap();
    assert_eq!(latest.version(), 3);
    let replacement = migrator
        .migrate(&original, &binding, &source, v2)
        .await
        .unwrap();
    assert_eq!(replacement.version(), 2);
    let cross = migrator
        .migrate(&original, &binding, &destination, v2)
        .await
        .unwrap();
    assert_eq!(cross.version(), 2);
    assert_eq!(cross.key(), &destination);
    for envelope in [&original, &replacement, &cross] {
        let decoded = migrator
            .provider()
            .decrypt(envelope, &binding)
            .await
            .unwrap();
        assert!(
            decoded.expose() == plaintext.expose(),
            "authenticated migration bytes mismatch"
        );
    }
    assert_eq!(original.ciphertext(), original_ciphertext);
    for bad in [
        Binding::new(b"other-tenant".to_vec(), binding.aad().to_vec()).unwrap(),
        Binding::new(binding.context().to_vec(), b"other-aad".to_vec()).unwrap(),
    ] {
        assert!(matches!(
            migrator.migrate(&original, &bad, &destination, v2).await,
            Err(Error::Rejected)
        ));
    }
    for bad in [
        Binding::new(b"other-tenant".to_vec(), binding.aad().to_vec()).unwrap(),
        Binding::new(binding.context().to_vec(), b"other-aad".to_vec()).unwrap(),
    ] {
        assert!(matches!(
            migrator.provider().decrypt(&cross, &bad).await,
            Err(Error::Rejected)
        ));
    }
    let source_only_token = admin
        .token(&unique(), &format!("rom-extras/{name}"), &name)
        .await;
    let source_only = Migrator::new(
        OpenBao::new(
            runtime_config(&base, &source_only_token, &name),
            Limits::default(),
        )
        .unwrap(),
        MigrationLimits::default(),
    )
    .unwrap();
    assert!(matches!(
        source_only
            .migrate(&original, &binding, &destination, v2)
            .await,
        Err(Error::Denied)
    ));
    for (alias, expected) in [
        ("unknown", Error::Invalid),
        ("denied", Error::Denied),
        ("missing", Error::Denied),
    ] {
        assert!(
            matches!(migrator.migrate(&original,&binding,&KeyRef::new(alias).unwrap(),v2).await,Err(e) if e==expected)
        );
    }
    assert_eq!(
        admin.status(&format!("transit/keys/{name}_missing")).await,
        404
    );
    assert!(matches!(
        migrator
            .migrate(
                &original,
                &binding,
                &destination,
                EncryptionVersion::pinned(99).unwrap()
            )
            .await,
        Err(Error::Rejected)
    ));
    assert!(matches!(
        migrator
            .migrate(
                &original,
                &binding,
                &destination,
                EncryptionVersion::pinned(i32::MAX as u64 + 1).unwrap()
            )
            .await,
        Err(Error::Invalid)
    ));
    // Total migration deadline applies even while the real TLS service is paused.
    let fast = Migrator::new(
        OpenBao::new(runtime_config(&base, token, &name), Limits::default()).unwrap(),
        MigrationLimits {
            deadline: Duration::from_millis(100),
            max_in_flight: 1,
        },
    )
    .unwrap();
    let paused = Paused::new();
    let timed = fast.migrate(&original, &binding, &destination, v2).await;
    paused.resume();
    assert!(matches!(timed, Err(Error::Timeout)));
    // Cancellation releases local admission without claiming remote cancellation.
    {
        use std::{future::Future, task::Poll};
        let mut pending = Box::pin(fast.migrate(&original, &binding, &destination, v2));
        std::future::poll_fn(|cx| {
            assert!(matches!(pending.as_mut().poll(cx), Poll::Pending));
            Poll::Ready(())
        })
        .await;
        drop(pending);
    }
    assert!(
        fast.migrate(&original, &binding, &destination, v2)
            .await
            .is_ok()
    );
    let evidence = std::path::PathBuf::from(
        std::env::var("ROM_EXTRAS_KMS_MIGRATION_EVIDENCE")
            .expect("private migration evidence root required"),
    )
    .join(&name);
    std::fs::create_dir_all(&evidence).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&evidence, std::fs::Permissions::from_mode(0o700)).unwrap();
    let stored = json!({"profile":name,"source":{"ciphertext":original.ciphertext(),"version":original.version()},"replacement":{"ciphertext":cross.ciphertext(),"version":cross.version()},"runtime_token":token});
    let captured = serde_json::to_vec(&stored).unwrap();
    assert!(
        !captured
            .windows(plaintext.expose().len())
            .any(|w| w == plaintext.expose())
    );
    let capture = evidence.join("host.json");
    std::fs::write(&capture, captured).unwrap();
    std::fs::set_permissions(capture, std::fs::Permissions::from_mode(0o600)).unwrap();
    assert!(container("stop"));
    assert!(container("start"));
    admin.unseal_existing().await;
    let after = OpenBao::new(runtime_config(&base, token, &name), Limits::default()).unwrap();
    let stored: serde_json::Value =
        serde_json::from_slice(&std::fs::read(evidence.join("host.json")).unwrap()).unwrap();
    let reopened = rom_kms::Envelope::new(
        SecretRef::new(name.clone()).unwrap(),
        destination.clone(),
        stored["replacement"]["version"].as_u64().unwrap(),
        stored["replacement"]["ciphertext"].as_str().unwrap().into(),
    )
    .unwrap();
    let decoded = after.decrypt(&reopened, &binding).await.unwrap();
    assert!(
        decoded.expose() == plaintext.expose(),
        "fresh client migration recovery mismatch"
    );
    // Native floors are administrative; the runtime never changes them.
    admin
        .post(
            &format!("transit/keys/{second}/config"),
            json!({"min_encryption_version":3}),
        )
        .await
        .unwrap();
    assert!(matches!(
        migrator
            .migrate(&original, &binding, &destination, v2)
            .await,
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
        migrator.migrate(&original, &binding, &source, v2).await,
        Err(Error::Rejected)
    ));
    let expiring=admin.post("auth/token/create",json!({"policies":[name],"no_default_policy":true,"ttl":"1s","explicit_max_ttl":"1s","renewable":false})).await.unwrap();
    let expired = Migrator::new(
        OpenBao::new(
            runtime_config(
                &base,
                expiring["auth"]["client_token"].as_str().unwrap(),
                &name,
            ),
            Limits::default(),
        )
        .unwrap(),
        MigrationLimits::default(),
    )
    .unwrap();
    tokio::time::sleep(Duration::from_millis(1200)).await;
    assert!(matches!(
        expired.migrate(&replacement, &binding, &source, v2).await,
        Err(Error::Denied)
    ));
    admin
        .post("auth/token/revoke", json!({"token":token}))
        .await
        .unwrap();
    assert!(matches!(
        migrator.migrate(&replacement, &binding, &source, v2).await,
        Err(Error::Denied)
    ));
    std::fs::write(evidence.join("verification.json"),serde_json::to_vec_pretty(&json!({"native_version":"2.7.1","latest":3,"explicit_destination":2,"cross_key":true,"aad_preserved":true,"floors_denial_revocation_expiry":true,"fresh_client_persistent_restart":true,"total_timeout_and_drop":true,"original_unchanged":true})).unwrap()).unwrap();
    println!(
        "Native authenticated KMS migration passed; retained evidence: {}",
        evidence.display()
    );
}
