# ROM-extras

Database providers and optional integrations for [ROM](https://github.com/pmikstacki/ROM).
ROM-extras preserves the Resource → action → atomic commit → event contract.

Development has started. No database provider is supported yet.
Implemented increments include a bounded SQL connection executor and [explicit OIDC issuer presets](docs/oidc-presets.md).
Separate [MySQL and MariaDB executor profiles](docs/mysql-fixtures.md) verify native transactions and restart persistence.
The delivery core prepares bounded JSON and Standard Webhooks signatures.
[HTTPS transport](docs/webhooks.md) has real receiver tests and durable Runtime integration on SQLite and redb.
[NATS JetStream publication](docs/nats.md) has live acknowledgement, deduplication, Runtime recovery, and TLS-first/reconnect tests.
[RabbitMQ publication](docs/rabbitmq.md) has mandatory confirmation, persistence/restart, redelivery, and uncertainty/cancellation tests.
[Kafka publication](docs/kafka.md) has live offset, duplicate, Runtime recovery, TLS/SASL, restart, and independent consumer tests.
[Azure Blob](docs/azure-blob.md) now implements the public blob port with persistent Azurite conformance and independent-consumer evidence.
The [S3 profiles](docs/s3-compatible.md) retain SeaweedFS’s failed strict burst evidence and add passing RustFS local public-port qualification.
The [secrets/KMS family](docs/secrets-kms.md) adds host contracts, OpenBao acceptance cases and reference-only native host preparation.
[Native OpenSearch and Qdrant probes](docs/projection-probes.md) qualify revision fences and restart behavior. The [OpenSearch write target](crates/rom-opensearch/README.md) has [native write qualification](docs/verification/opensearch-rust-write-2026-10-09.md); The [shared typed search contract](docs/verification/projection-search-core-2026-10-09.md) checks current authorization and hydrates candidates through public ROM reads. Native OpenSearch search and Qdrant Rust transport remain unimplemented.
[Projection checkpoints](docs/projection-checkpoints.md) implement durable local intent/recovery transactions, a bounded storage owner, pending-history reconstruction, immutable approved documents and core page orchestration; a bounded lifecycle host supervises managed startup/drop/shutdown; public SQLite/redb history is qualified; further provider integration and final family acceptance remain incomplete.
See [the complete goal](docs/goal.md), [the implementation plan](docs/superpowers/plans/2026-10-08-rom-extras.md), and [support status](docs/support.md).

## Verify locally

Use Rust 1.99, a native linker, CMake, Make, pkg-config, maintained OpenSSL development files, and Node.js.
OpenSSL also generates ephemeral synthetic test signing keys. See [Kafka build requirements](docs/kafka.md).
Run:

```sh
./scripts/check
```

The full actual-backend verifier is `./scripts/check-all`. It requires the configured services and Python 3 for projection probes.

GitHub hosts source code. GitHub Actions is disabled.
