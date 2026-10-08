//! Real OpenBao resolution through authorized public IdentityProvider Resources.
#[path = "../../../examples/secrets-host/activation.rs"]
mod activation;
mod secrets_fixture;
use rom::{Actor, Command, Resource, Runtime, Storage};
use rom_identity::{IdentityProvider, ProviderProfile};
use rom_openbao::{Config, Limits, OpenBao, SecretLocation};
use rom_secrets::{SecretBytes, SecretRef};
use std::{path::Path, sync::Arc};

fn adapter() -> OpenBao {
    let endpoint = std::env::var("ROM_EXTRAS_OPENBAO_ENDPOINT").unwrap();
    assert_eq!(endpoint, "https://127.0.0.1:55459");
    OpenBao::new(
        Config {
            endpoint,
            token: SecretBytes::new(
                std::env::var("ROM_EXTRAS_OPENBAO_TOKEN")
                    .unwrap()
                    .into_bytes(),
            )
            .unwrap(),
            ca_pem: Some(std::fs::read(std::env::var("ROM_EXTRAS_OPENBAO_CA").unwrap()).unwrap()),
            profile: SecretRef::new("openbao-fixture").unwrap(),
            secrets: vec![(
                SecretRef::new("oidc-client").unwrap(),
                SecretLocation::new("secret", "rom-extras/client", "credential").unwrap(),
            )],
            keys: Vec::new(),
        },
        Limits::default(),
    )
    .unwrap()
}
fn runtime(redb: bool, path: &Path) -> Runtime {
    let storage: Arc<dyn Storage> = if redb {
        Arc::new(rom_redb::Redb::open(path).unwrap())
    } else {
        Arc::new(rom_sqlite::Sqlite::open(path).unwrap())
    };
    Runtime::builder()
        .resource(
            IdentityProvider::definition()
                .policy(|actor, _, _| actor == &Actor::trusted("fixture", "operator"))
                .allow_all_fields(),
        )
        .build(storage, Runtime::shared_cpu_pool(2).unwrap())
        .unwrap()
}
fn provider() -> IdentityProvider {
    IdentityProvider {
        enabled: true,
        profile: ProviderProfile::OAuthIntrospectionService,
        issuer: "https://issuer.example.invalid".into(),
        audience: "rom".into(),
        endpoint: Some("host-approved-introspection".into()),
        credential_ref: Some("oidc-client".into()),
    }
}
fn excludes(bytes: &[u8], markers: &[&[u8]]) {
    for marker in markers {
        assert!(!marker.is_empty());
        assert!(
            !bytes.windows(marker.len()).any(|window| window == *marker),
            "private material entered a controlled capture"
        );
    }
}
#[tokio::test]
async fn native_stores_journal_and_host_captures_keep_only_approved_reference() {
    let markers: serde_json::Value = serde_json::from_slice(
        &std::fs::read(std::env::var("ROM_EXTRAS_OPENBAO_MARKERS").unwrap()).unwrap(),
    )
    .unwrap();
    let token = std::env::var("ROM_EXTRAS_OPENBAO_TOKEN").unwrap();
    let private = [
        markers["v1"].as_str().unwrap().as_bytes(),
        markers["v2"].as_str().unwrap().as_bytes(),
        token.as_bytes(),
    ];
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.superpowers/secrets-host");
    std::fs::create_dir_all(&root).unwrap();
    for redb in [false, true] {
        let unique = format!(
            "host-{redb}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let dir = root.join(unique);
        std::fs::create_dir(&dir).unwrap();
        let path = dir.join("database");
        let runtime = runtime(redb, &path);
        let actor = Actor::trusted("fixture", "operator");
        let receipt = runtime
            .execute(
                &actor,
                Command::create("provider", provider()).idempotency("provider"),
            )
            .await
            .unwrap();
        let resolver = adapter();
        let observed = secrets_fixture::Observed {
            inner: &resolver,
            runtime: &runtime,
            actor: &actor,
            disable_provider: false,
            calls: std::sync::atomic::AtomicUsize::new(0),
        };
        let (activation, material) = activation::prepare(&runtime, &actor, "provider", &observed)
            .await
            .unwrap();
        assert_eq!(activation.revision(), 1);
        assert_eq!(material.version(), 2);
        assert!(
            material.bytes().expose() == private[1],
            "host material mismatch"
        );
        let row = runtime
            .read::<IdentityProvider>(&actor, "provider")
            .await
            .unwrap();
        assert_eq!(
            row.value.as_ref().unwrap().credential_ref.as_deref(),
            Some("oidc-client")
        );
        excludes(
            &serde_json::to_vec(&row.value.unwrap().encode()).unwrap(),
            &private,
        );
        excludes(format!("{receipt:?}").as_bytes(), &private);
        let journal = runtime
            .journal(&actor, IdentityProvider::KIND, None)
            .await
            .unwrap();
        assert_eq!(journal.events.len(), 1);
        excludes(&serde_json::to_vec(&journal).unwrap(), &private);
        let denied = activation::prepare(
            &runtime,
            &Actor::trusted("fixture", "unauthorized"),
            "provider",
            &observed,
        )
        .await;
        let error = match denied {
            Err(error) => error,
            Ok(_) => panic!("unauthorized host resolved material"),
        };
        assert_eq!(error, rom_secrets::Error::Denied);
        assert_eq!(
            observed.calls.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "denied host must not resolve a secret"
        );
        excludes(error.to_string().as_bytes(), &private);
        drop(material);
        drop(activation);
        drop(resolver);
        runtime.shutdown().await.unwrap();
        drop(runtime);
        for entry in std::fs::read_dir(&dir).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_file() {
                excludes(&std::fs::read(entry.path()).unwrap(), &private);
            }
        }
        let reopened = self::runtime(redb, &path);
        let (activation, material) = activation::prepare(&reopened, &actor, "provider", &adapter())
            .await
            .unwrap();
        assert_eq!(activation.revision(), 1);
        assert_eq!(material.version(), 2);
        assert!(
            material.bytes().expose() == private[1],
            "reopened host material mismatch"
        );
        reopened.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn provider_disable_after_actual_resolution_rejects_prepared_material() {
    for redb in [false, true] {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../.superpowers/secrets-host")
            .join(format!(
                "race-{redb}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
        std::fs::create_dir_all(&dir).unwrap();
        let runtime = runtime(redb, &dir.join("database"));
        let actor = Actor::trusted("fixture", "operator");
        runtime
            .execute(
                &actor,
                Command::create("provider", provider()).idempotency("provider"),
            )
            .await
            .unwrap();
        let resolver = adapter();
        let observed = secrets_fixture::Observed {
            inner: &resolver,
            runtime: &runtime,
            actor: &actor,
            disable_provider: true,
            calls: std::sync::atomic::AtomicUsize::new(0),
        };
        let outcome = activation::prepare(&runtime, &actor, "provider", &observed).await;
        let error = match outcome {
            Err(error) => error,
            Ok(_) => panic!("disabled provider exposed prepared material"),
        };
        assert_eq!(error, rom_secrets::Error::Denied);
        assert_eq!(observed.calls.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert_eq!(
            runtime
                .read::<IdentityProvider>(&actor, "provider")
                .await
                .unwrap()
                .revision,
            2
        );
        runtime.shutdown().await.unwrap();
    }
}
