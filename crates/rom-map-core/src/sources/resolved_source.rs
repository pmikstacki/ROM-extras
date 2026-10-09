//! Validate explicit style overrides while retaining resolved dataset credits.
use crate::{Attribution, Error, Result};
use crate::{BrowserPolicy, TileKind, TileSource};
use serde_json::{Value, json};

pub(crate) fn prepare(
    source: &TileSource,
    native: &Value,
    kind: TileKind,
    policy: &BrowserPolicy,
    host_credit: &Attribution,
) -> Result<Value> {
    let mut metadata = source.to_browser_json();
    let expected_kind = match kind {
        TileKind::Raster => "raster",
        TileKind::Vector => "vector",
    };
    if metadata["type"].as_str() != Some(expected_kind) {
        return Err(Error::InvalidResponse);
    }
    let inherited = metadata["credits"]
        .as_array()
        .ok_or(Error::InvalidResponse)?
        .clone();
    metadata["tilejson"] = json!("3.0.0");
    // TileJSON center is not a MapLibre source option. An override can change
    // zoom/bounds without changing the unrelated manifest camera suggestion.
    metadata
        .as_object_mut()
        .ok_or(Error::InvalidResponse)?
        .remove("center");
    for field in [
        "minzoom",
        "maxzoom",
        "bounds",
        "scheme",
        "attribution",
        "tileSize",
    ] {
        if let Some(value) = native.get(field) {
            metadata[field] = value.clone();
        }
    }
    let mut prepared = TileSource::from_json(
        &serde_json::to_vec(&metadata).map_err(|_| Error::InvalidResponse)?,
        kind,
        policy,
        Attribution::new(host_credit.text(), host_credit.link())?,
    )?
    .to_browser_json();
    let credits = prepared["credits"]
        .as_array_mut()
        .ok_or(Error::InvalidResponse)?;
    for credit in inherited {
        if !credits.contains(&credit) {
            credits.push(credit);
        }
    }
    Ok(prepared)
}
