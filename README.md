# ROM-extras

Database providers and optional integrations for [ROM](https://github.com/pmikstacki/ROM).
ROM-extras preserves the Resource → action → atomic commit → event contract.

## Project status

ROM-extras is under development. Full ROM Storage providers are not supported yet.
The components below have implementation and test evidence within the limits stated in their guides.

### Databases

A shared bounded SQL executor has real transaction and restart tests for six database engines:

- [PostgreSQL](docs/postgres-fixture.md)
- [Microsoft SQL Server](docs/mssql-fixture.md)
- [MySQL and MariaDB](docs/mysql-fixtures.md)
- [CockroachDB](docs/cockroach-fixture.md)
- [Oracle](docs/oracle-fixture.md)

These are executor profiles. Full ROM Storage integration remains pending.

### Integrations

| Extension | Implemented and verified | Remaining work |
| --- | --- | --- |
| [Webhooks](docs/webhooks.md) | Bounded JSON, signatures, HTTPS receivers and SQLite/redb Runtime recovery. | Production receiver profiles and release acceptance. |
| [NATS JetStream](docs/nats.md) | Acknowledgements, deduplication, Runtime recovery, TLS and reconnect. | Production topology and release acceptance. |
| [RabbitMQ](docs/rabbitmq.md) | Mandatory confirms, persistence, redelivery, uncertainty handling and TLS. | Replicated failover and release acceptance. |
| [Kafka](docs/kafka.md) | Offsets, duplicates, Runtime recovery, TLS/SASL, restart and independent consumers. | Additional notification connectors and release acceptance. |
| [Azure Blob](docs/azure-blob.md) | Public blob port and persistent Azurite conformance. | Cloud profiles and further lifecycle faults. |
| [S3-compatible](docs/s3-compatible.md) | RustFS local public-port qualification; SeaweedFS strict burst failure retained. | Cloud profiles and further lifecycle faults. |
| [OIDC presets](docs/oidc-presets.md) | Explicit issuers and signed synthetic-token verification. | Live provider login and discovery profiles. |
| [Secrets and KMS](docs/secrets-kms.md) | Host contracts, OpenBao acceptance cases and reference-only native host preparation. | Further provider profiles and family acceptance. |

### Search and projections

| Component | Implemented and verified | Remaining work |
| --- | --- | --- |
| Shared core | [Durable checkpoints](docs/projection-checkpoints.md), bounded workers, public SQLite/redb history and [authorized search](docs/verification/projection-search-core-2026-10-09.md). | Generation switching and full projection-family acceptance. |
| [OpenSearch](crates/rom-opensearch/README.md) | [Native writes](docs/verification/opensearch-rust-write-2026-10-09.md) and [typed search with current authorization](docs/verification/opensearch-native-search-2026-10-09.md). | Generation switching and final provider acceptance. |
| [Qdrant](docs/projection-probes.md) | Rust wire transport, native revision-fence and restart probes. | Full projection target, vector queries and Runtime integration. |

### Maps

[Generic map providers](docs/maps.md) are in scope. The local typed-core increment has boundary and public-consumer tests.
Nominatim search/reverse has controlled HTTPS and public-consumer tests; native-service qualification remains pending.
OSRM routing passed qualification on an authored driving graph before and after restart.
Configured raster/vector TileJSON and MapLibre styles have controlled HTTPS and independent-consumer tests.
A local Nginx fixture qualifies metadata delivery before and after restart; tile content and rendering remain unqualified.
MapTiler search/reverse and style/raster/vector metadata have controlled HTTPS and independent-consumer tests.
The metadata adapters require explicit browser-token grants and permission for backend metadata processing; actual account qualification remains pending.
Browser URLs reject query credentials by default; an explicit host grant can admit one browser-intended token.
The [live HTTP host example](examples/maps-live-host/README.md) checks current authorized reads, exact Resource selection and controlled HTTPS geocoding.
The [pnpm frontend example](examples/maps-ui/README.md) uses the public rom-ui/maps package and controlled local host.
Production session integration and actual provider-service qualification remain pending.

### Roadmap

OpenTelemetry, additional notification connectors, and backup, migration and provenance-preserving import tools remain in scope.

- [Complete goal and decision policy](docs/goal.md)
- [Implementation plan](docs/superpowers/plans/2026-10-08-rom-extras.md)
- [Detailed support status](docs/support.md)

## Verify locally

Use Rust 1.99, a native linker, CMake, Make, pkg-config, maintained OpenSSL development files, and Node.js.
OpenSSL also generates ephemeral synthetic test signing keys. See [Kafka build requirements](docs/kafka.md).
Run:

```sh
./scripts/check
```

The full actual-backend verifier is `./scripts/check-all`. It requires the configured services and Python 3 for projection probes.
Its map UI gate requires pnpm, Chromium and `ROM_EXTRAS_MAP_UI_ARCHIVE` pointing to the [qualified public UI archive](examples/maps-ui/README.md).

GitHub hosts source code. GitHub Actions is disabled.
