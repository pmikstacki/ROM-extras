# Secrets and KMS assessment

Inspection date: 2026-10-08. Evidence category: source review only.

This report proposes the first real secrets/KMS profile under approved Task 7. No service was started or downloaded. No Rust dependency was added. OpenBao, Vault, AWS, and Azure conformance remains unexecuted. The complete family remains open.

## Published ROM boundary

Only published ROM commit `d7ef529040eec60dc869034c2d33130219db85fe` was inspected, through its immutable Cargo checkout. Private ROM working-tree code was not used.

`IdentityProvider.credential_ref` is an optional host secret-store reference. Resolution and rotation belong to the host. `rom-config` explicitly does not resolve secrets or activate provider sessions. Its guidance excludes credentials from input fields, provenance labels, and error context. These contracts permit extras-owned host adapters without a new Resource containing plaintext. [Published identity fields](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-identity/src/resources.rs), [published configuration contract](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-config/README.md).

The demo has an approved opaque reference map to immutable secret files, bounded reads, and redacted errors. It is demo code, not a public reusable resolver trait. No general secret resolver or KMS trait was found in the inspected public crates. Reuse the reference-only domain model; define the missing host contract in ROM-extras. Do not copy the demo implementation into another production package. [Published demo](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/demo/src/provider_profile/secrets.rs), [approved plan](../superpowers/plans/2026-10-08-rom-extras.md).

## First provider and alternatives

