//! Administrative setup for isolated real-service acceptance cases.
mod administration;
mod lifecycle;
pub(crate) use administration::{Admin, unique};
pub(crate) use lifecycle::{Paused, container};
