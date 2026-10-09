use crate::Host;
use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use rom_map_core::{Cancellation, RequestContext};
use rom_maps_host_example::read_session_points;
use std::{sync::Arc, time::Duration};
pub type SelectionHandler =
    Arc<dyn Fn(&crate::HostSession, &rom::Key) -> SelectionOutcome + Send + Sync>;
pub enum SelectionOutcome {
    Accepted,
    Rejected,
    Unknown,
}
impl SelectionOutcome {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
            Self::Unknown => "unknown",
        }
    }
}
pub(crate) async fn select(
    State(host): State<Host>,
    headers: HeaderMap,
    Json(body): Json<rom::Value>,
) -> Result<Json<rom::Value>, StatusCode> {
    let (origin, handler) = host.selection.as_ref().ok_or(StatusCode::NOT_FOUND)?;
    crate::origin::check(&headers, origin)?;
    let session = host
        .sessions
        .resolve(&headers)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let fields = body.as_object().ok_or(StatusCode::BAD_REQUEST)?;
    if fields.len() != 2 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let id = fields
        .get("id")
        .and_then(rom::Value::as_str)
        .filter(|s| !s.is_empty() && s.len() <= 4096)
        .ok_or(StatusCode::BAD_REQUEST)?;
    let generation = fields
        .get("generation")
        .and_then(rom::Value::as_str)
        .ok_or(StatusCode::BAD_REQUEST)?;
    if generation != session.generation || !(session.current)() {
        return Err(StatusCode::FORBIDDEN);
    }
    let mut outcome = SelectionOutcome::Rejected;
    let mut dispatched = false;
    if let Some(key) = host.keys.iter().find(|k| k.id == id) {
        let context = RequestContext::new(Duration::from_secs(5), Cancellation::new())
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        let points = read_session_points(
            &host.runtime,
            &session.actor,
            &host.kind,
            std::slice::from_ref(key),
            &context,
            || (session.current)(),
        )
        .await
        .map_err(|_| StatusCode::FORBIDDEN)?;
        if !points.is_empty() {
            dispatched = true;
            outcome = handler(&session, key);
        }
    }
    if !(session.current)() {
        if !dispatched {
            return Err(StatusCode::FORBIDDEN);
        }
        // A host callback may have completed; never replace dispatch with refusal.
        outcome = SelectionOutcome::Unknown;
    }
    Ok(Json(
        rom::json!({"generation":session.generation,"outcome":outcome.as_str()}),
    ))
}
