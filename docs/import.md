# Sourced action imports

`rom-import` prepares a bounded JSON input for an ordinary ROM domain action.
It uses published ROM `d7ef529040eec60dc869034c2d33130219db85fe`.
It does not fetch content, select actors or execute actions.

## Host configuration

Before fetching, prepare the managed source generation and capture its control revision.
Authorize the exact Resource, action and complete output origins.
Obtain the expected SHA-256 from a host-approved immutable manifest or equivalent policy.
A digest from an unauthenticated producer does not authenticate that producer.

Construct `ActionBinding` with the exact target, registered action, target revision, idempotency and original retry epoch.
Construct `SourceGrant` with the source owner, generation, output origins, control dependency, expiry and expected digest.
Call `ActionPlan::trusted(binding, grant, Limits::default())`.
Call `plan.prepare(&bytes)` after bounded content acquisition.
The complete JSON value becomes the action input.

```rust,ignore
let prepared = plan.prepare(&host_approved_bytes)?;
let (invocation, permit) = prepared.request();
let result = runtime.invoke_sourced(&service_actor, invocation, permit).await;
// Retain prepared and the original actor scope when result is Unknown.
```

The host retains authentication, authorization, session handling and source activation.
The permit grants no row or field access.
The registered Resource must have the matching source owner.
Output origins must cover the complete final value, including computed and retained fields.
Input field names cannot establish those origins.

## Limits and errors

| Admission | Default and hard maximum |
| --- | --- |
| Raw bytes | 65,536 |
| Value depth, with root at one | 32 |
| Value nodes, excluding object keys | 4,096 |
| Bytes per decoded string or object key | 16,384 |

`Limits::new` can reduce those limits. Zero limits are invalid.
Byte admission precedes parsing. Structural admission follows bounded parsing; it is not a streaming memory cap.
Public `rom::parse_json` rejects duplicate keys at every depth, trailing data and malformed JSON.
Scalar and array roots remain valid action inputs.
Signed and unsigned 64-bit integer tokens preserve their integer representation.
Arbitrary-precision numbers are not promised.

Binding action and idempotency labels must contain 1–256 bytes. An expected revision of zero is refused.
ROM validates source labels, keys, generation, dependency revision and expiry through `SourcePermit::trusted`.
The source version is `sha256:<64 lowercase hexadecimal digits>` of the approved exact bytes.
Equivalent JSON with different whitespace has a different version.
SHA-256 is not a signature, encryption or producer authentication.

Admission errors and Debug output contain no document content or grant labels.
The returned Invocation contains the input. The host must protect it from logs and public disclosure.
Runtime errors can contain application-selected identifiers; this crate does not execute or redact those errors.
ROM independently enforces its command-byte limit, which defaults to 16 KiB at the published pin.
An admitted document can therefore still be too large for execution.

## Recovery

`PreparedAction::request()` clones the unchanged original Invocation and SourcePermit.
Keep the same actor authority, principal kind and subject for receipt identity.
Keep the same input, target revision, provenance, idempotency and retry epoch.
Do not refetch changed content or silently prepare a new generation after an unknown outcome.

Runtime checks current authority before replay.
An expired or superseded grant can prevent replay of an already committed request.
That rejection does not establish rollback. Grant renewal requires explicit host reauthorization.
This core implements no persistence format, retry loop, controller or second Work ledger.

## Verified scope

Run the working [independent public consumer](../tests/import-public-consumer/src/main.rs):

```sh
mkdir -p /tmp/rom-import-example
ROM_EXTRAS_IMPORT_PATH=/tmp/rom-import-example cargo run --manifest-path tests/import-public-consumer/Cargo.toml --locked
```

Use a fresh directory for each run. The fixture preserves its SQLite and redb databases.
It executes a domain action that doubles imported scalar input and retains another field.
It verifies exact IDs, output provenance, denied actors, incomplete output coverage, target conflicts and exact replay.
It closes and reopens each database, then checks accepted attribution and replay.
A changed source control blocks replay without undoing the committed value.
The same consumer runs against the normalized Cargo archive through `./scripts/check-import`.
This qualification uses retry epoch zero; a pure boundary test checks nonzero identity preservation.

## File and HTTPS acquisition

`rom-import-transport` feeds the same approved `ActionPlan`. Its default build has no HTTP dependencies.
The host opens and authorizes a regular file, then calls `prepare_file(&plan, file)`.
The reader rewinds the handle and reads at most the configured document limit plus one byte.
Metadata does not establish immutability. The mandatory expected digest detects changed input.
Filesystem reads have no hard syscall deadline.

Enable the `http` feature for `HttpSource`.
Construct `HttpConfig::new(endpoint, agent, minimum_start_interval, concurrency)` with an explicit HTTPS endpoint.
Query strings, userinfo, fragments and ambiguous paths are refused.
Use `with_ca` to replace platform trust with an explicit private CA.
Use `with_bearer` for a server-only credential. Debug output does not disclose it.
The host must authorize the endpoint and its DNS resolution. This adapter is not a private-address firewall.

Construct `RequestContext` with a 1ms–60s deadline and a watch receiver.
Retain its sender. A true value or a closed channel cancels the operation.
Call `source.fetch(&plan, optional_strong_etag, &context).await`.
The deadline includes admission, start pacing, request, body reading and document preparation.
Synchronous parsing and filesystem calls cannot be preempted by this context.

Only HTTP200 with JSON and identity content encoding is accepted.
The client disables proxy discovery, redirects, decompression and automatic retry.
Each source admits 1–8 concurrent operations. Excess operations fail immediately.
Start pacing is explicit and local to that source instance.
HTTP429 returns an optional integer Retry-After hint of at most 3,600 seconds; it does not trigger retry.
The host owns cache policy, backoff and any subsequent attempt.
Conditional reads require a strong ETag and an identical single response ETag.
This conservative profile is stricter than general HTTP semantics. The expected digest remains mandatory.

The HTTP parser accepts at most 32 headers.
Retained header names and values are checked against 8,192 bytes after parsing.
This check is not a parser allocation limit.
Declared and streamed body lengths are bounded before accumulation.
An incoming chunk can already occupy library memory before that check.

## Transport qualification

Run `./scripts/check-import-transport` with the explicit owned fixture variables from the qualification environment.
Its independent consumer also runs against normalized archives for both import crates.
An authored TLS server checks protocol failures, limits, cancellation, pacing and concurrency.
Those controlled protocol checks do not qualify a third-party service.

A separate native Nginx1.29.8 fixture serves authenticated static JSON over TLS.
The consumer checks validators, wrong authentication, missing content, digest mismatch and service restart.
SQLite and redb fault hooks produce both pre-commit failure and a committed unknown outcome.
The consumer exits in a child process after each outcome.
Separate processes reopen both stores and replay the original request while Nginx remains stopped.
They verify the retained value, provenance, receipts and event counts without fetching content again.
The private recovery record is a qualification host example, not a generic recovery service or public grant format.

Multi-record orchestration, implicit deletion, SQL archives, external object manifests and migrations are not implemented here.
The full maintenance family remains incomplete.
See [source research](research/backup-import-2026-10-09.md) and [implementation plan](superpowers/plans/2026-10-09-sourced-import.md).

For the original pure-core increment, the full local verifier passed on 1,186 unchanged files.
The subsequent target-conflict test passed through both direct and packaged consumers.
See [the verification record](verification/import-core-2026-10-09.json) for the exact source snapshot and evidence limits.

For file and HTTPS acquisition, affected checks and the full local verifier passed on 1,211 unchanged source files.
See [the transport verification record](verification/import-transport-2026-10-09.json).
