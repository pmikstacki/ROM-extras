#![forbid(unsafe_code)]
#![deny(unused_must_use)]
mod host;
pub use host::{Host, HostSession, SessionResolver, router};
mod cache;

mod selection;
pub use selection::{SelectionHandler, SelectionOutcome};

mod geocoding;
mod origin;
