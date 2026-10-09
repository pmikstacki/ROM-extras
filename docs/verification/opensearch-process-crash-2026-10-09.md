# OpenSearch process interruption acceptance

The full local verifier completed with exit code 0 on 2026-10-09.
The [machine record](opensearch-process-crash-2026-10-09.json) contains compiler details, 255 source hashes, archive hashes, dependency inventories, and audits.
The [raw output](opensearch-process-crash-full-2026-10-09.log) preserves executed checks.

Runtime inputs started from baseline `5da2a332686793ece46e109d8f554cf716463235` plus the recorded uncommitted increment.
README-only commit `279356fe995677bc4c51bb74b8af0e6cf3ac7219` was published during the run.
All 255 frozen runtime files remained unchanged. Both lockfiles match the runtime baseline.

## Executed recovery

The actual OpenSearch child accepts live and tombstone revisions, loses the response, and retains a durable pending intent.
It confirms remote acceptance through an independent reader, then exits with code 86 without Rust destructors.
The parent restarts the persistent backend and inspects exact documents before replay.
Reopened SQLite/redb Runtime history reconstructs the pending page. Recovery publishes its cursor and clears the intent.
A second clean checkpoint reopen confirms the cursor, revisions, tombstones, and document digests.

The native process cases passed twice. Their ignored child entry is explicitly invoked by both parents.
Native typed search passed 14 cases. The independent OpenSearch consumer passed 17 cases.
Extracted Cargo archives executed the same process cases outside workspace feature unification.
Core native history and search suites passed, including their locked repetitions.

The run used `RUST_TEST_THREADS=1 ./scripts/check-all`, required fixture environments, and the configured Python executable.
It qualifies controlled single-node backend restart and explicit process exit.
It does not qualify power loss, SIGKILL, multi-node operation, or default parallel capacity.

## Requirement trace

Independent source review mapped every Task 2 and Task 3 requirement to existing and new evidence.

| Requirement | Implementation or evidence |
| --- | --- |
| Bounded inputs, revisions, original keys, stale and equal-revision cases | Core boundaries, documents, checkpoint recovery, worker and native-history suites |
| Intended initial failures | Previously published checkpoint, document and worker verification records |
| Immutable documents, durable intents, atomic cursor publication | Core document, metadata, checkpoint and worker modules |
| Accepted remote write followed by process interruption and reopen | Shared process crash case, both native storage profiles and extracted consumer |
| Cross-process writer ownership | Checkpoint ownership tests, including supported path aliases |
| Mixed Bulk failure and version conflicts | Native partial Bulk tests and OpenSearch service tests |
| Verified bounded transport, mappings and tombstones | OpenSearch transport, mapping, writes and hostile-response tests |
| Durability independent of refresh | Native restart checks and separate search visibility checks |
| Response loss and late stale writes | Native recovery, process crash, and delayed-operation probes |
| Required format, Clippy, tests, docs and full verifier | Successful complete raw log and unchanged source freeze |

Task 2 and Task 3 are accepted for their declared profiles.
Qdrant Rust transport, both-provider query acceptance, and generation switching remain incomplete.

## Preserved failures and correction

Earlier process fixtures incorrectly assumed four disclosed journal views and permitted tombstone reads.
They now use the actual journal head and the public ROM denied-read contract.
Archive compilation exposed an unavailable snapshot method and duplicate module declarations.
The consumer now uses public `StorageWorker::load` and one shared process helper.

An earlier full run exited 101 when a new index waited during recovery of hundreds of retained fixture indices.
The exact-index deadline remains 45 seconds. A separate aggregate readiness preflight allows 90 seconds before generation creation.
Historical fixtures, indices, source snapshots, and failed logs remain preserved.
Production request deadlines and service capacity did not change.

Root, independent consumer, and extracted archive audits report no vulnerabilities.
The public graph retains the unsuppressed informational unmaintained `paste` advisory.
Only the temporary packaged consumer lacks license metadata; the actual extras crates declare MIT.

The readiness decision follows the [OpenSearch cluster-health API](https://docs.opensearch.org/latest/api-reference/cluster-api/cluster-health/).
The interruption scope follows [Rust process exit semantics](https://doc.rust-lang.org/std/process/fn.exit.html).
Pre-replay inspection uses the [OpenSearch document GET API](https://docs.opensearch.org/latest/api-reference/document-apis/get-documents/).
