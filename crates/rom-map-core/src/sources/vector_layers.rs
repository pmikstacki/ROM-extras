//! Native vector-layer identity and zoom metadata admission.
use crate::{Error, Result};
use serde_json::{Value, json};
use std::collections::HashSet;
pub(crate) fn prepare(value: Option<&Value>, minzoom: u8, maxzoom: u8) -> Result<Value> {
    let layers = value
        .and_then(Value::as_array)
        .filter(|a| !a.is_empty())
        .ok_or(Error::InvalidResponse)?;
    if layers.len() > 128 {
        return Err(Error::TooLarge);
    }
    let mut ids = HashSet::new();
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
        let fields = layer
            .get("fields")
            .and_then(Value::as_object)
            .ok_or(Error::InvalidResponse)?;
        if fields.len() > 256 {
            return Err(Error::TooLarge);
        }
        for (key, value) in fields {
            super::validation::text(key, 128)?;
            let description = value.as_str().ok_or(Error::InvalidResponse)?;
            if description.len() > 1024 {
                return Err(Error::TooLarge);
            }
            if description.chars().any(char::is_control) {
                return Err(Error::InvalidResponse);
            }
        }
        let lower = super::tilejson::zoom(layer.get("minzoom"), minzoom)?;
        let upper = super::tilejson::zoom(layer.get("maxzoom"), maxzoom)?;
        if lower < minzoom || upper > maxzoom || lower > upper {
            return Err(Error::InvalidResponse);
        }
        prepared.push(json!({"id":id,"fields":fields,"minzoom":lower,"maxzoom":upper}));
    }
    Ok(json!(prepared))
}
