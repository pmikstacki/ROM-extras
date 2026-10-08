# OpenSearch and vector projections assessment

Inspection date: 2026-10-08. Evidence category: source review only.

This report assesses approved Task 8. No service was downloaded or started. No runtime code or dependency was changed. Proposed fixtures and acceptance scenarios remain unexecuted. Neither provider is complete.

## Public ROM contract

The inspected source is published ROM `d7ef529040eec60dc869034c2d33130219db85fe`, through its immutable Cargo checkout. Private working-tree code was not used.

`Runtime::journal` returns bounded, authorized `JournalBatch` pages. Its `JournalView` exposes position and `ProjectedView`, not storage `JournalEvent.identity`. A cursor contains generation, Resource kind, and position. Cursor advancement includes filtered facts. Derive delivery identity from generation/kind/position; do not require unavailable raw rows or storage identity. [Published journal API](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/journal.rs).

`ProjectedView` carries Resource key, revision, and optional permitted fields. It cannot decode as a complete Resource. `read_projected` performs current disclosure checks and rejects tombstones. `query_spec_projected` gives bounded authorized state selection. These are the existing seams for rebuilding and final result disclosure. [Published projection API](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/projection.rs).

ROM checks query and sort grants during its own query planning. The planner is private; extras cannot call it as a general full-text permission preflight. Full-text and vector query semantics are not existing ROM `QuerySpec` operators. Define a host authorization contract for approved searchable fields and search modes. Do not bypass this gap through a private core function. [Published query normalization](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/query_eval/normalization.rs), [published query operators](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/query_spec.rs).

A caller-filtered feed cannot serve as a complete global index. Revocation can filter a historical fact while its position is still acknowledged. Use a dedicated, explicitly authorized projection actor for approved export fields. If that actor's grants change, invalidate or rebuild the generation; do not silently keep excluded old fields. Current caller authorization still applies to every result.

## Concrete candidates

| Component | Inspected source identity | Status |
| --- | --- | --- |
| OpenSearch server | `3.9.0`, release 2026-09-29, tag commit `4ee42a94e87f66fbf1e62a9871b1b87f91e02472` | Official current release candidate; Apache-2.0. |
| Qdrant server | `1.19.2`, release 2026-10-05, annotated tag resolves to commit `016542aa5deb6c66380bb137badf73d54f742bde` | Recommended separate vector service candidate; Apache-2.0. |
| OpenSearch Rust SDK | `opensearch 2.4.0`, released 2026-02-05 | Official SDK alternative; Apache-2.0; inspected manifest has no declared rust-version. |
| Qdrant Rust SDK | `qdrant-client 1.19.0`, released 2026-08-04 | Official gRPC SDK alternative; Apache-2.0; inspected manifest has no declared rust-version. |

