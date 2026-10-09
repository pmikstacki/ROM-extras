# Native Rust OpenSearch write qualification

Inspection date: 2026-10-09. Baseline: 69ab0e4ef3855a4d91f3620ff1743f1d62ce66ca.
This increment qualifies fixed-generation writes over the accepted shared projection worker. Task3 and the full ROM-extras goal remain incomplete.

The [execution record](opensearch-rust-write-2026-10-09.json) identifies source hashes, archive hashes, service settings, compiler, commands and preserved failures.
The full command, RUST_TEST_THREADS=1 ./scripts/check-all, exited zero. All 236 frozen runtime inputs remained unchanged.
The [full output](opensearch-rust-full-verifier-serial-2026-10-09.log) includes every required gate. No absent-service skip or deadline relaxation was added.
A single harness thread changes test scheduling. The scenarios retain their own concurrency and existing deadlines.
Source: [Rust test harness execution](https://doc.rust-lang.org/rustc/tests/index.html).

## Implemented behavior

Verified mTLS uses a fixed HTTPS origin, no system proxy, no redirects and no implicit retry.
Request bodies and streamed responses are bounded to 1 MiB. A page admits at most 64 operations.
Strict external versions reject delayed older writes. Revisions above i64::MAX fail before dispatch.
Equal-version replay requires exact real-time stored-source equality, including the original key and complete selected values.
Persistent tombstones retain the revision fence and remove selected values and searchable fields.
Explicit mappings store other selected JSON values without indexing, including u64::MAX. Empty indexed-field configurations are qualified.

Generation inspection checks native mappings, request durability and a stable UUID during the adapter lifetime.
One total deadline covers creation and subsequent metadata inspection. Unknown creation requires inspection, not blind recreation.
Read-only inspect_prepared establishes native state without replay or local checkpoint publication.
Write acknowledgement does not establish search refresh visibility.

Sources: [Bulk external versions](https://docs.opensearch.org/latest/api-reference/document-apis/bulk/), [real-time retrieval](https://docs.opensearch.org/latest/api-reference/document-apis/get-documents/), [object mapping](https://docs.opensearch.org/latest/mappings/supported-field-types/object/), [Tokio timeout](https://docs.rs/tokio/1.53.1/tokio/time/fn.timeout.html).

## Executed coverage

Twenty adapter cases passed. Six use controlled mTLS sockets for redirects, response limits, malformed JSON, deadlines, wrong roots and owned fixture readiness.
Actual OpenSearch cases cover partial HTTP200 Bulk failure, equal replay/conflict, stale writes, tombstones, signed bounds and exact unindexed values.
A verified relay discards actual acknowledgements or holds actual requests beyond the client timeout. Native state is read before replay.
Actual service restart preserves live and tombstone documents and the generation UUID. A late old request cannot regress newer live or tombstone state.
Public ROM SQLite/redb history and native checkpoint reopen recover pending operations after actual acknowledgement loss.

An independent public consumer passed one native case. Three extracted Cargo archives passed their isolated consumer and Clippy checks.
Extracted projection core also passed fourteen native history cases twice, including the locked repeat.
The final review found no actionable source issue. The reviewer ran no tests or service operations.
Earlier findings required one total generation deadline, bounded owned fixture startup and canonical empty-object mapping. Their regressions are retained.

## Dependencies and limits

Root and public locks add only the local rom-opensearch package; existing registry versions and checksums remain unchanged.
The extracted consumer resolves 193 packages, 98 more than the previous history-only consumer. Those added versions already exist in the root graph.
Fresh root, public and archive audits found no known vulnerabilities. The public graph retains its unsuppressed paste maintenance warning.
Inventories record declared licenses, features and Rust requirements. Declared minima fit the executed Rust 1.99 toolchain.
Only the temporary archive consumer lacks license metadata. Manifest inspection is not completed redistribution review or native-service security assessment.

Three earlier full attempts failed. One rejected native index setup; its cause remains unresolved after a passing focused reproduction.
Later attempts failed unchanged NATS acknowledgement witnessing and core lifecycle test timeouts. Focused unchanged cases passed.
The serial full result does not explain these earlier failures or qualify default parallel scheduling under host contention.
An earlier native initialization returned Unknown while actual settings confirmed created indexes. Test setup reconciles through bounded read-only inspection.
Initial compilation and fixture-path failures are retained separately from intended behavioral RED evidence.

The supported evidence is a persistent single-node OpenSearch 3.9.0 fixture with two CPUs, 2 GiB memory and 512 MiB heap.
The writer gains only two scoped metadata-read actions. Private TLS keys and credentials are excluded from publication.
This increment does not implement typed search candidates, current query authorization, durable generation manifests or alias switching.
Qdrant Rust transport remains unfinished; its existing protocol probes are separate evidence. No multi-node durability or production capacity is claimed.
