//! Explicit issuer presets that delegate verification to ROM.
//!
//! Presets make no network requests and do not approve a discovery or key source.
//! Hosts retain authority, client, nonce, key-acquisition, and authorization policy.
//!
//! ```
//! use rom_oidc_presets::IssuerPreset;
//! let preset = IssuerPreset::keycloak("https://login.example/auth", "staff")?;
//! assert_eq!(preset.issuer(), "https://login.example/auth/realms/staff");
//! # Ok::<(), rom_auth::AuthError>(())
//! ```

mod issuer;
pub use issuer::IssuerPreset;
mod jwks;
pub use jwks::JwksKeys;
mod jwk_wire;
mod rsa_components;
