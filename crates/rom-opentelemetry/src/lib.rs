//! Host-owned payload-free OpenTelemetry instrumentation for public ROM observations.
mod error;
mod operation;
mod outcome;
mod runtime;
pub use error::TelemetryError;
pub use operation::Operation;
pub use outcome::Outcome;
pub use runtime::RuntimeMetrics;
