# Next family increment research

Inspection date: 2026-10-10 UTC.
Scope: read-only source review and primary-source web research. No native experiment was executed for this note.
ROM-extras baseline: `53490c36624cd5b05349f267703f042b3387078c`.
Public ROM boundary: `d7ef529040eec60dc869034c2d33130219db85fe`.
Only this research file was written. Historical fixtures, prototypes, worktrees and evidence remain retained.

## Recommendation

Implement a pure bounded issuer-bound OIDC JWKS snapshot and qualify actual local Keycloak authorization-code/PKCE login next.
Keep network acquisition, discovery approval, state, nonce, code redemption and sessions host-owned.
This closes the gap between synthetic presets and an actual identity provider without changing public ROM or projection semantics.
Prefer an added module in `rom-oidc-presets`; do not add a new HTTP abstraction or Rust registry dependency without a demonstrated need.

Cosine remains feasible, but its representation contract introduces new observable semantics and requires explicit profile binding.
An approximate readback check would weaken the current projection contract and is unacceptable.
Treat its section below as a future design alternative, not the selected implementation plan.

## Current evidence and boundaries

The maintained README and `docs/support.md` identify live OIDC login/discovery and Qdrant Cosine as open work.
`rom-qdrant/src/generation.rs` explicitly refuses `Distance::Cosine`.
`page.rs` binds prepared work to a physical generation and original document metadata.
`reconcile.rs` currently requires every retrieved float32 vector component to equal the uploaded component bit-for-bit.
`search.rs` has an existing `VectorSearchTarget` implementation; its distance mapping already names Cosine.

Retained evidence in `docs/research/qdrant-metric-reconciliation.md` reports actual Qdrant 1.19.2 experiments.
The exact arithmetic comparison checked 32 retained native points and found an eligible whole-vector reference for each.
Two 4096-dimensional counterexamples matched AVX only. This is prior experiment evidence, not maintained adapter acceptance.
The authored reference remains at `.superpowers/qdrant-cosine-reference-2026-10-09.rs`.

