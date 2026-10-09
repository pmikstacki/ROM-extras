//! Exact public ROM identity; authorization and selection remain host responsibilities.
use crate::Coordinate;
/// A host-selected ROM Resource location, with its unchanged public key.
pub struct ResourceLocation {
    key: rom::Key,
    coordinate: Coordinate,
}
impl ResourceLocation {
    /// Bind a previously host-approved Resource key without parsing, case folding or hashing it.
    pub fn new(key: rom::Key, coordinate: Coordinate) -> Self {
        Self { key, coordinate }
    }
    /// Exact original public ROM key.
    pub fn key(&self) -> &rom::Key {
        &self.key
    }
    /// Explicit validated host location.
    pub fn coordinate(&self) -> Coordinate {
        self.coordinate
    }
}
