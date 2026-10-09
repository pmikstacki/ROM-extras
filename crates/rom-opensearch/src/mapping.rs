//! Fixed physical generation and explicitly indexed text fields.
use rom_projection_core::{Error, Result};
use serde_json::{Value, json};
pub(crate) fn identifier(s: &str) -> Result<()> {
    if s.is_empty()
        || s.len() > 128
        || !s
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        || !s.as_bytes()[0].is_ascii_lowercase()
    {
        return Err(Error::Invalid);
    }
    Ok(())
}
pub(crate) fn fields(mut fields: Vec<String>) -> Result<Vec<String>> {
    if fields.len() > 64 {
        return Err(Error::TooLarge);
    }
    for f in &fields {
        identifier(f)?;
    }
    fields.sort();
    if fields.windows(2).any(|w| w[0] == w[1]) {
        return Err(Error::Conflict);
    }
    Ok(fields)
}
pub(crate) fn definition(profile: &str, fields: &[String]) -> Value {
    let indexed: serde_json::Map<String, Value> = fields
        .iter()
        .map(|f| (f.clone(), json!({"type":"text"})))
        .collect();
    let search = if indexed.is_empty() {
        json!({"type":"object", "dynamic":"strict"})
    } else {
        json!({"dynamic":"strict", "properties":indexed})
    };
    json!({"dynamic":"strict","_meta":{"rom_profile":profile},"properties":{
        "rom_profile":{"type":"keyword","index":false},"rom_kind":{"type":"keyword","index":false},"rom_id":{"type":"keyword","index":false},
        "rom_revision":{"type":"long","index":false},"rom_digest":{"type":"keyword","index":false},"rom_live":{"type":"boolean"},
        "rom_values":{"type":"object","enabled":false},"rom_search":search
    }})
}
