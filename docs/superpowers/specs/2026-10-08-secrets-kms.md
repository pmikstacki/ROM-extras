# Host secrets and KMS family

This increment executes approved Task 7. The whole family remains incomplete until real-service and release gates pass.
[Primary research](../../research/secrets-kms-assessment.md) establishes provider semantics and the public ROM host boundary.

## Intent and ownership

A Resource stores an approved opaque reference, never a resolved credential.
The host owns endpoint approval, provider authentication, reference mappings, activation, renewal, and rotation policy.
`rom-secrets` defines reference, version, owned secret-buffer, safe-error, and resolver contracts.
`rom-kms` defines approved key references, authenticated encryption inputs and ciphertext envelopes.
`rom-openbao` implements both contracts over one bounded internal HTTP transport.
These extras do not introduce a secret-bearing Resource or bypass Runtime authorization.

## Reference and memory contract

Use opaque aliases containing 1..=64 ASCII letters, digits, underscores or hyphens, starting with a letter.
This is host policy; provider paths remain separately configured validated segments.
The adapter resolves only aliases in the host-approved map. Resource text cannot choose endpoints or mount paths.
Version selection is explicit Latest or a positive pinned version. Return the actual version; never substitute latest for a missing pin.
No initial plaintext cache is provided. Each call consults the provider.

Owned secret bytes have a 4,096-byte cap. They implement neither Serialize, Clone, Copy nor plaintext Debug.
Explicit borrowed exposure exists for host activation. Drop clears owned bytes through zeroize 1.9.1.
This does not erase HTTP, parser, operating-system or host copies.
Resolved credentials must be nonempty; authenticated KMS plaintext may be empty.
The packages cannot prevent arbitrary application code from explicitly copying exposed bytes into an action.
Verify the provided host integration paths and state that limit.

## Provider and cryptography contract

First execute OpenBao 2.7.1 KV v2 and derived aes256-gcm96 Transit, selected through [official release and API](https://openbao.org/docs/api/secret/transit/).
Provision mounts, secret versions, keys and limited tokens outside the runtime adapter.
The runtime token reads approved data paths and invokes existing-key encrypt/decrypt; it cannot create, export or rotate keys.
Derivation context and authenticated associated data are separate bounded byte inputs, each at most 4,096 bytes.
Ciphertext envelopes identify provider profile and key alias, with bounded ciphertext. They contain no plaintext or authentication token.
Wrong key/profile, context or AAD must reject decryption. Never return unauthenticated bytes.
Unsupported provider capability fails explicitly; Vault, AWS and Azure require their own real profiles.

## Transport contract

Use verified HTTPS, host-approved endpoint and optional explicit trust CA. Disable redirects and ambient proxies.
Limit concurrent admitted calls before request construction; reject excess calls without an unbounded queue.
Configure connection and complete-operation deadlines; do not automatically retry.
Read at most 32 KiB of response bytes, including error bodies, before parsing JSON. Content-Length alone is insufficient.
Errors are fixed categories. Never attach upstream body, URL, request, credential or plaintext to an error or log.
Cancellation does not establish remote cancellation or a ROM commit outcome.

## Acceptance

Run malformed/unapproved-reference rejection, pinned/latest rotation, denied access, deleted/destroyed version and revoked token cases.
Run actual encrypt/decrypt, changed context/AAD, bad ciphertext, key rotation and minimum-version cases.
Verify response/input bounds, queue rejection, wrong CA/name and service unavailability.
Use persistent Raft with private TLS, not the in-memory development server. Restart and unseal the retained store.
Verify reference-only SQLite/redb state, journal and diagnostic paths through the host activation example.
Run independently packaged consumers, license/advisory/MSRV checks, affected tests, review and full verifier.
