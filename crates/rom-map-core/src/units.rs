//! Explicit nonnegative finite metres and seconds.
use crate::{Error, Result};
/// Finite nonnegative distance measured in metres.
#[derive(Clone, Copy, PartialEq)]
pub struct Metres(f64);
impl Metres {
    /// Validate a distance without silently changing its unit.
    pub fn new(value: f64) -> Result<Self> {
        validate(value)?;
        Ok(Self(value))
    }
    /// Exact admitted value in metres.
    pub fn as_metres(self) -> f64 {
        self.0
    }
}
/// Finite nonnegative duration measured in seconds.
#[derive(Clone, Copy, PartialEq)]
pub struct Seconds(f64);
impl Seconds {
    /// Validate a duration without silently changing its unit.
    pub fn new(value: f64) -> Result<Self> {
        validate(value)?;
        Ok(Self(value))
    }
    /// Exact admitted value in seconds.
    pub fn as_seconds(self) -> f64 {
        self.0
    }
}
fn validate(value: f64) -> Result<()> {
    if !value.is_finite() || value < 0.0 {
        Err(Error::InvalidUnit)
    } else {
        Ok(())
    }
}
