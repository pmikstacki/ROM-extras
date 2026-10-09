# Authorized projections implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement shared durable projection orchestration, OpenSearch full-text search, and a separately qualified Qdrant vector profile.

**Architecture:** Public ROM projected values feed an extras-owned durable worker. Provider adapters hide transport and native concurrency details. Query orchestration authorizes search before selecting candidates and returns only current ROM disclosure.

**Tech Stack:** Rust 1.99, edition 2024, public ROM `d7ef529040eec60dc869034c2d33130219db85fe`, reqwest 0.13.5 with explicit TLS features, OpenSearch 3.9.0, Qdrant 1.19.2.

**Spec:** [Authorized projections](../specs/2026-10-08-projections.md).

## Global constraints

- Keep crate roots and module roots as facades. Preserve existing public paths.
- Use no private ROM working-tree code or direct core-table mutation.
- Admit at most 64 operations/page, 16 KiB/document, and 1 MiB/request or response in the initial profile.
- Vectors contain 1 through 4096 finite dimensions, fixed per generation.
- Queries request 1 through 64 results and inspect at most 256 candidates.
- Default total request deadline is 5 seconds; connect deadline is at most 3 seconds.
- Research each decision using primary web sources. Record alternatives and uncertainty.
- Real fixtures, native consumers, packaged consumers, audits, review, and full local verification are mandatory.

## Review focus

- An old remote request can finish after local timeout; it must not regress a newer revision. Task 1 and Task 4 qualify the fence.
- Equal revision with different content must fail, including replay after restart. Task 2 and Task 3 pin this invariant.
- A query can leak protected-field membership despite final redaction. Task 5 rejects it before backend I/O.
- Filtered journal pages can advance without output; Task 2 preserves their cursors without inventing events.
- A backend alias switch can commit before local publication; Task 6 reconciles interruption instead of blindly repeating the switch.

## File ownership and layout

Root owns all implementation, specs, plans, fixtures, and verification records.
The research agent owns only `docs/research/qdrant-revision-fencing.md` during its current assignment.

Create `crates/rom-projection-core/src/{lib,error,limits,identity,document,checkpoint,worker,query,rebuild}.rs`.
These modules respectively export the facade, sanitized errors, bounds, stable identities, approved values, durable intents, page delivery, current disclosure, and generation recovery.
Create `crates/rom-opensearch/src/{lib,config,transport,mapping,writes,search,aliases}.rs` and corresponding `rom-qdrant` modules.
Keep provider-native request construction and acknowledgement parsing outside the core.
Create `tests/public-consumer/tests/projections.rs` and focused helper modules under `tests/public-consumer/tests/projection_fixture/`.
Create required scripts `scripts/check-projections` and `scripts/check-projection-package`.
Extend the existing `scripts/check-all` only after required actual fixtures can execute.
Do not add empty crates as completed deliverables.

### Task 1: Qualify remote revision fencing and fixtures

**Files:** `docs/research/qdrant-revision-fencing.md`, `docs/research/decision-log.md`, private `.superpowers/projection-fixture/`, public `docs/verification/projection-fixtures-2026-10-08.json`.

**Interfaces:** Consumes exact tagged provider source and REST contracts. Produces an evidence-backed provider write protocol before public Rust signatures are frozen.

- [ ] Inspect Qdrant conditional-update check/application locks, missing-point behavior, and revision numeric precision.
- [ ] Record the selected native fence and alternatives. If the source does not establish a fence, retain unknown outcomes and qualify terminal reconciliation before newer writes.
- [ ] Inspect available memory, disk, ports, and existing containers. Resolve official image digests without modifying unrelated services.
- [ ] Create persistent loopback-only services with private verified TLS and limited credentials.
- [ ] Run actual concurrent newer/older writes, lost acknowledgement, same-revision conflict, missing-point insertion, tombstone replay, and restart cases.
- [ ] Preserve failures and exact service configuration. Gate adapter implementation on a protocol that satisfies the spec.

### Task 2: Durable bounded projection worker

**Files:** `crates/rom-projection-core/`, `Cargo.toml`, `Cargo.lock`, `crates/rom-projection-core/tests/{boundaries,checkpoint_recovery,worker}.rs`.

