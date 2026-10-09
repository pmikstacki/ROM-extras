//! Preserve explicit native decoder, feature identity and browser cache semantics.
use crate::TileKind;
use crate::{Error, Result};
use serde_json::Value;
pub(crate) fn preserve(native: &Value, approved: &mut Value, kind: TileKind) -> Result<()> {
    if let Some(encoding) = native.get("encoding") {
        if !matches!(kind, TileKind::Vector) {
            return Err(Error::Unsupported);
        }
        if !matches!(encoding.as_str(), Some("mvt" | "mlt")) {
            return Err(Error::Unsupported);
        }
        approved["encoding"] = encoding.clone();
    }
    if let Some(promote) = native.get("promoteId") {
        if !matches!(kind, TileKind::Vector) {
            return Err(Error::Unsupported);
        }
        if let Some(name) = promote.as_str() {
            super::validation::text(name, 128)?;
        } else if let Some(fields) = promote.as_object() {
            if fields.is_empty() {
                return Err(Error::InvalidResponse);
            }
            if fields.len() > 128 {
                return Err(Error::TooLarge);
            }
            for (layer, property) in fields {
                super::validation::text(layer, 128)?;
                super::validation::text(property.as_str().ok_or(Error::InvalidResponse)?, 128)?;
            }
        } else {
            return Err(Error::InvalidResponse);
        }
        approved["promoteId"] = promote.clone();
    }
    if let Some(volatile) = native.get("volatile") {
        if !volatile.is_boolean() {
            return Err(Error::InvalidResponse);
        }
        approved["volatile"] = volatile.clone();
    }
    Ok(())
}
