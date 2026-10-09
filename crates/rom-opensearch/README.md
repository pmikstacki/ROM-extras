# rom-opensearch

Fixed-generation OpenSearch write target over the shared durable projection worker.

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
Durable generation manifests, typed candidates, current query authorization and alias switching remain unimplemented.

Actual single-node tests cover partial Bulk, response loss, delayed requests, service restart and SQLite/redb checkpoint recovery.
Controlled HTTPS tests cover redirects, response bounds, malformed JSON, deadlines and wrong trust roots.
These tests do not establish multi-node durability, arbitrary hardware failure or production capacity.
Final review, dependency qualification and the full verifier are required before integration.
