use rom::{Actor, Clock, Command, Error, PrincipalKind, Resource, Runtime, Storage};
use rom_auth::oidc::OidcTokenBindings;
use rom_identity::{
    IdentityGate, IdentityLink, IdentityProvider, ProviderActivation, ProviderProfile, User,
    link_key, linked_user_id,
};
use rom_oidc_presets::IssuerPreset;
use serde_json::Value;
use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};
struct Time(AtomicU64);
impl Clock for Time {
    fn now(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}
#[derive(Clone, Resource)]
#[resource(name = "oidc_native_notes")]
struct Note {
    value: String,
}
fn operator() -> Actor {
    Actor::trusted("host", "qualification")
}
fn runtime(storage: Arc<dyn Storage>, clock: Arc<Time>) -> Runtime {
    let gate = IdentityGate::default()
        .allow_host("host", PrincipalKind::Embedded, "qualification")
        .unwrap();
    Runtime::builder()
        .clock(clock)
        .actor_gate(Arc::new(gate))
        .resource(
            User::definition()
                .policy(|a, _, _| a == &operator())
                .allow_all_fields(),
        )
        .resource(
            IdentityProvider::definition()
                .policy(|a, _, _| a == &operator())
                .allow_all_fields(),
        )
        .resource(
            IdentityLink::definition()
                .policy(|a, _, _| a == &operator())
                .allow_all_fields(),
        )
        .resource(
            Note::definition()
                .policy(|a, _, _| {
                    a == &operator() || linked_user_id(a).as_deref() == Some("local-user/東京")
                })
                .allow_all_fields(),
        )
        .build(storage, Runtime::shared_cpu_pool(1).unwrap())
        .unwrap()
}
fn storage(redb: bool, path: &Path) -> Arc<dyn Storage> {
    if redb {
        Arc::new(rom_redb::Redb::open(path).unwrap())
    } else {
        Arc::new(rom_sqlite::Sqlite::open(path).unwrap())
    }
}
pub async fn qualify(input: &Value, root: &Path) {
    let now = input["now"].as_u64().unwrap();
    let value = |key: &str| input[key].as_str().unwrap();
    for redb in [false, true] {
        let dir = root.join(if redb { "redb" } else { "sqlite" });
        std::fs::create_dir(&dir).unwrap();
        let path = dir.join("database");
        let clock = Arc::new(Time(AtomicU64::new(now)));
        let rt = runtime(storage(redb, &path), clock.clone());
        rt.execute(
            &operator(),
            Command::create(
                "fixture-humans",
                IdentityProvider {
                    enabled: true,
                    profile: ProviderProfile::OidcRs256Human,
                    issuer: value("issuer").into(),
                    audience: value("client_id").into(),
                    endpoint: Some("host-approved-jwks-snapshot".into()),
                    credential_ref: None,
                },
            )
            .idempotency("provider"),
        )
        .await
        .unwrap();
        rt.execute(
            &operator(),
            Command::create(
                "local-user/東京",
                User {
                    enabled: true,
                    display_name: "Fixture user".into(),
                },
            )
            .idempotency("user"),
        )
        .await
        .unwrap();
        let activation = ProviderActivation::read(&rt, &operator(), "fixture-humans")
            .await
            .unwrap();
        let evidence = activation
            .verify(|authority, config| {
                IssuerPreset::exact_https(&config.issuer)?
                    .configure_jwks(authority, &config.audience, value("jwks").as_bytes())?
                    .authenticate(
                        value("id_token"),
                        value("nonce"),
                        OidcTokenBindings {
                            access_token: Some(value("access_token")),
                            authorization_code: Some(value("authorization_code")),
                        },
                        now,
                    )
            })
            .unwrap();
        // No link means no Actor, even with an authentic native ID token.
        assert!(matches!(evidence.bind(&rt).await, Err(Error::Denied)));
        let mut verifier = IssuerPreset::exact_https(value("issuer"))
            .unwrap()
            .configure_jwks(
                "fixture-humans",
                value("client_id"),
                value("jwks").as_bytes(),
            )
            .unwrap();
        let proof = verifier
            .authenticate(
                value("id_token"),
                value("nonce"),
                OidcTokenBindings {
                    access_token: Some(value("access_token")),
                    authorization_code: Some(value("authorization_code")),
                },
                now,
            )
            .unwrap();
        let subject = proof.subject();
        rt.execute(
            &operator(),
            Command::create(
                &link_key("fixture-humans", PrincipalKind::Human, subject),
                IdentityLink {
                    authority: "fixture-humans".into(),
                    principal_kind: "human".into(),
                    subject: subject.into(),
                    user_id: "local-user/東京".into(),
                    enabled: true,
                },
            )
            .idempotency("link"),
        )
        .await
        .unwrap();
        let actor = evidence.bind(&rt).await.unwrap();
        assert_eq!(actor.principal_kind(), PrincipalKind::Human);
        assert_eq!(actor.valid_until(), Some(now + 30));
        let id = "resource/exact-東京";
        rt.execute(
            &actor,
            Command::create(
                id,
                Note {
                    value: "approved".into(),
                },
            )
            .idempotency("note"),
        )
        .await
        .unwrap();
        assert_eq!(
            rt.read::<Note>(&actor, id)
                .await
                .unwrap()
                .value
                .unwrap()
                .value,
            "approved"
        );
        let forged = Actor::trusted("fixture-humans", subject).with_kind(PrincipalKind::Human);
        assert!(matches!(
            rt.read::<Note>(&forged, id).await,
            Err(Error::Denied)
        ));
        clock.0.store(now + 30, Ordering::SeqCst);
        assert!(matches!(
            rt.read::<Note>(&actor, id).await,
            Err(Error::Denied)
        ));
        clock.0.store(now, Ordering::SeqCst);
        rt.execute(
            &operator(),
            Command::replace(
                "local-user/東京",
                User {
                    enabled: false,
                    display_name: "Fixture user".into(),
                },
            )
            .at_revision(1)
            .idempotency("disable"),
        )
        .await
        .unwrap();
        assert!(matches!(
            rt.read::<Note>(&actor, id).await,
            Err(Error::Denied)
        ));
        rt.shutdown().await.unwrap();
        drop(rt);
        let reopened = runtime(storage(redb, &path), clock);
        assert!(matches!(
            reopened.read::<Note>(&actor, id).await,
            Err(Error::Denied)
        ));
        assert_eq!(
            reopened
                .read::<Note>(&operator(), id)
                .await
                .unwrap()
                .value
                .unwrap()
                .value,
            "approved"
        );
        reopened.shutdown().await.unwrap();
        drop(reopened);
        for file in std::fs::read_dir(&dir).unwrap() {
            let file = file.unwrap();
            if file.file_type().unwrap().is_file() {
                let data = std::fs::read(file.path()).unwrap();
                for secret in ["id_token", "access_token", "authorization_code", "nonce"] {
                    assert!(
                        !data
                            .windows(value(secret).len())
                            .any(|w| w == value(secret).as_bytes()),
                        "credential persisted in native store"
                    );
                }
            }
        }
    }
    println!(
        "Native SQLite/redb linked identity, expiry, current authorization, restart and credential exclusion verified."
    );
}
