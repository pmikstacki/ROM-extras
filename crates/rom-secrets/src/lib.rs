//! Host-owned secret references, bounded buffers and resolution contracts.
//!
//! Resolved material is not a Resource value. Hosts must explicitly expose bytes
//! to activate a client. Arbitrary host copies are outside this module's control.
mod error;
mod material;
mod reference;
mod resolver;
pub use error::Error;
pub use material::{ResolvedSecret, SecretBytes};
pub use reference::{SecretRef, Version};
pub use resolver::SecretResolver;
