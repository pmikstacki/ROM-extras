# RabbitMQ Runtime and receiver verification

Date: 2026-10-08. Base: 01b2dc656c2fd2a2ee642f5a455f78e69c96563d.
Production adapter code and public paths are unchanged. This increment adds required native Runtime conformance.

The loopback proxy discards actual server basic.ack on channel1, delivery tag1.
A separate broker connection reads false JSON and the exact intent ID while Runtime processing remains pending.
The adapter reaches Unknown. SQLite/redb reopening retains Pending, Unknown, identity, and attempt1.
After the fixture clock advances, explicit host rebinding allows Runtime attempt2 to reach Accepted and Done.
Two broker messages retain the same ID. RabbitMQ does not perform message_id deduplication.

The fixture receiver commits one action with the message ID as its durable idempotency key.
Its independent consumer connection closes without ACK after the effect commits. The broker automatically requeues that delivery.
The reopened receiver handles redelivery and publication duplicates through receipt replay, then acknowledges them.
Another reopening verifies one retained counter increment. The test observes the broker's redelivered flag.
Current source disclosure denial and service revocation produce two Denied records without further publication.
Final producer reopening retains Accepted and performs no further work.

The receiver is a test application. Reopening occurs within the same test process.
This does not establish process-crash, power-loss, arbitrary downstream effects, or distributed exactly-once guarantees.
The single-node RabbitMQ4.3.2 quorum fixture is plaintext. TLS, replicated failover, and release admission remain open.
All queues, databases, fixture volume data, and failed test output are preserved.

Initial compilation diagnostics identified wrong public ROM result assumptions; corrected tests use published Snapshot semantics.
The missing-proxy red run failed at readiness. The initial implemented proxy and native scenario then passed.
A stronger unacknowledged connection-closure scenario required ShortString conversion; that compilation diagnostic is retained.
Final targeted test passed once, with both SQLite and redb scenarios. See rabbitmq-runtime-2026-10-08.log.

Full command: ./scripts/check-all. Terminal exit:0.
Evidence: check-all-rabbitmq-runtime-2026-10-08.log.
This includes required real PostgreSQL, MSSQL, NATS plaintext/TLS, RabbitMQ, HTTPS, and independent public consumer checks.
Rust1.99, GCC13.3, CMake3.30.5, OpenSSL3.5.8, and Node22.16 supply the toolchain.
The source witness records post-command hashes; it is not a complete pre/post release fence.
Read-only source review found no concrete defects. The reviewer did not independently execute the full gate.

The lockfile adds only native adapter dev edges to rom-rabbitmq; no package versions are added or changed.
The existing dependency license inventory remains applicable to the unchanged package set.
Fresh cargo-audit0.22.2 found zero vulnerabilities and no warnings in the workspace graph.
Consumer dependencies and lockfile are unchanged. Private credential values are checked before publication without printing them.
The complete extras goal and parent delivery task remain active.
