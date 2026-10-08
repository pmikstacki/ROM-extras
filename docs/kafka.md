# Kafka publication increment

`rom-kafka` constructs a private producer from the host's `rdkafka::ClientConfig` and an exact topic.
Pass a deadline from 1 ms through 300 seconds and the shared `PayloadLimit`.
Construction does not establish broker readiness. Provision topics before delivery.

The host owns brokers, certificates, SASL, ACLs, replication, retention, minimum in-sync replicas, and flush policy.
The adapter enforces all acknowledgements, zero message retries, one admitted operation, bounded native buffers, and disabled automatic topic creation.
It refuses `transactional.id`. Session idempotence is disabled because it conflicts with zero retries.

The record contains the unchanged delivery ID as its key, exact prepared JSON, and `content-type: application/json`.
The shared core validates identity and bounded payloads before enqueue. No actor or Resource metadata is appended.

Preparation refusal is Permanent. Busy admission and immediate native QueueFull are Retryable before enqueue.
Accepted requires a successful delivery report with a nonnegative partition and offset.
Other uncertain reports and deadlines are Unknown. Unknown or cancellation retires the producer adapter.
The host explicitly constructs a replacement. Dropping the future does not cancel native publication.

Repeated keys can create separate records. Receivers must deduplicate durable effects.
No consumer-effect, global ordering, or exactly-once guarantee is claimed.
The application and message deadline retain 1 ms; the native socket timeout has a 10 ms floor.
This follows the pinned [librdkafka configuration](https://raw.githubusercontent.com/confluentinc/librdkafka/v2.12.1/CONFIGURATION.md).

## Build and verify

The locked client is rdkafka 0.39.0 with rdkafka-sys 4.10.0+2.12.1 and bundled librdkafka 2.12.1.
Default features are disabled. The selected features are Tokio and SSL.
Install Make, pkg-config, a C toolchain, and maintained OpenSSL development files.
Use `OPENSSL_DIR` and `PKG_CONFIG_PATH` when development files are outside standard paths.
Use `OPENSSL_STATIC=1` only when the host deliberately supplies a static library.
Do not substitute a stale vendored OpenSSL release. Native advisories require review independently of Cargo audit.

The local evidence uses signed, checksum-verified OpenSSL 3.5.9, configured without shared libraries.
See [the client assessment](research/kafka-client-assessment.md) and preserved native build/signature logs.

Set `ROM_EXTRAS_KAFKA_BROKERS` to the labelled local fixture address, then run `./scripts/check-kafka`.
The fixture requires `127.0.0.1:55449` and label `rom-extras.fixture=kafka`.
It executes Apache Kafka 4.3.1 in single-node KRaft mode with a persistent named volume.
The executed image digest is `sha256:77e3df9054047a88b520d0cc46e16696d3b22022e1d580aeccd2632df6532837`.
The host fixture sets flush intervals to one message and one millisecond. This is not a multi-node durability test.

Tests read actual offsets, keys, bodies, and headers, and demonstrate duplicate records after repeated identity.
They restart the broker and read retained records. A paused broker demonstrates busy admission and cancellation retirement.
Queued publication can complete after cancellation. A missing topic remains absent; no automatic topic creation is inferred.
An independent public consumer provisions and reads its own topic through public APIs.

A native one-entry queue test verifies immediate QueueFull refusal without retirement.
A bounded protocol proxy drops an actual successful ProduceResponse after broker append.
A direct consumer reads the exact record while delivery remains pending; the outcome then becomes Unknown.
Explicit replacement and same-ID retry create a second record. This does not establish a deduplicated consumer effect.
Runtime recovery, TLS/SASL, and release packaging remain pending.
The initial live run returned Unknown unexpectedly; its log and topic data are preserved.
Later runs passed, but the initial cause remains undetermined. Do not treat the repeat as a root-cause fix.
