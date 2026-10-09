use rom_map_core::{Coordinate, Error, Metres, Provenance, Result, Route, RouteGeometry, Seconds};
use serde_json::Value;
/// Normalize bounded OSRM GeoJSON route output using host-approved graph provenance.
/// The boolean distinguishes native HTTP200 success from the explicitly admitted HTTP400 response.
/// OSRM does not supply a route identifier; graph identity must come from the operator's deployed dataset.
pub fn route_response(bytes: &[u8], success: bool, graph: Provenance) -> Result<Option<Route>> {
    if bytes.len() > 1048576 {
        return Err(Error::TooLarge);
    }
    let value: Value = serde_json::from_slice(bytes).map_err(|_| Error::InvalidResponse)?;
    let code = value
        .get("code")
        .and_then(Value::as_str)
        .ok_or(Error::InvalidResponse)?;
    if !success {
        return match code {
            "NoRoute" | "NoSegment" => Ok(None),
            "TooBig" => Err(Error::TooLarge),
            "Ok" => Err(Error::InvalidResponse),
            _ => Err(Error::Rejected),
        };
    }
    if code != "Ok" {
        return Err(Error::InvalidResponse);
    }
    let routes = value
        .get("routes")
        .and_then(Value::as_array)
        .ok_or(Error::InvalidResponse)?;
    if routes.len() > 1 {
        return Err(Error::TooLarge);
    }
    let route = routes.first().ok_or(Error::InvalidResponse)?;
    let distance = Metres::new(quantity(route, "distance")?).map_err(|_| Error::InvalidResponse)?;
    let duration =
        Seconds::new(quantity(route, "duration")?).map_err(|_| Error::InvalidResponse)?;
    let geometry = route.get("geometry").ok_or(Error::InvalidResponse)?;
    if geometry.get("type").and_then(Value::as_str) != Some("LineString") {
        return Err(Error::InvalidResponse);
    }
    let coordinates = geometry
        .get("coordinates")
        .and_then(Value::as_array)
        .ok_or(Error::InvalidResponse)?;
    if coordinates.len() > 8192 {
        return Err(Error::TooLarge);
    }
    let coordinates = coordinates
        .iter()
        .map(|position| {
            let position = position
                .as_array()
                .filter(|p| p.len() == 2)
                .ok_or(Error::InvalidResponse)?;
            Coordinate::new(
                position[0].as_f64().ok_or(Error::InvalidResponse)?,
                position[1].as_f64().ok_or(Error::InvalidResponse)?,
            )
            .map_err(|_| Error::InvalidResponse)
        })
        .collect::<Result<Vec<_>>>()?;
    let geometry = RouteGeometry::line_string(coordinates).map_err(|_| Error::InvalidResponse)?;
    Ok(Some(Route::new(geometry, distance, duration, graph)))
}
fn quantity(value: &Value, key: &str) -> Result<f64> {
    value
        .get(key)
        .and_then(Value::as_f64)
        .ok_or(Error::InvalidResponse)
}
