//! Local controlled fixture host; no production authentication or deployment.
use axum::{
    Json,
    http::{HeaderMap, HeaderValue, StatusCode, header},
    routing::post,
};
use rom::{Actor, Command, Key, Resource, Runtime};
use rom_map_core::Attribution;
use rom_map_live_host_example::{Host, HostSession, SelectionOutcome, SessionResolver, router};
use rom_maptiler::{AttributionProfile, Config, MapTiler};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
#[derive(Clone, Resource)]
#[resource(name = "demo_places")]
struct Place {
    title: String,
    longitude: rom::FiniteF64,
    latitude: rom::FiniteF64,
    private_note: String,
}
#[derive(Default)]
struct FixtureState {
    generation: u64,
    allowed: bool,
    unknown: bool,
    selected: Option<String>,
}
struct Sessions(Arc<Mutex<FixtureState>>);
impl SessionResolver for Sessions {
    fn resolve(&self, headers: &HeaderMap) -> Option<HostSession> {
        let cookie = headers.get(header::COOKIE)?.to_str().ok()?;
        if !cookie
            .split(';')
            .any(|part| part.trim() == "rom_fixture_session=synthetic-session")
        {
            return None;
        }
        let generation = self.0.lock().ok()?.generation;
        let state = self.0.clone();
        HostSession::new(
            Actor::trusted("local-browser-fixture", "reader"),
            generation.to_string(),
            Arc::new(move || {
                state
                    .lock()
                    .is_ok_and(|s| s.allowed && s.generation == generation)
            }),
        )
        .ok()
    }
}
fn origin(headers: &HeaderMap) -> Result<(), StatusCode> {
    if headers.get("origin").and_then(|v| v.to_str().ok()) != Some("http://127.0.0.1:55467") {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(())
}
#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    assert_eq!(
        args.len(),
        3,
        "controlled provider endpoint, CA and database path required"
    );
    let runtime = Runtime::builder()
        .resource(
            Place::definition()
                .policy(|a, _, _| a.authority == "local-browser-fixture")
                .field_policy(|a, _, f, _| a.subject == "writer" || f != "private_note"),
        )
        .build(
            Arc::new(rom_sqlite::Sqlite::open(&args[2]).unwrap()),
            Runtime::shared_cpu_pool(2).unwrap(),
        )
        .unwrap();
    let key = Key {
        kind: Place::KIND.into(),
        id: "ę/0001:Straße".into(),
    };
    runtime
        .execute(
            &Actor::trusted("local-browser-fixture", "writer"),
            Command::create(
                &key.id,
                Place {
                    title: "Approved place".into(),
                    longitude: rom::FiniteF64::new(21.0).unwrap(),
                    latitude: rom::FiniteF64::new(52.0).unwrap(),
                    private_note: "never expose to browser".into(),
                },
            )
            .idempotency("demo-place"),
        )
        .await
        .unwrap();
    let provider = MapTiler::new(
        Config::new(
            &args[0],
            "ROMBrowserFixture/1",
            "fixture-maptiler",
            Duration::ZERO,
            AttributionProfile::new("fixture credit", Attribution::new("Fixture", None).unwrap())
                .unwrap(),
        )
        .unwrap()
        .with_server_key("synthetic-secret".into())
        .unwrap()
        .with_ca(std::fs::read(&args[1]).unwrap())
        .unwrap(),
    )
    .unwrap();
    let state = Arc::new(Mutex::new(FixtureState::default()));
    let selection_state = state.clone();
    let host = Host::new(
        runtime,
        Place::KIND.into(),
        vec![key],
        Arc::new(Sessions(state.clone())),
    )
    .unwrap()
    .with_geocoding("http://127.0.0.1:55467", Arc::new(provider))
    .unwrap()
    .with_selection(
        "http://127.0.0.1:55467",
        Arc::new(move |session, key| {
            let mut s = selection_state.lock().unwrap();
            if !s.allowed || s.generation.to_string() != session.generation() {
                return SelectionOutcome::Rejected;
            }
            if s.unknown {
                return SelectionOutcome::Unknown;
            }
            s.selected = Some(key.id.clone());
            SelectionOutcome::Accepted
        }),
    )
    .unwrap();
    let connect_state = state.clone();
    let revoke_state = state.clone();
    let unknown_state = state.clone();
    let app=router(host)
 .route("/fixture/session",post(move|headers:HeaderMap|{let state=connect_state.clone();async move{origin(&headers)?;let mut s=state.lock().unwrap();s.generation=s.generation.checked_add(1).unwrap();s.allowed=true;s.unknown=false;s.selected=None;let generation=s.generation.to_string();let mut response_headers=HeaderMap::new();response_headers.insert(header::SET_COOKIE,HeaderValue::from_static("rom_fixture_session=synthetic-session; HttpOnly; SameSite=Strict; Path=/"));response_headers.insert(header::CACHE_CONTROL,HeaderValue::from_static("no-store"));Ok::<_,StatusCode>((response_headers,Json(rom::json!({"generation":generation}))))}}))
 .route("/fixture/revoke",post(move|headers:HeaderMap|{let state=revoke_state.clone();async move{origin(&headers)?;let mut s=state.lock().unwrap();s.generation=s.generation.checked_add(1).unwrap();s.allowed=false;s.selected=None;Ok::<_,StatusCode>(Json(rom::json!({"generation":s.generation.to_string()})))}}))
 .route("/fixture/unknown",post(move|headers:HeaderMap|{let state=unknown_state.clone();async move{origin(&headers)?;state.lock().unwrap().unknown=true;Ok::<_,StatusCode>(StatusCode::NO_CONTENT)}}));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:55468")
        .await
        .unwrap();
    println!("Controlled local ROM host ready");
    axum::serve(listener, app).await.unwrap();
}
