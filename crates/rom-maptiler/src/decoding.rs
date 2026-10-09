use rom_map_core::{
    Accuracy, Attribution, BoundingBox, Coordinate, Error, GeocodeResult, GeocodeResults,
    Provenance, Result,
};
use serde_json::Value;
/// Exact host-reviewed native attribution with its approved plain-text equivalent.
pub struct AttributionProfile {
    native: String,
    plain: Attribution,
}
impl AttributionProfile {
    pub(crate) fn approve(&self, native: Option<&Value>) -> Result<Attribution> {
        if native.and_then(Value::as_str) != Some(self.native.as_str()) {
            return Err(Error::InvalidResponse);
        }
        self.credit()
    }
    pub(crate) fn credit(&self) -> Result<Attribution> {
        Attribution::new(self.plain.text(), self.plain.link())
    }
    /// Bind reviewed provider markup to plain text; arbitrary native HTML is never rendered or stripped.
    pub fn new(native: &str, plain: Attribution) -> Result<Self> {
        if native.len() > 8192 {
            return Err(Error::TooLarge);
        }
        if native.trim().is_empty() || native.contains('\0') {
            return Err(Error::InvalidQuery);
        }
        Ok(Self {
            native: native.into(),
            plain,
        })
    }
}
fn coordinate(value: &Value) -> Result<Coordinate> {
    let a = value
        .as_array()
        .filter(|a| a.len() == 2)
        .ok_or(Error::InvalidResponse)?;
    Coordinate::new(
        a[0].as_f64().ok_or(Error::InvalidResponse)?,
        a[1].as_f64().ok_or(Error::InvalidResponse)?,
    )
    .map_err(|_| Error::InvalidResponse)
}
/// Decode a bounded forward/reverse FeatureCollection while retaining exact, reindex-unstable native IDs.
pub fn geocode_response(
    bytes: &[u8],
    limit: usize,
    provider: &str,
    profile: &AttributionProfile,
) -> Result<GeocodeResults> {
    if bytes.len() > 1048576 {
        return Err(Error::TooLarge);
    }
    if !(1..=10).contains(&limit) {
        return Err(Error::InvalidQuery);
    }
    let root: Value = serde_json::from_slice(bytes).map_err(|_| Error::InvalidResponse)?;
    if root.get("type").and_then(Value::as_str) != Some("FeatureCollection")
        || root.get("attribution").and_then(Value::as_str) != Some(profile.native.as_str())
    {
        return Err(Error::InvalidResponse);
    }
    let features = root
        .get("features")
        .and_then(Value::as_array)
        .ok_or(Error::InvalidResponse)?;
    if features.len() > limit {
        return Err(Error::TooLarge);
    }
    let mut results = Vec::with_capacity(features.len());
    for feature in features {
        if feature.get("type").and_then(Value::as_str) != Some("Feature")
            || feature.pointer("/geometry/type").and_then(Value::as_str) != Some("Point")
        {
            return Err(Error::InvalidResponse);
        }
        let point = coordinate(
            feature
                .pointer("/geometry/coordinates")
                .ok_or(Error::InvalidResponse)?,
        )?;
        if let Some(center) = feature.get("center") {
            coordinate(center)?;
        }
        let bounds = if let Some(bbox) = feature.get("bbox") {
            let b = bbox
                .as_array()
                .filter(|b| b.len() == 4)
                .ok_or(Error::InvalidResponse)?;
            Some(
                BoundingBox::new(
                    b[0].as_f64().ok_or(Error::InvalidResponse)?,
                    b[1].as_f64().ok_or(Error::InvalidResponse)?,
                    b[2].as_f64().ok_or(Error::InvalidResponse)?,
                    b[3].as_f64().ok_or(Error::InvalidResponse)?,
                )
                .map_err(|_| Error::InvalidResponse)?,
            )
        } else {
            None
        };
        let credit = Attribution::new(profile.plain.text(), profile.plain.link())?;
        let provenance = Provenance::new(
            provider,
            feature
                .get("id")
                .and_then(Value::as_str)
                .ok_or(Error::InvalidResponse)?,
            credit,
        )?;
        results.push(GeocodeResult::new(
            feature
                .get("place_name")
                .and_then(Value::as_str)
                .ok_or(Error::InvalidResponse)?,
            point,
            Accuracy::Unknown,
            provenance,
            bounds,
        )?);
    }
    GeocodeResults::new(results, limit)
}
