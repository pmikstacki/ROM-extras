//! Preserve layer identities and verify source references before browser disclosure.
use crate::{Error, Result};
use serde_json::{Map, Value, json};
pub(crate) fn prepare(value: Option<&Value>, sources: &Map<String, Value>) -> Result<Value> {
    let layers = value
        .and_then(Value::as_array)
        .ok_or(Error::InvalidResponse)?;
    if layers.len() > 1024 {
        return Err(Error::TooLarge);
    }
    let mut ids = std::collections::HashSet::new();
    let mut prepared = Vec::new();
    for layer in layers {
        let id = layer
            .get("id")
            .and_then(Value::as_str)
            .ok_or(Error::InvalidResponse)?;
        super::validation::text(id, 128)?;
        if !ids.insert(id) {
            return Err(Error::InvalidResponse);
        }
        let kind = layer
            .get("type")
            .and_then(Value::as_str)
            .ok_or(Error::InvalidResponse)?;
        if !matches!(
            kind,
            "background"
                | "raster"
                | "fill"
                | "line"
                | "symbol"
                | "circle"
                | "heatmap"
                | "fill-extrusion"
        ) {
            return Err(Error::Unsupported);
        }
        match layer.get("source") {
            Some(source) => {
                let source = source.as_str().ok_or(Error::InvalidResponse)?;
                let selected = sources.get(source).ok_or(Error::InvalidResponse)?;
                if kind == "background"
                    || kind == "raster" && selected["type"] != "raster"
                    || kind != "raster" && selected["type"] != "vector"
                {
                    return Err(Error::InvalidResponse);
                }
                if selected["type"] == "vector" {
                    let source_layer = layer
                        .get("source-layer")
                        .and_then(Value::as_str)
                        .ok_or(Error::InvalidResponse)?;
                    let native_layers = selected
                        .get("vector_layers")
                        .and_then(Value::as_array)
                        .ok_or(Error::InvalidResponse)?;
                    if !native_layers.iter().any(|native| {
                        native.get("id").and_then(Value::as_str) == Some(source_layer)
                    }) {
                        return Err(Error::InvalidResponse);
                    }
                } else if layer.get("source-layer").is_some() {
                    return Err(Error::InvalidResponse);
                }
            }
            None if kind == "background" && layer.get("source-layer").is_none() => {}
            None => return Err(Error::InvalidResponse),
        }
        if let Some(source_layer) = layer.get("source-layer") {
            super::validation::text(source_layer.as_str().ok_or(Error::InvalidResponse)?, 128)?;
        }
        let mut lower = 0.0;
        let mut upper = 30.0;
        for (field, bound) in [("minzoom", &mut lower), ("maxzoom", &mut upper)] {
            if let Some(value) = layer.get(field) {
                *bound = value
                    .as_f64()
                    .filter(|n| n.is_finite() && (0.0..=30.0).contains(n))
                    .ok_or(Error::InvalidResponse)?;
            }
        }
        if lower > upper {
            return Err(Error::InvalidResponse);
        }
        prepared.push(Value::Object(super::style::copy_fields(
            layer,
            &[
                "id",
                "type",
                "source",
                "source-layer",
                "minzoom",
                "maxzoom",
                "filter",
                "layout",
                "paint",
            ],
        )));
    }
    Ok(json!(prepared))
}
