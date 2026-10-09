use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use rom::{Actor, Runtime};
use rom_map_core::{GeocodeQuery, GeocodeResults, Geocoding, ProviderFuture, RequestContext};
use rom_map_live_host_example::{Host, HostSession, SessionResolver, router};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tower::ServiceExt;
struct Session;
impl SessionResolver for Session {
    fn resolve(&self, _: &axum::http::HeaderMap) -> Option<HostSession> {
        HostSession::new(
            Actor::trusted("fixture", "reader"),
            "epoch1".into(),
            Arc::new(|| true),
        )
        .ok()
    }
}
struct Provider(Arc<AtomicUsize>);
impl Geocoding for Provider {
    fn geocode<'a>(
        &'a self,
        _: &'a GeocodeQuery,
        _: &'a RequestContext,
    ) -> ProviderFuture<'a, GeocodeResults> {
        Box::pin(async move {
            self.0.fetch_add(1, Ordering::SeqCst);
            GeocodeResults::new(vec![], 1)
        })
    }
}
#[tokio::test]
async fn explicit_host_geocoder_empty_results_and_stale_refusal() {
    let path = std::env::temp_dir().join(format!(
        "rom-map-geocode-{}-{}",
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
    let calls = Arc::new(AtomicUsize::new(0));
    let app = router(
        Host::new(runtime, "Places".into(), vec![], Arc::new(Session))
            .unwrap()
            .with_geocoding("http://127.0.0.1:55467", Arc::new(Provider(calls.clone())))
            .unwrap(),
    );
    for (generation, status) in [("epoch1", StatusCode::OK), ("stale", StatusCode::FORBIDDEN)] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/geocode")
                    .header("origin", "http://127.0.0.1:55467")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        rom::json!({"generation":generation,"text":"Place","limit":1}).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), status);
    }
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}
