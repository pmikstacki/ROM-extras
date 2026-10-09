use rom::{Actor, Runtime};
use rom_map_core::Attribution;
use rom_map_live_host_example::{Host, HostSession, SessionResolver, router};
use rom_maptiler::{AttributionProfile, Config, MapTiler};
use std::{sync::Arc, time::Duration};
struct Session;
impl SessionResolver for Session {
    fn resolve(&self, headers: &axum::http::HeaderMap) -> Option<HostSession> {
        if headers.get("authorization")?.to_str().ok()? != "Bearer synthetic-session" {
            return None;
        }
        HostSession::new(
            Actor::trusted("fixture", "reader"),
            "epoch1".into(),
            Arc::new(|| true),
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
#[ignore = "explicit controlled HTTPS provider and CA required"]
async fn actual_host_http_to_controlled_provider_https() {
    let endpoint = std::env::var("ROM_MAP_HOST_ENDPOINT").unwrap();
    let ca = std::fs::read(std::env::var("ROM_MAP_HOST_CA").unwrap()).unwrap();
    let provider = MapTiler::new(
        Config::new(
            &endpoint,
            "ROMHostFixture/1",
            "fixture-maptiler",
            Duration::ZERO,
            AttributionProfile::new("fixture credit", Attribution::new("Fixture", None).unwrap())
                .unwrap(),
        )
        .unwrap()
        .with_server_key("synthetic-secret".into())
        .unwrap()
        .with_ca(ca)
        .unwrap(),
    )
    .unwrap();
    let path = std::env::temp_dir().join(format!(
        "rom-http-geocode-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let runtime = Runtime::builder()
        .build(
            Arc::new(rom_sqlite::Sqlite::open(path).unwrap()),
            Runtime::shared_cpu_pool(2).unwrap(),
        )
        .unwrap();
    let app = router(
        Host::new(runtime, "Places".into(), vec![], Arc::new(Session))
            .unwrap()
            .with_geocoding("http://127.0.0.1:55467", Arc::new(provider))
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
        .timeout(Duration::from_secs(6))
        .build()
        .unwrap();
    let url = format!("http://{address}/geocode");
    let request = rom::json!({"generation":"epoch1","text":"Warsaw & test","limit":1});
    let response = client
        .post(&url)
        .bearer_auth("synthetic-session")
        .header("origin", "http://127.0.0.1:55467")
        .json(&request)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers().get("cache-control").unwrap(), "no-store");
    let dto: rom::Value = response.json().await.unwrap();
    assert_eq!(dto["suggestions"][0]["sourceId"], "municipality.00001");
    assert_eq!(dto["suggestions"][0]["attribution"], "Fixture");
    assert_eq!(dto["suggestions"][0]["accuracy"]["kind"], "unknown");
    assert_eq!(dto["suggestions"][0]["longitude"], 21.0);
    assert!(!dto.to_string().contains("synthetic-secret"));
    assert!(!dto.to_string().contains("synthetic-session"));
    assert!(
        !dto["suggestions"][0]
            .as_object()
            .unwrap()
            .contains_key("resourceId")
    );
    let rejected=client.post(&url).bearer_auth("synthetic-session").header("origin","http://127.0.0.1:55467").json(&rom::json!({"generation":"epoch1","text":"Warsaw & test","limit":1,"endpoint":"https://unapproved.invalid/"})).send().await.unwrap();
    assert_eq!(rejected.status(), 400);
}
