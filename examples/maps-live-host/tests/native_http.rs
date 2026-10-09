use axum::http::HeaderMap;
use rom::{Actor, Command, Key, Resource, Runtime};
use rom_map_live_host_example::{Host, HostSession, SelectionOutcome, SessionResolver, router};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
#[derive(Clone, Resource)]
#[resource(name = "http_places")]
struct Place {
    title: String,
    longitude: rom::FiniteF64,
    latitude: rom::FiniteF64,
    private_note: String,
}
struct FixtureSession(Arc<AtomicBool>);
impl SessionResolver for FixtureSession {
    fn resolve(&self, headers: &HeaderMap) -> Option<HostSession> {
        let authorization = headers.get("authorization")?.to_str().ok()?;
        if authorization == "Bearer synthetic-revoke-before-send" {
            let checks = AtomicUsize::new(0);
            return HostSession::new(
                Actor::trusted("local-http-fixture", "reader"),
                "fixture-epoch-changing".into(),
                Arc::new(move || checks.fetch_add(1, Ordering::SeqCst) < 2),
            )
            .ok();
        }
        if authorization != "Bearer synthetic-session" {
            return None;
        }
        let allowed = self.0.clone();
        HostSession::new(
            Actor::trusted("local-http-fixture", "reader"),
            "fixture-epoch-00001".into(),
            Arc::new(move || allowed.load(Ordering::SeqCst)),
        )
        .ok()
    }
}
struct Server(tokio::task::JoinHandle<()>);
impl Drop for Server {
    fn drop(&mut self) {
        self.0.abort();
    }
}
#[tokio::test]
async fn actual_http_discloses_only_current_authorized_fields() {
    let path = std::env::temp_dir().join(format!(
        "rom-live-map-http-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let runtime = Runtime::builder()
        .resource(
            Place::definition()
                .policy(|a, _, _| a.authority == "local-http-fixture")
                .field_policy(|a, _, field, _| a.subject == "writer" || field != "private_note"),
        )
        .build(
            Arc::new(rom_sqlite::Sqlite::open(path).unwrap()),
            Runtime::shared_cpu_pool(2).unwrap(),
        )
        .unwrap();
    let key = Key {
        kind: Place::KIND.into(),
        id: "ę/0001:Straße".into(),
    };
    runtime
        .execute(
            &Actor::trusted("local-http-fixture", "writer"),
            Command::create(
                &key.id,
                Place {
                    title: "Approved place".into(),
                    longitude: rom::FiniteF64::new(21.0).unwrap(),
                    latitude: rom::FiniteF64::new(52.0).unwrap(),
                    private_note: "never disclose this".into(),
                },
            )
            .idempotency("create-place"),
        )
        .await
        .unwrap();
    let allowed = Arc::new(AtomicBool::new(true));
    let unknown = Arc::new(AtomicBool::new(false));
    let handler_mode = unknown.clone();
    let revoke_during_selection = Arc::new(AtomicBool::new(false));
    let handler_revoke = revoke_during_selection.clone();
    let handler_allowed = allowed.clone();
    let expected_key = key.clone();
    let app = router(
        Host::new(
            runtime,
            Place::KIND.into(),
            vec![key.clone()],
            Arc::new(FixtureSession(allowed.clone())),
        )
        .unwrap()
        .with_selection(
            "http://127.0.0.1:55467",
            Arc::new(move |actor, selected| {
                assert_eq!(actor.actor().subject, "reader");
                assert!(actor.is_current());
                assert_eq!(actor.generation(), "fixture-epoch-00001");
                if handler_revoke.load(Ordering::SeqCst) {
                    handler_allowed.store(false, Ordering::SeqCst);
                    return SelectionOutcome::Accepted;
                }
                assert_eq!(selected, &expected_key);
                if handler_mode.load(Ordering::SeqCst) {
                    SelectionOutcome::Unknown
                } else {
                    SelectionOutcome::Accepted
                }
            }),
        )
        .unwrap(),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let _server = Server(tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    }));
    let client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .unwrap();
    let endpoint = format!("http://{address}/points?kind=Other&id=private");
    assert_eq!(client.get(&endpoint).send().await.unwrap().status(), 401);
    let response = client
        .get(&endpoint)
        .bearer_auth("synthetic-session")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers().get("cache-control").unwrap(), "no-store");
    let dto: rom::Value = response.json().await.unwrap();
    assert_eq!(dto["generation"], "fixture-epoch-00001");
    assert_eq!(dto["kind"], Place::KIND);
    assert_eq!(dto["points"][0]["id"], key.id);
    assert_eq!(dto["points"][0]["longitude"], 21.0);
    assert_eq!(dto["points"][0]["latitude"], 52.0);
    assert!(!dto.to_string().contains("private_note"));
    assert!(!dto.to_string().contains("never disclose"));
    assert!(!dto.to_string().contains("synthetic-session"));
    let revoked = client
        .get(&endpoint)
        .bearer_auth("synthetic-revoke-before-send")
        .send()
        .await
        .unwrap();
    assert_eq!(revoked.status(), 403);
    assert_eq!(revoked.headers().get("cache-control").unwrap(), "no-store");
    let body = revoked.text().await.unwrap();
    assert!(!body.contains("Approved place"));
    assert!(!body.contains("ę/0001:Straße"));
    let selection_endpoint = format!("http://{address}/selection");
    let body = rom::json!({"id":key.id,"generation":"fixture-epoch-00001"});
    for expected in ["accepted", "unknown"] {
        unknown.store(expected == "unknown", Ordering::SeqCst);
        let response = client
            .post(&selection_endpoint)
            .bearer_auth("synthetic-session")
            .header("origin", "http://127.0.0.1:55467")
            .json(&body)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        let dto: rom::Value = response.json().await.unwrap();
        assert_eq!(dto["outcome"], expected);
        assert_eq!(dto["generation"], "fixture-epoch-00001");
    }
    let missing = client
        .post(&selection_endpoint)
        .bearer_auth("synthetic-session")
        .header("origin", "http://127.0.0.1:55467")
        .json(&rom::json!({"id":"missing","generation":"fixture-epoch-00001"}))
        .send()
        .await
        .unwrap();
    assert_eq!(missing.status(), 200);
    assert_eq!(
        missing.json::<rom::Value>().await.unwrap()["outcome"],
        "rejected"
    );
    revoke_during_selection.store(true, Ordering::SeqCst);
    let uncertain = client
        .post(&selection_endpoint)
        .bearer_auth("synthetic-session")
        .header("origin", "http://127.0.0.1:55467")
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(uncertain.status(), 200);
    assert_eq!(
        uncertain.json::<rom::Value>().await.unwrap()["outcome"],
        "unknown"
    );
    allowed.store(false, Ordering::SeqCst);
    assert_eq!(
        client
            .get(&endpoint)
            .bearer_auth("synthetic-session")
            .send()
            .await
            .unwrap()
            .status(),
        403
    );
}
