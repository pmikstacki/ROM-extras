# Explicit OIDC issuer presets

`rom-oidc-presets` configures ROM's existing `OidcIdTokenAdapter`.
It validates issuer configuration without fetching keys or issuing network requests.
The host must select an issuer-bound `TrustedKeys` source or approve public JWKS bytes.

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

## Host-approved JWKS snapshot

```rust
use rom_auth::{AuthError, OidcIdTokenAdapter};
use rom_oidc_presets::{IssuerPreset, JwksKeys};

fn verifier(bytes: &[u8]) -> Result<OidcIdTokenAdapter<JwksKeys>, AuthError> {
    IssuerPreset::keycloak("https://login.example", "staff")?
        .configure_jwks("employees", "rom-web", bytes)
}
```

The host approves exact issuer-bound bytes before it calls `configure_jwks`.
The immutable snapshot performs no discovery, HTTP, certificate resolution or background refresh.
Replace both the snapshot and verifier after approving changed provider keys.
ROM's key-cache interval does not establish provider revocation when its source remains this fixed snapshot.
Old approved keys remain trusted until the host replaces that source.

This is a bounded RS256 verification profile, not a parser for every JWK use.
JSON is limited to 65536 bytes, 32 total entries and eight selected signing keys.
Each entry requires an exact nonempty `kid` of at most 64 bytes and a `kty` of at most 32 bytes.
Duplicate IDs across eligible and ineligible entries are rejected. Known duplicate JSON fields are rejected.
Selected keys require `kty=RSA`, `alg=RS256`, absent or `sig` use, and absent or exactly `["verify"]` key operations.
Encryption keys and unsupported types or algorithms are skipped. An empty eligible set is rejected.
Private RSA parameters and symmetric `k` material are rejected, including explicit null values.
Recognized optional parameters cannot be null. Unknown extensions are ignored without resolving their URLs or certificates.

RSA moduli must be odd and contain 2048..=8192 bits.
Exponents must be odd and within 3..=u32::MAX.
Both use canonical, unpadded base64url minimal unsigned integers, without leading zero bytes.
These upper bounds and eligibility rules are local profile restrictions, not universal JWK requirements.
Errors and snapshot Debug omit raw keys, IDs and response text.
The existing public ROM verifier retains signature, issuer, audience, nonce and optional token-hash checks.

## Verified scope

Fifteen Rust tests and one doctest cover configuration, JWKS boundaries and signed synthetic tokens.
Eleven Python host tests cover exact single-use callbacks, expiry, endpoint control, private evidence and injected transport failures.
The injected tests check limits, 429 without retry, timeout and safe errors.
Controlled real TLS readers prove absolute deadline interruption for trickled response and chunk headers.
These faults are not attributed to Keycloak.

Actual local Keycloak 26.8.0 passed authorization-code/S256-PKCE login over trusted TLS.
It rejected a wrong verifier and repeated code redemption.
Independent source and normalized archive consumers checked actual ID tokens, native signing-key rotation and persistent service restart.
They checked current Resource authorization and persisted credential exclusion on native SQLite and redb.
The [host example](../examples/oidc-host/README.md) identifies the controlled configuration and reproducible gate.
A wrong-issuer rejection is covered by signed synthetic tests, but the native consumer does not repeat that negative case.
The Keycloak fixture uses persistent H2 and development mode, with only its TLS port published on loopback.
It is not a production database, session, deployment or browser qualification.
Entra, Dex, Auth0 and additional provider-specific login profiles remain unqualified.

Primary sources: [JWK format](https://www.rfc-editor.org/rfc/rfc7517.html),
[RSA JWK and algorithm requirements](https://www.rfc-editor.org/rfc/rfc7518.html),
[Keycloak OIDC endpoints](https://www.keycloak.org/securing-apps/oidc-layers),
[container and development guidance](https://www.keycloak.org/server/containers), and
[TLS configuration](https://www.keycloak.org/server/enabletls).
Keycloak is Apache-2.0 licensed; consult its distribution notices before redistribution.
See the [research](research/oidc-jwks-native-2026-10-10.md), [family design](superpowers/specs/2026-10-08-oidc-presets.md) and [support status](support.md).
