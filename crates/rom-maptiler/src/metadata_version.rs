//! Preserve explicitly selected native TileJSON zoom defaults.
use rom_map_core::{Error, Result};
use serde_json::{Value, json};
/// Native manifest version selected explicitly by the host, not guessed from an endpoint.
#[derive(Clone, Copy)]
pub enum TileJsonVersion {
    /// TileJSON2.0.0, with zoom restricted to22.
    V2_0,
    /// TileJSON2.1.0, with zoom restricted to22.
    V2_1,
    /// TileJSON2.2.0, with zoom restricted to30.
    V2_2,
    /// TileJSON3.0.0, with zoom restricted to30.
    V3_0,
}
impl TileJsonVersion {
    pub(crate) fn prepare(self, native: &mut Value) -> Result<()> {
        let (name, max) = match self {
            Self::V2_0 => ("2.0.0", 22),
            Self::V2_1 => ("2.1.0", 22),
            Self::V2_2 => ("2.2.0", 30),
            Self::V3_0 => ("3.0.0", 30),
        };
        if native.get("tilejson").and_then(Value::as_str) != Some(name) {
            return Err(Error::Unsupported);
        }
        for field in ["minzoom", "maxzoom"] {
            if native
                .get(field)
                .is_some_and(|v| v.as_u64().is_none_or(|v| v > max))
            {
                return Err(Error::InvalidResponse);
            }
        }
        if native.get("maxzoom").is_none() {
            native["maxzoom"] = json!(max);
        }
        native["tilejson"] = json!("3.0.0");
        Ok(())
    }
}
