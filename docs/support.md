# Support and verification status

Date: 2026-10-08.
No database or external-service provider is supported yet.

Kafka now has an initial native publication increment. See [its contract and remaining gates](kafka.md).
Kafka also has actual post-append acknowledgement-loss evidence and native QueueFull refusal.
SQLite/redb Runtime recovery, current-authority denial, finite retry stops, and manual replay with durable receiver receipts are verified.
A native Kafka SASL_SSL single-node profile verifies CA/name/password failures, restart retention, explicit host reconnect, and an independent TLS consumer.
This does not complete the Kafka provider or the delivery family.

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

## MySQL and MariaDB execution increment

Five native executor tests passed separately against MySQL 8.4.11 and MariaDB 11.8.9.
They verify timeout uncertainty, later commit, panic retirement, explicit rollback after statement failure, and binary identities after restart.
The independent consumer pins mysql 28.0.3 with Rust compression and native-tls features.
Its final 403-package graph has no reported cargo-audit vulnerabilities or warnings at RustSec revision `550efd3d587a29b2e2c2b21b17a440da4fede999`.
These plaintext loopback fixtures do not establish verified TLS or full ROM Storage support.
See [fixture boundaries](mysql-fixtures.md) and [executed evidence](verification/mysql-mariadb-execution-2026-10-08.md).

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
| MySQL, MariaDB | Five live executor tests each, binary identity and restart evidence; full Storage providers and verified TLS remain pending. |
| CockroachDB | Seven real executor cases on v26.3.2, including actual 40001 retry and graceful restart; full Storage remains pending. |
| Oracle | Seven real OCI executor cases on 23.26.3, including RAW identity, full rollback and orderly restart; full Storage remains pending. |
| OIDC | Preset configuration and signed synthetic-token integration verified; live provider login remains pending. |
| OpenTelemetry | Public diagnostic interface publication and OTLP implementation remain pending. |
| Webhooks | HTTPS/signing/destination policy and native Runtime recovery/authorization tests implemented; production receiver and release acceptance remain pending. |
| NATS | Acknowledged JetStream publication, broker recovery/redelivery, and native Runtime retry/revocation verified; single-node TLS-first token/reconnect profile verified; production topology and release remain pending. |
| RabbitMQ | Mandatory publication, native Runtime, actual confirm loss, receiver receipts, TLS and explicit reconnect verified; replicated failover and release remain pending. |
| Kafka, notifications | Kafka publication, actual acknowledgement loss, native Runtime/receipt recovery and TLS/SASL verified; additional notification connectors and release remain pending. |
| S3-compatible, Azure Blob | Azure port and persistent Azurite conformance implemented; basic native BlobService lifecycle verified on Azure and both local S3 profiles; RustFS local public-port qualification passes; RustFS actual HTTP response loss and explicit recovery verified on SQLite/redb; cloud and further lifecycle faults remain pending. |
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
At this publication increment, Runtime integration and direct post-commit acknowledgement loss were still pending.
The following increments supply Runtime, acknowledgement-loss, and single-node TLS/reconnect evidence.
See [configuration, fixture, and limits](nats.md). No full NATS provider support is claimed.

## NATS post-commit acknowledgement loss increment

A test-only loopback proxy discards an actual publish acknowledgement for a positive stream sequence.
A direct broker connection verifies the stored ID and false JSON while the original delivery still awaits its deadline.
Retry preserves the ID and receives Accepted without a second stored message within the configured duplicate window.
Native SQLite/redb Runtime tests preserve intent, Unknown, attempts, and final Accepted across reopening.
Current source disclosure denial and service revocation produce persisted Denied records without further broker publication.
At this acknowledgement-loss increment, TLS/reconnect and release conformance remained open.
The following TLS increment supplies single-node evidence. This does not establish indefinite exactly-once delivery.

## NATS TLS-first and reconnect increment

The real TLS-first broker requires a CA-verified server certificate and token authentication.
Tests reject unrelated CA, hostname mismatch, incorrect token, and TLS-first connection to a plaintext port.
The same client emits disconnect/connect events across broker restart. The unchanged adapter then acknowledges duplicate and new publications.
An independent consumer executes the public TLS options and reads the exact stored false JSON.
This single-node profile does not establish clustered failover, certificate rotation, mTLS/JWT, or packaged release support.

## RabbitMQ confirmed publication increment

rom-rabbitmq prepares exact bounded JSON and publishes mandatory persistent messages through a dedicated private confirm channel.
Accepted requires Ack without return. Confirmed NO_ROUTE is Retryable; uncertainty retires the adapter until explicit host rebinding.
AMQP message_id preserves identity but does not deduplicate; IDs over255 bytes are refused without truncation or payload encoding.
Real RabbitMQ4.3.2 tests verify false payload/properties, two same-ID messages, restart persistence, unacknowledged redelivery,
NO_ROUTE/reprovision, preparation refusal, overload, timeout/cancellation, eventual buffered writes, and no later publication from retired adapters.
An independent consumer obtains a real confirmation and reads persistent false JSON with the unchanged ID.
Queue durability is a host invariant; passive binding checks existence only. The fixture CLI records durable quorum queues.
The following increment supplies native Runtime and actual post-commit confirmation-loss evidence.
Verified TLS and production release acceptance remain open.
The selected client can trace returned payloads; hosts must filter dependency logs. See [configuration and limits](rabbitmq.md).

## RabbitMQ post-commit confirmation loss and durable receiver increment

