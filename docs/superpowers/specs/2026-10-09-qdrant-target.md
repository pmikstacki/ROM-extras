# Fixed-generation Qdrant projection target

Extend the existing public `PreparedWrite` API with a real `ProjectionTarget`.
Use the published ROM revision and existing projection HTTP transport.

The host provisions a unique concrete collection and never reuses its name.
The writer uses a collection-scoped `prw` JWT over verified HTTPS.
Administrative deletion, restoration, aliases and configuration changes are excluded during pending work.
REST does not expose Qdrant's internal collection UUID. Mutable metadata cannot fence copied-marker recreation.

A public `Generation` binds the derived document profile, physical name, generation nonce, dimensions and metric.
Admit Dot, Euclid and Manhattan with exact float32 readback. Reject Cosine until its normalization contract is qualified.
Require one shard and replica, one named `embedding` vector, float32, and no quantization or sparse vectors.
Expose a pure collection definition for host provisioning. Never send administrator operations through the writer.

Prepare at most 64 unique documents and 1 MiB of total encoded conditional requests before durable intent.
Bind each prepared page to all generation settings. Preserve exact ROM keys and the full u64 revision fence.
Keep selected JSON encoded as a payload string. Compare all payload fields and actual vector bits.

Require completed native writes with `wait=true&ordering=strong`, then inspect stored points.
Reject missing, changed, colliding or superseded points. No HTTP acknowledgement alone creates an observation.
Check collection configuration before dispatch and after readback under one total operation deadline.
An inspect-only method reconciles prepared work without another write.
Dropping a future cancels local waiting; native revision conditions still fence surviving older requests.
No implicit retry or checkpoint advancement follows an unknown outcome.

Qualify all admitted metrics, exact retry, stale/conflicting revisions, tombstones, full-u64 boundaries,
scoped credential denial, changed payload/vector, generation drift, and inspect-only recovery against native Qdrant.
Separate authored response fixtures from native evidence. Verify an independent public consumer and normalized archives.
Run affected gates and the full local verifier before integration. Keep historical fixtures and evidence.

Search authorization, distributed qualification, Cosine, rebuild/cutover and a native immutable identity fence remain open.
