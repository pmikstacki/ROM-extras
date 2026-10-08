# Support and verification status

Date: 2026-10-08.
No database or external-service provider is supported yet.

The SQL connection executor is implemented and has no external dependencies. It does not implement ROM Storage yet.
Its 13 tests cover bounded admission, connection affinity, timeout uncertainty, panic retirement, shutdown, and handle lifecycle.
Three executor integration tests passed against PostgreSQL 18.6. These are not ROM Storage conformance tests.
An acknowledged control row survived a server restart. See [the fixture](postgres-fixture.md) and [verification evidence](verification/executor-2026-10-08.md).
The proposed compatibility baseline is public ROM commit `d7ef529040eec60dc869034c2d33130219db85fe`.
Local ROM commit `1a2b7934139f3d7bb4cb04aaf4a90bd9405ab470` is unavailable through the GitHub commit API.
The published baseline does not export incremental Work support or the new diagnostic reader.
SQL Storage and telemetry integration require a later public commit; complete-ledger serialization is prohibited as a substitute.

The workspace declares Rust 1.99 to match the published ROM baseline.
The independent consumer pins postgres 0.19.14 and Tokio 1.53.1 behind its PostgreSQL fixture feature.
Cargo-audit 0.22.2 found no advisories or warnings across its 104 locked packages on 2026-10-08.
This result uses RustSec commit `550efd3d587a29b2e2c2b21b17a440da4fede999` and does not establish future advisory status.
The dependency manifest license inventory is preserved with verification evidence; redistribution review remains required before release.
Oracle requires a separate native-client and redistribution review.

## OIDC configuration increment

`rom-oidc-presets` implements explicit HTTPS, Keycloak realm, and concrete Entra tenant issuer configuration.
It delegates verification to the pinned public ROM `OidcIdTokenAdapter`.
Six configuration tests and three signed synthetic-token tests passed, including rotation and exact issuer rejection.
A regression test covers Unicode whitespace and controls found during independent review.
The independent public consumer now imports the preset package.
After this increment, cargo-audit 0.22.2 found no advisories or warnings in 102 workspace and 165 consumer packages.
Both scans used RustSec commit `550efd3d587a29b2e2c2b21b17a440da4fede999` on 2026-10-08.
Manifest license inventories contain no absent declarations. Native cryptographic dependency notices still require release redistribution review.
No live provider login or discovery profile is supported yet. See [usage and limits](oidc-presets.md).

## SQL Server execution increment

Three real executor tests passed against SQL Server 2025 RTM-CU9, build 17.0.5005.3.
They cover a Tokio host, commit after caller timeout, and rollback after connection retirement.
The fixture also checks server-side encryption and disabled delayed durability.
A separately acknowledged control row survived server restart on its preserved named volume.
The Tiberius 0.13.0 published archive checksum is `e07324791de2bdaed058af4aa433a0b5ec5c0beac407a9797419c19805df2415`.
Cargo-audit 0.22.2 reported zero vulnerabilities or warnings in 207 consumer packages with RustSec commit `550efd3d587a29b2e2c2b21b17a440da4fede999`.
No MSSQL Storage provider or production TLS profile is supported yet. See [the fixture](mssql-fixture.md).

## Delivery preparation increment

`rom-delivery-core` prepares bounded JSON bytes from typed ROM deliveries, preserving stable identity and attempt.
It includes Standard Webhooks HMAC-SHA256 signing with current and previous host keys.
Five preparation tests, two signing tests, and one doctest passed. The signature vector was independently generated with Node.js crypto.
Independent source review found no actionable defect; the full local verifier passed with both SQL fixtures.
Cargo-audit 0.22.2 reported no vulnerabilities or warnings for 113 workspace and 209 consumer packages.
Both scans used RustSec commit `550efd3d587a29b2e2c2b21b17a440da4fede999` on 2026-10-08.
The HTTPS transport increment is implemented; see [its boundaries and verification](webhooks.md).
Native SQLite/redb channel reopening, persisted outcomes, authorization revocation, and bounded Runtime retries are verified.
No production webhook or broker provider is supported yet.

