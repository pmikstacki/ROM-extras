# RustFS local public-port qualification

Date: 2026-10-08. Source baseline: ROM-extras 7d59c1ea104d86c89cfbf11bda632a060e60ac7a plus the recorded fixture-helper changes.
Public ROM revision: d7ef529040eec60dc869034c2d33130219db85fe. No production adapter or dependency changes.

## Selection and fixture

[Garage source review](../research/garage-conditional-create-assessment.md) finds no conditional-create guard in its inspected PUT path.
[RustFS source review](../research/rustfs-conditional-create-assessment.md) checks normal PUT's lock, repeated precondition, and publication.
The artifact version command reports RustFS 1.0.1 and commit 6de965ae3c965a78ff819fbcd7acd4aa44177d92.
[Artifact facts](rustfs-candidate-2026-10-08.json) retain image ID, repository digests, selected digest, and isolated version command.
A version check alone is not runtime qualification.

The candidate publishes only 127.0.0.1:55456, uses generated private credentials, and preserves dedicated data/log directories.
It runs with one CPU and 1 GiB; [fixture facts](rustfs-fixture-2026-10-08.json) record actual configuration.
A finite HTTP health check returned 200. Signed CreateBucket returned 200; GetBucketVersioning returned 200 without Status.
[Bootstrap facts](rustfs-bootstrap-2026-10-08.json) record those administrative observations separately from public ROM tests.
No admin/console port, TLS, replication, or disk quota is claimed.

## Native probe, red, and green

The unchanged independent native prototype recorded one acceptance and seven AlreadyExists responses in 20–23 ms.
Its HEAD found the object; it did not read the winner or test restart.
See [native probe](rustfs-native-probe-2026-10-08.log) and the source hashes in the fixture facts.
This preliminary probe is not the public ROM acceptance suite.

The existing public-port test initially failed its SeaweedFS-only endpoint assertion with RustFS configured.
[The retained red log](rustfs-profile-red-2026-10-08.log) establishes the fixture-selection limitation before changing the helper.
The helper now selects fixed endpoint/container/label identities. The unset legacy profile still selects SeaweedFS.
The fixture facade contains only declarations and exports; behavior remains in its named profile module.

All four public-port S3 tests and Clippy passed through scripts/check-s3 on RustFS.
The strict test runs sixteen fresh keys with eight barrier-released competing values for each key.
It verifies one acknowledged winner, seven Conflict results, winner bytes, and later rejection without overwrite.
The suite also checks public blob conformance, confirmed denial, acknowledged false bytes after restart, and explicit reconciliation.
[The green log](rustfs-profile-green-2026-10-08.log) records zero failed or ignored tests.
The reconciliation test does not require an Unknown outcome; this run does not establish actual acknowledgement-loss handling.

A mismatched endpoint and an unknown profile each failed with exit 101 before provider access.
See [endpoint rejection](rustfs-mismatched-endpoint-2026-10-08.log) and [profile rejection](s3-unknown-profile-2026-10-08.log).
SeaweedFS basic public conformance passed again through the same helper; [its retained run](seaweedfs-profile-retained-2026-10-08.log) does not replace the strict failed result.

## Limits

This increment qualifies the exact local RustFS public-port profile.
It does not qualify cloud S3, multi-node locking, verified TLS, abrupt crash recovery, actual lost PUT acknowledgement, or BlobService lifecycle.
The SeaweedFS strict failure remains recorded and reproducible through its own profile.
The complete ROM-extras goal and packaged-release gates remain active.

## Complete configured verifier

The full scripts/check-all command exited zero with RustFS selected.
All 104 runtime inputs remained identical before and after execution.
[Source identities](rustfs-source-inputs-2026-10-08.json) record the complete input hashes, baseline HEAD, changed runtime files, and full-log hash.
[The full log](rustfs-full-verifier-2026-10-08.log) retains the executed required database, delivery, TLS, Azure, and S3 profiles.
Both Cargo manifests and lockfiles are unchanged from the preceding S3 baseline.
Secret inspection found no configured fixture account secrets in the published log.

This green configured gate supersedes the earlier failed whole-verifier result only for the selected RustFS profile.
It does not change SeaweedFS qualification or complete the still-pending extension families.
