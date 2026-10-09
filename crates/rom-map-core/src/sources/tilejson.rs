//! Strict bounded admission of hosted TileJSON 3.0.0 and explicit browser disclosure.
use crate::BrowserPolicy;
use crate::{Attribution, BoundingBox, Coordinate, Error, Result};
use serde_json::{Value, json};
/// Native format chosen by the host, never inferred from filename.
#[derive(Clone, Copy)]
pub enum TileKind {
    /// Raster image tiles.
    Raster,
    /// Vector tiles with declared native layer metadata.
    Vector,
}
/// Approved metadata, distinct from backend configuration or a request to activate a map.
pub struct TileSource {
    browser: Value,
    kind: TileKind,
}
impl TileSource {
    /// Admit TileJSON under explicit host origins and host-reviewed credit, without network I/O.
    /// Optional malformed fields are rejected under this strict admission profile.
    pub fn from_json(
        bytes: &[u8],
        kind: TileKind,
        policy: &BrowserPolicy,
        credit: Attribution,
    ) -> Result<Self> {
        if bytes.len() > 1048576 {
            return Err(Error::TooLarge);
        }
        let native: Value = serde_json::from_slice(bytes).map_err(|_| Error::InvalidResponse)?;
        if native.get("tilejson").and_then(Value::as_str) != Some("3.0.0") {
            return Err(Error::Unsupported);
        }
        let tiles = native
            .get("tiles")
            .and_then(Value::as_array)
            .ok_or(Error::InvalidResponse)?;
        if tiles.is_empty() {
            return Err(Error::InvalidResponse);
        }
        if tiles.len() > 16 {
            return Err(Error::TooLarge);
        }
        let tiles = tiles
            .iter()
            .map(|v| policy.approve(v.as_str().ok_or(Error::InvalidResponse)?))
            .collect::<Result<Vec<_>>>()?;
        let minzoom = zoom(native.get("minzoom"), 0)?;
        let maxzoom = zoom(native.get("maxzoom"), 30)?;
        if minzoom > maxzoom {
            return Err(Error::InvalidResponse);
        }
        let scheme = match native.get("scheme") {
            None => "xyz",
            Some(v) => match v.as_str() {
                Some("xyz") => "xyz",
                Some("tms") => "tms",
                _ => return Err(Error::InvalidResponse),
            },
        };
        let mut credits = vec![credit_json(&credit)];
        if let Some(native_credit) = native.get("attribution") {
            let native_credit =
                Attribution::new(native_credit.as_str().ok_or(Error::InvalidResponse)?, None)?;
            if native_credit.text() != credit.text() {
                credits.push(credit_json(&native_credit));
            }
        }
        let mut browser = json!({"type": match kind { TileKind::Raster=>"raster",TileKind::Vector=>"vector" },"tiles":tiles,"scheme":scheme,"minzoom":minzoom,"maxzoom":maxzoom,"credits":credits,"approvedOrigins":policy.origins()});
        if let Some(size) = super::raster_size::prepare(native.get("tileSize"), kind)? {
            browser["tileSize"] = json!(size.pixels());
        }
        let bounds = if let Some(value) = native.get("bounds") {
            let a = value
                .as_array()
                .filter(|a| a.len() == 4)
                .ok_or(Error::InvalidResponse)?;
            let n = a
                .iter()
                .map(|v| v.as_f64().ok_or(Error::InvalidResponse))
                .collect::<Result<Vec<_>>>()?;
            if n[0] > n[2] {
                return Err(Error::InvalidResponse);
            }
            let bounds = BoundingBox::new(n[0], n[1], n[2], n[3])?;
            browser["bounds"] = json!(n);
            Some(bounds)
        } else {
            None
        };
        if let Some(value) = native.get("center") {
            let a = value
                .as_array()
                .filter(|a| a.len() == 3)
                .ok_or(Error::InvalidResponse)?;
            let center = Coordinate::new(
                a[0].as_f64().ok_or(Error::InvalidResponse)?,
                a[1].as_f64().ok_or(Error::InvalidResponse)?,
            )?;
            let z = zoom(Some(&a[2]), 0)?;
            if z < minzoom || z > maxzoom || bounds.is_some_and(|b| !b.contains(center)) {
                return Err(Error::InvalidResponse);
            }
            browser["center"] = json!([
                center.longitude_degrees(),
                center.latitude_degrees(),
                f64::from(z)
            ]);
        }
        if let TileKind::Vector = kind {
            browser["vector_layers"] =
                super::vector_layers::prepare(native.get("vector_layers"), minzoom, maxzoom)?;
        }
        Ok(Self { browser, kind })
    }
    /// Explicit browser metadata. Consuming this value does not activate network access.
    /// Hosts must enforce disclosed origin controls, including redirected browser requests.
    pub fn to_browser_json(&self) -> Value {
        self.browser.clone()
    }
    /// Host-admitted tile format, independent of filename or MIME inference.
    pub fn kind(&self) -> TileKind {
        self.kind
    }
}
pub(crate) fn zoom(value: Option<&Value>, default: u8) -> Result<u8> {
    match value {
        None => Ok(default),
        Some(value) => value
            .as_u64()
            .filter(|v| *v <= 30)
            .map(|v| v as u8)
            .ok_or(Error::InvalidResponse),
    }
}
fn credit_json(credit: &Attribution) -> Value {
    json!({"text":credit.text(),"link":credit.link()})
}
