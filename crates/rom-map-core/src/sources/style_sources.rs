//! Resolve sources before disclosing a style to a browser.
use crate::{Attribution, Error, Result};
use crate::{BrowserPolicy, TileKind, TileSource};
use serde_json::{Map, Value, json};
pub(crate) fn prepare(
    value: Option<&Value>,
    policy: &BrowserPolicy,
    resolved: &[(&str, &TileSource)],
    credits: &mut Vec<Value>,
    host_credit: &Attribution,
) -> Result<Map<String, Value>> {
    let sources = value
        .and_then(Value::as_object)
        .ok_or(Error::InvalidResponse)?;
    if sources.len() > 128 {
        return Err(Error::TooLarge);
    }
    let mut resolved_ids = std::collections::HashSet::new();
    for (id, _) in resolved {
        super::validation::text(id, 128)?;
        if !sources.contains_key(*id) || !resolved_ids.insert(*id) {
            return Err(Error::InvalidResponse);
        }
    }
    let mut prepared = Map::new();
    for (id, native) in sources {
        super::validation::text(id, 128)?;
        let kind = match native.get("type").and_then(Value::as_str) {
            Some("raster") => TileKind::Raster,
            Some("vector") => TileKind::Vector,
            _ => return Err(Error::Unsupported),
        };
        let mut approved = if let Some(url) = native.get("url") {
            policy.approve(url.as_str().ok_or(Error::InvalidResponse)?)?;
            if native.get("tiles").is_some() {
                return Err(Error::InvalidResponse);
            }
            let source = resolved
                .iter()
                .find(|(name, _)| *name == id)
                .ok_or(Error::Unsupported)?
                .1;
            super::resolved_source::prepare(source, native, kind, policy, host_credit)?
        } else {
            if resolved_ids.contains(id.as_str()) {
                return Err(Error::InvalidResponse);
            }
            let mut metadata = Value::Object(super::style::copy_fields(
                native,
                &[
                    "tiles",
                    "scheme",
                    "minzoom",
                    "maxzoom",
                    "bounds",
                    "attribution",
                    "vector_layers",
                    "tileSize",
                ],
            ));
            metadata["tilejson"] = json!("3.0.0");
            if metadata.get("maxzoom").is_none() {
                // Inline MapLibre sources differ from actual TileJSON manifests.
                metadata["maxzoom"] = json!(22);
            }
            TileSource::from_json(
                &serde_json::to_vec(&metadata).map_err(|_| Error::InvalidResponse)?,
                kind,
                policy,
                Attribution::new(host_credit.text(), host_credit.link())?,
            )?
            .to_browser_json()
        };
        if approved.get("type") != native.get("type") {
            return Err(Error::InvalidResponse);
        }
        for tile in approved
            .get("tiles")
            .and_then(Value::as_array)
            .ok_or(Error::InvalidResponse)?
        {
            policy.approve(tile.as_str().ok_or(Error::InvalidResponse)?)?;
        }
        if let Some(values) = approved.get("credits").and_then(Value::as_array) {
            for credit in values {
                if !credits.contains(credit) {
                    credits.push(credit.clone());
                }
            }
        }
        let object = approved.as_object_mut().ok_or(Error::InvalidResponse)?;
        object.remove("credits");
        object.remove("approvedOrigins");
        object.remove("center");
        super::source_properties::preserve(native, &mut approved, kind)?;
        prepared.insert(id.clone(), approved);
    }
    Ok(prepared)
}
