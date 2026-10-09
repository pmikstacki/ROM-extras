# Native search deadline and mapping-drift qualification

Inspection date: 2026-10-09. Baseline: cafbaadfe3f383fad3ff391e7aa49eea1557da5e.
This increment adds four real-service tests and shares administrative fixture TLS construction. Production transport and public APIs are unchanged.

For SQLite and redb, a verified relay holds an actual successful four-candidate search response beyond the five-second provider deadline.
Search returns `TargetFailure::Unknown`, without publishing values. Three requests establish initial metadata inspection and one native retrieval, without retry.
The test observes completion between 4.5 and 7 seconds. This allowance is a test scheduling bound, not production capacity qualification.

A separate case changes the newly created test index's actual mapping while the real search response is held.
It adds an unindexed keyword field, then releases the unchanged actual response bytes.
Final mapping inspection rejects the candidates before final settings inspection. Four requests establish that rejection path.
Original Resource revisions remain unchanged. The index and its documents remain available.
This is mapping-drift evidence, not UUID replacement, alias switching or a continuous generation fence.

Fourteen native search cases and fifteen independent consumer cases pass. Normalized archives repeat the shared cases.
Extracted core search and native history repeat twice, including locked executions. Source-only review reports no actionable findings.
The [full verifier](opensearch-search-fences-full-2026-10-09.log) exited zero with all 253 frozen inputs unchanged.
The [execution record](opensearch-search-fences-2026-10-09.json) identifies sources, compiler, archives and three fresh dependency graphs.
All three audits report zero vulnerabilities. The public graph retains the unsuppressed paste maintenance advisory.
Root and public locks remain byte-identical to the baseline. Serial harness execution does not qualify default parallel scheduling capacity.

Task2 still needs actual process interruption after remote acceptance before local completion.
Task3 still needs one combined loss/restart/checkpoint-reopen case verifying newer and tombstoned state.
Qdrant Rust transport, vector queries and durable generation switching remain incomplete. The full ROM-extras goal is active.
