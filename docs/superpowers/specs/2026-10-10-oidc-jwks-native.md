# Bounded OIDC keys and native login

This implements the approved OIDC family within the complete ROM-extras goal.
Public ROM stays pinned to d7ef529040eec60dc869034c2d33130219db85fe.
Root owns maintained files. Research and review agents own private reports only.

## Intended behavior

Add `IssuerPreset::configure_jwks(authority, client_id, bytes)` returning the existing ROM verifier with an immutable `JwksKeys` source.
The host approves issuer-bound JWKS bytes. This operation performs no HTTP, discovery, key URL resolution or token verification itself.
Keep existing `configure`, exact issuers and public paths stable. ROM retains signature, claim, nonce and proof-expiry checks.
Host discovery, bounded HTTPS acquisition, clock, single-use state/code/nonce, sessions and current authorization remain separate obligations.

Limit JSON to65536 bytes,32 total entries,8 selected keys,64-byte exact key IDs and RSA moduli2048..8192 bits.
Reject empty sets, duplicate IDs before algorithm filtering, duplicate recognized JSON fields and invalid selected keys.
Require selected keys to declare `kty=RSA`, `alg=RS256`, use absent or sig, and operations absent or exactly verify.
Skip legitimate unsupported key types/algorithms and encryption keys; never use them for signatures.
Ignore extension parameters without dereferencing certificates or URLs. Reject private RSA parameters and symmetric secret material.
RSA n/e use canonical unpadded base64url minimal unsigned integers. Reject leading zeros, invalid encodings, even moduli and exponents outside odd3..u32MAX.
Optional recognized parameters, when present, must not be null. Errors and Debug omit raw keys, IDs, certificates and response text.
No new registry package or second signature implementation is needed.

The source is an immutable host-approved snapshot, not an online refresh adapter.
Rotation requires replacing the snapshot and verifier. Repeated fetch from the same snapshot cannot establish provider revocation.
Do not infer30-second provider revocation from ROM's30-second key-cache interval.

## Native qualification

Use an explicit isolated local Keycloak26.8.0 fixture with trusted TLS, persistent data and bounded resources.
No public server or paid account is required. Do not provision Nominatim or production identity infrastructure.
Exercise actual authorization-code/S256-PKCE login, nonce, state, bounded discovery/JWKS and one-use code redemption.
Pass actual ID tokens and public JWKS to independent Rust source and normalized archive consumers.
Check exact issuer/client/nonce binding, human identity, wrong inputs, native key rotation/restart and current ROM authorization on SQLite/redb.
Record mock/synthetic cases separately from actual native login. Do not claim Entra, Dex, Auth0 or production session qualification.

## Acceptance

Preserve failing missing API and behavioral RED tests. Cover bounds, key eligibility, private fields, ambiguity, encoding and actual synthetic signatures.
Run native consumers, dependency integrity/audits, final review, affected checks and the full local verifier on frozen source before integration.
Preserve all fixture data, archives, prototypes, worktrees and evidence. Record unresolved profiles rather than declaring the entire family complete.
