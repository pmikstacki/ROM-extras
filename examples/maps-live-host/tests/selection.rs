use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use rom::{Actor, Runtime};
use rom_map_live_host_example::{Host, HostSession, SelectionOutcome, SessionResolver, router};
use std::sync::Arc;
use tower::ServiceExt;
struct Fixture;
impl SessionResolver for Fixture {
    fn resolve(&self, _: &axum::http::HeaderMap) -> Option<HostSession> {
        HostSession::new(
            Actor::trusted("fixture", "reader"),
            "epoch1".into(),
            Arc::new(|| true),
        )
        .ok()
    }
}
#[tokio::test]
async fn stale_selection_and_unapproved_origin_are_refused() {
    let path = std::env::temp_dir().join(format!(
        "rom-map-select-{}-{}",
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
    let host = Host::new(runtime, "Places".into(), vec![], Arc::new(Fixture))
        .unwrap()
        .with_selection(
            "http://127.0.0.1:55467",
            Arc::new(|_, _| panic!("Rejected requests must not select")),
        )
        .unwrap();
    let app = router(host);
    for (origin, generation) in [
        ("http://unapproved.invalid", "epoch1"),
        ("http://127.0.0.1:55467", "stale"),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/selection")
                    .header("origin", origin)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        rom::json!({"id":"Exact/0001","generation":generation}).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
    assert_eq!(SelectionOutcome::Unknown.as_str(), "unknown");
    let large = "x".repeat(16385);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/selection")
                .header("origin", "http://127.0.0.1:55467")
                .header("content-type", "application/json")
                .body(Body::from(large))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}