## HTTPS transport increment

Two destination/admission tests and one real Node HTTPS receiver test passed.
The receiver checks signatures independently and counts requests, including deadline, lost acknowledgement, and redirect cases.
The full local verifier passed, including both real database fixtures. Source review findings about counters and deadline coverage were resolved.
Cargo-audit 0.22.2 found zero vulnerabilities or warnings in 195 workspace and 251 consumer locked packages.
Both scans used RustSec commit `550efd3d587a29b2e2c2b21b17a440da4fede999` on 2026-10-08.
License manifest inventories have no absent declarations; redistribution review remains open.
Three Runtime integration tests now verify durable receiver deduplication and ROM authorization/reopening.
They run on both native SQLite and redb, with a killed/restarted Node receiver using SQLite WAL/FULL.
Cargo-audit 0.22.2 found no vulnerabilities or warnings in the updated 211-package workspace graph.
The unchanged independent-consumer graph remains 251 packages.
The Node 22.16 fixture SQLite API is experimental; this is not a production receiver recommendation.

## Whole-goal tracking

| Family | Implementation and remaining acceptance |
| --- | --- |
| Shared SQL | Connection executor verified; Storage protocol awaits public incremental Work APIs. |
| PostgreSQL | Three live executor tests and control-row restart evidence; full Storage provider remains pending. |
| MSSQL | Three live executor tests, encryption, and restart control evidence; full Storage provider remains pending. |
| MySQL, MariaDB, CockroachDB, Oracle | Driver research exists; provider implementation and real-backend conformance remain pending. |
| OIDC | Preset configuration and signed synthetic-token integration verified; live provider login remains pending. |
| OpenTelemetry | Public diagnostic interface publication and OTLP implementation remain pending. |
| Webhooks | HTTPS/signing/destination policy and native Runtime recovery/authorization tests implemented; production receiver and release acceptance remain pending. |
| NATS | Acknowledged JetStream publication and broker recovery/redelivery verified; native Runtime and production TLS profile remain pending. |
| Kafka, RabbitMQ, notifications | Shared delivery preparation/signing implemented; provider implementations and real acknowledgement/recovery remain pending. |
| S3-compatible, Azure Blob | Existing ROM adapter review and missing-operation implementation/real fixtures remain pending. |
| Secrets and KMS | Bounded family contract, provider implementation, and service verification remain pending. |
| OpenSearch and vectors | Projection contract, implementation, authorization checks, and real fixtures remain pending. |
| Backup, migration, import | SQL archive contract and provenance-preserving tools remain pending. |
| Release | Packaged-consumer checks, compatibility, redistribution review, and whole-goal acceptance remain pending. |

The full goal stays active. A verified increment does not complete its extension family or the whole repository scope.

## NATS JetStream publication increment

`rom-nats` implements bounded prepared JSON publication to an existing file-backed stream and exact configured subject.
It waits for actual JetStream acknowledgement and preserves unknown outcomes and Runtime IDs.
One comprehensive live broker test and an independent public-consumer test passed against NATS 2.15.0.
The broker test verifies retained messages and deduplication across restart, consumer redelivery, and buffered publication after timeout.
Source review found no actionable defects and identified the timeout-before-persistence evidence limit.
The full local verifier passed across HTTPS, PostgreSQL, MSSQL, and NATS fixtures.
Cargo-audit 0.22.2 found zero vulnerabilities/warnings in 237 workspace and 273 consumer packages.
Both scans used RustSec commit `550efd3d587a29b2e2c2b21b17a440da4fede999` on 2026-10-08.
Manifest license inventories have no absent declarations; redistribution review remains required.
Native Runtime integration, direct acknowledgement-loss injection after persistence, and production TLS/reconnect remain pending.
See [configuration, fixture, and limits](nats.md). No full NATS provider support is claimed.
