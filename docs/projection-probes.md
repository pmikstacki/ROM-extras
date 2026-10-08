# Native projection protocol probes

These are actual-service probes, not completed ROM search adapters or durable projection workers.
The [projection contract](superpowers/specs/2026-10-08-projections.md) and [implementation plan](superpowers/plans/2026-10-08-projections.md) retain the remaining scope.

## Executed services

| Profile | Pinned image | REST endpoint | Experimental fixture allowance |
| --- | --- | --- | --- |
| Qdrant 1.19.2 | `docker.io/qdrant/qdrant@sha256:0e8273b9130ca3b0dd9dcfaa8f55342066711f8aaeb2aa072e53344c42ecf57f` | `https://127.0.0.1:55461` | 1 CPU, 512 MiB |
| OpenSearch 3.9.0 | `docker.io/opensearchproject/opensearch@sha256:13487e0953520edf6cc866dfc672fd67a70ab122aa7d84d4a6c97106f84d3f84` | `https://127.0.0.1:55460` | 2 CPUs, 2 GiB; JVM heap 512 MiB |

Both use persistent fixture storage and verified private TLS. These allowances are not production sizing recommendations or vendor minima.
OpenSearch uses separate operator, writer, and reader client certificates. Writer operations are limited to `rom_extras_*` indexes.
Qdrant uses separate private write and read-only API keys. Its unused gRPC service is explicitly disabled.
Private credentials and TLS keys remain outside Git under `.superpowers/`.
OpenSearch CA, operator, and client private keys are outside the server-mounted TLS directory after initialization.
The server retains only its node private key in that directory.

## Run the required gate

The probes currently require the isolated `/root/ROM-extras` fixture configuration. They are not a portable provisioning command.
Keep the private fixture files and actual services available. A missing service or credential fails verification; there is no skip.
Provide a Python 3 executable through `ROM_EXTRAS_PYTHON` when `python3` is absent from `PATH`.

```bash
./scripts/check-projection-probes
```

`./scripts/check-all` includes this gate. It restarts only the named Qdrant and OpenSearch fixture containers.
It preserves existing collections, indexes, and service data. Each execution creates uniquely named test datasets.
The shared transport rejects redirects and system proxies. Responses are bounded to 1 MiB.
Python socket timeouts are per operation. These probes do not qualify the future production adapter's total request deadline.

## Qdrant evidence

The [source report](research/qdrant-revision-fencing.md) identifies conditional upsert under a native submission fence.
The probe compares exact u64 revisions through scalar u32 halves, with indexed and unindexed conditions.
It verifies old and equal revisions, concurrent initial inserts, and empty-vector tombstones.
An explicit physical-delete counterexample allows stale insertion. Physical deletion is outside the proposed supported tombstone profile.

The TLS proxy holds a real request until its client times out. A newer direct write completes before the old request is forwarded.
Distinct vectors verify that the old request cannot replace newer vectors or payload.
Another case drops the actual response after Qdrant confirms completion. Direct retrieval establishes acceptance before newer state and stale replay.
Persistent restart retains newer state and tombstones, including u64::MAX. These scenarios do not implement a ROM checkpoint.

## OpenSearch evidence

The probe verifies native external-version conflicts, the signed revision limit, and persistent tombstone documents.
It inspects every Bulk item and proves mixed success/failure under HTTP 200.
The reader cannot write. The writer cannot create an index outside its approved pattern.
Live queries exclude tombstones. Persistent restart retains documents, i64::MAX, and the tombstone version fence.
Current ROM authorization and Resource result disclosure remain unimplemented; backend credential scopes do not replace them.

## Fixture preparation failures

OpenSearch initially blocked index creation above its default 90% disk watermark despite approximately 185 GiB available.
The fixture now uses absolute free-space reserves: low 20 GiB, high 10 GiB, and flood stage 5 GiB.
Disk protection remains enabled. No host-wide setting or unrelated service was changed.
The monitor removed the earlier block asynchronously; authoritative settings and logs were checked before proceeding.
See [official cluster settings](https://docs.opensearch.org/latest/install-and-configure/configuring-opensearch/cluster-settings/).

The first single-type security initialization uploaded configuration but timed out waiting for the remaining required types.
The complete initialization then loaded all nine expected types together, with certificate and hostname validation enabled.
The original command outputs remain private historical evidence. They are not counted as passing setup.
See [official initialization](https://docs.opensearch.org/latest/security/configuration/security-admin/).

The first OpenSearch probe incorrectly sent NDJSON bytes through JSON encoding. It failed before the Bulk request.
The corrected probe sends the already encoded NDJSON body directly. The initial failure output is preserved.
An independent review found equal vectors in the first delayed-write probe and an unbounded wrong-version readiness branch.
Distinct vectors and a fail-fast version assertion corrected those weaknesses before the final gate.

## Remaining acceptance

Implement durable intents, checkpoints, exclusive worker ownership, generation rebuilds, and alias-switch recovery.
Implement typed bounded adapter interfaces and total request deadlines.
Qualify actual OpenSearch acknowledgement loss, journal gaps, current ROM grants, paused-query revocation, and protected-field query rejection.
Execute native SQLite/redb consumers, independent Cargo archive consumers, dependency audits, and runtime review.
No multi-node, universal secret-erasure, distributed exactly-once, or complete-provider claim follows from these probes.