**Interfaces:** Consumes the Task 1 fenced provider operation. Produces `Worker::apply_page` with durable intents and a committed cursor; `Worker::recover` resolves pending operations before accepting later same-key work.
Define the concrete checkpoint transaction interface from the qualified recovery protocol, before writing its callers.
The [checkpoint transaction contract](../specs/2026-10-08-projection-checkpoints.md) now specifies target signatures, result categories, bounds, and format.
Implement page transactions first. Reserved switch tags reject unsupported work until Task 6 format qualification.
Accepted dirty-file recovery is required in Task 2; temporary fail-closed behavior is not its final acceptance.

- [ ] Write boundary and recovery tests first: oversize input, overflow, zero revision, key collision, stale revision, equal-revision mismatch, filtered empty page, and interrupted checkpoint commit.
- [ ] Run the tests and preserve their intended initial failure.
- [ ] Implement validation, immutable approved document identity, durable operation intent, and atomic local checkpoint publication.
- [ ] Test crash after acceptance before checkpoint, reopen, replay, and unknown outcome without cursor advancement.
- [ ] Test exclusive writer admission across processes. A process mutex alone is insufficient.
- [ ] Run format, denied-warning Clippy, tests, doctests, and docs before committing the deliverable.

### Task 3: OpenSearch adapter

**Files:** `crates/rom-opensearch/`, `crates/rom-opensearch/tests/service.rs`, workspace manifests and lockfile.

**Interfaces:** Implements the Task 2 fenced write seam; returns original keys and indexed revisions as bounded candidates.

- [ ] Write real-service failing tests for partial Bulk errors, external-version conflict, signed revision overflow, and equal-revision content mismatch.
- [ ] Implement verified bounded HTTP transport, explicit mappings, stable IDs, per-item acknowledgements, and persistent tombstone documents.
- [ ] Qualify request durability separately from refresh visibility.
- [ ] Inject acknowledgement loss and late stale writes. Verify newer and tombstoned state after backend and checkpoint restart.
- [ ] Run provider and core checks and commit only with exact actual-service evidence.

### Task 4: Qdrant adapter

**Files:** `crates/rom-qdrant/`, `crates/rom-qdrant/tests/service.rs`, workspace manifests and lockfile.

**Interfaces:** Implements the same Task 2 write seam with Task 1's qualified revision fence. Accepts fixed-model host vectors and returns key/revision candidates without scores or vectors.

- [ ] Write real-service failing tests for delayed old operations, missing-point conditional insertion, same-revision mismatch, collision, invalid vectors, and tombstone replay.
- [ ] Implement the qualified conditional protocol, bounded point operations, completion-status parsing, and vector query profile.
- [ ] Verify `wait=true` completion separately from local checkpoint durability.
- [ ] Restart persistent backend and checkpoints after lost acknowledgement. Verify exact revisions and no resurrection.
- [ ] Run provider and core checks and commit with separate Qdrant evidence.

### Task 5: Native public ROM feed and authorized search

**Files:** `examples/projections/`, public consumer feature and `tests/projections.rs`, `tests/projection_fixture/{mod,resources,authorization,capture}.rs`.

**Interfaces:** Consumes public `Runtime::journal`, `query_spec_projected`, and `read_projected`; the host supplies current search authorization before provider selection.

- [ ] Write tests on both SQLite and redb for page delivery, duplicate replay, filtered cursor progress, and forced retention gaps.
- [ ] Implement public feed mapping with explicit export policy and current query authorization.
- [ ] Pause actual query response, revoke Resource or field access, resume, and assert current permitted output only.
- [ ] Assert a denied protected-field query makes zero backend calls.
- [ ] Assert stale candidates, finite candidate budgets, and controlled capture sanitization for both providers.
- [ ] Run independent consumer compilation, native cases, and Clippy.

### Task 6: Rebuild and generation switch recovery

**Files:** core `rebuild.rs`, provider `aliases.rs`, `tests/projection_fixture/rebuild.rs`.

**Interfaces:** Consumes public head/snapshot/history and Task 2 checkpoints. Produces a generation manifest only after actual alias reconciliation.

- [ ] Write tests for snapshot overlap, mutation during scan, retention gap, changed export grants, and interrupted alias acknowledgement.
- [ ] Implement fresh-generation population, revision reconciliation, completeness checks, durable switch intent, and actual alias inspection after restart.
- [ ] Verify previous generations remain available until explicit retirement.
- [ ] Execute all scenarios on both actual backends and both ROM storage profiles.

