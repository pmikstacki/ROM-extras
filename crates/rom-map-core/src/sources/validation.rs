//! Bounded plain identifier validation shared by source and style contracts.
use crate::{Error, Result};
pub(crate) fn text(value: &str, limit: usize) -> Result<()> {
    if value.len() > limit {
        return Err(Error::TooLarge);
    }
    if value.trim().is_empty() || value.chars().any(char::is_control) {
        return Err(Error::InvalidResponse);
    }
    Ok(())
}
