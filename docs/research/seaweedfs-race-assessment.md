# SeaweedFS 4.48 conditional-create race assessment

Date: 2026-10-08. Evidence: versioned source review. No fixture or race was executed by this research agent. The parent reports two-way conformance passed, while an eight-way race returned `Unknown` after approximately 3.26 seconds. That timing alone does not identify the failing network or server stage.

## Fixed public client behavior

Published ROM revision `d7ef529` constructs its S3 adapter with a three-second request timeout, one-second connect timeout, and zero retries. Conditional creates use `If-None-Match: *`. Preserve these settings when qualifying the requested eight-way scenario. See [public configuration](https://github.com/pmikstacki/ROM/blob/d7ef529/crates/rom-blob-object-store/src/configuration.rs) and [object_store create implementation](https://github.com/apache/arrow-rs-object-store/blob/v0.14.2/src/aws/mod.rs).

`object_store` 0.14.2 maps HTTP 412 to `Error::Precondition` and HTTP 409 to `Error::AlreadyExists`. ROM maps both to `BlobError::Conflict`. Therefore a received HTTP 409 alone does not explain ROM `Unknown`. Inspect the complete typed SDK cause before attributing that outcome to ConditionalRequestConflict. See [SDK response mapping](https://github.com/apache/arrow-rs-object-store/blob/v0.14.2/src/client/retry.rs) and [ROM error mapping](https://github.com/pmikstacki/ROM/blob/d7ef529/crates/rom-blob-object-store/src/errors.rs).

The SDK create path does not enable conflict retries. The update path separately enables them for concurrent If-Match operations. Do not infer update retry behavior applies to ROM create. See [versioned S3 operations](https://github.com/apache/arrow-rs-object-store/blob/v0.14.2/src/aws/mod.rs).

AWS documents that concurrent create-if-absent requests permit one winner and reject subsequent writes with 412. It also documents 409 for conflicts involving concurrent deletion. The requested no-delete eight-way race should retain its strict one-success/seven-conflicts result. See [AWS conditional writes](https://docs.aws.amazon.com/AmazonS3/latest/userguide/conditional-writes.html).

## Versioned SeaweedFS write paths

SeaweedFS 4.48 checks conditional headers before processing the upload. At metadata finalization, it can route a write condition to the owner filer. A routed precondition failure maps to the S3 precondition error. If routing is unavailable or fails, it falls back to an object write lock and checks the headers again under that lock. See [PUT finalization](https://github.com/seaweedfs/seaweedfs/blob/4.48/weed/s3api/s3api_object_handlers_put.go) and [conditional routing](https://github.com/seaweedfs/seaweedfs/blob/4.48/weed/s3api/s3api_object_routed_write.go).

The lock factory obtains a short-lived lock, then opportunistically extends its TTL to fifteen seconds. That TTL protects ownership; it is not the HTTP client's total request deadline. See [gateway lock configuration](https://github.com/seaweedfs/seaweedfs/blob/4.48/weed/s3api/s3api_server.go).

The lock client retries ordinary `lock already owned` contention without a total bound. `AttemptToLock` sleeps one second after transport or lock-server error responses. Consequently, repeated contention on the fallback path can exceed a three-second client timeout. This is a source-established possible mechanism, not proof that the reported race used this path. See [versioned lock client](https://github.com/seaweedfs/seaweedfs/blob/4.48/weed/cluster/lock_client.go).

The metadata-save code explicitly uses `context.Background()` so it can finish after HTTP cancellation. A timed-out caller can therefore lose acknowledgement while server work continues. Later object existence does not convert the original unknown result into a confirmed conflict. See [PUT metadata context](https://github.com/seaweedfs/seaweedfs/blob/4.48/weed/s3api/s3api_object_handlers_put.go).

The gateway caches unreachable owner information and owns a shared read cache. The comments describe read-route avoidance and shared chunk reads. They do not establish a conditional-create cache defect. Avoid attributing the reported write timeout to caching without request-stage evidence. See [gateway fields](https://github.com/seaweedfs/seaweedfs/blob/4.48/weed/s3api/s3api_server.go).

## Missing causal evidence

The parent reports startup filer-unavailable messages without a race-specific server error. That does not establish readiness at the race start or prove the server received every request. Record readiness and race timing independently.

Collect each submitted operation's terminal result, elapsed time, and attempt identity. Preserve aggregate counts even if the first assertion fails. Obtain native SDK error type, response status where available, and a sanitized source-chain category. Do not record credentials, signatures, payloads, or raw signed URLs.

Capture whether finalization used routing or the lock fallback, and whether metadata finished after the client deadline. Inspect filer health, owner/ring initialization, volume readiness, and lock-server responses for the exact request interval. Keep diagnostic inspection separate from the production adapter; do not copy its implementation to manufacture passing results.

## Qualification without weakening the contract

Keep eight simultaneous producers, zero SDK retries, and the public three-second deadline. Repeat the strict race on fresh unique keys after explicit server readiness. Warm up unrelated keys only if the warm-up becomes a documented fixture readiness step. Do not pre-create the raced key or stagger producers.

Investigate server topology and native configuration that keep conditional writes on a healthy owner route. Validate those changes against exact 4.48 source and runtime evidence. Do not assume a `mini` flag or cache setting exists. The mini command orchestrates several services; its successful S3 bind alone does not prove all internal services are ready. See [mini implementation](https://github.com/seaweedfs/seaweedfs/blob/4.48/weed/command/mini.go).

If a healthy, documented profile repeatedly produces Unknown under this required burst, retain the failed evidence and report that profile unqualified. Select another maintained S3-compatible backend or investigate an upstream fix. Do not count Unknown as Conflict, raise the adapter timeout, add client retries, or reduce concurrency merely to obtain a green result.

After a strict passing race, independently read and hash the winner's bytes. Confirm losers never overwrite them. Repeat across fresh keys and after server restart using preserved storage. These observations prove only the tested profile and load.

A recovered winner does not prove acknowledged success for all callers. TLS, lost acknowledgement recovery, authorization changes, native SQLite/redb lifecycle, orphan grace, and trusted cleanup remain separate acceptance gates. Source review here establishes possible mechanisms and a diagnostic plan; it does not establish the actual timeout cause or a supported SeaweedFS profile.

## Executed lock-path reproduction

A separate verbose reproduction used the same image, named volume, CPU limit, and memory limit.
The binary reports commit `530be3e37`; inspect [its lock client](https://github.com/seaweedfs/seaweedfs/blob/530be3e37/weed/cluster/lock_client.go).
The sanitized trace records 20 lock attempts, seven acquisitions, and six contention responses for this diagnostic object.
Acquisitions include approximately one-, two-, and three-second offsets.
This establishes distributed lock use in this reproduction, not every earlier run.

The native client recorded one acceptance, six AlreadyExists responses, and one typed timeout at 3004 ms.
The object existed afterward. Verbose logging changes scheduling; do not replace the original qualification result with these counts.
The trace selector found no CreateEntry events; that does not establish absence of metadata creation.
See [sanitized trace](../verification/s3-lock-trace-2026-10-08.json) and [native results](../verification/s3-traced-native-2026-10-08.log).

A signed, read-only GetBucketVersioning request returned HTTP 200 with no Status element.
[AWS documents this response](https://docs.aws.amazon.com/AmazonS3/latest/API/API_GetBucketVersioning.html) for a bucket whose versioning was never enabled or suspended.
The [executed observation](../verification/s3-bucket-versioning-2026-10-08.json) therefore excludes enabled versioning as this fixture's explanation.
The administrative request used [AWS SigV4](https://docs.aws.amazon.com/AmazonS3/latest/developerguide/sig-v4-header-based-auth.html), a three-second timeout, and no ambient HTTP proxy.

[Exact-commit routing](https://github.com/seaweedfs/seaweedfs/blob/530be3e37/weed/s3api/s3api_object_routed_write.go) permits an unversioned If-None-Match wildcard condition.
PrimaryForKey returns no owner before the client has a ring. Routed RPC failures can also trigger lock fallback.
The captured private log contains no routed-PUT failure marker. That absence alone does not establish why routing was unavailable.
Investigate ring delivery and gateway initialization before changing fixture topology.

The trace container required SIGKILL after its ten-second stop allowance and exited 137.
This is not graceful drain evidence. The stopped trace container and original volume remain preserved.
The original fixture was restored and public conformance passed again.
The strict eight-way qualification remains failed; no client deadline, retries, or concurrency requirement changed.
