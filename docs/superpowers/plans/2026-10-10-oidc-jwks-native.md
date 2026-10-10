# OIDC JWKS and native login implementation plan

> **For agentic workers:** Use superpowers:executing-plans inline. Root owns all maintained edits; one final reviewer owns its private report.

**Goal:** Parse host-approved public JWKS into existing ROM OIDC verification and qualify actual local authorization-code login.
**Architecture:** Pure bounded snapshot in existing presets. A controlled host performs TLS discovery/login/acquisition; public Rust consumers verify identity and authorization.
**Tech Stack:** Pinned rom-auth, existing serde/serde_json/base64 and OpenSSL; isolated Keycloak26.8.0; no new registry package.
**Spec:** ../specs/2026-10-10-oidc-jwks-native.md

## Constraints and ownership

Keep issuer and verifier APIs stable. Do not add network discovery to the preset constructor.
Root owns new `jwks`, `jwk_wire` and `rsa_components` modules, tests, consumers, scripts and guides.
Only private cache archives may retire inactive compiler working copies after identical-byte verification. Preserve final binaries and service data.

## Review focus

- Duplicate kid across eligible/ineligible entries must not select an arbitrary key.
- JSON null/duplicate names and private parameters must not be silently downgraded.
- Snapshot reuse must not imply server-side revocation or live refresh.
- Native code login must use PKCE/nonce/state rather than service or password token grants.
- Provider proof must preserve principal kind, expiry and current Resource authorization.

## Task1: Pure snapshot

- [x] Add failing public API/eligibility/size/private-field/canonical integer tests and signed synthetic-token integration.
- [x] Preserve missing API RED and implement named modules with façade exports and stable configure paths.
- [x] Verify pure snapshot tests, existing presets, Clippy, docs and independent consumer graph.

## Task2: Native login and integration

- [x] Qualify controlled Keycloak TLS authorization-code/PKCE, bounded acquisition and one-use host flow.
- [x] Run independent source/archive consumers on actual ID tokens, SQLite/redb authorization, rotation and native restart.
- [x] Verify dependencies, audits, source review, affected gates and full verifier with unchanged source.
- [x] Document actual verified profiles and remaining gaps; prepare authorized publication.

Commit and publication results are recorded in the private progress ledger after execution.
