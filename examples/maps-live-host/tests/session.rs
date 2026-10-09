use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use rom::{Actor, Runtime};
use rom_map_live_host_example::{Host, HostSession, SessionResolver, router};
use std::sync::Arc;
use tower::ServiceExt;
struct Denied;
impl SessionResolver for Denied {
    fn resolve(&self, _: &axum::http::HeaderMap) -> Option<HostSession> {
        None
    }
}
#[tokio::test]
async fn denied_http_request_never_discloses_points() {
    let path = std::env::temp_dir().join(format!(
        "rom-live-host-denied-{}-{}",
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
    let app = router(Host::new(runtime, "Places".into(), vec![], Arc::new(Denied)).unwrap());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/points")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
#[test]
fn opaque_current_host_session_without_client_actor() {
    let session = HostSession::new(
        Actor::trusted("fixture", "reader"),
        "epoch-00001".into(),
        Arc::new(|| true),
    )
    .unwrap();
    assert_eq!(session.generation(), "epoch-00001");
    assert!(
        HostSession::new(
            Actor::trusted("fixture", "reader"),
            "".into(),
            Arc::new(|| true)
        )
        .is_err()
    );
}
