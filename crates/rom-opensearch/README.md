# rom-opensearch

Fixed-generation OpenSearch write and typed search target over the shared projection contracts.

Use a derived DocumentMapping profile and a fixed physical index. TlsConfig requires verified HTTPS and a PEM client identity.
Transport disables redirects, system proxies and implicit retries. It bounds request and streamed response bytes to 1 MiB.
The initial name profile admits lowercase ASCII names beginning with a letter, at most 128 bytes.
Explicit text fields admit selected string or null values. Other selected JSON values remain unindexed in stored source.

Preparation admits at most 64 operations and rejects revisions above i64::MAX before intent or network dispatch.
Strict external versions fence delayed older writes. Same-version replay requires exact actual stored-source equality.
Persistent tombstone documents remove selected values and searchable fields while retaining the revision fence.
Request translog durability is separate from refresh visibility. No search visibility is inferred from write completion.

inspect_prepared reads actual native state without replay or local checkpoint publication.
Generation inspection checks actual mappings, durability and a stable index UUID during the adapter lifetime.
SearchTarget accepts only a core-approved typed query bound to the configured profile, physical generation and indexed field.
It uses explicit all-terms/any-terms match modes, live/kind/profile filters and one finite candidate page.
Responses contain only original-key metadata and revisions. Timed-out, partial, malformed, foreign and oversized results fail closed.
One total provider deadline includes generation inspection before and after search. The existing transport byte bounds also apply.
Search checks current host query permission and hydrates through public ROM, excluding stale, denied, deleted or field-hidden candidates.
The current result is not an atomic multi-row authorization snapshot. Finite candidate budgets may produce shorter results.

Durable generation manifests and alias switching remain unimplemented. Native search qualification is in progress.

Actual single-node tests cover partial Bulk, response loss, delayed requests, service restart and SQLite/redb checkpoint recovery.
Search tests cover native match modes, original Unicode keys, finite budgets, current stale/deleted filtering and protected-field denial with zero relay requests.
A verified relay holds an actual four-candidate HTTP200 reply while query, row or field authority is revoked without Resource mutation.
Only current permitted values may be returned. A still-visible note does not expose hidden title membership.
Controlled HTTPS tests cover redirects, response bounds, malformed JSON, deadlines and wrong trust roots.
These tests do not establish multi-node durability, arbitrary hardware failure or production capacity.
Final review, dependency qualification and the full verifier are required before integration.
