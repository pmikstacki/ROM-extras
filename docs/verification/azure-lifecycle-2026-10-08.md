# Azure BlobService lifecycle verification

Date: 2026-10-08. Extras baseline: 177f8931661b39700f0305ea8901a2c0a92b3b50.
Published ROM revision remains d7ef529040eec60dc869034c2d33130219db85fe.
This increment adds tests and three existing dev-dependency edges; production adapter code is unchanged.
All 360 locked package identities and checksums remain unchanged. The independent-consumer lockfile is unchanged.

## Executed scenarios

Two new integration tests run each scenario on SQLite and redb.
Both use actual Azure adapter operations against the existing persistent Azurite fixture.
The corrected [Azure profile](azure-lifecycle-corrected-2026-10-08.log) passed ten workspace tests, one independent-consumer test, and both Clippy profiles.
The final source includes the subsequent reviewed assertion about detached bytes after reopening.

The first scenario reserves a Pending Blob with the digest and length of false bytes.
A wrong same-length payload returns Conflict before any provider create call.
A stream error after a partial chunk returns Backend without provider create or reservation revision change.
A complete upload reaches Attached; owner reads return independently verified bytes.
A different principal gets Denied before a provider GET. Repeating the Ready upload does not issue another create.

After service and Runtime drain, all Runtime-owning handles are dropped before reopening the same native database.
The reopened Ready reference retains revision 2 and upload revision 1, and retrieves the same false bytes.
Logical detachment returns a stable maintenance key. Repeated detachment retains that key.
A further reopen retains Detached; read returns Missing and upload returns Conflict without another create.
A direct provider read checks retained detached bytes after reopening. No cleanup authorization is inferred.

The second scenario pauses the test wrapper after a real SDK create succeeds.
A direct Azure read confirms the object exists before the Resource deletion.
The owner deletes the Pending reservation. Releasing the wrapper yields Unattached with cause Denied and the matching object receipt.
After native reopening, direct Resource read is Denied, query is empty, and provider bytes remain present.
The receipt retains SHA256 digest and byte count. The same test executes separately for both native backends.

## Evidence limits and failures

The wrapper supplies test ordering and counts; it delegates actual Azure operations.
It pauses after SDK success, not on the network acknowledgement. This is not a wire acknowledgement-loss experiment.
Native reopening is orderly shutdown, not a killed-process or power-loss test.
The test uses trusted fixture principals, not a live identity provider or production credential flow.

The initial test expected Conflict after deletion. It failed because public Runtime returns Denied for a tombstone before read authorization.
The test now preserves that public result. No production outcome mapping was changed.
The initial log also retains an unused-import warning; the corrected source removes that import.
See [the initial failed run](azure-lifecycle-initial-2026-10-08.log) and [primary-source assessment](../research/azure-blob-lifecycle-assessment.md).

Independent source review found one evidence gap: detached provider bytes needed checking after native reopen.
The final assertion addresses it. The reviewer did not execute cloud or lifecycle tests.
The root executes the required final verifier below.

## Final verification and remaining gates

The final `./scripts/check-all` exited 0, including all required fixture profiles and the reviewed post-reopen byte assertion.
All 100 captured Rust, manifest, lockfile, and script inputs remained identical before and after execution.
See [the full log](azure-lifecycle-full-2026-10-08.log) and [source identity](azure-lifecycle-inputs-2026-10-08.json).
Raw logs retain native whitespace; staged whitespace checks exclude raw logs and check code, JSON, scripts, and prose.

Actual post-publication wire acknowledgement loss, current revocation during I/O, cancellation/drain, corrupted-object reads, and trusted orphan reconciliation remain pending.
Cloud Azure, verified TLS, bearer-token authentication, crash recovery, and packaged release also remain pending.
The full extension family and ROM-extras goal remain active.