Sources: [OpenSearch release](https://github.com/opensearch-project/OpenSearch/releases/tag/3.9.0), [OpenSearch tag API](https://api.github.com/repos/opensearch-project/OpenSearch/git/ref/tags/3.9.0), [server license](https://github.com/opensearch-project/OpenSearch/blob/3.9.0/LICENSE.txt), [Qdrant release](https://github.com/qdrant/qdrant/releases/tag/v1.19.2), [resolved tag](https://api.github.com/repos/qdrant/qdrant/git/tags/721cf8793c0ee5953273b9e1d1bf4613adcd34bc), [Qdrant license](https://github.com/qdrant/qdrant/blob/v1.19.2/LICENSE), [OpenSearch SDK manifest](https://github.com/opensearch-project/opensearch-rs/blob/v2.4.0/opensearch/Cargo.toml), [Qdrant SDK manifest](https://github.com/qdrant/rust-client/blob/v1.19.0/Cargo.toml).

Recommend the existing bounded HTTP approach with `reqwest 0.13.5` for both first profiles. Required operations have documented REST APIs. OpenSearch's SDK itself uses reqwest 0.13 and defaults to native TLS. Qdrant's SDK uses tonic/prost gRPC and default snapshot features can add reqwest. Avoid adding those feature graphs before a demonstrated need. This is an implementation choice, not a claim that SDK clients cannot enforce bounds. [OpenSearch SDK](https://github.com/opensearch-project/opensearch-rs/blob/v2.4.0/opensearch/Cargo.toml), [Qdrant SDK](https://github.com/qdrant/rust-client/blob/v1.19.0/Cargo.toml).

Reqwest 0.13.5 declares Rust 1.85 and MIT/Apache-2.0. Its defaults include Rustls, HTTP/2, and system proxies. Rustls defaults include aws-lc-rs; native TLS can require OpenSSL. Pin explicit features and audit their resolved graph. A declared client MSRV does not establish transitive MSRV. No new advisory, feature, license, native build, or minimum-Rust audit was executed. [Exact manifest](https://github.com/seanmonstar/reqwest/blob/v0.13.5/Cargo.toml).

Qdrant is selected as a distinct real vector profile because it provides bounded REST point operations, vector queries, and generation aliases. OpenSearch k-NN is an alternative, but qualifying it alone would not satisfy the plan's independently tested vector profile. pgvector is another possible later SQL profile. Neither alternative was selected or qualified here.

## Shared projection state and acknowledgement

Define `rom-projection-core` around public projected values and cursors. Keep mapping, transport, and backend error parsing in provider modules. Checkpoint state needs projection name, mapping/model version, physical generation, ROM generation/kind/position, and durable operation state. Own the checkpoint store in extras; do not write private ROM tables or treat checkpoint writes as Resource actions.

Admit one bounded page per projection worker. Order writes per Resource key and revision. Stable backend IDs derive from kind/id using an unambiguous encoding. Qdrant IDs accept UUID or unsigned integer forms; retain the original ROM key in approved payload metadata and verify identity after lookup. A digest-derived UUID requires collision handling. [Qdrant point identity](https://qdrant.tech/documentation/manage-data/points/).

Persist the checkpoint only after every operation in that page has confirmed backend success. A page cursor can acknowledge filtered facts even when no output operation exists. Failure preserves the earlier checkpoint. A crash after backend acceptance but before checkpoint publication causes replay. Replayed operations must not regress newer state. Index writes and local checkpoints are separate commits; do not claim distributed exactly-once delivery.

Before dispatch, bound page rows, encoded bytes, document bytes, vector dimensions, and concurrent calls. Bound streamed response bytes before parsing. Apply connect and total request deadlines. Disable implicit adapter retries initially. Suppressed acknowledgements retain unknown write outcomes and leave checkpoints unchanged. Sanitized errors must exclude credentials, source fields, embeddings, query text, and backend bodies.

## OpenSearch write and delete profile

Use stable document IDs and explicit mappings. Disable dynamic field creation in the first mapping. The Bulk API can report per-item failures despite an HTTP success. Inspect every item before checkpoint publication. External versioning rejects older revisions; equal revisions require a defined duplicate reconciliation policy. Avoid unconditional `external_gte` overwrite unless identical revision content is established. [Bulk API](https://docs.opensearch.org/latest/api-reference/document-apis/bulk/).

External versions are signed 64-bit values, while ROM revisions are u64. Define an overflow rejection or separate revision representation. Do not truncate. Physical deletes retain version metadata only temporarily; `index.gc_deletes` defaults to 60 seconds. Therefore, a late older insert can resurrect a physically deleted document after that window. Prefer persistent tombstone documents or a separately durable revision ledger. Remove searchable fields and vectors from tombstones. [Delete version lifetime](https://docs.opensearch.org/latest/api-reference/document-apis/delete-document/).

Use `index.translog.durability=request` for the local durable profile. This provides the documented fsync boundary before acknowledgement. Search visibility is separate: refresh controls when accepted writes become searchable. Qualify `refresh=wait_for` or explicit refresh for deterministic acceptance; do not imply ROM commits synchronously update search. [Index durability](https://docs.opensearch.org/latest/install-and-configure/configuring-opensearch/index-settings/), [index visibility](https://docs.opensearch.org/latest/api-reference/document-apis/index-document/).

## Qdrant revision and vector profile

An ordinary upsert overwrites an existing point ID. `wait=true` requests completion rather than only admission; inspect the returned operation status. `ordering=strong` serializes writes through a leader but does not compare ROM revisions. It can reduce availability when that leader is unavailable. These settings do not alone prevent stale delivery from replacing newer data. [Upsert API](https://api.qdrant.tech/api-reference/points/upsert-points), [ordering guarantees](https://qdrant.tech/documentation/scaling/consistency-guarantees/).

Qdrant's current docs describe conditional updates and update modes. Exact 1.19.2 source contains conditional upsert operations and update filters. Initial implementation must inspect its conditional check/application boundary or use a durable single-writer revision ledger. Do not advertise native revision fencing from a payload field alone. Nonexistent conditional upserts require special care: docs describe default insert behavior. [Conditional update guidance](https://qdrant.tech/documentation/manage-data/points/), [tagged operations](https://github.com/qdrant/qdrant/blob/v1.19.2/lib/collection/src/operations/point_ops.rs).

Specify one fixed vector dimension, metric, and model identifier per generation. Require finite numbers and bounded dimensions before dispatch. Use deterministic, host-supplied vectors in the first real-server fixture. This proves storage/query integration without claiming a production embedding model. Embedding acquisition is an independent bounded host boundary with its own privacy, version, and failure contract.

Physical point deletion loses the payload revision fence. Preserve a durable tombstone ledger or a qualified tombstone representation that cannot enter vector results. Test stale replay after deletion and restart. The actual delete acknowledgement and WAL persistence profile must be qualified against the executed service; no crash durability claim follows from source review alone.

## Rebuild, gaps, and generation switch

ROM documents head-then-snapshot-then-subscribe recovery, with intentional overlap reconciled by revision. Capture each kind's head. Scan bounded state pages with public `query_spec_projected` and stable ID continuation. Apply post-head events to the fresh backend generation. Paging is not an atomic global snapshot; revision reconciliation is mandatory. A retention or generation gap invalidates incremental continuity. Surface `HistoryGap`; do not silently skip to latest. [Published recovery contract](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/journal.rs), [published query selection](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/query_eval/read.rs).

Verify mapping, model, checkpoints, sampled values, and bounded completeness before switching. OpenSearch supports an atomic multi-action alias switch. Qdrant supports atomic collection alias changes. Neither transaction also commits the extras checkpoint manifest. Record a switch intent, perform the provider operation, and reconcile its actual alias target after restart. Keep prior generations until explicit retirement. [OpenSearch aliases](https://docs.opensearch.org/latest/api-reference/alias/aliases-api/), [Qdrant aliases](https://qdrant.tech/documentation/manage-data/collections/).

## Query security and eventual consistency

The backend is a candidate selector. For every candidate, call current `Runtime::read_projected` under the requesting actor. Drop denied, missing, tombstoned, or stale-revision results. Return current permitted values, not indexed source. Query output can be incomplete while the index lags. Apply a bounded candidate budget and return fewer results when authorization rejects candidates. Do not scan indefinitely to fill a requested count.

Current result redaction does not prevent search membership or score leakage from protected fields. Only index and query fields approved for this search profile. Require a current host query authorization decision before backend I/O. Do not expose backend scores, highlights, aggregations, total counts, debug plans, or stored embeddings by default. Arbitrary backend DSL, scripts, cross-index lookups, and user-supplied index names are outside the first query API.

Permission metadata in the index can prefilter candidates but cannot establish current ROM authorization. If search depends on a field newly revoked from the caller, fail before its query or use an explicitly authorized search policy. Do not claim universal protected-field security from a final `read_projected` call alone. Query revision mismatch must not return stale snippets or vectors. [Published disclosure and query grants](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/projection.rs), [published query planning](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/query_eval/normalization.rs).

Specify small typed queries, page/candidate limits, response-byte limits, vector limits, and total deadlines. Qdrant supports `with_vector=false` and bounded payload selection; OpenSearch supports restricted source retrieval. These reduce disclosure but do not replace ROM authorization. [Qdrant query API](https://api.qdrant.tech/api-reference/search/query-points).

## Real local fixture proposals

OpenSearch's official image candidate is `opensearchproject/opensearch:3.9.0`. Use single-node discovery and persistent data. Publish only a fixed loopback REST port, for example `127.0.0.1:55460` to 9200. Keep 9300, 9600, and Dashboards unpublished. Configure verified private TLS and a limited fixture user; do not adopt the docs' insecure curl example as acceptance. The docs require `vm.max_map_count=262144` and recommend at least 4 GB for Docker Desktop. Host-wide changes and memory availability need an explicit fixture preparation review. [Official Docker profile](https://docs.opensearch.org/latest/install-and-configure/install-opensearch/docker/).

Qdrant's official image candidate is `qdrant/qdrant:v1.19.2`. Its documented persistent mount is `/qdrant/storage`. Publish only `127.0.0.1:55461` to REST 6333. Keep gRPC and peer ports unpublished. Exact tagged config defaults to no TLS and no configured API key. Override with verified private TLS, a private write key, and a separate query key where applicable. Restrict request size. Disable unused gRPC through an explicit null configuration, not by omitting an overlay entry. [Official quickstart](https://qdrant.tech/documentation/quickstart/), [tagged configuration](https://github.com/qdrant/qdrant/blob/v1.19.2/config/config.yaml).

No immutable image digest, minimum viable fixture CPU/RAM allowance, actual TLS setup, or service restart was verified. Qdrant's installation guidance requires POSIX-compatible persistent block storage; memory depends on vectors and indexing. Choose bounded datasets and record measured resource limits. Do not claim production cluster behavior from single-node tests. [Qdrant installation](https://qdrant.tech/documentation/installation/).

## Acceptance and open work

Write a family spec before runtime changes. Implement the shared worker/checkpoint/rebuild/query boundary, OpenSearch, and one Qdrant profile. Run both against real pinned services with public ROM SQLite and redb consumers.

Required evidence covers duplicate delivery; crash after backend acceptance before checkpoint; old revision after newer revision; physical/tombstone deletion; replay after delete retention; unavailable services; bounded bulk errors; and verified restart recovery. Force a real journal retention gap. Rebuild a new generation and verify switch recovery after an interrupted alias acknowledgement.

Authorization tests must revoke Resource and field access while the backend request is paused. Verify current disclosure rejects the result. Include a protected-field search that is rejected before backend I/O. Inspect bounded errors and controlled captures for secret data. Keep explicit eventual-consistency limits and test candidate-budget exhaustion.

Packaged consumers, exact compiler/lockfile/features, native dependency audits, service versions/digests/configuration, full local verifier, and review remain required. Real service ACK-loss and multi-node consistency need distinct evidence profiles. Source review and successful mocks cannot complete this family.
