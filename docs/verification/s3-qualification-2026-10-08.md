# S3-compatible candidate qualification

Date: 2026-10-08. Extras baseline: 6849bf6207f6191baa30738dba6249add401b861.
Public ROM revision: d7ef529040eec60dc869034c2d33130219db85fe. Native SDK: object_store 0.14.2.
Production adapter code is unchanged. The independent consumer adds public ROM adapter and conformance dependencies.

## Actual execution

The official SeaweedFS 4.48 image was launched with generated private credentials and preserved named storage.
Only the loopback S3 endpoint is exposed. The exact digest, arguments, labels, mounts, and memory limit are recorded separately.
`mini -h` returned its documented help with exit 2; this is not a failed server launch.
The selected [fixture facts](seaweedfs-fixture-2026-10-08.json) omit credentials.

The first consumer run completed the public two-way conformance suite, then failed on an eight-way create returning Unknown.
A separate fresh-key rerun reproduced that result at approximately three seconds.
Splitting the scenarios preserves the failed strict qualification while executing the remaining cases independently.
A fixture mutex prevents its restart scenario from interrupting another test.

The final four-test run had three passes and one failure.
It verifies published conformance, confirmed credential denial, restart retention, and uncertainty/reconciliation separately.
The strict burst qualification still fails. Feature-enabled Clippy exits 0 with warnings denied.
See [the four-test log](s3-uncertainty-profile-2026-10-08.log) and [Clippy](s3-clippy-final-2026-10-08.log).
No tests skip missing fixture access.

## Native diagnostic and interpretation

An independent native client uses the same three-second request timeout, one-second connect timeout, conditional PUT, and zero retries.
Eight candidates start at a barrier. The retained diagnostic object exists after caller completion.
The final diagnostic identifies reqwest 0.13.5 through the error source chain and calls its typed is_timeout/is_connect/status methods.
Four generic errors contain typed timeout causes near 3000 ms; they are not typed connection failures and have no reported HTTP response status.
Other requests receive one acceptance and three AlreadyExists outcomes in that recorded native run.
See [typed diagnostics](s3-native-typed-diagnostic-2026-10-08.log), [source](s3-native-typed-source-2026-10-08.txt), [manifest](s3-native-typed-manifest-2026-10-08.txt), and [source identity](s3-native-typed-identity-2026-10-08.json).

Earlier prototype substring flags are preserved, but cannot establish HTTP 500 or 503 statuses.
The first prototype compilation missed ObjectStoreExt; its failure is retained before the corrected run.
Raw credentials, SDK reason strings, and server startup credentials are not published.
The private server log and diagnostic prototype remain preserved locally.

Versioned sources show one-second ordinary lock-contention waits and possible publication after caller cancellation.
The observed timing is consistent with that path, but does not trace the actual server cause.
SDK 409 and 412 map to Conflict; neither explains Unknown by itself.
See [the source-only assessment](../research/seaweedfs-race-assessment.md).

## Review and dependency evidence

Independent source review found no endpoint or restart-target defect.
It recommended a separate Unknown/reconciliation scenario, without replacing the strict failed qualification.
The added test keeps at most one acknowledged winner, validates the winner's outcome category, verifies unchanged bytes on eight retry conflicts, and performs no cleanup.
This is orderly restart and explicit retry evidence, not automatic crash recovery or cloud authorization.

The updated consumer graph contains 427 packages and retains all 422 previous package identities and checksums.
Cargo-audit reports zero vulnerabilities and warnings; dependency manifest license declarations are present.
The first audit launcher lacked cargo-audit in PATH. Its error is retained; the installed pinned audit binary subsequently ran successfully.
The workspace lockfile is unchanged. Redistribution and packaged-consumer acceptance remain pending.

## Full verifier

The complete verifier exited 101 at the new strict S3 gate after the preceding required profiles passed.
All 102 captured inputs remained identical before and after execution.
See [the full failed gate](s3-full-verifier-2026-10-08.log) and [source inputs](s3-source-inputs-2026-10-08.json).
The missing-fixture S3 command exits 1. The strict candidate failure remains required and is not bypassed.
The whole ROM-extras goal remains active. No supported S3 provider or full extension completion is claimed.

## Separate verbose reproduction and restoration

The [lock-path assessment](../research/seaweedfs-race-assessment.md#executed-lock-path-reproduction) records an additional diagnostic run.
Its native outcomes were one acceptance, six AlreadyExists responses, and one typed timeout.
A sanitized server trace confirms distributed lock activity for that diagnostic object.
A subsequent read-only GetBucketVersioning observation confirms the fixture has no versioning Status.
These observations narrow the investigation without replacing the strict failed gate.

The trace container required forced termination and remains preserved in exited state.
After restoring the original fixture, the filtered public conformance test exited zero.
See [restored conformance](s3-restored-conformance-2026-10-08.log).
No production code, dependency graph, or verifier requirement changed in this diagnostic increment.
The complete verifier's latest recorded result remains exit 101 at strict S3 qualification.
