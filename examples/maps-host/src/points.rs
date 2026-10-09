use rom::{Actor, Key, ProjectedView, Runtime};
use rom_map_core::{Coordinate, Error, RequestContext, ResourceLocation, Result};
/// Host-approved fields only; native geocoder IDs are never Resource identities.
pub struct ApprovedPoint {
    pub location: ResourceLocation,
    pub title: String,
}
/// Convert a current public projection, omitting unavailable or invalid location fields.
pub fn projected_point(expected: &Key, view: ProjectedView) -> Result<Option<ApprovedPoint>> {
    if &view.key != expected {
        return Err(Error::Rejected);
    }
    let Some(fields) = view.value else {
        return Ok(None);
    };
    let Some(title) = fields.get("title").and_then(rom::Value::as_str) else {
        return Ok(None);
    };
    if title.len() > 4096 {
        return Ok(None);
    }
    let Some(lon) = fields.get("longitude").and_then(rom::Value::as_f64) else {
        return Ok(None);
    };
    let Some(lat) = fields.get("latitude").and_then(rom::Value::as_f64) else {
        return Ok(None);
    };
    let Ok(coordinate) = Coordinate::new(lon, lat) else {
        return Ok(None);
    };
    Ok(Some(ApprovedPoint {
        location: ResourceLocation::new(view.key, coordinate),
        title: title.into(),
    }))
}
/// Host chooses an exact kind and keys. Runtime checks current actor and field disclosure.
/// This read-only example performs no selection, session changes or mutations.
pub async fn read_points(
    runtime: &Runtime,
    actor: &Actor,
    kind: &str,
    keys: &[Key],
    context: &RequestContext,
) -> Result<Vec<ApprovedPoint>> {
    context
        .run(async {
            if keys.len() > 200 {
                return Err(Error::TooLarge);
            }
            let mut seen = std::collections::BTreeSet::new();
            for key in keys {
                if key.kind != kind || !seen.insert((&key.kind, &key.id)) {
                    return Err(Error::InvalidQuery);
                }
            }
            let mut points = Vec::new();
            for key in keys {
                let view = match runtime.read_projected(actor, kind, &key.id).await {
                    Ok(view) => view,
                    Err(rom::Error::Missing | rom::Error::Denied) => continue,
                    Err(_) => return Err(Error::Unavailable),
                };
                if let Some(point) = projected_point(key, view)? {
                    points.push(point);
                }
            }
            Ok(points)
        })
        .await
}
