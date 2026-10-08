# Explicit OIDC issuer presets

`rom-oidc-presets` configures ROM's existing `OidcIdTokenAdapter`.
It validates issuer configuration without fetching keys or issuing network requests.
The host must select an issuer-bound `TrustedKeys` source.

```rust
use rom_auth::{AuthError, OidcIdTokenAdapter, jwt::TrustedKeys};
use rom_oidc_presets::IssuerPreset;

fn staff_verifier<K: TrustedKeys>(keys: K) -> Result<OidcIdTokenAdapter<K>, AuthError> {
    IssuerPreset::keycloak("https://login.example/auth", "staff")?
        .configure("employees", "rom-web", keys)
}
```

| Preset | Configuration |
| --- | --- |
| Keycloak | Explicit HTTPS deployment base, including context path; one ASCII unreserved realm segment. |
| Microsoft Entra public cloud | One concrete GUID tenant; issuer includes `/v2.0`. Multi-tenant aliases are rejected. |
| Explicit issuer | Exact HTTPS issuer for other providers, sovereign clouds, or realm names outside the generated preset profile. |

The exact issuer retains its original bytes, including trailing slash.
Issuer length is bounded to 2048 bytes. Credentials, whitespace, controls, backslashes, query, and fragment components are rejected.
Generated Keycloak realms use 1..=256 ASCII unreserved bytes, excluding `.` and `..`.

The returned adapter keeps ROM's RS256 human ID-token profile, nonce validation, bounded expiry, and issuer/audience binding.
It does not grant Resource authorization. Preserve principal kind and expiry when the host constructs an actor.
The host owns one-use login state, authorization code, and retained nonce.
Key discovery, network deadlines, and key endpoint trust remain host responsibilities.

Six configuration tests and three signed synthetic-token tests passed locally.
The signed tests cover all presets, exact issuer mismatch, wrong audience/nonce, expiry, wrong signatures, and key rotation.
These tests do not establish live provider login support. No Keycloak, Entra, Dex, or Auth0 service was contacted.
See the [family design](superpowers/specs/2026-10-08-oidc-presets.md) and [support status](support.md).
