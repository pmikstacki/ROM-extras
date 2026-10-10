# Host secrets and KMS

`rom-secrets` defines bounded opaque references, explicit versions, secret buffers and fixed error categories.
`rom-kms` defines approved key aliases, separate derivation context/AAD, and ciphertext envelopes.
`rom-openbao` shares one bounded HTTPS transport for OpenBao KV v2 and derived aes256-gcm96 Transit.
These packages do not add secret-bearing Resources or a database persistence implementation.

## Host integration

Store an approved alias in the public `IdentityProvider.credential_ref` field.
Keep endpoint approval, TLS trust, tokens, mount paths and key provisioning in private host configuration.
See the executable [preparation example](../examples/secrets-host/activation.rs).
It authorizes a provider read, resolves the alias, and checks the captured activation again.
A provider disable during actual resolution rejects the prepared result on SQLite and redb.

Preparation is not authentication. Configure the verifier from that exact activation before following ROM's verification and binding contracts.
Do not cache a proof under a different provider revision.
The example does not send a resolved credential to a Resource action, journal or diagnostic capture.
The executed native tests check retained state, journal, receipt/error captures and database files for fixture markers and the runtime token.
Applications can explicitly copy borrowed secret bytes. These tests do not establish universal prevention of such copies.

## Bounds and failures

The initial host policy admits at most 256 secret aliases and 256 key aliases.
Alias names contain at most 64 ASCII bytes; provider paths use separately validated segments.
Owned secret material contains at most 4096 bytes.
Context and AAD are separate inputs, each limited to 4096 bytes.
Ciphertext contains at most 16384 bytes.

The default transport admits four concurrent operations, reads at most 32768 response bytes and has a five-second deadline.
Excess admission returns Busy without a queue. Success and error bodies share the streaming response bound.
HTTPS identity verification remains enabled. Ambient proxies, redirects and client retries are disabled.
The adapter returns fixed categories without upstream bodies, URLs, credentials or plaintext.
Secret buffers lack plaintext Debug, Clone and Serialize. Zeroize clears their owned bytes on drop.
HTTP, parser, operating-system and explicit host copies are outside that erasure claim.

Deleted or destroyed KV versions return NotFound; the adapter does not substitute the latest version.
Revoked or expired tokens return Denied; previous successful reads do not supply fallback material.
The runtime Transit token needs only existing-key update permissions, not create, export or rotation permissions.
Missing-key tests verify actual denial and administrative absence before and after encryption.

## Local profile and verification

The isolated profile uses OpenBao2.7.1, a private verified TLS certificate and persistent single-node Raft.
Host port55459 is bound only to loopback.
The 1CPU/512MiB allowance is experimental, not a vendor minimum or production sizing result.
The retained configuration's disable_mlock field is unsupported in this version and has no established effect.
No locked-memory, replicated-quorum or production key-custody claim is made.

Provide private endpoint, CA, runtime token, expected-material and test-only administrative files through the documented fixture variables.
Do not put their contents in Git or command output.
Run the required gate:

```sh
./scripts/check-openbao
```

Service cases run serially because pause and restart controls share the isolated fixture.
Docker commands have separate outer deadlines. A pause guard restores the service before outcome assertions.
After restart, explicit unseal is followed by default health200 before further administration.
The [health API](https://openbao.org/docs/api/system/health/) separates active service from standby reads.
Two administrative HTTP500 failures after restart remain preserved; their exact upstream cause is unproven.
No mutation retry or skipped acceptance case replaces those failures.

The gate also compiles and runs a consumer against extracted Cargo archives, with local patches for the unpublished family packages.
This tests distributed manifests and sources; it does not publish to a registry.
The [research](research/secrets-kms-assessment.md) and [decision log](research/decision-log.md) identify primary sources and uncertainties.
Vault, AWS and Azure require separate provider profiles and actual-service evidence.
The full ROM-extras goal remains active.

## Executed acceptance

The latest full `check-all` run exited0 with 155 unchanged runtime inputs.
The family gate passed 18 boundary/configuration/service tests and two compile-fail doctests.
Two additional native-host scenarios passed on both SQLite and redb.
The archive consumer performed actual KV and Transit operations and passed Clippy.
All three assessed dependency graphs have zero reported RustSec vulnerabilities.
These Rust audits do not assess the separate OpenBao server binary.
See [source hashes and execution evidence](verification/secrets-kms-host-package-full-verifier-2026-10-08.json) and [unchanged raw output](verification/secrets-kms-host-package-full-verifier-2026-10-08.log).

## Authenticated envelope migration

Use the [migration contract](kms-migration.md) to change an approved key alias or explicit encryption version with unchanged context/AAD.
The host conditionally persists the returned ciphertext. Native rewrap does not preserve this profile's AAD.
