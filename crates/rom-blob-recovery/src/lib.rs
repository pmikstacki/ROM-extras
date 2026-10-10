//! Bounded host-approved external blob recovery over public ROM ports.
//! The host authenticates manifests, retains source objects and owns deployment cutover.
//! Completion covers supplied entries only; no cross-store snapshot or automatic retry is promised.
mod context;
mod error;
mod manifest;
mod recovery;
mod report;
pub use context::Context;
pub use error::{Cause, Failure, Phase, Publication};
pub use manifest::{Entry, Limits, Manifest};
pub use recovery::{copy, verify};
pub use report::Report;
