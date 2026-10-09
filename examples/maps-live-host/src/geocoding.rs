use crate::Host;
use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use rom_map_core::{Accuracy, Cancellation, Capabilities, Error, GeocodeQuery, RequestContext};
use rom_maps_host_example::query_suggestions;
use std::time::Duration;
pub(crate) async fn geocode(
    State(host): State<Host>,
    headers: HeaderMap,
    Json(body): Json<rom::Value>,
) -> Result<Json<rom::Value>, StatusCode> {
    let (origin, provider) = host.geocoding.as_ref().ok_or(StatusCode::NOT_FOUND)?;
    crate::origin::check(&headers, origin)?;
    let session = host
        .sessions
        .resolve(&headers)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let fields = body.as_object().ok_or(StatusCode::BAD_REQUEST)?;
    if fields.len() != 3 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let generation = fields
        .get("generation")
        .and_then(rom::Value::as_str)
        .ok_or(StatusCode::BAD_REQUEST)?;
    if generation != session.generation || !(session.current)() {
        return Err(StatusCode::FORBIDDEN);
    }
    let text = fields
        .get("text")
        .and_then(rom::Value::as_str)
        .ok_or(StatusCode::BAD_REQUEST)?;
    let limit = fields
        .get("limit")
        .and_then(rom::Value::as_u64)
        .and_then(|v| usize::try_from(v).ok())
        .ok_or(StatusCode::BAD_REQUEST)?;
    let query = GeocodeQuery::new(text, limit).map_err(|_| StatusCode::BAD_REQUEST)?;
    let context = RequestContext::new(Duration::from_secs(5), Cancellation::new())
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let caps = Capabilities::new().with_geocoding(provider.as_ref());
    let results = query_suggestions(&caps, &query, &context, || (session.current)())
        .await
        .map_err(|e| match e {
            Error::Rejected => StatusCode::FORBIDDEN,
            Error::RateLimited { .. } => StatusCode::TOO_MANY_REQUESTS,
            Error::Timeout => StatusCode::GATEWAY_TIMEOUT,
            Error::InvalidQuery | Error::TooLarge => StatusCode::BAD_REQUEST,
            _ => StatusCode::SERVICE_UNAVAILABLE,
        })?;
    let dto = rom::json!({"generation":session.generation,"suggestions":results.results().iter().map(|r|rom::json!({"label":r.label(),"longitude":r.coordinate().longitude_degrees(),"latitude":r.coordinate().latitude_degrees(),"provider":r.provenance().provider(),"sourceId":r.provenance().source_id(),"attribution":r.provenance().attribution().text(),"attributionLink":r.provenance().attribution().link(),"accuracy":match r.accuracy(){Accuracy::Unknown=>rom::json!({"kind":"unknown"}),Accuracy::Radius(radius)=>rom::json!({"kind":"radius","value":radius.as_metres(),"unit":"metres"})}})).collect::<Vec<_>>()});
    if !(session.current)() {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(Json(dto))
}
