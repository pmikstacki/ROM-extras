# KMS migration protocol and bounded execution

Inspection date: 2026-10-10 UTC. Reviewed official OpenBao `v2.7.1` source afresh and existing maintained modules.
Reviewed `.superpowers/next-after-cosine-2026-10-10.md`.
No native calls, builds, container operations, fixture changes or maintained edits occurred.
Research source inspection is separate from the executed qualification record.

## Version selection and integer bounds

Single-item encrypt declares `key_version` as framework `TypeInt` and reads it as Go `int`.
It passes the requested value to the encryption policy.
The response contains both `data.ciphertext` and `data.key_version`.
For requested zero, the response selects the policy's current LatestVersion; otherwise it returns the requested value.
The policy remains locked during the operation.
The endpoint has separate Create/Update operations, so runtime permissions must continue to exclude key creation.
[Tagged encrypt source](https://raw.githubusercontent.com/openbao/openbao/v2.7.1/internal/builtin/logical/transit/path_encrypt.go).

The framework decodes TypeInt with mapstructure WeakDecode into a Go int.
It is not a published unsigned 64-bit version representation.
[Tagged field decoder](https://raw.githubusercontent.com/openbao/openbao/v2.7.1/sdk/framework/field_data.go).
The Go specification permits int to be 32 or 64 bits.
Do not derive a portable remote bound from the client's Rust usize or u64 type.
[Go integer types](https://go.dev/ref/spec#Numeric_types).

Recommend a distinct `Latest` selector and validated `Pinned` selector with range `1..=i32::MAX` for this provider profile.
This portable host limit is intentionally smaller than a 64-bit server's possible int range.
Encode Latest as integer zero or omit the field; encode Pinned as an exact JSON integer.
Reject zero-as-Pinned, values above the declared bound, and invalid selector construction before outbound I/O.
Do not truncate, cast with wrapping, accept fractional JSON or silently substitute latest.
A 64-bit-only profile could instead admit through i64::MAX, but it must declare that architecture assumption.

The policy chooses latest for zero and rejects negative, future and below-minimum-encryption versions.
Existing-version availability remains a provider check, not a property established by validating the selector.
Derived keys use the context bytes in key derivation.
The associated-data factory supplies AAD to AEAD Seal/Open.
[Tagged key policy](https://raw.githubusercontent.com/openbao/openbao/v2.7.1/sdk/helper/keysutil/policy.go).

Validate response version as a positive integer within the selected provider bound.
Require equality between that field and the ciphertext prefix version.
For Pinned, also require equality with the requested version.
Apply the response-field consistency check to Latest too, rather than maintaining weaker latest parsing.
Treat missing, negative, fractional, overflow or contradictory values as fixed Protocol errors.
Envelope.version remains the actual encryption version, never zero or a requested-latest marker.
Do not promise Latest is still latest after the response: later administrative rotation can change it.

## Context and AAD preservation

Encrypt and decrypt decode context from standard Base64.
The decrypt request separately carries associated_data into an associated-data factory when nonempty.
Provider authentication rejects mismatched AAD; do not convert rejection into empty plaintext.
[Tagged decrypt source](https://raw.githubusercontent.com/openbao/openbao/v2.7.1/internal/builtin/logical/transit/path_decrypt.go).

The existing Binding requires nonempty context and permits empty AAD.
Each component has a 4096-byte cap and stays a distinct byte sequence.
Existing request code uses standard Base64 for both fields.
Preserve those exact bytes through migration; do not concatenate, canonicalize or interpret them as strings.
Empty AAD encodes as an empty string, and server factories are omitted in that case.
That does not weaken a nonempty-AAD request: its exact bytes must still be supplied.

Native rewrap is unsuitable for this migration profile.
Its schema omits associated_data, and it calls policy Decrypt and Encrypt without an AAD factory.
Do not describe decrypt+encrypt migration as plaintext-free server rewrap.
[Tagged rewrap source](https://raw.githubusercontent.com/openbao/openbao/v2.7.1/internal/builtin/logical/transit/path_rewrap.go).

## Optional generic capability

Preserve existing Kms::encrypt latest behavior, decrypt behavior and public import paths.
Prefer a separate optional versioned-encryption trait extending Kms.
Its explicit selector operation must either honor the selector or return Unsupported.
A default operation on Kms can also preserve existing implementers, but Pinned must default to Unsupported, never latest.
There is no need to add a general SDK or expose Transit URLs to application code.

The migration API owns orchestration: authenticate source, retain one bounded SecretBytes, encrypt destination, validate replacement, return Envelope.
Use one unchanged Binding for the first profile, with an approved destination KeyRef and selector.
The caller receives no plaintext migration result.
The concrete OpenBao adapter must validate its destination alias and selector before decrypting the source.
If the generic layer promises this ordering for all providers, expose a synchronous preflight contract in the optional capability.
Do not claim a generic helper can inspect another provider's private alias map without such a contract.
Local preflight does not prove live provider authorization or key availability.

Check replacement profile/key/version against the declared target contract.
Leave the source envelope unchanged on every path.
Host persistence, conditional replacement and retirement of old ciphertext stay separate.
There is no distributed transaction, automatic retry, migration receipt, or implicit key-floor change.
Repeated explicit migration can generate distinct randomized ciphertext for equivalent authenticated bytes.

## Admission, deadline and cancellation contract

Use a whole-migration admission gate with try-acquire, acquired before provider I/O.
Hold that permit through source authentication, plaintext lifetime, destination encryption and response validation.
Refuse excess calls with Busy and no waiting queue.
The existing HTTP semaphore limits one HTTP call at a time, not all plaintext held between the two calls.
One migration instance's bound applies only to calls through that instance; constructing many wrappers is a host resource-policy concern.

Tokio try-acquire refuses unavailable permits immediately, and dropping a permit releases it.
[Tagged semaphore implementation](https://raw.githubusercontent.com/tokio-rs/tokio/tokio-1.53.1/tokio/src/sync/semaphore.rs).

Use one absolute deadline for the complete admitted operation; do not reset it before encryption.
Wrap the complete future with an outer timeout.
Check the deadline after source authentication, before starting destination encryption, and before returning the replacement.
Keep the existing per-request deadline as an additional bound.
Bound synchronous parsing/encoding work with existing byte limits.

The exact Tokio 1.53.1 implementation polls the inner future before its timer.
A non-yielding inner future can exceed the timeout and still return success.
Dropping the timeout future cancels local polling and drops captured values.
Document the cooperative scheduling requirement and active Tokio time driver.
Do not claim hard preemption of blocking providers or arbitrary CPU work.
[Tagged timeout implementation](https://raw.githubusercontent.com/tokio-rs/tokio/tokio-1.53.1/tokio/src/time/timeout.rs).

Do not spawn detached migration work.
Caller cancellation must drop its captured SecretBytes and admission permit, without starting a subsequent phase locally.
Cancellation during an HTTP request does not establish remote cancellation or rollback.
SecretBytes already owns Zeroizing memory; HTTP/parser/host copies remain outside that erasure guarantee.
Timeout or encryption response loss returns no replacement and leaves original host persistence untouched.

No new registry package is required. If rom-kms owns the execution gate, reuse exact existing Tokio 1.53.1 time/sync features.
That adds a dependency edge to rom-kms; independently assess its manifest and archive consumer graph.
Alternatively, keep the abstract capability in rom-kms and bounded execution in an existing host adapter module.
Avoid moving HTTP implementation into the generic contract.
Tokio's tagged manifest declares MIT and Rust 1.71; the extras workspace declares MIT and Rust 1.99.
OpenBao source remains MPL-2.0.
[Tokio manifest](https://raw.githubusercontent.com/tokio-rs/tokio/tokio-1.53.1/tokio/Cargo.toml),
[OpenBao license](https://raw.githubusercontent.com/openbao/openbao/v2.7.1/LICENSE).

## Focused proof required before acceptance

These are proposed tests. This research executed none of them.

1. Create two uniquely named derived nonexportable AES-256-GCM keys using separate fixture administration.
2. Encrypt source A at version 1 with binary plaintext, nonempty binary context and nonempty binary AAD.
3. Rotate A twice. Prove Latest encrypt selects version 3, then explicit version 2 encrypt selects exactly 2.
4. Migrate the stored version-1 envelope to A/version2. Authenticate equal bytes and reject changed context or AAD.
5. Rotate B through version 3. Migrate A's source to approved B/version2 with identical Binding; check destination key identity/version.
6. Try decrypting B's replacement with altered AAD and context separately. Both must reject rather than expose bytes.
7. Raise B's encryption floor to 3; requesting 2 must reject without silently switching to 3. Test absent/future versions separately.
8. Exercise invalid selector and unapproved target preflight with outbound request counts. Verify failure precedes source decryption.
9. Use controlled generic provider futures to hold a decrypted buffer while encryption is pending. Check Busy, timeout and cancellation release capacity.
10. Check no destination invocation after the deadline or source authentication failure. Do not infer remote cancellation from a dropped future.
11. Repeat version2-after-latest3 and cross-alias migration through extracted Cargo archives in an independent workspace.
12. Retain source and replacement ciphertext evidence, then verify replacement through a fresh client after existing same-volume restart/unseal qualification.

Reuse the owned loopback TLS OpenBao 2.7.1 fixture, existing bounded transport and source/package gates.
Do not reset mounts, delete historical keys, weaken TLS or introduce a second service image.
Native service failures and package failures must fail the required gates.
Retain logs and version/configuration/image identity evidence without tokens or plaintext markers in public output.
This increment does not qualify another provider, full SQL Storage, replicated custody or the entire ROM-extras goal.
