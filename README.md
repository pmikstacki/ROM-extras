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
The [S3 candidate profile](docs/s3-compatible.md) has real conformance and uncertainty evidence; its strict burst qualification currently fails.
See [the complete goal](docs/goal.md), [the implementation plan](docs/superpowers/plans/2026-10-08-rom-extras.md), and [support status](docs/support.md).

## Verify locally

Use Rust 1.99, a native linker, CMake, Make, pkg-config, maintained OpenSSL development files, and Node.js.
OpenSSL also generates ephemeral synthetic test signing keys. See [Kafka build requirements](docs/kafka.md).
Run:

```sh
./scripts/check
```

GitHub hosts source code. GitHub Actions is disabled.