### Task 7: Package and full verification

**Files:** `scripts/check-projections`, `scripts/check-projection-package`, `scripts/check-all`, package consumer fixtures, `docs/projections.md`, README, and `docs/verification/`.

**Interfaces:** Consumes the completed runtime and native examples. Produces reproducible evidence for the declared profiles, without claiming other providers or multi-node support.

- [ ] Build independent consumers from extracted Cargo archives with normalized manifests.
- [ ] Run actual backend smoke tests outside workspace feature unification.
- [ ] Audit resolved graphs, features, licenses, advisories, compiler compatibility, and declared Rust minimum.
- [ ] Add required gates without absent-service skips. Freeze runtime inputs before the full verifier.
- [ ] Run independent review and address findings. Run the full local verifier and preserve raw output.
- [ ] Record exact hashes, service digests/settings, lockfiles, compiler, commands, limits, and failures.
- [ ] Commit and publish the verified branch increment. Keep the whole ROM-extras goal active until all original families are complete.

## Self-review

All spec requirements have task owners. Task 1 intentionally precedes concrete worker transaction signatures: the remote fencing protocol is not yet qualified.
Task 2 must finalize those signatures in this plan before implementation. This document does not claim that an unresolved protocol is executable.
Tasks 3 and 4 independently qualify backend guarantees; Tasks 5 and 6 establish public ROM integration and recovery.
Task 7 covers package consumers and the repository-wide acceptance gates.

## Task 1 execution checkpoint

The native Qdrant conditional protocol has source and actual single-node fencing evidence, including delayed requests and response loss.
Both immutable service images run with persistent verified TLS fixtures. OpenSearch has actual version, Bulk, scope, and restart evidence.
The [probe guide](../../projection-probes.md) states the exact tested scope and preserved failures.
The required `scripts/check-projection-probes` gate is included in `scripts/check-all`.
This completes fixture preparation and the Qdrant protocol experiments, not Task 2 or the whole family.
The [checkpoint contract](../specs/2026-10-08-projection-checkpoints.md) specifies the concrete transaction and exact-observation seam before Task 2 callers.
OpenSearch response-loss injection remains an adapter qualification requirement in Task 3.

## Task 2 checkpoint-store increment

`rom-projection-core` implements bounded validated metadata and actual redb page intent/completion transactions.
Controlled native faults distinguish absent commits from already synchronized commits, retire the handle, and reconcile exact tokens after reopen.
Actual child-process exit exercises dirty accepted-format recovery. Future dirty formats retain unchanged original bytes.
Independent review required retained completion metadata to validate resulting cursors and affected keys; actual malformed-state regressions cover the correction.
The [guide](../../projection-checkpoints.md) keeps the exact limited scope and missing runtime work explicit.
The bounded worker, cancellable startup lifecycle, retained authorized-history reconstruction, provider integration, and full Task 2 acceptance remain pending.
No Task 2 checkbox is complete merely because the checkpoint store exists.

## Task 2 startup cancellation increment

The store now accepts a shared monotonic cancellation token for open/reopen.
Checks occur at admission, 256-record scan boundaries, and 64-KiB copy boundaries. Native repair has cooperative abort callbacks.
This supplies startup lifecycle support; it does not complete worker admission, shutdown/join, or retained authorized-history reconstruction.

## Task 2 bounded storage-owner increment

`StorageWorker` owns dedicated native execution with one queued operation, typed checkpoint methods, asynchronous responses, and shutdown/drop join.
It reuses the existing executor with fixed private captures. A dropped response retains durable accepted work.
Actual files cover durable intent, atomic publication, queue overload, dropped response, job panic, and ownership after join.
Initialization and destruction still require the host's bounded blocking context. The asynchronous host lifecycle bridge is not yet implemented.
The storage thread is not the full page worker; history reconstruction, immutable approved documents and actual provider orchestration remain pending.

## Task 2 pending-history metadata increment

`PendingHistory` reconstructs the immutable old interval from bounded public-shaped authorized journal batches and host mapping metadata.
It validates overrun, preserves historical tombstones, collapses repeated keys and requires full intent equality before returning.
It retains no projected values and performs no checkpoint writes. Actual files verify that failed reconstruction preserves pending intent and old cursor.
This component is not actual SQLite/redb feed qualification or the full worker. Approved document mapping, dispatch orchestration and host lifecycle remain pending.
