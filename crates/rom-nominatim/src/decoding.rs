use rom_map_core::{
    Accuracy, Attribution, BoundingBox, Coordinate, Error, GeocodeResult, GeocodeResults,
    Provenance, Result,
};
use serde_json::Value;
/// Decode bounded native JSONv2 search results without inventing ROM identities.
/// Missing OSM references use service-scoped `nominatim-place/` identity, unstable across reimports.
pub fn search_results(bytes: &[u8], limit: usize, provider: &str) -> Result<GeocodeResults> {
    if bytes.len() > 1048576 {
        return Err(Error::TooLarge);
    }
    if !(1..=40).contains(&limit) {
        return Err(Error::InvalidQuery);
    }
    let value: Value = serde_json::from_slice(bytes).map_err(|_| Error::InvalidResponse)?;
    let values = value.as_array().ok_or(Error::InvalidResponse)?;
    if values.len() > limit {
        return Err(Error::TooLarge);
    }
    let results = values
        .iter()
        .map(|v| place(v, provider))
        .collect::<Result<Vec<_>>>()?;
    GeocodeResults::new(results, limit)
}
fn place(value: &Value, provider: &str) -> Result<GeocodeResult> {
    let source_id = match (value.get("osm_type"), value.get("osm_id")) {
        (None, None) => format!("nominatim-place/{}", native_id(value.get("place_id"))?),
        (Some(kind), Some(id)) => {
            let kind = kind.as_str().ok_or(Error::InvalidResponse)?;
            if !matches!(kind, "node" | "way" | "relation") {
                return Err(Error::InvalidResponse);
            }
            format!("{kind}/{}", native_id(Some(id))?)
        }
        _ => return Err(Error::InvalidResponse),
    };
    let coordinate = Coordinate::new(number(value.get("lon"))?, number(value.get("lat"))?)
        .map_err(|_| Error::InvalidResponse)?;
    let bounds = if let Some(bounds) = value.get("boundingbox") {
        let bounds = bounds
            .as_array()
            .filter(|a| a.len() == 4)
            .ok_or(Error::InvalidResponse)?;
        let south = number(bounds.first())?;
        let north = number(bounds.get(1))?;
        let west = number(bounds.get(2))?;
        let east = number(bounds.get(3))?;
        // Native Nominatim bounding boxes do not establish wrapping semantics.
        if west > east {
            return Err(Error::InvalidResponse);
        }
        Some(BoundingBox::new(west, south, east, north).map_err(|_| Error::InvalidResponse)?)
    } else {
        None
    };
    let attribution = Attribution::new(
        text(value, "licence")?,
        Some("https://www.openstreetmap.org/copyright"),
    )?;
    let provenance = Provenance::new(provider, &source_id, attribution)?;
    GeocodeResult::new(
        text(value, "display_name")?,
        coordinate,
        Accuracy::Unknown,
        provenance,
        bounds,
    )
}
fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or(Error::InvalidResponse)
}
fn number(value: Option<&Value>) -> Result<f64> {
    let value = value.ok_or(Error::InvalidResponse)?;
    let number = if let Some(text) = value.as_str() {
        text.parse::<f64>().map_err(|_| Error::InvalidResponse)?
    } else {
        value.as_f64().ok_or(Error::InvalidResponse)?
    };
    if !number.is_finite() {
        return Err(Error::InvalidResponse);
    }
    Ok(number)
}
/// Decode a single reverse result or the exact native no-coverage response.
pub fn reverse_results(bytes: &[u8], provider: &str) -> Result<GeocodeResults> {
    if bytes.len() > 1048576 {
        return Err(Error::TooLarge);
    }
    let value: Value = serde_json::from_slice(bytes).map_err(|_| Error::InvalidResponse)?;
    let object = value.as_object().ok_or(Error::InvalidResponse)?;
    if object.len() == 1 && object.get("error").and_then(Value::as_str) == Some("Unable to geocode")
    {
        return GeocodeResults::new(Vec::new(), 1);
    }
    if object.contains_key("error") {
        return Err(Error::InvalidResponse);
    }
    GeocodeResults::new(vec![place(&value, provider)?], 1)
}

fn native_id(value: Option<&Value>) -> Result<String> {
    let value = value.ok_or(Error::InvalidResponse)?;
    if let Some(id) = value.as_str() {
        if id.is_empty() || id.len() > 64 || !id.bytes().all(|c| c.is_ascii_digit()) {
            return Err(Error::InvalidResponse);
        }
        Ok(id.to_owned())
    } else {
        Ok(value.as_u64().ok_or(Error::InvalidResponse)?.to_string())
    }
}
