//! Bounded native metadata admission, with no network I/O or browser activation.
use crate::{AttributionProfile, MapBrowserGrant, TileJsonVersion};
use rom_map_core::{Error, MapStyle, RasterTileSize, Result, TileKind, TileSource};
use serde_json::{Value, json};
use url::Url;
use zeroize::Zeroizing;
/// Host-approved metadata decoder; server credentials cannot create browser grants.
pub struct MapMetadata {
    pub(crate) server_key: Zeroizing<String>,
    pub(crate) grant: MapBrowserGrant,
    pub(crate) attribution: AttributionProfile,
}
impl MapMetadata {
    /// Prepare independent server and browser keys with reviewed native attribution.
    /// No endpoint is selected and no network connection starts.
    pub fn new(
        server_key: String,
        grant: MapBrowserGrant,
        attribution: AttributionProfile,
    ) -> Result<Self> {
        let server_key = Zeroizing::new(server_key);
        if server_key.len() > 4096 {
            return Err(Error::TooLarge);
        }
        if server_key.is_empty() || server_key.chars().any(char::is_control) {
            return Err(Error::InvalidQuery);
        }
        if grant.token.contains(server_key.as_str()) {
            return Err(Error::Rejected);
        }
        Ok(Self {
            server_key,
            grant,
            attribution,
        })
    }
    pub(crate) fn resource(&self, native: &Value) -> Result<Value> {
        let text = native.as_str().ok_or(Error::InvalidResponse)?;
        if text.len() > 4096 {
            return Err(Error::TooLarge);
        }
        let url = Url::parse(text).map_err(|_| Error::Rejected)?;
        self.no_encoded_disclosure(url.path())?;
        if url.fragment().is_some() {
            return Err(Error::Rejected);
        }
        let approved = if url.query().is_some() {
            let pairs = url.query_pairs().collect::<Vec<_>>();
            if pairs.len() != 1 || pairs[0].0 != "key" || pairs[0].1 != self.server_key.as_str() {
                return Err(Error::Rejected);
            }
            let (base, _) = text.split_once('?').ok_or(Error::Rejected)?;
            format!("{base}?key={}", self.grant.token)
        } else {
            text.to_owned()
        };
        Ok(json!(self.grant.policy.approve(&approved)?))
    }
    /// Admit a strict supported native manifest subset, retaining version defaults and original layer IDs.
    /// Raster logical size must be host-selected; vector sources cannot declare one.
    pub fn tilejson(
        &self,
        bytes: &[u8],
        version: TileJsonVersion,
        kind: TileKind,
        size: Option<RasterTileSize>,
    ) -> Result<TileSource> {
        let mut native = parse(bytes)?;
        version.prepare(&mut native)?;
        let credit = self.attribution.approve(native.get("attribution"))?;
        native["attribution"] = json!(credit.text());
        super::metadata_sources::tiles(self, &mut native)?;
        match (kind, size) {
            (TileKind::Raster, Some(size)) => {
                if native
                    .get("tileSize")
                    .is_some_and(|v| v.as_u64() != Some(u64::from(size.pixels())))
                {
                    return Err(Error::InvalidResponse);
                }
                native["tileSize"] = json!(size.pixels());
            }
            (TileKind::Vector, None) => {}
            _ => return Err(Error::InvalidQuery),
        }
        let source = TileSource::from_json(
            &serde_json::to_vec(&native).map_err(|_| Error::InvalidResponse)?,
            kind,
            &self.grant.policy,
            credit,
        )
        .map_err(admission_error)?;
        self.no_disclosure(&source.to_browser_json())?;
        Ok(source)
    }
    /// Admit a restricted MapLibre-v8 style with exact, explicitly resolved source bindings.
    /// Never fetch nested manifests or activate browser resources.
    pub fn style(&self, bytes: &[u8], resolved: &[(&str, &TileSource)]) -> Result<MapStyle> {
        let mut native = parse(bytes)?;
        super::metadata_sources::style(self, &mut native)?;
        let prepared = MapStyle::from_json(
            &serde_json::to_vec(&native).map_err(|_| Error::InvalidResponse)?,
            &self.grant.policy,
            &[self.attribution.credit()?],
            resolved,
        )
        .map_err(admission_error)?;
        self.no_disclosure(&prepared.to_browser_json())?;
        Ok(prepared)
    }
    fn no_disclosure(&self, value: &Value) -> Result<()> {
        let leaked = match value {
            Value::String(v) => self.no_encoded_disclosure(v).is_err(),
            Value::Array(values) => values.iter().any(|v| self.no_disclosure(v).is_err()),
            Value::Object(values) => values.iter().any(|(key, v)| {
                self.no_encoded_disclosure(key).is_err() || self.no_disclosure(v).is_err()
            }),
            _ => false,
        };
        if leaked { Err(Error::Rejected) } else { Ok(()) }
    }
    fn no_encoded_disclosure(&self, text: &str) -> Result<()> {
        let mut decoded = std::borrow::Cow::Borrowed(text);
        for _ in 0..8 {
            if decoded.contains(self.server_key.as_str()) {
                return Err(Error::Rejected);
            }
            let next = percent_encoding::percent_decode_str(&decoded)
                .decode_utf8()
                .map_err(|_| Error::Rejected)?;
            if next == decoded {
                return Ok(());
            }
            decoded = std::borrow::Cow::Owned(next.into_owned());
        }
        // Reject excessive nested encoding rather than disclosing an uninspected value.
        Err(Error::Rejected)
    }
}
fn admission_error(error: Error) -> Error {
    match error {
        Error::InvalidCoordinate
        | Error::InvalidBounds
        | Error::InvalidGeometry
        | Error::InvalidUnit
        | Error::InvalidQuery => Error::InvalidResponse,
        other => other,
    }
}
fn parse(bytes: &[u8]) -> Result<Value> {
    if bytes.len() > 1048576 {
        return Err(Error::TooLarge);
    }
    let native: Value = serde_json::from_slice(bytes).map_err(|_| Error::InvalidResponse)?;
    if !native.is_object() {
        return Err(Error::InvalidResponse);
    }
    Ok(native)
}
