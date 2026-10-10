# Authenticated envelope migration

`rom-kms::Migrator` authenticates an existing envelope and returns a replacement under an explicit destination version.
It preserves the original envelope, derivation context and authenticated associated data (AAD).
The host authorizes selection and conditionally persists the returned replacement through its normal write contract.
There is no transaction between KMS encryption and host persistence.

## Public integration

Keep the existing `Kms::encrypt` path for latest encryption.
Use `EncryptionVersion::pinned` for explicit selection. Unsupported providers return `Unsupported` before migration decrypt.
Existing Kms implementations do not need new methods to preserve latest behavior.
Providers implement both local `validate_encryption` and `encrypt_at` to support pinned selection.

```rust,ignore
use rom_kms::{EncryptionVersion, MigrationLimits, Migrator};
let migrator = Migrator::new(openbao, MigrationLimits::default())?;
// The host previously authorized this source, binding and destination alias.
let replacement = migrator.migrate(
    &stored_envelope, &binding, &approved_destination,
    EncryptionVersion::pinned(2)?,
).await?;
// The host conditionally stores replacement against its captured source revision.
```

An approved destination can be the same alias or another configured alias in the same profile.
The returned envelope must match the destination, requested version and source profile.
The adapter rejects contradictory metadata rather than publishing a replacement.
A repeated migration can produce different randomized ciphertext for the same authenticated plaintext.
A lost encrypt response does not establish that host persistence committed. There is no automatic retry or migration receipt.

## Limits and cancellation

Default migration policy admits four operations with a five-second total deadline.
Allowed limits are1..=64 concurrent operations and1ms..=60s deadline.
Share one Migrator instance to apply that admission bound across host requests.
Admission returns Busy without a queue and covers decrypt, retained plaintext and encrypt.
The independent HTTP transport retains its existing per-call admission and response limits.

One absolute deadline covers both provider operations and validation before returning the replacement.
Explicit expiry checks reject a late successful provider result.
Call migration inside a Tokio runtime with its time driver enabled. No hidden runtime is created.
Scheduling is cooperative; the deadline does not preempt blocking provider code.
Dropping the future releases local admission and owned plaintext. It does not establish remote cancellation or rollback.
No detached worker, plaintext cache or background updater is created.

At most one4096-byte SecretBytes is retained per admitted migration.
Its owned bytes are zeroized on drop, including failure and cancellation paths.
Provider, HTTP/parser, operating-system and explicit host copies remain outside universal erasure claims.
Migration returns no plaintext, but existing explicit Kms decrypt remains available to authorized hosts.

## OpenBao profile

The adapter uses existing derived AES-256-GCM96 Transit keys and exact context/AAD for both operations.
OpenBao2.7.1 native rewrap omits AAD, so it cannot implement this authenticated envelope profile.
Migration uses decrypt followed by encrypt. Bounded plaintext therefore exists locally.
Runtime permissions remain existing-key update permissions; key creation, rotation and floor changes stay administrative.

Pinned versions are restricted to1..=i32::MAX for portable Go TypeInt decoding.
Latest omits key_version and returns the actual positive version, subject to the same profile bound.
The response key_version must match the ciphertext prefix and any requested pinned version.
A future version, unavailable old version or encryption/decryption floor rejection never falls back to latest.
Migrate and conditionally verify stored replacements before an administrator raises the decryption floor.

HTTPS certificate/name checks, approved origins, streaming response bounds and fixed errors remain unchanged.
Proxies, redirects and automatic retries remain disabled.
Secrets/KMS support for Vault, AWS and Azure remains unqualified by these OpenBao tests.

## Reproduce qualification

Configure the existing private OpenBao endpoint, CA, limited runtime token, marker file and test-only administrative file.
Set ROM_EXTRAS_KMS_MIGRATION_EVIDENCE to an absolute private directory for retained ciphertext evidence.
Native keys and policies use new names for each invocation. Historical runs are preserved.

```sh
export ROM_EXTRAS_KMS_MIGRATION_EVIDENCE="$PWD/.superpowers/kms-migration-native-runs"
./scripts/check-openbao
```

The [independent archive consumer](../tests/packaged-secrets/main.rs) exercises public package APIs.
Its [shared native scenario](../crates/rom-openbao/tests/fixture/migration.rs) tests explicit version2 after latest3 and migration between approved keys.
It tests wrong context/AAD, source-only-token destination denial, absent keys, future versions, floors, expiry and revocation.
It stores ciphertext evidence and verifies the replacement with a fresh client after same-volume restart and explicit unseal.
Private host.json includes a test runtime token; it is protected configuration, not a public capture.
Public output and verification metadata contain no token or plaintext marker.

Generic authored tests establish admission, combined deadline, response mismatch and future-drop behavior.
Native pause/drop cases add protocol evidence without proving remote cancellation.
This local single-node TLS/Raft profile does not establish replicated custody, offsite recovery or production sizing.
See [primary-source research](research/kms-migration-2026-10-10.md).
