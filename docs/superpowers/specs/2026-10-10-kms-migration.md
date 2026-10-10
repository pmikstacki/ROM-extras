# Authenticated KMS envelope migration

Extend existing host encryption contracts without changing existing public paths or requiring administrative runtime permissions.
The host authorizes selection and conditional persistence. Migration returns a replacement; it never mutates the original envelope or commits a Resource.

## Public contract

Add `EncryptionVersion::{Latest,Pinned(NonZeroU64)}` with `pinned(u64)` rejecting zero.
Add default `Kms::validate_encryption(key, version)` and `Kms::encrypt_at(key, plaintext, binding, version)`.
Existing implementers retain latest encryption; pinned selection defaults to Unsupported before decrypt.
Providers validate approved destinations locally and never substitute latest for pinned encryption.

`Migrator<K>` owns a provider and validated `MigrationLimits`: deadline1ms..=60s, max_in_flight1..=64; defaults5s/four.
`migrate(source, binding, destination, pinned_version)` validates selection before admission or decrypt.
Latest is invalid for migration. One try-acquire permit covers both calls and owned plaintext; saturation returns Busy without a queue.
One absolute deadline covers decrypt, encrypt and result validation. Explicit expiry checks prevent a late provider result from publication.
Timeout is cooperative scheduling, not hard CPU preemption. Future drop releases owned plaintext and admission; it does not establish remote cancellation.
No detached task, automatic retry, plaintext cache, bulk updater or cross-service transaction exists.
The replacement must match destination alias, source profile and requested version; mismatch returns Protocol.
Same context and AAD authenticate both operations. Decrypted SecretBytes remains internal and bounded4096bytes; normal drop zeroizes its owned buffer.
HTTP/parser/provider/host copies are outside universal erasure claims. Callers share one Migrator to obtain its concurrency bound.

## OpenBao profile

Use existing TLS/no-proxy/no-redirect/no-retry/bounded transport and existing-key update permissions.
Keep existing encrypt latest behavior; share its encoding/parser with versioned encryption.
Portable pinned range is1..=i32::MAX, reflecting Go TypeInt portability, not a claim about installed version count.
Validate data.key_version, ciphertext prefix and requested pinned version. No algorithm downgrade or omitted AAD.
Native rewrap is excluded because tagged2.7.1 decrypts/encrypts with nil AAD.

## Required evidence

Generic tests: zero/version bounds, unsupported before decrypt, destination validation, exact binding/bytes, wrong profile/key/version response,
Busy across decrypt+encrypt, combined deadline, late synchronous success, cancellation before/between/during calls and permit recovery.
Real OpenBao: new retained keys; v1 source with binary plaintext/nonemptyAAD; rotate to2 then3; explicit destination2 stays2;
latest stays3; second approved key; wrong context/AAD; absent/unapproved/denied destination; future version; encryption/decryption floors;
revoked and expired tokens; original source remains; same-volume restart/fresh client decrypt.
Repeat representative native migration through normalized Cargo archives and an independent consumer using only public API.
Authored fault tests are separate from actual service qualification. Scan public/log/host captures for runtime token and plaintext marker.
Run affected gates, dependency/license/MSRV/advisory checks, fresh review and full frozen local verifier before publication.

## Limits

Qualification applies to owned OpenBao2.7.1 single-node verified TLS/Raft fixture only.
Vault/AWS/Azure, replicated quorum, production custody, automatic retention-floor changes and universal zeroization remain unqualified.
SQL Storage still requires absent public ROM preparation/ownership boundaries. Complete ROM-extras goal remains active.
