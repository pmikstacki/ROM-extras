# S3 real HTTP response-loss verification

Date: 2026-10-08. Baseline ROM-extras: 4e41831fcdb62fc86c9c483871f3bc593ac62c79 plus this recorded acceptance increment.
Published ROM: d7ef529040eec60dc869034c2d33130219db85fe. RustFS image/profile remains the [previously recorded local fixture](rustfs-fixture-2026-10-08.json).
No production provider, public constructor, SDK retry count, timeout, dependency manifest, or lockfile changed.

## Actual fault and recovery

[Primary-source assessment](../research/s3-wire-ack-loss-assessment.md) specifies the experiment and distinguishes SDK retries from lower transport behavior.
The standard-library test proxy binds numeric loopback, preserves the signed request header/path/body bytes, and forwards to the configured fixed backend.
It reads the complete success response before suppressing all client-facing response bytes for exactly one successful PUT.
It does not fabricate an HTTP status or suppress a failed backend write.
Regular responses announce the proxy's own Connection: close semantics and remove nominated hop-by-hop headers.
Raw requests, credentials, signed URLs, and payloads are never logged.

The scenario runs independently on SQLite and redb through the existing native lifecycle helpers:

| Point | Required and observed result |
| --- | --- |
| First upload | Caller Unknown; exactly one forwarded PUT, one backend success, one suppressed response |
| Native reservation | Pending, revision 1 unchanged |
| Independent direct read | Actual false bytes exist on RustFS |
| Explicit same upload retry | Second PUT confirms Conflict; provider GET verifies bytes before Ready attachment |
| Attached state | Ready, revision 2, upload_revision 1 |
| Repeated Ready upload | No third PUT |
| Drain/drop/reopen | Persisted Ready revision 2 and exact bytes on both native stores |

The final captured scenario records two forwarded PUTs, one success, one conflict, one suppressed response, and two proxy GETs per native store.
The first GET verifies conflict recovery; the second is an authorized BlobService read before reopening.
Objects and native database artifacts remain preserved; no cleanup is inferred from Unknown.

## Executed acceptance and diagnostic evidence

The transparent baseline compiled and reached one real backend success before failing the expected Unknown assertion.
It did not inject response loss. This establishes sensitivity to omission of the intended wire fault.
With response suppression, the first upload correctly returned Unknown, Pending remained unchanged, and direct retrieval verified bytes.
The initial recovery then failed with Backend: observed counts were PUT 2, success 1, conflict 1, GET 0, suppressed 1.
The fixture had forwarded a keepalive response header while closing that client connection.
After response-only connection correction, both native recovery scenarios passed with GETs reaching the proxy.
This diagnosis is separate from a production adapter bug.

The first complete S3 gate passed seven tests but failed warnings-denied Clippy on three collapsible-if expressions.
Those source-only style fixes were applied before the restored S3 gate exited 0 with seven tests and Clippy.
Azure's affected gate also exited 0, preserving its ten workspace tests and one independent-consumer test.
Independent source review found no actionable issues; it did not rerun these commands.

Unmodified evidence: [confirmed transparent baseline](s3-wire-transparent-confirmed-2026-10-08.log),
[failed recovery counts](s3-wire-ack-loss-counts-2026-10-08.log), [connection correction](s3-wire-ack-loss-fixed-2026-10-08.log),
[first gate and Clippy findings](s3-wire-ack-loss-gate-2026-10-08.log), [restored S3 gate](s3-wire-ack-loss-gate-restored-2026-10-08.log),
[Azure compatibility](s3-wire-azure-compatibility-2026-10-08.log), and [final native counts](s3-wire-ack-loss-final-counts-2026-10-08.log).
The workspace and consumer manifests and lockfiles are byte-identical to the baseline; this increment adopts no dependency.

## Complete configured verifier

The complete `./scripts/check-all` run exited 0 on the configured persistent local SQL, messaging, Azure and RustFS fixtures.
All 111 frozen runtime inputs had identical paths and SHA-256 hashes after the run.
The final S3 gate passed seven tests with no ignored cases, followed by warnings-denied Clippy.
See [full raw log](s3-wire-full-verifier-2026-10-08.log) and [source-input hashes](s3-wire-source-inputs-2026-10-08.json).
Final prose review found one ambiguous SeaweedFS coverage sentence; it now states that only its two shared BlobService cases passed.
The response-loss scenario was executed on RustFS only.

## Bounded fixture and remaining limits

Headers are limited to 32 KiB, bodies to 4096 bytes, accepted connections to 128, and worker lifetime to 60 seconds.
Connect is bounded to one second; individual framing/write operations have absolute two-second budgets.
The complete recovery test has a 45-second outer timeout, and normal completion joins the proxy worker before reopen.
Unsupported transfer encoding, expectation, duplicate or missing required length, framing-field connection nomination, and unexpected object paths fail explicitly.
Interim response storage is bounded; protocol switching is unsupported.
This is a narrow Content-Length HTTP/1 fixture, not a general HTTP proxy.

This local profile does not establish cloud TLS, abrupt backend or process crash durability, caller cancellation/drain faults, corrupt reads, or current revocation during I/O.
It does not establish this fault scenario for SeaweedFS or other S3 implementations.
SeaweedFS's strict burst qualification remains failed. Trusted detachment, grace, and quiescence are still required for physical cleanup.
The complete ROM-extras goal remains active.
