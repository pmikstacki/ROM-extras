//! Bounded Qdrant wire preparation. Native adapter qualification remains incomplete.
mod writes;
pub use writes::PreparedWrite;
#[cfg(test)]
mod writes_tests;
