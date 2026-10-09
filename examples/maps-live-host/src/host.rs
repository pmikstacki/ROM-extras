use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, StatusCode},
    routing::get,
};
use rom::{Actor, Key, Runtime};
use rom_map_core::{Cancellation, Error, RequestContext, Result};
use rom_maps_host_example::read_session_points;
use std::{sync::Arc, time::Duration};
/// Supplied by the application's authenticated-session integration.
pub trait SessionResolver: Send + Sync {
    fn resolve(&self, headers: &HeaderMap) -> Option<HostSession>;
}
/// Server-resolved identity and a current generation check. Never read an Actor from JSON.
pub struct HostSession {
    pub(crate) actor: Actor,
    pub(crate) generation: String,
    pub(crate) current: Arc<dyn Fn() -> bool + Send + Sync>,
}
impl HostSession {
    /// Host callbacks must compare generation and serialize selection with their session store.
    pub fn actor(&self) -> &Actor {
        &self.actor
    }
    pub fn is_current(&self) -> bool {
        (self.current)()
    }

    pub fn new(
        actor: Actor,
        generation: String,
        current: Arc<dyn Fn() -> bool + Send + Sync>,
    ) -> Result<Self> {
        if generation.is_empty()
            || generation.len() > 128
            || generation.chars().any(char::is_control)
        {
            return Err(Error::InvalidQuery);
        }
        Ok(Self {
            actor,
            generation,
            current,
        })
    }
    pub fn generation(&self) -> &str {
        &self.generation
    }
}
/// Host-owned Runtime and Resource scope, outside the map capability core.
#[derive(Clone)]
pub struct Host {
    pub(crate) runtime: Runtime,
    pub(crate) kind: String,
    pub(crate) keys: Arc<Vec<Key>>,
    pub(crate) sessions: Arc<dyn SessionResolver>,
    pub(crate) geocoding: Option<(String, Arc<dyn rom_map_core::Geocoding>)>,
    pub(crate) selection: Option<(String, crate::SelectionHandler)>,
}
impl Host {
    pub fn with_geocoding(
        mut self,
        origin: &str,
        provider: Arc<dyn rom_map_core::Geocoding>,
    ) -> Result<Self> {
        self.geocoding = Some((crate::origin::admit(origin)?, provider));
        Ok(self)
    }
    pub fn with_selection(
        mut self,
        origin: &str,
        handler: crate::SelectionHandler,
    ) -> Result<Self> {
        self.selection = Some((crate::origin::admit(origin)?, handler));
        Ok(self)
    }

    pub fn new(
        runtime: Runtime,
        kind: String,
        keys: Vec<Key>,
        sessions: Arc<dyn SessionResolver>,
    ) -> Result<Self> {
        if kind.is_empty() || keys.len() > 200 || keys.iter().any(|k| k.kind != kind) {
            return Err(Error::InvalidQuery);
        }
        let mut unique = std::collections::BTreeSet::new();
        if keys.iter().any(|k| !unique.insert(k.id.as_str())) {
            return Err(Error::InvalidQuery);
        }
        Ok(Self {
            runtime,
            kind,
            keys: Arc::new(keys),
            sessions,
            selection: None,
            geocoding: None,
        })
    }
}
/// Local host route preparation; the application still chooses its listener and authentication.
pub fn router(host: Host) -> Router {
    Router::new()
        .route("/points", get(points))
        .route("/geocode", axum::routing::post(crate::geocoding::geocode))
        .route("/selection", axum::routing::post(crate::selection::select))
        .layer(DefaultBodyLimit::max(16 * 1024))
        .layer(axum::middleware::from_fn(crate::cache::no_store))
        .with_state(host)
}
async fn points(
    State(host): State<Host>,
    headers: HeaderMap,
) -> std::result::Result<Json<rom::Value>, StatusCode> {
    let session = host
        .sessions
        .resolve(&headers)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let context = RequestContext::new(Duration::from_secs(5), Cancellation::new())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let points = read_session_points(
        &host.runtime,
        &session.actor,
        &host.kind,
        &host.keys,
        &context,
        || (session.current)(),
    )
    .await
    .map_err(|e| match e {
        Error::Rejected => StatusCode::FORBIDDEN,
        Error::Timeout => StatusCode::GATEWAY_TIMEOUT,
        _ => StatusCode::SERVICE_UNAVAILABLE,
    })?;
    let response = rom::json!({"generation":session.generation,"kind":host.kind,"points":points.iter().map(|p|rom::json!({"id":p.location.key().id,"title":p.title,"longitude":p.location.coordinate().longitude_degrees(),"latitude":p.location.coordinate().latitude_degrees()})).collect::<Vec<_>>()});
    if !(session.current)() {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(Json(response))
}
