# Native typed OpenSearch search qualification

Inspection date: 2026-10-09. Baseline: 4e043b7665d5bb20cc2e27254dcb37e7e220cbc9.
This increment adds actual OpenSearch SearchTarget. The full ROM-extras goal remains incomplete.

The target accepts only core-approved bounded text queries. It binds the profile, physical generation and indexed field before I/O.
AND/OR match queries filter live documents, kind and profile. Only original-key and revision metadata become candidates.
Complete response validation rejects timed-out, partial, early-terminated, foreign, duplicate or malformed candidates.
Current public ROM reads supply values; backend document values and scores are not returned.
One provider deadline covers generation inspection, retrieval, parsing and final inspection.

Ten actual-service cases cover SQLite/redb current hydration, stale/deleted filtering, explicit modes and finite candidate budgets.
A verified relay holds actual four-candidate responses while tests revoke query, row or field grants without Resource mutation.
Protected-field denial produces zero provider requests. Current reads prove an allowed field remains visible when the queried field becomes hidden.
Four parser tests cover controlled protocol failures; these are separate from real-service evidence.
Eleven independent consumer cases repeat native writes and search. Normalized archives repeat the same shared native search fixture.
Extracted core search and native history run twice, including locked repeats. Source-only review reports no actionable finding.

Native setup now waits for the exact index's primary shard through read-only verified mTLS ClusterHealth.
Earlier full and isolated failures are retained. Creation metadata existed before primary activation; production Unknown semantics remain unchanged.
The corrected isolated restart→recovery sequence passed both stores. Readiness evidence observed zero→one active primary after 16,339 ms.
HTTP408 is a pending health-wait response in OpenSearch3.9. Production deadlines and writer permissions remain unchanged.

The [full verifier](opensearch-native-search-full-2026-10-09.log) exited zero with all 252 frozen inputs unchanged.
The [execution record](opensearch-native-search-2026-10-09.json) records compiler, sources, archives and three fresh dependency audits.
All three graphs report zero vulnerabilities. The public graph retains the unsuppressed paste maintenance advisory.
No registry package versions or checksums changed; public lock changes only local dependency edges.
Serial harness qualification does not establish default parallel scheduling capacity or a multi-row authorization snapshot.
Qdrant Rust transport, vector queries, durable generation manifests and alias switching remain unimplemented.