Recommend OpenBao `2.7.1` for the first local profile. The official release API reports a non-prerelease published on 2026-10-01. Its tag resolves to `a5db72cef75c24b920ade02065b18dd8eb666bac`. The source license is MPL-2.0. Official registries include `ghcr.io/openbao/openbao`, `quay.io/openbao/openbao`, and `docker.io/openbao/openbao`. No image digest or pull was verified. Pin the actual platform digest during fixture preparation. [Exact release](https://github.com/openbao/openbao/releases/tag/v2.7.1), [tag API](https://api.github.com/repos/openbao/openbao/git/ref/tags/v2.7.1), [license](https://github.com/openbao/openbao/blob/v2.7.1/LICENSE), [official installation](https://openbao.org/docs/install/).

Vault remains a second real profile, not a synonym for tested OpenBao behavior. The official latest release API returned Vault `2.1.2`, published on 2026-10-07. Its tagged license uses BSL terms and a restricted additional production-use grant. It is not the same license as OpenBao. Vault exposes KV v2 and Transit, but protocol similarity does not prove complete compatibility. Keep a separate service/version acceptance matrix. [Vault release](https://github.com/hashicorp/vault/releases/tag/v2.1.2), [tagged license](https://github.com/hashicorp/vault/blob/v2.1.2/LICENSE), [KV v2](https://developer.hashicorp.com/vault/api-docs/secret/kv/kv-v2), [Transit](https://developer.hashicorp.com/vault/api-docs/secret/transit).

Prefer a small HTTP adapter over adding an unassessed third-party Vault client. The required operations are bounded KV reads and Transit encrypt/decrypt. Administrative provisioning remains outside the runtime adapter. This choice follows the documented HTTP endpoints, not an assumption that a Rust SDK is mandatory. [Tagged HTTP API](https://github.com/openbao/openbao/blob/v2.7.1/website/content/docs/api/index.mdx).

## Secret reference and resolution contract

OpenBao KV v2 reads use `/v1/{mount}/data/{path}?version={number}`. The body contains nested `data.data` and version metadata. Omitting the version selects latest. Writes create versions; CAS is an administrative write constraint. Deleted or destroyed versions must not become an empty credential. Custom metadata can accompany reads even without permission to its metadata endpoint. Discard unneeded metadata. [Versioned API](https://openbao.org/docs/api/secret/kv/kv-v2/), [exact tagged API source](https://github.com/openbao/openbao/blob/v2.7.1/website/content/docs/api/secret/kv/kv-v2.mdx).

Propose a typed, opaque `SecretRef` resolved through a host-approved map. Separate endpoint, namespace, mount, path segments, field selector, and version selector. Do not interpret arbitrary Resource text as an HTTP URL. Reject unknown aliases, empty segments, traversal segments, malformed versions, control characters, fragments, and ambiguous percent encoding before I/O. Construct URLs from validated segments. These restrictions are proposed host policy; the server API does not enforce this exact grammar.

Expose explicit `Latest` and `Pinned(version)` selection. Return the actual version with a secret buffer. Do not silently replace a missing pinned version with latest. Initially perform each resolution without a plaintext cache. A later cache needs an expiry and revocation contract. Rotation changes latest; pinned references remain stable only while their version is retained. [KV retention and version selection](https://openbao.org/docs/api/secret/kv/kv-v2/).

Propose a 2,048-byte reference cap, a 4,096-byte decoded secret cap, and a 32 KiB HTTP body cap. Bound the complete streamed body before JSON parsing, including error responses. Limit concurrent calls and queued captured bytes. Do not trust Content-Length alone. These are initial family limits, not provider maxima. The 4,096-byte secret limit matches the published introspection adapter's existing maximum credential length. [Published introspection validation](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-auth/src/introspection.rs).

## KMS semantics

Provision a non-exportable Transit AEAD key outside the runtime adapter. Keep plaintext backup disabled. Use server-generated nonces and disable convergent encryption. Encryption returns provider ciphertext carrying a key version. Rotation changes the encryption version; older ciphertext remains decryptable until version policy excludes it. Key rotation does not itself rewrite application ciphertext. [Transit API](https://openbao.org/docs/api/secret/transit/).

Keep derivation context distinct from authenticated associated data. Transit `context` is base64 input for derived keys. `associated_data` is base64 AAD for supported AEAD ciphers. Recommend an initial derived `aes256-gcm96` profile with host-approved context and authenticated metadata. The docs' AAD algorithm spelling differs from the key-type spelling; qualify the exact profile against 2.7.1 rather than trusting that wording. [Tagged API](https://github.com/openbao/openbao/blob/v2.7.1/website/content/docs/api/secret/transit.mdx), [tagged request fields](https://github.com/openbao/openbao/blob/v2.7.1/internal/builtin/logical/transit/path_encrypt.go).

Propose separate bounded `KeyRef`, plaintext bytes, derivation context bytes, and AAD bytes. Return an envelope with provider profile, immutable key identity, algorithm, and ciphertext. Do not store plaintext or authentication credentials in the envelope. Canonicalize non-secret context before encryption. Reject duplicate or ambiguous context fields. Apply a 4,096-byte plaintext limit and bounded encoded ciphertext/context limits before dispatch. These limits and envelope fields require the family spec; they are not existing ROM APIs.

An incorrect derivation context or AAD must fail decryption. It must never return unchecked bytes. A denied call, invalid ciphertext, unsupported algorithm, size limit, malformed response, unavailable service, and timeout need fixed error categories. Do not forward server text, request bodies, URLs containing private paths, or HTTP client Debug output. A transport failure can mean a request reached the provider; it does not imply a ROM commit. Do not automatically create keys through encrypt upsert. Restrict the token to existing approved keys. [Transit endpoint and upsert behavior](https://github.com/openbao/openbao/blob/v2.7.1/website/content/docs/api/secret/transit.mdx).

## Transport, authorization, and memory

Use verified HTTPS with explicit endpoint approval. Disable redirects and ambient proxies in the first adapter. Give each call a connect deadline and total deadline. Disable automatic adapter retries initially. Enforce a concurrent-call limit before request construction. Host cancellation drops local work but does not establish remote cancellation.

OpenBao's listener defaults to TLS. It exposes server-side request size and duration controls, but these do not replace client response bounds. Configure a private fixture CA and verify the endpoint name. Do not disable certificate verification for acceptance tests. Keep the cluster listener unpublished. [Listener configuration](https://openbao.org/docs/configuration/listener/tcp/).

Use a limited token that can read only approved KV data paths and invoke encrypt/decrypt for one existing key. A setup token separately enables engines, writes versions, creates keys, and rotates them. No runtime root token, key export, mount administration, or generic list capability is needed. Namespace belongs to approved host configuration. OpenBao documents 403 for authentication/permission failures, 404 that can conceal a forbidden path, and 503 for sealed/unavailable service. Therefore, 404 cannot always prove that a secret never existed. [Policies](https://openbao.org/docs/concepts/policies/), [tagged HTTP statuses](https://github.com/openbao/openbao/blob/v2.7.1/website/content/docs/api/index.mdx).

Secret buffers should not implement plaintext Debug or Serialize. Controlled explicit exposure is necessary to configure a host client. `zeroize 1.9.1` is already resolved in the repository; its manifest declares Rust 1.85 and MIT/Apache-2.0. It clears owned memory but cannot erase copies in HTTP, JSON, operating-system, or host client buffers. Do not claim complete plaintext erasure. Bound the unavoidable copies and their lifetimes. [Exact zeroize manifest](https://github.com/RustCrypto/utils/blob/zeroize-v1.9.1/zeroize/Cargo.toml).

`reqwest 0.13.5` is already resolved. Its tagged manifest declares Rust 1.85 and MIT/Apache-2.0. Defaults include Rustls, HTTP/2, and system proxies. Rustls's default provider includes native aws-lc-rs dependencies. Native TLS instead uses platform TLS and OpenSSL on relevant systems. Choose features explicitly after graph review; do not label either choice dependency-free. No new graph, advisory audit, or MSRV build was performed. [Exact reqwest manifest](https://github.com/seanmonstar/reqwest/blob/v0.13.5/Cargo.toml).

Provider audit devices have their own retention and redaction policy. OpenBao normally HMACs most string values, but configuration can exempt fields. Client redaction cannot prove provider audit redaction. Keep fixture audit output private and inspect sanitized assertions. [Official audit documentation](https://openbao.org/docs/audit/).

## AWS and Azure profiles remain in scope

| Profile | Source-confirmed semantics | Required later evidence |
| --- | --- | --- |
| AWS Secrets Manager | `VersionId` pins; absent selectors use `AWSCURRENT`. Both selectors must agree. SecretString and SecretBinary are distinct. IAM needs GetSecretValue and sometimes KMS Decrypt. | Real IAM denial, rotation, binary values, bounded reads, regional outage behavior. |
| AWS KMS | Symmetric encrypt supports up to 4,096 plaintext bytes. Encryption context must match exactly on decrypt. Context is non-secret and can appear in CloudTrail. Asymmetric keys do not support this context. | Actual key policy, changed context rejection, rotation, encrypted envelope identity, cloud TLS/auth and retries. |
| Azure Key Vault secrets | Values are strings with service limit 25 KB. The family can impose a smaller bound. | Versioned identifiers, RBAC denial, rotation, actual credential authority and verified cloud endpoint. |
| Azure Key Vault keys | Encrypt API accepts algorithm-specific AAD/IV and returns key identity. Algorithm availability depends on key type/profile. RSA does not acquire AEAD semantics by attaching metadata. | Select an actual AEAD-capable service/key profile or separately specify envelope encryption; reject unsupported context capability. |

Sources: [AWS secret reads](https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_GetSecretValue.html), [AWS Encrypt](https://docs.aws.amazon.com/kms/latest/APIReference/API_Encrypt.html), [AWS context](https://docs.aws.amazon.com/kms/latest/developerguide/encrypt_context.html), [Azure secrets](https://learn.microsoft.com/en-us/azure/key-vault/secrets/about-secrets), [Azure Encrypt REST 2025-07-01](https://learn.microsoft.com/en-us/rest/api/keyvault/keys/encrypt/encrypt?view=rest-keyvault-keys-2025-07-01).

Cloud SDK versions, exact features, licenses, native graphs, and MSRV remain unselected. Use first-party SDK source when selecting them. AWS documents async Rust service clients with credential/retry middleware. That middleware requires explicit bounded settings and real-profile tests. OpenBao execution cannot establish AWS or Azure support. A local emulator cannot replace live cloud authorization or KMS behavior. [AWS SDK](https://aws.amazon.com/sdk-for-rust/).

## Recommended next increment and real acceptance

Write the family spec first. Add the host secret-reference resolver and bounded encryption/decryption contracts to `rom-secrets` and `rom-kms`. Implement OpenBao KV v2 and Transit over one internal bounded HTTP transport. Preserve provider-specific semantics behind explicit capability/profile declarations. Do not combine secret resolution with Resource mutation or a background key-rotation action.

Use a real OpenBao 2.7.1 service with private TLS, persistent Raft data, fixed node identity, and explicit initialization/unseal handling. Pin its executed image and configuration. Development mode, including TLS development mode, is in-memory and starts unsealed. It is useful for an initial API probe but cannot prove restart persistence. [Raft storage](https://openbao.org/docs/configuration/storage/raft/), [exact development-mode contract](https://github.com/openbao/openbao/blob/v2.7.1/website/content/docs/commands/server.mdx).

Required first acceptance scenarios:

1. Reject malformed or unapproved references before any outbound request.
2. Resolve latest and pinned values. Rotate KV data and distinguish their results. Reject missing, deleted, and destroyed selected versions.
3. Use a limited token. Verify denied paths, denied key operations, expired/revoked tokens, and no metadata disclosure through errors.
4. Encrypt/decrypt actual bytes. Reject altered context, altered AAD, malformed ciphertext, and oversized inputs before exposure or dispatch as appropriate.
5. Rotate the Transit key. Verify new encryption version and old decryption behavior. Qualify minimum-version rejection separately.
6. Reject wrong CA and endpoint name. Stop or seal the service and verify bounded failure without an empty secret fallback.
7. Restart persistent storage and unseal. Verify retained KV versions and ciphertext recovery through fresh host clients.
8. Scan only controlled capture buffers for secret/plaintext markers. Verify references, never resolved values, enter SQLite/redb Resource values, journal, diagnostics, and provenance.
9. Build the independently packaged public consumer and run required dependency, license, MSRV, and full verifier gates.

The final isolation assertion applies to integration-controlled paths. Arbitrary application code can explicitly copy exposed secret bytes into an action; these packages cannot prevent that universally. Document this boundary and test the provided host activation example.

OpenBao's current release includes security fixes. Inspect its exact release notes and advisories before execution; this source review is not a no-advisory claim. Retain independent future Vault, AWS, and Azure profiles, host credential renewal, envelope migration, and release evidence in the full goal.