A loopback proxy drops the first actual publisher basic.ack, with channel1 and delivery tag1.
A separate broker connection reads the preserved ID and false JSON while Runtime still awaits confirmation.
Both native backends retain Unknown and attempts across reopening. Explicit host rebinding allows Runtime's second attempt to reach Accepted.
Two messages retain the same ID. The fixture receiver commits one Resource effect with a durable ROM idempotency receipt.
Its connection closes without consumer ACK after commit. Actual broker redelivery and publication duplicates replay the reopened receipt.
Another receiver reopening retains one effect. Current source/service denial prevents further publication.
This verifies the fixture application pattern, not a production receiver library or unrestricted exactly-once effects.
TLS, replicated failover, and packaged release remain open.

## RabbitMQ native TLS and explicit host reconnect increment

The separate real broker disables plaintext AMQP and terminates TLS directly.
Tests verify the server CA and hostname, then authenticate with a fixture password.
Unrelated CA, incorrect name, incorrect password, and TLS on the plaintext listener fail for their expected causes.
After broker restart, a fresh verified host connection reads the retained persistent message.
An explicitly rebound adapter confirms a new publication. This does not claim automatic client recovery.
The independent consumer exercises public lapin TLS configuration and the unchanged adapter.
The profile adds a fixture CA alongside native roots; it is not exclusive certificate pinning or mTLS.
Clustered failover, certificate rotation, client-certificate authentication, and packaged release remain open.

## Azure Blob port increment

`rom-azure-blob` implements create-only storage, ETag-conditioned bounded reads, metadata, and idempotent maintenance deletion.
Eight workspace tests and one independent consumer test passed against persistent Azurite and local native-HTTP fixtures.
Review corrected endpoint tests that were masked by malformed credentials; mutation evidence verifies the correction.
The 360-package workspace and 422-package consumer graphs have no reported audit vulnerabilities or warnings.
This establishes local emulator behavior, not Azure cloud support or complete BlobService integration.
See [the adapter contract](azure-blob.md) and [retained evidence](verification/azure-blob-2026-10-08.md).


## Azure native BlobService lifecycle increment

Two additional test scenarios run on both SQLite and redb with actual Azure adapter operations.
They verify rejected staging/digest input, owner-only reads, Ready/Detached reopening, and retained unattached objects after reservation deletion.
No new production lifecycle or cleanup implementation is added.
The initial Conflict expectation was corrected to the public tombstone Denied result; the failed run remains preserved.
See [executed scope and remaining gates](verification/azure-lifecycle-2026-10-08.md).


## S3-compatible candidate qualification

SeaweedFS 4.48 passes published blob conformance, confirmed credential denial, and acknowledged false-byte retention after restart.
An eight-way Unknown/reconciliation test passes without overwrite or automatic cleanup.
A separate strict eight-way qualification fails with typed native HTTP timeout evidence near three seconds.
RustFS 1.0.1 separately passes the four public-port cases, including sixteen eight-way races.
The required S3 gate also runs two shared BlobService tests and a real HTTP response-loss recovery test, each on SQLite and redb.
The two shared BlobService cases also pass on SeaweedFS; its strict burst remains unqualified.
See [shared native lifecycle evidence](verification/blob-lifecycle-shared-2026-10-08.md).
The S3 gate remains required in `check-all` for the selected profile; these results do not establish complete provider support.
See [qualification scope](s3-compatible.md) and [preserved evidence](verification/s3-qualification-2026-10-08.md).

## S3 HTTP response-loss recovery increment

A test-only loopback proxy observes RustFS success for one create before suppressing every client response byte.
The caller receives Unknown while the native reservation remains Pending at revision 1.
Independent direct retrieval verifies the actual object bytes.
Explicit upload retry receives a create conflict, verifies existing bytes, and attaches Ready at revision 2.
Repeated Ready upload makes no additional PUT; Ready bytes survive orderly reopen on SQLite and redb.
See [wire-fault evidence](verification/s3-wire-ack-loss-2026-10-08.md).
This local HTTP/1 profile does not establish cloud TLS, backend crash durability, universal transport retry behavior, or safe orphan deletion.

## CockroachDB shared executor increment

The official v26.3.2 single-node fixture reuses the same executor and PostgreSQL wire driver.
Seven cases verify typed execution, post-wait-timeout commit, panic rollback, duplicate 23505 rollback, actual 40001 complete-transaction reread,
byte identity and acknowledged false-row retention after confirmed graceful restart.
The required check-cockroach gate is part of check-all. It rejects missing configuration, wrong fixed endpoints and hostaddr overrides.
See [fixture](cockroach-fixture.md) and [verification](verification/cockroach-executor-2026-10-08.md).
This development-only NoTls profile is not a complete Storage provider, replicated ownership fence, quorum test or production licensing determination.


## Oracle OCI shared executor increment

Seven cases passed on Oracle Free 23.26.3.0.0 using rust-oracle 0.6.3 and verified pristine Instant Client Basic 23.26.3.0.0.
They cover worker execution, later commit after caller timeout, panic retirement, ORA-00001 full rollback,
RAW identity and NUMBER zero, bounded queue rejection, and acknowledged row retention after orderly restart.
The fixture uses a fixed loopback endpoint and a user with bounded tablespace privileges.
The required check-oracle gate is part of check-all; missing configuration fails.
The consumer dependency graph adds ten packages, preserves all previous identities and checksums, and has zero reported RustSec vulnerabilities.
This Rust audit does not assess native OCI security. Native libraries remain unmodified outside Git.
See [fixture scope](oracle-fixture.md), [fixture facts](verification/oracle-fixture-2026-10-08.json), and [dependency inventory](verification/oracle-dependencies-2026-10-08.json).
Full Storage, TLS, ambiguous commit fault injection, Transaction Guard, replicated durability and packaged release acceptance remain pending.
