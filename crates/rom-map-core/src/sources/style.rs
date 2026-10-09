//! Host-approved MapLibre style descriptors without opaque nested metadata fetches.
use crate::{Attribution, Coordinate, Error, Result};
use crate::{BrowserPolicy, TileSource};
use serde_json::{Map, Value, json};
/// Bounded style data after host origin and attribution approval.
pub struct MapStyle {
    browser: Value,
}
impl MapStyle {
    /// Resolve hosted TileJSON references explicitly before disclosure.
    /// The initial strict profile accepts raster/vector sources and rejects unsupported resource loaders.
    pub fn from_json(
        bytes: &[u8],
        policy: &BrowserPolicy,
        credits: &[Attribution],
        resolved: &[(&str, &TileSource)],
    ) -> Result<Self> {
        if bytes.len() > 1048576 {
            return Err(Error::TooLarge);
        }
        if credits.is_empty() {
            return Err(Error::InvalidResponse);
        }
        if credits.len() > 16 || resolved.len() > 128 {
            return Err(Error::TooLarge);
        }
        let native: Value = serde_json::from_slice(bytes).map_err(|_| Error::InvalidResponse)?;
        if native.get("version").and_then(Value::as_u64) != Some(8) {
            return Err(Error::Unsupported);
        }
        for unsupported in [
            "font-faces",
            "imports",
            "terrain",
            "light",
            "sky",
            "projection",
            "state",
            "transition",
            "centerAltitude",
        ] {
            if native.get(unsupported).is_some() {
                return Err(Error::Unsupported);
            }
        }
        let mut prepared = json!({"version":8});
        if let Some(name) = native.get("name") {
            super::validation::text(name.as_str().ok_or(Error::InvalidResponse)?, 1024)?;
            prepared["name"] = name.clone();
        }
        if let Some(center) = native.get("center") {
            let a = center
                .as_array()
                .filter(|a| a.len() == 2)
                .ok_or(Error::InvalidResponse)?;
            Coordinate::new(
                a[0].as_f64().ok_or(Error::InvalidResponse)?,
                a[1].as_f64().ok_or(Error::InvalidResponse)?,
            )?;
            prepared["center"] = center.clone();
        }
        for field in ["zoom", "bearing", "pitch", "roll"] {
            if let Some(value) = native.get(field) {
                let n = value.as_f64().ok_or(Error::InvalidResponse)?;
                if !n.is_finite() || field == "zoom" && !(0.0..=30.0).contains(&n) {
                    return Err(Error::InvalidResponse);
                }
                prepared[field] = value.clone();
            }
        }
        let mut approved_credits: Vec<Value> = credits
            .iter()
            .map(|c| json!({"text":c.text(),"link":c.link()}))
            .collect();
        let mut sources = super::style_sources::prepare(
            native.get("sources"),
            policy,
            resolved,
            &mut approved_credits,
            &credits[0],
        )?;
        prepared["layers"] = super::style_layers::prepare(native.get("layers"), &sources)?;
        for source in sources.values_mut() {
            source
                .as_object_mut()
                .ok_or(Error::InvalidResponse)?
                .remove("vector_layers");
        }
        prepared["sources"] = Value::Object(sources);
        if let Some(glyphs) = native.get("glyphs") {
            let glyphs = glyphs.as_str().ok_or(Error::InvalidResponse)?;
            if !glyphs.contains("{fontstack}") || !glyphs.contains("{range}") {
                return Err(Error::InvalidResponse);
            }
            prepared["glyphs"] = json!(policy.approve(glyphs)?);
        }
        if let Some(sprite) = native.get("sprite") {
            prepared["sprite"] = sprites(sprite, policy)?;
        }
        if approved_credits.len() > 256 {
            return Err(Error::TooLarge);
        }
        Ok(Self {
            browser: json!({"style":prepared,"credits":approved_credits,"approvedOrigins":policy.origins()}),
        })
    }
    /// Browser descriptor; this method does not activate external connections.
    /// The host must enforce origin controls for redirected browser requests too.
    pub fn to_browser_json(&self) -> Value {
        self.browser.clone()
    }
}
fn sprites(value: &Value, policy: &BrowserPolicy) -> Result<Value> {
    if let Some(url) = value.as_str() {
        return Ok(json!(policy.approve(url)?));
    }
    let sprites = value
        .as_array()
        .filter(|a| !a.is_empty())
        .ok_or(Error::InvalidResponse)?;
    if sprites.len() > 16 {
        return Err(Error::TooLarge);
    }
    let mut ids = std::collections::HashSet::new();
    let mut urls = std::collections::HashSet::new();
    let mut prepared = Vec::new();
    for sprite in sprites {
        let id = sprite
            .get("id")
            .and_then(Value::as_str)
            .ok_or(Error::InvalidResponse)?;
        super::validation::text(id, 128)?;
        let url = policy.approve(
            sprite
                .get("url")
                .and_then(Value::as_str)
                .ok_or(Error::InvalidResponse)?,
        )?;
        if !ids.insert(id) || !urls.insert(url.clone()) {
            return Err(Error::InvalidResponse);
        }
        prepared.push(json!({"id":id,"url":url}));
    }
    Ok(json!(prepared))
}
pub(crate) fn copy_fields(native: &Value, fields: &[&str]) -> Map<String, Value> {
    fields
        .iter()
        .filter_map(|field| {
            native
                .get(*field)
                .map(|value| ((*field).to_owned(), value.clone()))
        })
        .collect()
}
