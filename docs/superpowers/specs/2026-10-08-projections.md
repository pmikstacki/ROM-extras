# Authorized OpenSearch and vector projections

Date: 2026-10-08. Status: implementation contract for approved Task 8; runtime implementation remains absent.

## Goal and evidence

Implement rebuildable, eventual read models using shared projection orchestration and independently qualified OpenSearch and Qdrant adapters.
Use only public ROM interfaces from `d7ef529040eec60dc869034c2d33130219db85fe`.
The [assessment](../../research/projections-assessment.md) records primary sources, exact candidate versions, alternatives, and qualification limits.
User approval covers execution. Every later choice still requires primary web research and a decision-log entry.

## Ownership and public interface

ROM remains authoritative for mutations, revisions, authorization, and current returned values.
Extras owns projection checkpoints and backend generations. It must not write private ROM tables.
A dedicated projection actor must have approved export grants. A caller-filtered feed cannot establish global index completeness.
Changing export grants or the mapping/model identity invalidates the generation until rebuild completes.
The host must provide an explicit search policy for approved fields and modes.
Authorize each query before backend I/O. Redacting results alone does not prevent membership leakage from a protected search field.

Expose typed full-text and fixed-dimension vector queries. Do not accept raw provider DSL, arbitrary index names, scripts, or cross-index requests.
Each candidate carries its original Resource key and indexed revision.
Return only current `Runtime::read_projected` values under the requesting actor.
Discard missing, denied, tombstoned, and revision-mismatched candidates.
Do not return backend scores, highlights, counts, aggregations, embeddings, or indexed source fields.
A bounded candidate budget can produce fewer results than requested. Do not scan indefinitely.

## Shared durable protocol

Persist projection identity, mapping/model identity, physical generation, per-kind ROM cursors, per-key revision state, and pending operations.
Admit one bounded page at a time per generation worker.
Validate all operations before network dispatch. Preserve missing, null, false, zero, and empty values distinctly.
Persist an operation intent before sending its backend write.
Advance the checkpoint only after every page operation has confirmed success and its durable local state is committed.
An empty authorized page can still advance its cursor over filtered facts.
Unknown outcomes retain the pending intent and the earlier checkpoint.
Crash after backend acceptance requires replay or authoritative reconciliation, not an exactly-once claim.
Equal-revision replay must establish identical content. Conflicting equal-revision content is an invariant failure.
Older revisions cannot overwrite newer values or resurrect tombstones, including after restart and delayed acknowledgements.

A local single-writer ledger alone does not fence a previously submitted remote request.
A timed-out request can complete after the next request. Qdrant needs a qualified conditional server-side fence or terminal-operation reconciliation.
Do not dispatch a newer same-key write while an unresolved older operation can still regress it.
This is an admission requirement, not an implementation claim.

Use unambiguous stable IDs. Enforce OpenSearch's 512-byte document-ID limit before I/O.
For Qdrant digest-derived UUIDs, persist and verify the original key. A different key at the same ID is a collision error.
Retain durable tombstone revision state. Remove searchable fields and vectors from tombstone representations.
Do not rely on OpenSearch's temporary physical-delete version retention.

## Bounds and transport

The first fixture profile uses at most 64 operations per page, 16 KiB per document, and 1 MiB per encoded request or response.
The vector profile admits 1 through 4096 finite dimensions. A generation fixes dimension, metric, and model identifier.
Query limits are 1 through 64 results and at most 256 candidates.
These are conservative application policy choices, not vendor maximums. Measure them in real-service qualification.
Use a 5-second default total request deadline and a connect deadline no greater than 3 seconds.
Allow configuration only within documented finite bounds; reject zero limits and arithmetic overflow.
Use verified HTTPS, fixed approved origins, private credentials, bounded streamed parsing, and explicit admission limits.
Disable system proxies, redirects, and implicit transport retries in the first profile.
Errors must be fixed classifications without request bodies, query text, source fields, credentials, vectors, or backend bodies.

## Provider-specific guarantees

OpenSearch candidate: `3.9.0`. Use explicit mappings with dynamic field creation disabled.
Use external version comparison. Reject ROM revisions that exceed the signed backend range; never truncate.
Inspect all Bulk item results, including when HTTP status reports success.
Reconcile equal-version conflicts by approved metadata/content identity rather than treating all conflicts as successful replay.
Use persistent tombstone documents and request translog durability. Search refresh is a separate visibility boundary.

Qdrant candidate: `1.19.2`. Use REST with `wait=true` and inspect completion status.
Strong write ordering alone does not compare ROM revisions.
Inspect tagged conditional-update source and qualify missing-point, concurrent-update, delete, and delayed-request cases before selecting revision fencing.
Use deterministic host-provided vectors in fixtures. This does not qualify a production embedding model.

## Rebuild and recovery

Capture each kind's journal head, then scan bounded public projected state pages, then consume post-head history.
Reconcile snapshot/event overlap by revision. Do not claim an atomic global snapshot.
Surface retention or generation gaps as `HistoryGap`. Do not silently skip lost history.
Create a new physical generation for rebuild. Validate mapping, model, checkpoints, bounded completeness, and values before publication.
Persist alias-switch intent before the provider switch. Reconcile actual alias targets after interruption.
Backend alias changes and the local manifest are separate commits.
Preserve previous generations until explicit retirement.

## Qualification

Execute both real services with persistent storage, verified private TLS, fixed loopback ports, and immutable image digests.
Proposed REST ports are 55460 for OpenSearch and 55461 for Qdrant; verify availability before starting services.
Record exact settings and resource limits. Do not modify global host settings or stop unrelated services.
The observed host `vm.max_map_count` is 1048576; no increase is required by the documented OpenSearch 262144 minimum.
This host observation does not establish sufficient service memory or production readiness.

Required scenarios: duplicates, conflicting same revision, stale revision, tombstones, delayed stale writes, partial bulk errors, unavailable service, response limits, and deadlines.
Inject actual acknowledgement loss and crash after acceptance before checkpoint publication.
Reopen local checkpoints and restart each backend. Verify exact state, cursor, and pending-operation recovery.
Force a real ROM journal gap. Rebuild and interrupt alias acknowledgement, then reconcile the actual target.
Test current Resource and field revocation while a query is paused, plus denied protected-field search before backend I/O.
Run native public consumers on SQLite and redb and independent Cargo archive consumers.
Inspect controlled captures for credentials and protected values. Record exact source, lockfiles, compiler, features, commands, and raw output.
Run affected checks, dependency audits, independent review, and the full local verifier before integration.
Do not advertise multi-node guarantees from single-node fixtures.

## Native-fence follow-up evidence

The [Qdrant report](../../research/qdrant-revision-fencing.md) identifies conditional upsert under a native submission fence.
Select strict comparison of scalar u32 revision halves and whole-point writes for the single-node candidate profile.
Persist same-ID tombstones with `rom_live=false` and an empty named-vector map.
The [exploratory probe](../../verification/qdrant-fence-probe-2026-10-08.json) confirms these representations on the actual server, including u64::MAX.
HTTP completion can represent a rejected-condition no-op. Reconcile stored identity, revision, and deterministic content before local publication.
Actual delayed-request, lost-acknowledgement, tombstone-restart, and ROM integration cases remain acceptance requirements.
