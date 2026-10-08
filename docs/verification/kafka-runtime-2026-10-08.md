# Kafka native Runtime verification

Base da3450d0c6b713ae39b9f35f92634493afa04572; uncommitted Runtime increment executed2026-10-08.
Rust1.99.0 b940084d7, native librdkafka2.12.1, static OpenSSL3.5.9, Node22.16, and the preserved Kafka4.3.1 fixture were used.
Source witness files record SHA256 of executed transport/test sources, shared receipt fixture, manifests, lockfiles, and profile script.

The first command refused a stale lock under --locked. Offline metadata added only existing SQLite/redb dev dependency edges to rom-kafka.
No package/version/source changes occurred; the independent consumer lock remains unchanged.
The first live Runtime test failed: default5s Runtime supervision persisted TimedOut before the10s adapter deadline.
Published Builder and channel execution source established the cause. Explicit15s supervision remains below the30s lease.
The corrected test persisted adapter Unknown without relabelling TimedOut. Preserve both failed logs.

The recovery test passed on SQLite and redb.
An ordinary action commits a false notification intent. Store reopen retains Pending work, and broker watermarks remain zero before processing.
The existing bounded proxy drops actual successful ProduceResponse. A direct native consumer reads offset0 with the stable ID and exact false.
The test polls process_work again and verifies it remains pending before completion.
Unknown/attempt1/Pending survives reopening. At host clock1, explicit producer replacement retries the unchanged ID.
Accepted/attempt2/Done is persisted, and the direct broker contains exactly two records.
Current source denial and service revocation stop two additional intents as Denied without delivery results or watermark growth.
These stopped records survive reopening, and terminal work is not reprocessed.

The receiver commits a ROM action receipt using the delivery ID. Its count is one.
It closes without committing a Kafka offset, reopens its SQLite/redb Runtime, and manually assigns Beginning.
Replaying offset0 and the same-ID record at offset1 leaves one effect; a further store reopen retains count1.
This is explicit replay and durable receipt evidence, not automatic consumer-group checkpoint recovery.
The shared generic receipt fixture moved from RabbitMQ tests to tests/common; its behavior and RabbitMQ import facade remain unchanged.

A separate real native test delivers to an absent topic twice through fresh host adapters and persisted Runtime work.
Both attempts return Unknown. After attempt2, Stopped(Attempts), attempts2, and Unknown survive store reopen.
The final check advances the clock to60, past retry eligibility. No further work runs.
Broker metadata confirms the missing topic was not created.
The finite retry test passed on both native stores.

Reviewer inspected source and found no concrete defects.
One suggested coverage improvement advanced the final exhausted-budget clock past retry eligibility. Review did not independently execute live tests.
The initial full ./scripts/check-all exited0, recorded in kafka-runtime-full-2026-10-08.log.
The final full ./scripts/check-all after clock advancement exited0; output is kafka-runtime-final-2026-10-08.log.
Reopen occurs within one process. No process-crash, power-loss, replicated-cluster, TLS/SASL, or packaged-release guarantee is established.
Kafka and the parent delivery family remain incomplete.
