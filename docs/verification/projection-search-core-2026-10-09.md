# Typed search core qualification

Inspection date: 2026-10-09. Baseline: f71d8b406d7a759d3432eb88f6be9df4a555819f.
This increment implements shared search orchestration. Native OpenSearch search transport and the full ROM-extras goal remain incomplete.

## Contract and executed coverage

A trusted asynchronous host policy approves a fixed kind, field, mode and generation before backend I/O.
Only the core can construct ApprovedTextQuery. A compile-fail doctest checks the public construction boundary.
The coordinator checks current approval after candidate retrieval and before successful publication.
Candidates carry original keys and revisions. Runtime::read_projected supplies current values through public ROM v0.0.3.
Missing, denied, deleted, stale or field-hidden candidates are omitted. Invalid candidate pages fail before hydration.
One bounded page admits at most 256 candidates and returns at most 64 current views. Query text admits at most 4096 bytes.
Query and candidate Debug output excludes text and identifiers. Existing metadata Debug output remains unchanged after the shared macro refactor.

Twenty-three tests exercise real public SQLite/redb Resources with a controlled candidate target.
They cover current hydration, row and field revocation without revision changes, paused-query revocation, closed-source errors and invalid pages.
Protected-field scope denial makes zero backend calls. Bounds and sensitive Debug checks also pass.
The independent public consumer repeats all 23 cases. Extracted Cargo packages repeat all 23 cases twice, including --locked.
Extracted native history also repeats fourteen cases twice. Two compile-fail doctests pass.
Source-only review found no actionable issue. The reviewer ran no tests or service operations.
A review corrected the locked package invocation. Historical search package output ran 21 cases once; it did not establish a locked search repeat.

## Evidence and limits

The [execution record](projection-search-core-2026-10-09.json) identifies compiler, source hashes, archive hashes and fresh dependency inventories.
The [full verifier](projection-search-full-verifier-2026-10-09.log) exited zero with all 243 frozen inputs unchanged.
The command uses RUST_TEST_THREADS=1; scenarios retain their own concurrency and existing deadlines.
This does not qualify default parallel scheduling under host contention. Earlier write qualification failures remain preserved.
Root and public locks are byte-identical to the accepted baseline. Root/public/archive audits report zero known vulnerabilities.
The public audit retains its unsuppressed paste maintenance warning. Only the temporary archive consumer lacks license metadata.
Inventories record declared licenses, Rust requirements and selected features; this is not a completed redistribution review.
Native candidate retrieval is controlled in this increment. Existing native OpenSearch write checks do not qualify native search.
Host policy is trusted and must use current authority. This contract does not establish an atomic multi-row authorization snapshot.
Concurrent changes after each authoritative read are outside that guarantee. Stale or denied candidates may yield shorter results.

The initial API/lint/captured-policy compilation failures are separate from the intended 18-case behavioral RED.
An intermediate fixture reused an unchanged replacement, which public ROM correctly treated as a no-op.
Changing its title created the intended stale revision. Both failed and corrected outputs are retained.

Decisions use [public ROM hydration](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/projection.rs),
[OpenSearch match modes](https://docs.opensearch.org/latest/query-dsl/full-text/match/) and
[finite search controls](https://docs.opensearch.org/latest/api-reference/search-apis/search/).
These sources guide the interface; native OpenSearch filter behavior still needs an executed test.
