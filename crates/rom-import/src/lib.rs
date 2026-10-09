//! Bounded JSON action imports using host-approved public ROM source grants.
//! This crate owns no transport, actor, source controller or retry loop.
mod action;
mod document;
mod error;
pub use action::{ActionBinding, ActionPlan, PreparedAction, SourceGrant};
pub use document::Limits;
pub use error::Error;