The pinned public ROM source was inspected in the Cargo checkout, independently of the mutable `/root/ROM` workspace.
`rom-auth/src/oidc.rs` exposes `OidcIdTokenAdapter` and `OidcTokenBindings`.
`rom-auth/src/jwt.rs` exposes synchronous `TrustedKeys::fetch`, with host-owned issuer binding, acquisition bounds and duplicate-key rejection.
There is no discovery/JWKS network API in that public auth boundary.
An extras acquisition adapter can implement `TrustedKeys` without changing public ROM.
[Pinned public auth source](https://github.com/pmikstacki/ROM/tree/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-auth/src).

The workspace declares Rust 1.99, edition 2024 and MIT licensing.
Its existing `rom-oidc-presets` dependencies include URL 2.5.8 and ROM's `oidc` feature.
All proposed work must preserve existing public API paths and pass affected gates plus the full local verifier.

## Future Cosine alternative

1. Admit Cosine generations with an explicit versioned representation marker. Preserve existing non-Cosine marker bytes and behavior.
2. Preserve the original approved document digest and vector identity. Do not replace them with the native normalized vector.
3. Prepare a stable finite float32 unit vector using float64 norm arithmetic for each nonzero original vector.
   Define zero-vector behavior explicitly; retaining zero with zero similarity is a feasible bounded profile.
4. Use the same preparation rule for Cosine queries. This avoids large finite raw-query overflow and aligns write/query representation.
5. For exact readback, compute eligible tagged Qdrant preprocessing results from the submitted vector.
   Require the actual vector to equal one complete candidate, rather than mixing components from different candidates.
6. Keep exact payload, original key, revision, tombstone, generation, bounded request and checkpoint checks unchanged in strength.
7. Reject an unknown normalization profile before dispatch. Version admission must be explicit; later Qdrant releases need qualification.
8. Continue to reject nonfinite query scores. Do not present transport success as mathematical ranking qualification.

This is a proposed representation contract. The implementer must resolve signed zero, rounding boundaries and version admission in tests.
Do not claim float64 host normalization bypasses Qdrant normalization: the retained 4096D experiment disproves that assumption.
Do not use approximate component tolerance as the reconciliation proof.

### Primary basis

Qdrant's tagged scalar metric dispatch selects CPU implementations and preprocesses Cosine vectors.
[Scalar and dispatch implementation, v1.19.2](https://raw.githubusercontent.com/qdrant/qdrant/v1.19.2/lib/segment/src/spaces/simple.rs).
The AVX path uses fused accumulation and reciprocal multiplication.
[AVX implementation, v1.19.2](https://raw.githubusercontent.com/qdrant/qdrant/v1.19.2/lib/segment/src/spaces/simple_avx.rs).
The SSE path uses separate multiplication/addition and division by the computed length.
[SSE implementation, v1.19.2](https://raw.githubusercontent.com/qdrant/qdrant/v1.19.2/lib/segment/src/spaces/simple_sse.rs).
The NEON path has a separate arithmetic sequence.
[NEON implementation, v1.19.2](https://raw.githubusercontent.com/qdrant/qdrant/v1.19.2/lib/segment/src/spaces/simple_neon.rs).
The tagged preprocessing helper supplies normalization threshold behavior.
[Threshold implementation, v1.19.2](https://raw.githubusercontent.com/qdrant/qdrant/v1.19.2/lib/segment/src/spaces/tools.rs).
Collections explicitly select vector distance and datatype.
[Official collection contract](https://qdrant.tech/documentation/manage-data/collections/).

Qdrant 1.19.2 is Apache-2.0 licensed.
If implementation derives arithmetic code from upstream, preserve attribution and applicable license/notice obligations.
[Tagged license](https://github.com/qdrant/qdrant/blob/v1.19.2/LICENSE).
Keep safe Rust arithmetic within the repository's `unsafe_code = forbid` rule.
The existing reference uses `f32::mul_add` to model fused rounding.
[Rust primitive contract](https://doc.rust-lang.org/std/primitive.f32.html#method.mul_add).

### Native qualification

Reuse the preserved Qdrant 1.19.2 TLS/JWT fixture at explicit loopback port 55471.
Use fresh collection names and nonces; keep all collections and private fixture configuration retained.
The adapter uses REST over verified HTTPS with collection-scoped `prw` credentials.
Administrator provisioning remains outside the writer; browser code and embedding generation remain host responsibilities.

Qualify dimensions 1, 3, 15, 16, 17, 31, 32, 33 and 4096 with zero, non-unit, subnormal and maximum finite vectors.
Include dominant-component cases that expose CPU accumulation order.
Verify exact native retrieval, equal-revision retry, changed original identity, payload/vector corruption and tombstone/resurrection.
Prove finite expected query ordering for authored basis vectors and normalized extreme values.
Run through current ROM authorization/hydration on both SQLite and redb.
Repeat direct and packaged consumer execution.
Lose a completed native write response, restart the same retained service volume, inspect before replay and recover pending checkpoint intent.
Run existing Dot/Euclid/Manhattan regression gates, affected checks and `scripts/check-all`.

Keep actual service evidence separate from controlled 429, timeout, redirect, oversized-body and malformed-response cases.
Native ARM, other releases, quantized vectors and distributed Qdrant remain unqualified unless separately executed.
The copied-marker collection recreation limit remains unchanged; no REST metadata marker is an immutable collection identity.

## Selected OIDC increment: actual Keycloak profile

Select Keycloak 26.8.0 as a pinned local native fixture, not an external demo service.
The official release supplies a 166 MB archive and records SHA-256 `9e41da899f838a58cd510fc98ed4f7cadc715aed5683e42aca20a0c9a2a3980a` for its tarball.
[Official release](https://github.com/keycloak/keycloak/releases/tag/26.8.0).
Its current OpenJDK installation guide requires OpenJDK 25.
[Official native installation](https://www.keycloak.org/getting-started/getting-started-zip).
Keycloak is Apache-2.0 licensed.
[Tagged license](https://github.com/keycloak/keycloak/blob/26.8.0/LICENSE.txt).

Use one local realm, local synthetic user and public client with authorization code and S256 PKCE.
Give the host exact HTTPS issuer, explicit approved authorization/token/JWKS endpoints and private CA material.
Keycloak supplies realm-specific discovery, token and public certificate endpoints.
[Official OIDC endpoints](https://www.keycloak.org/securing-apps/oidc-layers).
Configure native PEM TLS and an explicit free loopback HTTPS port, for example 55473 after checking availability.
Disable exposed plaintext HTTP. Default HTTPS is 8443; the documented port option permits isolation.
[Official TLS configuration](https://www.keycloak.org/server/enabletls).
Use persistent `dev-file` only as a local qualification profile, not production database acceptance.
[Official database configuration](https://www.keycloak.org/server/all-config).

Implement a pure bounded issuer-bound JWKS snapshot in extras. The host performs the bounded HTTPS discovery and JWKS acquisition.
Verify discovery issuer exact equality before trusting metadata; never follow token-supplied key URLs.
[OIDC Discovery specification](https://openid.net/specs/openid-connect-discovery-1_0.html).
In the host consumer, admit only approved HTTPS origins, explicit CA, deadlines and bounded bodies; reject redirects.
The pure parser rejects duplicate key IDs and duplicate critical JSON fields before constructing a key map.
Support eligible RS256 public signing keys while rejecting private key material and inappropriate key uses.
Do not reuse or weaken projection HTTP for identity acquisition. Keep the pure parser independent of transport.
Feed a complete admitted key set through the existing public `TrustedKeys` boundary.
Avoid blocking an asynchronous runtime with synchronous HTTP. An explicitly refreshed bounded snapshot source satisfies `TrustedKeys` synchronously.
Bind a snapshot to the exact configured issuer and reject use with another preset before adapter construction.
Enforce the public ROM limits: at most eight eligible keys and a 64-byte nonempty key ID.
Bound JSON input bytes, nesting, total keys, RSA modulus/exponent lengths and decoded key material before allocation where possible.
Reject private RSA fields; admit public RSA verification keys with suitable `use`, `key_ops` and RS256 algorithm metadata.
Define mixed-algorithm native JWKS handling explicitly: Keycloak can advertise non-RS256 keys, which need safe filtering rather than accidental rejection.
[JWK/JWKS fields](https://www.rfc-editor.org/rfc/rfc7517.html), [RSA public and private parameters](https://www.rfc-editor.org/rfc/rfc7518.html#section-6.3.1).
Prefer `IssuerPreset::configure_jwks(authority, client_id, bytes)` returning a verifier with an immutable host-pinned key snapshot.
Rotation requires a new host-approved snapshot and a new verifier; parser failure must not replace a working snapshot implicitly.
Document that ROM cache refresh returns the same pinned keys. It does not refresh provider state or establish key revocation.
The verifier's thirty-second evidence/cache limits do not imply thirty-second provider-key revocation.

Proposed parser profile: at most 64 KiB JSON, 32 total keys and eight eligible RS256 keys.
Require canonical unpadded base64url unsigned-minimal `n`/`e`, RSA modulus 2048..8192 bits and odd exponent 3..=u32::MAX.
RFC 7518 requires at least 2048 bits for RS256; the maximum modulus and exponent bounds are explicit local profile choices.
Require `alg=RS256`, `use=sig` or absent, and `key_ops=[verify]` or absent for admitted keys.
Ignore unknown extension members as RFC 7517 requires. Reject duplicate known fields and globally duplicate nonempty key IDs.
The duplicate-ID rule is a stricter ROM profile: RFC 7517 only recommends distinct IDs and permits equivalent alternatives.
Reject private RSA fields `d`, `p`, `q`, `dp`, `dq`, `qi` and `oth`, including explicit null values.
Reject malformed selected RS256 keys; safely skip unsupported public algorithms and encryption-only entries within total bounds.
The exact native Keycloak JWKS contents must be observed rather than inferred from default configuration.

Native acceptance must obtain an actual code through a local browser login and redeem it with the retained PKCE verifier.
Use random one-use state and retained nonce, exact registered redirect URI, `response_type=code`, `scope=openid` and S256 challenge.
Validate returned state before redemption; consume each login attempt once even when later validation fails.
[OIDC authorization-code/state/nonce contract](https://openid.net/specs/openid-connect-core-1_0.html#CodeFlowAuth).
Send the original verifier to the token endpoint; a wrong verifier must fail natively.
[PKCE S256 contract](https://www.rfc-editor.org/rfc/rfc7636.html).
Verify the real ID token using `IssuerPreset`, public ROM adapter, retained nonce and real provider JWKS.
Qualify wrong nonce/audience/issuer, one-use state, code replay, PKCE failure and native key rotation across service restart.
Exercise public and packaged consumers. Keep Resource authorization and session policy in the host.
Do not replace native login with password grant, client credentials or synthetic signing.
Entra and Auth0 profiles remain unavailable without external accounts; source review does not accept them.

Dex 2.46.0 is a smaller conceptual alternative with static local users and persistent storage.
[Local connector](https://dexidp.io/docs/connectors/local/), [release](https://github.com/dexidp/dex/releases/tag/v2.46.0).
However, its release view offers source archives rather than a native binary; compiling expands dependency footprint.
The tagged module requires Go 1.27.0 and is Apache-2.0 licensed.
[Go module](https://raw.githubusercontent.com/dexidp/dex/v2.46.0/go.mod), [license](https://raw.githubusercontent.com/dexidp/dex/v2.46.0/LICENSE).
Do not infer native binary availability or disk savings without measuring a pinned acquisition route.

## Other alternatives

| Candidate | Implementable without external credentials | Native acceptance and reason for order |
| --- | --- | --- |
| Pure JWKS snapshot and real Keycloak login | Yes, local native archive and user | Preferred: moves OIDC beyond synthetic verification while leaving network and login policy with the host. |
| Cosine target and queries | Yes, existing native fixture | Later: requires explicitly bound representation semantics and exact whole-vector readback; tolerance is insufficient. |
| Generic generation switching | Yes, existing search services | Larger cross-provider lifecycle contract. Specify durable catch-up, current authorization and crash-safe cutover before implementation. |
| SMS provider connector | Protocol implementation only | No actual provider acceptance with current credentials. Test simulations do not establish SMS delivery. |

Qdrant documents atomic alias operations, but current adapters deliberately bind concrete physical generations.
An alias alone does not prove ROM checkpoint catch-up, pending-intent safety or authorization continuity.
[Official collection aliases](https://qdrant.tech/documentation/manage-data/collections/#collection-aliases).
Do not present a provider alias switch as completed generic generation switching.

Twilio's current test credential contract requires account credentials and simulates operations without contacting real phone numbers.
It does not emit SMS status callbacks; new-console accounts cannot create legacy test credentials.
Therefore no-credential local emulation can qualify protocol handling only.
[Official test credential limitations](https://www.twilio.com/docs/iam/test-credentials).
Keep SMS and other notification connectors in the full goal even when actual backend access is unavailable.

## Resource limits and remaining decisions

Read-only `df -h /root/ROM-extras` reported 5.3 GiB available during this research, below the earlier 7.6 GiB task snapshot.
Disk availability is changing. Recheck before provisioning or building; preserve all historical artifacts.
For Cosine, reserve a bounded small set of new collections and reuse the current build target.
For Keycloak, measure archive, extracted distribution, JDK and database footprint before starting.
Set an explicit heap limit and controlled cache sizes; do not claim a successful memory cap until native login has passed under it.
No Docker provisioning or service launch was performed by this research agent.

Unresolved Cosine choices: marker/version admission, exact eligible arithmetic modes, zero/signed-zero behavior and representation binding.
Unresolved OIDC choices: final snapshot API naming, strict mixed-key parsing tests, PKCE/browser host integration and native key rotation commands.
Both increments require local tests and actual backend evidence before support status can change.

## Implementation follow-up

The preceding recommendation was source research, not executed qualification.
Root subsequently qualified the official amd64 container pinned to digest `d79bc4bf1c54e802735ef91926b5c003de1fbb50b1a93382611972277219c9ad`.
The actual image reports Red Hat OpenJDK 21.0.12.1; no host JDK was installed.
The [tagged container build](https://raw.githubusercontent.com/keycloak/keycloak/26.8.0/quarkus/container/Dockerfile) uses `java-21-openjdk-headless`.
The native distribution guide's current JDK recommendation is not the pinned container's runtime evidence.
Keycloak's [tagged license](https://github.com/keycloak/keycloak/blob/26.8.0/LICENSE.txt) is Apache-2.0.

The local host profile uses literal loopback IP addresses to avoid DNS acquisition.
An absolute deadline timer shuts down the held socket during headers and body parsing.
The [Python socket contract](https://docs.python.org/3/library/socket.html#socket.socket.shutdown) supplies shutdown semantics.
The [Timer contract](https://docs.python.org/3/library/threading.html#timer-objects) supplies scheduling and cancellation.
Per-read socket timeouts alone do not bound a complete [HTTP header read](https://docs.python.org/3/library/http.client.html#http.client.HTTPConnection).
Separate controlled TLS regressions prove interruption of trickled response and chunk headers; these are not Keycloak fault experiments.
