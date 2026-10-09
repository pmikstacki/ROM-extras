//! Rewrite only known native resource fields; core admission remains authoritative.
use crate::MapMetadata;
use rom_map_core::{Error, Result};
use serde_json::{Value, json};
pub(crate) fn tiles(metadata: &MapMetadata, native: &mut Value) -> Result<()> {
    let tiles = native
        .get_mut("tiles")
        .and_then(Value::as_array_mut)
        .ok_or(Error::InvalidResponse)?;
    if tiles.len() > 16 {
        return Err(Error::TooLarge);
    }
    for tile in tiles {
        *tile = metadata.resource(tile)?;
    }
    Ok(())
}
pub(crate) fn style(metadata: &MapMetadata, native: &mut Value) -> Result<()> {
    let sources = native
        .get_mut("sources")
        .and_then(Value::as_object_mut)
        .ok_or(Error::InvalidResponse)?;
    if sources.len() > 128 {
        return Err(Error::TooLarge);
    }
    for source in sources.values_mut() {
        if let Some(url) = source.get_mut("url") {
            *url = metadata.resource(url)?;
        }
        if source.get("tiles").is_some() {
            tiles(metadata, source)?;
        }
        if source.get("attribution").is_some() {
            source["attribution"] = json!(
                metadata
                    .attribution
                    .approve(source.get("attribution"))?
                    .text()
            );
        }
    }
    if let Some(glyphs) = native.get_mut("glyphs") {
        *glyphs = metadata.resource(glyphs)?;
    }
    if let Some(sprite) = native.get_mut("sprite") {
        if sprite.is_string() {
            *sprite = metadata.resource(sprite)?;
        } else {
            let sprites = sprite.as_array_mut().ok_or(Error::InvalidResponse)?;
            if sprites.len() > 16 {
                return Err(Error::TooLarge);
            }
            for sprite in sprites {
                let url = sprite.get_mut("url").ok_or(Error::InvalidResponse)?;
                *url = metadata.resource(url)?;
            }
        }
    }
    Ok(())
}
