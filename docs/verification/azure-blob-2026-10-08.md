# Azure Blob adapter verification

Date: 2026-10-08. Published ROM pin: d7ef529040eec60dc869034c2d33130219db85fe.
Baseline extras commit: eb9b1ffc822624edfb7d28108eb996fd6cd3a774. This report describes the subsequent Azure increment.
[Source hashes](azure-source-inputs-2026-10-08.json) identify all 98 executed inputs independently of the later commit.

## Executed checks

`./scripts/check-azure` passed with the corrected configuration tests.
Eight workspace tests passed: three configuration, three real-emulator, and two local native-HTTP interruption tests.
The independent Azure consumer passed its one integration test and feature-enabled Clippy.
Workspace feature-enabled Clippy also passed. See [the corrected log](azure-corrected-2026-10-08.log).

The emulator tests call published `rom_conformance::blob::basic`.
They exercise missing operations, create-only conflicts, concurrent arbitration, empty and exact-limit bytes, bounded reads, metadata, and idempotent deletion.
An acknowledged false payload survives a labelled emulator restart and a fresh adapter read.
A wrong valid-format key is denied while the correct account confirms that no object was created.
Local HTTP fixtures separately verify immediate overload, Unknown after a withheld response, and ETag-conditioned read rejection.
These fixtures do not establish post-write persistence, cloud TLS, or cloud authorization.

## Failures and review corrections

The first API test failed because the adapter did not exist. A later compilation failed on missing crate-level documentation.
Both failed logs remain preserved.
An expanded emulator run failed when one test restarted the shared service during another test's conformance operation.
A fixture-wide mutex now serializes emulator tests. The isolated rerun passed.

Independent source review found endpoint tests that used an invalid base64 key.
That key could reject construction even if endpoint validation was removed.
The controlled mutation passed the original tests, demonstrating the review defect.
The corrected tests use valid synthetic credentials and include positive client construction.
They failed against the same removed endpoint checks, then passed after restoring the implementation.
See [old false-positive evidence](azure-masked-endpoint-mutation-2026-10-08.log) and [corrected negative evidence](azure-endpoint-mutation-red-2026-10-08.log).
The mutation is absent from the delivered source. Review was source inspection; the root executed the tests.

Fixture namespace initialization first returned HTTP 403 because Content-Type differed from the signed fields.
Explicitly matching the signed content type returned HTTP 201. The sanitized initialization log retains both observations.
This does not claim a more specific Azure error category than the recorded response establishes.

## Fixture and dependencies

[Selected fixture facts](azurite-fixture-2026-10-08.json) record the official Azurite image digest, port, limits, and preserved volume.
The container exposes only the loopback blob port and disables telemetry. Keys remain in ignored mode-600 files.
The fixture is an emulator, not the Azure service.

Cargo-audit reported zero vulnerabilities and warnings for the 360-package workspace and 422-package consumer graphs.
Both scans use RustSec revision 550efd3d587a29b2e2c2b21b17a440da4fede999.
The inventories have no missing manifest license declarations and no removed previous locked package identities or checksums.
Declared Rust versions are at most 1.99; some dependencies have no declared version.
These observations do not complete native redistribution or packaged release review.
Audit JSON and dependency inventories are adjacent to this report.

## Remaining acceptance

The missing-fixture profile exits 1, preventing an absent backend from passing.
The first full-verifier launch exited 2 before tests because OpenSSL was missing from PATH.
Its log is retained; the corrected environment includes the test-signing CLI and maintained native OpenSSL.
The corrected `./scripts/check-all` finished with exit 0.
Its 98 code, manifest, lockfile, and script inputs were identical before and after execution.
The gate includes formatting, denied-warning Clippy/docs, workspace tests, independent consumers, MSRV, and all required live profiles.
See [the complete gate log](azure-full-verifier-2026-10-08.log).
Real Azure, verified cloud TLS, bearer-token authentication, BlobService lifecycle, actual committed-write acknowledgement loss, and release acceptance remain pending.
S3-compatible work and the other extension families remain in the goal.


Raw logs retain command output, including native test-runner trailing whitespace and final blank lines.
The staged whitespace check excludes those raw logs; code, manifests, scripts, JSON, and prose pass without exclusions.
This preserves historical evidence under the writing rules. No test result or raw diagnostic was rewritten.
