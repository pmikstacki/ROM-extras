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

File and HTTP transports, interrupted acknowledgement and process-restart recovery remain pending.
Multi-record orchestration, implicit deletion, SQL archives, external object manifests and migrations are not implemented here.
The full maintenance family remains incomplete.
See [source research](research/backup-import-2026-10-09.md) and [implementation plan](superpowers/plans/2026-10-09-sourced-import.md).

The full local verifier passed on 1,186 unchanged files.
The subsequent target-conflict test passed through both direct and packaged consumers.
See [the verification record](verification/import-core-2026-10-09.json) for the exact source snapshot and evidence limits.
