# Research-backed decisions

Inspection date: 2026-10-08. Tests remain separate evidence from source review.

| Decision | Source and consequence |
| --- | --- |
| Create a separate public repository | The approved plan follows ROM's public open-source intent. [GitHub CLI](https://cli.github.com/manual/gh_repo_create) provides explicit public repository creation. |
| Disable GitHub Actions | ROM quality policy requires local verification. [GitHub API](https://docs.github.com/en/rest/actions/permissions#set-github-actions-permissions-for-a-repository) exposes the repository setting; GET verified `enabled: false`. |
| Use a published immutable ROM revision | [GitHub commit API](https://docs.github.com/en/rest/commits/commits#get-a-commit) rejected the local-only SHA. Public `d7ef529040eec60dc869034c2d33130219db85fe` is the compatibility candidate; unsupported exports remain blocked. |
| Start independent connection execution while incremental APIs await publication | [Postgres implementation](https://docs.rs/postgres/0.19.14/postgres/#implementation) runs a Tokio-backed synchronous client. [Tiberius](https://docs.rs/tiberius/0.13.0/tiberius/) uses async I/O. Both need adapter-owned execution. |
| Use one worker per connection with a bounded standard-library queue | [Rust sync_channel](https://doc.rust-lang.org/std/sync/mpsc/fn.sync_channel.html) provides FIFO bounded admission. [try_send](https://doc.rust-lang.org/std/sync/mpsc/struct.SyncSender.html#method.try_send) rejects a full buffer without queue waiting. No third-party channel dependency is necessary initially. |
| Keep post-admission timeouts uncertain | [recv_timeout](https://doc.rust-lang.org/std/sync/mpsc/struct.Receiver.html#method.recv_timeout) only limits waiting. It does not cancel the submitted operation. Keep the ticket for later outcome inspection. |
| Retire a connection after an unwinding job panic | [catch_unwind](https://doc.rust-lang.org/std/panic/fn.catch_unwind.html) catches unwinding panics only. Connection invariants are not established after an arbitrary driver panic; queued operations must not reuse it. Abort panics remain process failures. |
| Explicit shutdown drains admitted jobs | [Rust channel disconnection](https://doc.rust-lang.org/std/sync/mpsc/struct.Receiver.html#method.recv) permits buffered messages to be read after senders close. [JoinHandle](https://doc.rust-lang.org/std/thread/struct.JoinHandle.html) joins explicitly; dropping the last handle must not imply cancellation. |
| Verify connection destruction and terminal consumption | [JoinHandle::join](https://doc.rust-lang.org/std/thread/struct.JoinHandle.html#method.join) waits for thread completion and reports unwinding panics. Review identified missing test coverage; tests now check destruction affinity and completion, destructor panic, and consumed outcomes. |
| Pin the actual PostgreSQL fixture image | [Docker Library PostgreSQL](https://github.com/docker-library/docs/tree/master/postgres) describes official images. Executed PostgreSQL 18.6 Alpine image digest is recorded in the fixture instructions. A moving tag alone would not reproduce the server. |
| Use native fixture deadlines and test commit uncertainty | [PostgreSQL statement_timeout](https://www.postgresql.org/docs/18/runtime-config-client.html) bounds server statements. [postgres connect_timeout](https://docs.rs/postgres/0.19.14/postgres/config/struct.Config.html#method.connect_timeout) bounds connection attempts. Neither turns a caller wait timeout into confirmed rollback. |
| Restart with the installed container engine's option | [Podman restart](https://docs.podman.io/en/latest/markdown/podman-restart.1.html) uses `--time`. The host Docker command is Podman compatibility mode. A failed `--timeout` attempt is retained with the corrected experiment. |
| Update the audit tool without changing product dependencies | [RustSec](https://rustsec.org/) provides the advisory database. [cargo-audit 0.22.2](https://docs.rs/crate/cargo-audit/0.22.2) parsed the current database; Nix's 0.21.0 failed on CVSS 4. Preserve both outcomes. |
| Preserve manifest license and MSRV inventory | [Cargo metadata](https://doc.rust-lang.org/cargo/commands/cargo-metadata.html) exposes package licenses, source, and rust_version. Record the locked graph without treating manifest declarations as completed redistribution review. |
| Delegate OIDC verification to public ROM | [Published ROM adapter](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-auth/src/oidc/adapter.rs) owns signatures, claims, key refresh, and human evidence. Presets only select exact issuers. No duplicate verifier or permissive fallback. |
| Preserve exact HTTPS issuer binding | [OIDC Discovery](https://openid.net/specs/openid-connect-discovery-1_0.html) requires an identical issuer and no query or fragment. [url 2.5.8](https://docs.rs/url/2.5.8/url/struct.Url.html) validates components; retain the original string, rather than normalized parser output. |
| Limit generated presets to explicit Keycloak realms and concrete Entra tenants | [Keycloak endpoints](https://www.keycloak.org/securing-apps/oidc-layers) retain the deployment context. [Microsoft Entra OIDC](https://learn.microsoft.com/en-us/entra/identity-platform/v2-protocols-oidc) distinguishes tenant-specific and multi-tenant metadata. Exact HTTPS issuers cover other provider/cloud layouts without guessing trust. |
| Test returned adapters with signed synthetic tokens | [jsonwebtoken 11.1.0](https://docs.rs/jsonwebtoken/11.1.0/jsonwebtoken/) provides signing with ROM's selected backend. [serde_json 1.0.151](https://docs.rs/serde_json/1.0.151/serde_json/) constructs malformed claim scenarios. Both are test dependencies; the preset delegates verification to ROM. Live provider acceptance remains unverified. |
| Reject Unicode whitespace and controls before parsing issuers | [Rust char predicates](https://doc.rust-lang.org/std/primitive.char.html#method.is_whitespace) cover Unicode. Review reproduced acceptance of NBSP and NEL with ASCII-only predicates. A failing regression test preceded the character-based fix. |
| Generate ephemeral signing fixtures outside product code | [OpenSSL genpkey](https://docs.openssl.org/3.5/man1/openssl-genpkey/) documents explicit RSA generation. Tests generate two 2048-bit keys in memory, without committing private material or depending on a real identity account. |
| Verify async SQL Server execution while Storage APIs await publication | [Tiberius 0.13.0](https://docs.rs/tiberius/latest/tiberius/) requires compatible asynchronous I/O. Worker-owned Tokio runtime and TDS client reuse the shared bounded executor. This validates transport execution, not complete ROM persistence. |
| Use an isolated SQL Server 2025 Developer fixture | [Microsoft's container guide](https://learn.microsoft.com/en-us/sql/linux/install-upgrade/quickstart-install-docker?view=sql-server-ver17) specifies x86-64 Linux, memory prerequisites, edition, credentials, and persistence. Local architecture/resources were checked; pin the executed digest and preserve a named volume. |
| Select TDS 7.3 and Rustls explicitly for the fixture | [Tiberius release features](https://github.com/tiberius-rs/tiberius/blob/v0.13.0/Cargo.toml) provide explicit TLS backends. Disable native TLS and Windows authentication defaults. Tokio needs explicit `net` and `time` features; an initial compile failure exposed the missing network feature. |
| Scope certificate bypass to the fixed loopback fixture | [Tiberius trust_cert](https://docs.rs/tiberius/latest/tiberius/struct.Config.html#method.trust_cert) bypasses certificate validation. The fixture requires encryption but has no production trust claim; live CA/hostname/expiry acceptance remains pending. |
| Keep delivery scheduling in ROM | [Published channel execution](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/channels/execution.rs) persists delivery outcomes and performs current authorization. Extras prepare/send payloads; no duplicate retry ledger or authorization bypass. |
| Bound exact JSON preparation before transport | [serde_json::to_writer](https://docs.rs/serde_json/1.0.151/serde_json/fn.to_writer.html) supports a byte-limited writer. Default 64 KiB and configurable 1..=1 MiB are explicit ROM-extras operational choices. Preserve scalar distinctions; custom Input encoder computation remains outside the output-byte bound. |
| Use Standard Webhooks signing with bounded rotation | [Standard Webhooks](https://github.com/standard-webhooks/standard-webhooks/blob/main/spec/standard-webhooks.md) defines stable ID, fresh attempt timestamp, exact body bytes, HMAC-SHA256, and multiple signatures. Use current and one previous 32-byte key. Receiver freshness and deduplication remain required. |
| Preserve supported ROM delivery IDs without delimiter ambiguity | [Published notification IDs](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/channels/intentions.rs) are SHA256 hex. The wire profile accepts ASCII alphanumeric, hyphen, and underscore IDs up to 2048 bytes; rejects dot and header injection without rewriting identity. |
| Use audited primitives and limit secret visibility | [RustCrypto HMAC](https://docs.rs/hmac/0.12.1/hmac/) supplies the MAC; [zeroize](https://docs.rs/zeroize/latest/zeroize/) clears stored key buffers. Signing keys have no Debug/serialization API. No guarantee is made about every intermediate MAC state or caller copy. |
| Disable hidden HTTP retries as well as redirects | [reqwest 0.13.5 ClientBuilder](https://docs.rs/reqwest/0.13.5/reqwest/struct.ClientBuilder.html) documents protocol-NACK retries by default. Use [retry::never](https://docs.rs/reqwest/0.13.5/reqwest/retry/fn.never.html), no redirect, no proxy, and HTTPS only. Runtime remains the retry owner. |
| Pin all validated destination addresses for the client lifetime | [resolve_to_addrs](https://docs.rs/reqwest/0.13.5/reqwest/struct.ClientBuilder.html#method.resolve_to_addrs) supplies exact socket addresses while preserving the URL hostname for TLS. Reject mixed public/private sets and IP-literal override mismatches. Hosts must rebuild when resolution changes. |
| Use a conservative public-address profile | [IANA IPv4](https://www.iana.org/assignments/iana-ipv4-special-registry) and [IANA IPv6](https://www.iana.org/assignments/iana-ipv6-special-registry) identify special allocations. Reject all listed special-purpose prefixes, including globally reachable protocol allocations; admit IPv6 only within 2000::/3 outside special prefixes. This is an explicit destination restriction, not a universal routability classifier. |
| Interpret HTTP acknowledgement without reading response payloads | [RFC 9110](https://www.rfc-editor.org/rfc/rfc9110.html) defines status classes but cannot prove application rollback. A receiver must contractually acknowledge durable acceptance with 2xx. Lost acknowledgement and server errors remain Unknown. Body inspection is zero bytes; HTTP header/parser allocations still belong to the selected client. |
| Separate fixture CA and server certificate | [OpenSSL x509 micro-CA options](https://docs.openssl.org/3.5/man1/openssl-x509/) sign a CSR with explicit CA:FALSE and serverAuth extensions. Rustls rejected the first fixture with CaUsedAsEndEntity; retain that diagnostic and correct the fixture rather than disable TLS verification. |
| Verify extras through published durable ROM channel contracts | [Published channel conformance](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/tests/persistence/tests/channels.rs) exposes native SQLite/redb reopen, reaction records, service revocation, current source-field disclosure, and attempt budgets. Extras tests register the real HTTPS transport rather than duplicate Work scheduling. Native adapters are dev dependencies pinned to the same ROM commit. |
| Use a separate durable receiver fixture for lost acknowledgements | [Node 22.16 SQLite](https://nodejs.org/download/release/v22.16.0/docs/api/sqlite.html) supplies test-only DatabaseSync with bound statements. [SQLite synchronous](https://www.sqlite.org/pragma.html#pragma_synchronous) documents WAL FULL commit synchronization. Atomically commit dedup ID/body and attempt count before dropping the socket, then kill/restart the receiver and retry through reopened ROM. This tests process restart, not power-loss durability. The installed Node API is experimental; retain its warning and make no production Node recommendation. |
| Exercise source revocation through a committed Resource mutation | [Published current/historical source checks](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/channels/execution.rs) require current disclosure immediately before send. Change Resource state so its field policy denies the service, then assert the durable Denied record and zero receiver effects. Keep policies as public fn pointers; no private or mutable global authorization bypass. |
| Require JetStream publish acknowledgement, not Core NATS or queue admission | [NATS JetStream](https://docs.nats.io/concepts/jetstream) persists streams; [async-nats Context](https://docs.rs/async-nats/0.50.0/async_nats/jetstream/context/struct.Context.html) returns PublishAckFuture separately. Await both phases and verify expected stream/positive sequence before Accepted. |
| Use Runtime ID within a bounded broker duplicate window | [NATS headers](https://github.com/nats-io/nats.docs/blob/master/nats-concepts/jetstream/headers.md) defines Nats-Msg-Id and Nats-Expected-Stream. Preserve IDs and bind exact streams; dedup does not establish indefinite exactly-once effects. |
| Require file storage and explicit single-node fixture fsync | [NATS configuration](https://github.com/nats-io/nats.docs/blob/master/running-a-nats-service/configuration/README.md) documents default delayed sync and always. Fixture varz reports sync_always=true, with a persistent named volume. Host production topology/durability remains an explicit contract. |
| Pin executed NATS 2.15.0 and minimal client features | [Official NATS image](https://hub.docker.com/_/nats) and [client 0.50.0](https://docs.rs/async-nats/0.50.0/async_nats/) are primary sources. Cargo info records Apache-2.0, MSRV1.88, and features. Select JetStream/AWS-LC/NUID without unrelated services, KV/object-store, or websockets. Audit and redistribution review remain required. |
| Keep fixture token authentication separate from server URL | [async-nats options source](https://github.com/nats-io/nats.rs/blob/9b382a2a01b5404cd66bee6c2b4f0c82c9943063/async-nats/src/options.rs) provides with_token; URL userinfo selects username/password instead. Initial fixture authentication failed; corrected private access configuration supplies a separate token without printing credentials. |
| Distinguish buffered timeout from lost acknowledgement after persistence | [async-nats publication source](https://github.com/nats-io/nats.rs/blob/9b382a2a01b5404cd66bee6c2b4f0c82c9943063/async-nats/src/jetstream/context.rs) separately queues a client command and awaits its ack future. Real broker pause proves Unknown followed by later publication, not post-commit acknowledgement loss. Review preserved that acceptance requirement as open. |
| Verify durable consumer redelivery with confirmed acknowledgement | [async-nats Message](https://docs.rs/async-nats/latest/async_nats/jetstream/message/struct.Message.html) describes redelivery without acknowledgement and double_ack confirmation. The live test preserves the same stream sequence across restart, confirms the acknowledgement, and observes no pending consumer ack. This is broker semantics, not application exactly-once processing. |
| Inject loss on the actual server publish acknowledgement | [NATS client protocol](https://docs.nats.io/reference/protocols/client) defines MSG/HMSG length framing and separate headers. The test proxy forwards client bytes unchanged, drops one matching positive stream/sequence JSON acknowledgement, and preserves its non-secret witness. A direct broker read occurs before the adapter deadline; neither a broker pause nor client queue acceptance substitutes for this evidence. |
| Keep all fixture client connections behind the acknowledgement proxy | [Pinned async-nats options](https://github.com/nats-io/nats.rs/blob/9b382a2a01b5404cd66bee6c2b4f0c82c9943063/async-nats/src/options.rs) exposes ignore_discovered_servers. Use it for the proxy connection only. Production hosts retain their topology policy. Readiness has a finite deadline and a child guard that terminates the proxy on failure. |
| Reuse native ROM retry and authorization without another ledger | [Published notification execution](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/channels/execution.rs) checks current authority and persists delivery outcomes. Tests bind the real NATS adapter and reopen native SQLite/redb before delivery, after Unknown, and after Accepted. Native stores and derive are test dependencies at the unchanged public ROM pin. |
| Require a verified TLS-first host profile without plaintext fallback | [NATS TLS](https://docs.nats.io/learn/security/encryption) explains plaintext INFO before ordinary TLS and handshake_first before protocol bytes. [Client options](https://docs.rs/async-nats/latest/async_nats/struct.ConnectOptions.html) documents require_tls, tls_first, CA loading, connection timeout, capacities, and reconnect limits. The independent consumer executes the documented public options. Keep host configuration outside delivery payloads; no disabled certificate verifier. |
| Distinguish negative TLS causes instead of accepting any connection failure | [Pinned connector source](https://github.com/nats-io/nats.rs/blob/9b382a2a01b5404cd66bee6c2b4f0c82c9943063/async-nats/src/connector.rs) maps handshake failures to IO and tries resolved socket addresses. Tests identify UnknownIssuer and the actual certificate-not-valid-for-name diagnostic, plus AuthorizationViolation for the wrong token. An initial test expected the Rust enum name in formatted output; preserved diagnostic showed its human-readable wording. |
| Test reconnect on the same client with finite host limits | [Client options](https://docs.rs/async-nats/latest/async_nats/struct.ConnectOptions.html) provides event_callback, max_reconnects, reconnect_delay_callback, client_capacity, and subscription_capacity. The real labelled broker restarts, the same client emits disconnect/connect events, and the same adapter republishes with preserved identity. Counts are entries/consecutive attempts, not byte limits or indefinite retries. Cluster discovery, certificate renewal, and production topology remain explicit unverified host choices. |
| Require mandatory confirmed RabbitMQ publication | [RabbitMQ confirms](https://www.rabbitmq.com/docs/confirms) states that unroutable publications can receive Ack and that mandatory returns precede confirmation. lapin Ack(None) is Accepted; Ack with NO_ROUTE is Retryable; uncertainty is Unknown. Persistent delivery_mode2 on host-provisioned durable queues establishes the documented broker acceptance profile, not consumer effects. |
| Preserve AMQP identity without claiming broker deduplication | [AMQP0-9-1 specification](https://www.rabbitmq.com/resources/specs/amqp0-9-1.pdf) defines shortstr message-id. Reject IDs over255 bytes before encoding, retain ROM SHA256 IDs, and never truncate. Repeating message_id creates separate real queue messages; consumers must deduplicate durable effects. |
| Keep queue durability an explicit host invariant | [RabbitMQ queues](https://www.rabbitmq.com/docs/queues) describes passive declaration and queue lifecycle; it is not a configuration inspector. The adapter probes existing queue existence and uses a private confirm channel. Fixture provisioning creates durable quorum queues; CLI records actual type/durability. No unsupported configuration claim is inferred from passive lookup. |
| Select explicit lapin4.12.0 runtime/crypto features | [Pinned lapin](https://github.com/amqp-rs/lapin/tree/5226cfb1d05b965369d87f75f505a76943274b6a) declares MIT/MSRV1.88 and configurable Tokio/Rustls. Disable default features; select Tokio, Rustls/AWS-LC, native roots. New locked graphs require fresh audit/license/MSRV inventories and independent consumer checks. |
| Retire uncertain publisher channels instead of accumulating unresolved confirms | [PublisherConfirm](https://docs.rs/lapin/latest/lapin/struct.PublisherConfirm.html) retains dropped unresolved confirmations for wait_for_confirms. Use one admitted publication and retire on uncertainty/cancellation; do not claim buffered writes are cancelled. Default [connection properties](https://github.com/amqp-rs/lapin/blob/5226cfb1d05b965369d87f75f505a76943274b6a/src/connection_properties.rs) disable recovery/retry; hosts explicitly rebind. |
| Record the client's payload-bearing return traces | [Returned-message source](https://github.com/amqp-rs/lapin/blob/5226cfb1d05b965369d87f75f505a76943274b6a/src/returned_messages.rs) emits a warning with returned-message Debug. The adapter does not log bodies or expose Debug; hosts must filter dependency traces. This known limitation remains explicit rather than treating external logging as payload-safe. |
| Verify actual broker application readiness and preserve fixed node data identity | [RabbitMQ diagnostics](https://www.rabbitmq.com/docs/man/rabbitmq-diagnostics.8) separates runtime ping from application check_running. [Official Docker image](https://github.com/docker-library/rabbitmq) supplies4.3.2 management Alpine. Record executed digest, fixed hostname, persistent volume, limits, and server/Erlang versions; preserve the initial startup-not-ready failure. |
| Allocate independent receiver databases exclusively even with equal clock stamps | [Rust atomic fetch_add](https://doc.rust-lang.org/stable/core/sync/atomic/type.AtomicU64.html) supplies a process-local sequence. [Directory creation](https://doc.rust-lang.org/std/fs/fn.create_dir.html) fails if the path exists. The full gate found an existing HTTPS SQLite lock; four preserved directories for five receiver starts strongly suggest timestamp collision, without directly recording the failed receiver path. A fixed-stamp concurrent test reproduces sharing under create_dir_all and passes with exclusive allocation, bounded collision retry, and Unix0700 creation. Do not mask accidental database sharing with SQLite busy timeout. |
| Inject RabbitMQ acknowledgement loss at the actual protocol boundary | [AMQP specification](https://www.rabbitmq.com/resources/specs/amqp0-9-1.pdf) defines the frame header and terminator. [RabbitMQ extended XML](https://raw.githubusercontent.com/rabbitmq/amqp-0.9.1-spec/main/xml/amqp0-9-1.extended.xml) defines basic class60, ack method80, delivery tag and multiple flag. A test-only loopback proxy drops the first matching server method frame. A direct broker read while delivery remains pending distinguishes actual accepted publication from a synthetic timeout. Client credentials pass through without inspection or logs. |
| Verify durable receiver idempotency separately from publisher acceptance | [RabbitMQ acknowledgements](https://www.rabbitmq.com/docs/confirms) separates publisher confirms from consumer acknowledgement and requires consumer duplicate handling. Connection closure automatically requeues unacknowledged deliveries. [Published ROM command pipeline](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/execution/command.rs) replays stored receipts before mutation. The fixture commits an action with message_id as idempotency key, closes without ACK, reopens native storage, and acknowledges replayed duplicates. This establishes one fixture Resource effect, not universal exactly-once processing. |
| Verify native RabbitMQ TLS without adding a second connection API | [RabbitMQ TLS](https://www.rabbitmq.com/docs/ssl) defines direct TLS, disabled TCP listeners, server certificates, and peer verification. [Pinned lapin connection](https://github.com/amqp-rs/lapin/blob/5226cfb1d05b965369d87f75f505a76943274b6a/src/connection.rs) accepts public OwnedTLSConfig and runtime. [OwnedTLSConfig](https://docs.rs/tcp-stream/0.34.14/tcp_stream/struct.OwnedTLSConfig.html) supplies extra PEM certificates; existing native roots remain active. Test server CA/name verification plus password authentication, expected-cause negatives, retained messages across restart, and explicit host reconnect/rebind. Do not claim exclusive trust pinning, mTLS, or automatic recovery. |
| Verify mounted-key ownership against the executed image | [RabbitMQ TLS file requirements](https://www.rabbitmq.com/docs/ssl) require effective broker-user read access. The first fixture start failed eacces because files used UID999; executing id in the pinned Alpine image established UID100/GID101. Correct ownership without broadening0600 permissions, preserve the failed log, and mount only required leaf/config/CA files. CA signing keys remain outside the container. |

| Construct and validate a private Kafka producer | [Pinned ClientConfig](https://raw.githubusercontent.com/fede1024/rust-rdkafka/v0.39.0/src/config.rs) provides safe native configuration inspection before construction. Enforce all acknowledgements, zero retries, bounded queues, and no automatic topics. Refuse transactional identity. Native defaults do not establish this profile. |
| Separate enqueue from Kafka delivery acknowledgement | [FutureProducer](https://docs.rs/rdkafka/0.39.0/rdkafka/producer/struct.FutureProducer.html) exposes immediate send_result and a delivery future. Await an actual nonnegative partition/offset; uncertainty retires the adapter without claiming cancellation. Serial admission must precede the retirement check to prevent a concurrent caller publishing after retirement. |
| Retain a finite 1 ms deadline with a valid native socket timeout | [librdkafka2.12.1 configuration](https://raw.githubusercontent.com/confluentinc/librdkafka/v2.12.1/CONFIGURATION.md) permits socket.timeout.ms only from10ms. A new regression failed before the10ms socket floor and passed afterward. Application/message deadlines remain1ms, never zero/infinite. |
| Use configure/Make and maintained external OpenSSL | The pinned [native build](https://github.com/fede1024/rust-rdkafka/blob/v0.39.0/rdkafka-sys/build.rs) supports configure/Make. CMake failed because its generated zero-valued OIDC macro was still tested with ifdef, requiring curl headers. Preserve failed evidence rather than adding unused curl. [OpenSSL advisories](https://openssl-library.org/news/vulnerabilities/) and [signed releases](https://openssl-library.org/source/) justify3.5.9 instead of the older vendored3.6.3. Official signature and SHA256 were checked before local static build. |

| Drop an actual successful Kafka ProduceResponse without letting metadata bypass the fault | Apache Kafka4.3.1 [MetadataResponse](https://raw.githubusercontent.com/apache/kafka/4.3.1/clients/src/main/resources/common/message/MetadataResponse.json) and [ProduceResponse](https://raw.githubusercontent.com/apache/kafka/4.3.1/clients/src/main/resources/common/message/ProduceResponse.json) define broker endpoints, flexible version9+, partition error and base offset. A bounded test-only proxy correlates API keys/versions with response correlation IDs, rewrites only the verified single-broker loopback port, and drops the first error-free nonnegative-offset Produce response. An independent direct consumer verifies the record while the adapter future remains pending. This is single-partition fault evidence, not a generic Kafka proxy or consumer-effect guarantee. |
| Execute native QueueFull without external service dependence | [FutureProducer send_result](https://docs.rs/rdkafka/0.39.0/rdkafka/producer/struct.FutureProducer.html#method.send_result) immediately reports native admission failure. A held record fills the actual one-entry librdkafka queue against a bound non-Kafka listener. A second adapter delivery returns Retryable and does not retire. This verifies pre-enqueue refusal, not a synthetic Rust semaphore substitute. |

The fault framing uses Apache's [RequestHeader](https://raw.githubusercontent.com/apache/kafka/4.3.1/clients/src/main/resources/common/message/RequestHeader.json), [ResponseHeader](https://raw.githubusercontent.com/apache/kafka/4.3.1/clients/src/main/resources/common/message/ResponseHeader.json), and [wire types](https://raw.githubusercontent.com/apache/kafka/4.3.1/clients/src/main/java/org/apache/kafka/common/protocol/types/Type.java). Only the required single-broker fixture is accepted; malformed or unsupported fields close the test connection. No arbitrary payload search is used to identify acknowledgements.

| Keep Kafka delivery deadlines below Runtime supervision and its durable lease | Published ROM [Builder](https://raw.githubusercontent.com/pmikstacki/ROM/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/execution/builder.rs) permits explicit delivery_timeout only below the lease. [Channel execution](https://raw.githubusercontent.com/pmikstacki/ROM/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/channels/execution.rs) preserves TimedOut separately from adapter Unknown. Initial live test observed TimedOut under the5s default versus10s adapter deadline. Explicit15s supervision below30s lease permits10s adapter Unknown to persist. No outcome mapping is weakened. |
| Let published Runtime own Kafka retry identity, current authority, and finite attempts | [Work ledger](https://raw.githubusercontent.com/pmikstacki/ROM/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/reaction_work/ledger.rs) and [budget control](https://raw.githubusercontent.com/pmikstacki/ROM/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/reaction_work/control.rs) persist attempts/outcomes and terminal stops. Real SQLite/redb tests reopen Pending and Unknown, retain the ID through explicit host producer replacement, verify Accepted/Done, reject revoked source/service publication, and stop durably after two Unknown attempts. The Kafka adapter adds no retry ledger. |
| Separate Kafka replay from durable Resource receipt deduplication | [rdkafka Consumer](https://docs.rs/rdkafka/0.39.0/rdkafka/consumer/trait.Consumer.html) distinguishes explicit assignment and group offset commits. The fixture disables auto commits and manually assigns Beginning after closing a consumer. [Published command pipeline](https://raw.githubusercontent.com/pmikstacki/ROM/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/execution/command.rs) replays retained receipts. Commit a Resource effect, reopen its native store, replay two records using the unchanged ID, and verify one effect after further reopen. This proves manual replay/receipt idempotency, not automatic consumer-group checkpoint recovery or universal exactly-once effects. The generic receipt fixture is shared with RabbitMQ. |

| Verify native Kafka password authentication only over TLS | The [Apache4.3.1 image configure script](https://raw.githubusercontent.com/apache/kafka/4.3.1/docker/resources/common-scripts/configure) requires PKCS12 filename/password files and a JAAS JVM path for SASL_SSL. [Kafka SSL configuration](https://kafka.apache.org/43/security/encryption-and-authentication-using-ssl/) and [native2.12.1 configuration](https://raw.githubusercontent.com/confluentinc/librdkafka/v2.12.1/CONFIGURATION.md) establish CA and hostname verification. A separate labelled fixture has SASL_SSL broker and SSL controller listeners, only127.0.0.1 bindings, persisted volume and verifiedlocalhostDNS SAN. No plaintext listener is added. Credentials and signer keys remain outside published artifacts. |
| Match native address family to the actual loopback listener | [Native broker.address.family](https://raw.githubusercontent.com/confluentinc/librdkafka/v2.12.1/CONFIGURATION.md) supports v4/v6/any. localhost resolves IPv6 and IPv4 here; the fixture binds IPv4 only. A diagnostic native producer accepted with any/v4 but forcedv6 recorded a sanitized IPv6 refusal. Selectv4 in the fixture configuration without disabling hostname checks or changing the adapter's production defaults. The initial adapter Unknown had a later stored record; its cause remains undetermined. This address-family experiment does not establish that initial cause. |
| Distinguish certificate/authentication failures without exposing native error text | [ClientContext](https://github.com/fede1024/rust-rdkafka/blob/v0.39.0/src/client.rs) supplies typed error callbacks plus raw reason/log strings. An independent native test client discards strings and stores only bounded allowlisted bits. Wrong CA/name require SSL; wrong password requires authentication. TLS on verified-live plaintext Kafka observed BrokerTransportFailure, consistent with [native SSL transport](https://raw.githubusercontent.com/confluentinc/librdkafka/v2.12.1/src/rdkafka_ssl.c). A positive plaintext metadata probe prevents an unavailable peer from satisfying this negative. These causes are independent client evidence, not a new adapter diagnostic API. |
| Wait for bounded broker application readiness after Kafka restart | [Consumer metadata](https://docs.rs/rdkafka/0.39.0/rdkafka/consumer/trait.Consumer.html#tymethod.fetch_metadata) permits finite request timeout. The first restart read failed before readiness. Require correct topic/partition/leader and no metadata errors with300ms requests inside20sec overall before reading. The host explicitly constructs a fresh verified producer; no automatic recovery claim is made. |
| Supply CLI configuration and PKCS12 password files explicitly | [OpenSSL req](https://docs.openssl.org/3.5/man1/openssl-req/) defines explicit configuration; the locally installed binary lacked its default file. Use the configuration from the verified3.5.9 source and retain the original error. [Java keytool](https://docs.oracle.com/en/java/javase/21/docs/specs/man/keytool.html) supports file-sourced passwords. The executed image uses Java21.0.11; no credentials appear in command arguments. |
| Keep the all-TLS fixture on matching loopback endpoints | [Podman5.2.3 network modes](https://docs.podman.io/en/v5.2.3/markdown/podman-run.1.html) defines host networking. It lets broker/controller advertisedlocalhost endpoints match their actual listeners here. Both are explicitly bound127.0.0.1; no published-port assumption or production topology recommendation follows. |

## 2026-10-08: MySQL and MariaDB native executor profiles

Use synchronous `mysql::Conn` 28.0.3 inside the existing dedicated `Executor<C>` worker. This avoids another connection pool or per-query async runtime. [Driver API](https://docs.rs/mysql/28.0.3/mysql/) and the [source assessment](mysql-mariadb-driver-assessment.md) establish the boundary; live profile evidence is separate.

Use separate official MySQL 8.4 and MariaDB 11.8 images, resolved to executed digests, with persistent named volumes. [Official MySQL metadata](https://github.com/docker-library/official-images/blob/master/library/mysql) and [MariaDB metadata](https://github.com/docker-library/official-images/blob/master/library/mariadb) identify provenance. Record actual versions and flush settings. Restrict fixture TCP endpoints to explicit loopback addresses; disable driver socket preference. [Driver options](https://docs.rs/mysql/28.0.3/mysql/struct.OptsBuilder.html) distinguish socket routing and per-I/O deadlines.

Use `VARBINARY` for the identity experiment. [MySQL byte semantics](https://dev.mysql.com/doc/refman/8.4/en/binary-varbinary.html) and [MariaDB VARBINARY](https://mariadb.com/docs/server/reference/data-types/string-data-types/varbinary) distinguish byte strings from character collations. This experiment does not finalize ROM's schema, key-length limits or numeric representation.

Require explicit rollback after a statement-only duplicate-key rejection. [MySQL InnoDB error handling](https://dev.mysql.com/doc/refman/8.4/en/innodb-error-handling.html) explains the distinction. An observed rollback after panic/disconnection does not classify a lost commit acknowledgement.

The initial Rustls feature selection resolved `rustls-pemfile` 2.2.0, flagged unmaintained by [RUSTSEC-2025-0134](https://rustsec.org/advisories/RUSTSEC-2025-0134.html). Preserve that audit and initial inventory. Select the driver's `native-tls` feature instead; its [implementation](https://github.com/blackbeam/rust-mysql-simple/blob/v28.0.3/src/io/tls/native_tls_io.rs) supports certificate and hostname verification. [native-tls](https://docs.rs/native-tls/0.2.18/native_tls/) uses OpenSSL on this Linux host. Reuse the separately verified maintained OpenSSL 3.5.9 installation, without vendored OpenSSL. This selection requires a fresh resolved-graph audit and full gate. Enabling a TLS backend is not evidence of a verified MySQL/MariaDB TLS handshake.

## 2026-10-08: Azure Blob public-port increment

Implement Azure through published `rom_blob::BlobStore`, because the current object-store adapter has no public arbitrary-provider constructor.
Reuse BlobService and its Resource lifecycle. Preserve private ROM boundaries; do not copy private implementation.
[Published adapter](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-blob-object-store/src/adapter.rs) and [family assessment](object-storage-assessment.md) establish this choice.
A future public generic constructor is an alternative; it cannot be assumed today.

Use object_store 0.14.2 with only the Azure provider feature, explicit credentials and a fixed credential type.
Avoid its ambient emulator endpoint branch. Supply an explicit numeric-loopback account path instead.
[Builder source](https://github.com/apache/arrow-rs-object-store/blob/v0.14.2/src/azure/builder.rs) and [builder API](https://docs.rs/object_store/0.14.2/object_store/azure/struct.MicrosoftAzureBuilder.html) establish these controls.
Zero SDK retries and finite Tokio admission/deadlines preserve caller ownership of uncertain writes.
Default HTTP proxy and redirect behavior remains a documented SDK limitation, not a disabled behavior claim.

Use create-only PUT and ETag-conditioned bounded GET rather than unconditional writes or unconditioned two-request reads.
[Azure conditional requests](https://learn.microsoft.com/en-us/rest/api/storageservices/specifying-conditional-headers-for-blob-service-operations) establish these operations.
The 16-MiB object ceiling follows ROM's existing object-store profile; smaller configured limits remain available.
[Published profile](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-blob-object-store/src/configuration.rs) is the compatibility reference.
These limits bound object bytes, not every SDK allocation.

Use official Azurite 3.37.0 with a persistent named volume, explicit custom account, loopback binding, and telemetry disabled.
[Azurite README](https://github.com/Azure/Azurite) documents these controls and its emulator scope.
Its tested digest is recorded separately. Cloud TLS and authorization need a real account and remain unverified.
Fixture SharedKey initialization follows [the official signing procedure](https://learn.microsoft.com/en-us/rest/api/storageservices/authorize-with-shared-key); credentials stay private.

Endpoint regression tests must use valid credentials so another validation cannot mask the intended rejection.
The initial masked tests passed after endpoint policy removal; corrected tests failed against that same mutation.
This follows the repository's negative-fixture quality gate, verified locally rather than inferred from research.
The SDK constructor's separate credential validation in the pinned builder source establishes the independent failure path.

Validate account/container syntax from [Azure naming rules](https://learn.microsoft.com/en-us/azure/azure-resource-manager/management/resource-name-rules) and [blob naming](https://learn.microsoft.com/en-us/rest/api/storageservices/naming-and-referencing-containers--blobs--and-metadata).
Use [Tokio timeout](https://docs.rs/tokio/1.53.1/tokio/time/fn.timeout.html) for the complete operation, without implying synchronous work is preemptible.
Serialize tests that restart their shared emulator with an async mutex held across awaits.
[Tokio Mutex](https://docs.rs/tokio/1.53.1/tokio/sync/struct.Mutex.html) supports that lifetime; the preserved failed run establishes the local interference.
A distinct fixture per test is an alternative, but duplicates service provisioning without improving this single-emulator contract.

## 2026-10-08: Azure BlobService integration on native storage

Reuse published BlobService staging, SHA256 verification, current authorization, and conditional attachment.
Add real Azure provider tests on separately reopened SQLite and redb, instead of implementing another blob lifecycle.
[Public operations](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-blob/src/service/operations.rs) and [staging](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-blob/src/staging.rs) establish these contracts.

A deleted reservation must preserve the observed Denied cause, not the test's initial assumed Conflict.
[Public Runtime read](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/query.rs) rejects a tombstone before authorization.
A distinct stale-revision case remains separate. The preserved initial failure verifies the mistaken test assumption.

Use a test-only wrapper to pause after confirmed provider creation and establish the attachment race.
Direct Azure reads independently verify retained bytes. The wrapper is not a network acknowledgement-loss substitute.
The assessment [documents lifecycle ownership and trusted cleanup](azure-blob-lifecycle-assessment.md) from primary sources.
Drain BlobService, drain Runtime, then drop Runtime owners before native reopen.
Retain unattached and detached bytes because neither a receipt nor a missing Resource proves grace and quiescence.
No automatic deletion is added; reconciliation and actual wire failure evidence remain separate work.


## 2026-10-08: Maintained S3 fixture candidate

Qualify SeaweedFS 4.48 first through the existing public ROM S3 adapter.
The [candidate assessment](s3-compatible-fixture-assessment.md) compares current official source, distribution, license, and conditional-write behavior.
MinIO community now states it is no longer maintained; preserve ROM's historical profile without selecting it as a maintained fixture.
[Current MinIO README](https://github.com/minio/minio/blob/master/README.md) establishes that change.
[SeaweedFS quickstart](https://github.com/seaweedfs/seaweedfs/blob/master/README.md) identifies the project image and authenticated mini launch.
Garage remains an alternative needing its own conditional-create qualification.
The actual 4.48 artifact download and isolated version check are recorded separately; neither establishes S3 conformance.


## 2026-10-08: Real S3 qualification and uncertainty

Reuse the public S3 Adapter and public blob conformance in an independent consumer.
Versioned [SeaweedFS mini](https://github.com/seaweedfs/seaweedfs/blob/4.48/weed/command/mini.go) establishes private credential, bucket, persistence, and service controls.
The real fixture is loopback-only with preserved named storage and fixed image digest.
No private ROM implementation is copied.

Keep strict eight-way responsiveness qualification separate from Unknown/reconciliation correctness.
The strict test fails; source inspection cannot convert an uncertain write into confirmed rollback or Conflict.
[Versioned lock and SDK assessment](seaweedfs-race-assessment.md) identifies possible contention without claiming traced causality.
Use [reqwest typed errors](https://docs.rs/reqwest/0.13.5/reqwest/struct.Error.html) rather than substring status searches for diagnostic causes.
Actual typed native timeouts are recorded; possible server-path causality remains unresolved.

Add the S3 candidate gate to the full verifier even while qualification is incomplete.
Retain failing evidence and the original deadline, retry policy, and eight simultaneous requests.
A separate test verifies all outcome categories, at most one acknowledged winner, and unchanged bytes after explicit retry conflicts.
Keep uncertain objects because the public maintenance contract requires grace and quiescence before deletion.
The original full scope, cloud gates, and provider alternatives remain active.


## 2026-10-08: Trace S3 locking before changing the fixture

Use a separate verbose reproduction and retain the original failed qualification.
[Exact binary lock source](https://github.com/seaweedfs/seaweedfs/blob/530be3e37/weed/cluster/lock_client.go) identifies trace stages and one-second contention waits.
The [executed assessment](seaweedfs-race-assessment.md#executed-lock-path-reproduction) confirms lock use but leaves routing availability unresolved.
[AWS GetBucketVersioning](https://docs.aws.amazon.com/AmazonS3/latest/API/API_GetBucketVersioning.html) supports the read-only check that excludes enabled versioning.
Preserve timeout and uncertain outcomes; investigate ring initialization without weakening eight-way qualification.
Record forced trace termination separately from orderly lifecycle evidence.


## 2026-10-08: Distinguish finalization from recovery locking

Do not infer missing routing from distributed locks alone.
[Exact SeaweedFS PUT source](https://github.com/seaweedfs/seaweedfs/blob/530be3e37/weed/s3api/s3api_object_handlers_put.go#L967) also locks recovery after routed precondition rejection.
[Executed stage evidence](../verification/s3-recovery-path-2026-10-08.json) confirms S3 ring delivery before the traced burst and no routed-failure warnings.
Preserve the failed profile and investigate maintained alternatives without altering the public client's fixed policy.
Conditional-create source enforcement is a prerequisite for selecting an alternative; generic S3 compatibility is insufficient.


## 2026-10-08: Select independent S3 fixture profiles

Keep one public-port acceptance suite and bind each explicit profile to a fixed loopback endpoint and restart target.
[Garage exact source review](garage-conditional-create-assessment.md) excludes that candidate's inspected PUT behavior.
[RustFS exact source review](rustfs-conditional-create-assessment.md) establishes precondition checking under its commit guard.
Select RustFS 1.0.1 for an additional local qualification, with one CPU and 1 GiB per official testing guidance.
Preserve SeaweedFS, its volume, and the strict failed evidence as a separate profile.
No scenario, public deadline, retry limit, constructor, or dependency changes.
The configured fixture controls the existing verifier gate; passing RustFS does not qualify SeaweedFS or cloud S3.


## 2026-10-08: Share native blob lifecycle acceptance

Use the public BlobStore contract to run identical native BlobService scenarios on Azure and S3.
[The lifecycle assessment](s3-blob-lifecycle-assessment.md) establishes staging, authorization, attachment, detachment, and drain/reopen behavior at the pinned public revision.
Extract existing Azure test orchestration into one shared harness; preserve its assertions and test names.
Add optional published SQLite/redb dependencies to the independent consumer's s3-lifecycle feature.
Require all existing S3 port cases and the two native lifecycle cases through scripts/check-s3.

Retain post-provider-success pause as an attachment race, not a wire acknowledgement fault.
A mutation that omits real provider creation must fail during verified reading; restored source must pass again.
Keep detached and unattached objects because [public trusted-delete requirements](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-blob/src/storage.rs) require grace and quiescence.
Use shared acceptance only for these common guarantees; retain separate provider TLS, credential, retention, and wire fault tests.

## S3 actual HTTP response loss and native recovery

Date: 2026-10-08. Source: [primary-source assessment](s3-wire-ack-loss-assessment.md).
Select a bounded test-only loopback HTTP/1 proxy to observe backend success, then suppress the response.
A post-SDK pause cannot establish a lost wire acknowledgement; retain that earlier scenario separately.
Preserve signed request bytes and the production public adapter. Count forwarded PUT requests instead of assuming every transport layer disables retries.
Use the existing RustFS profile and both public native stores. No dependency or production API changes are needed.
The proxy accepts bounded Content-Length framing; unsupported framing fails rather than guessing message length.
[HTTP/1 message length](https://www.rfc-editor.org/rfc/rfc9112.html#section-6.3) governs the parser.
A finite standard-library worker with [socket timeouts](https://doc.rust-lang.org/std/net/struct.TcpStream.html) isolates test-only blocking I/O.
The proxy closes client connections after framed responses, so set response-only Connection: close and remove nominated hop-by-hop fields.
This follows [RFC 9110 section 7.6.1](https://www.rfc-editor.org/rfc/rfc9110.html#section-7.6.1) and [RFC 9112 section 9.6](https://www.rfc-editor.org/rfc/rfc9112.html#section-9.6).
An initial fixture preserved the backend keepalive response header while closing; recovery failed before GET reached the backend.
After correcting that hop's connection semantics, real recovery GET and both native reopen cases passed.
Do not broaden the result to cloud TLS, process crash, other S3 services, or cleanup authorization.

## CockroachDB shared PGwire executor qualification

Date: 2026-10-08. Source: [primary-source assessment](cockroachdb-executor-assessment.md).
Select official v26.3.2 and reuse pinned postgres 0.19.14 with the existing connection-owning Executor.
Extract unchanged PostgreSQL deadline and panic assertions into shared PGwire acceptance helpers; keep server-specific settings separate.
The existing immutable public ROM revision lacks required full incremental Storage exports, so no private implementation is copied.
Require actual 40001 from controlled concurrent transactions, followed by explicit rollback and one complete fresh transaction reread.
Keep 23505 application rejection and ambiguous 40003 separate; do not retry an unknown non-idempotent commit.
Use BYTEA primary-key checks rather than assuming text collation preserves Resource identity.
[Compatibility](https://www.cockroachlabs.com/docs/v26.3/postgresql-compatibility) and [retry semantics](https://www.cockroachlabs.com/docs/v26.3/transaction-retry-error-reference) support these choices.
The exact executable restricts single-node RPC to localhost 26257; use its advertised separate SQL listener on 26259 with only host loopback 55457 published.
Exact binary help and rejected startup attempts remain local evidence; networking behavior was confirmed on the executed artifact.
Select experimental two-CPU/2GiB limits and disk storage under the documented development-only single-node licensing scope.
An initial abrupt restart caused a cold DDL 57014 under three seconds; keep the failure and select a ten-second statement budget for Cockroach only.
[Session-variable documentation](https://www.cockroachlabs.com/docs/v26.3/set-vars) supports configuring it at connection initialization.
A statement timeout does not establish rollback. PostgreSQL's existing three-second statement budget remains unchanged.
Require stopped exit0 before restarting the same persistent container, with bounded 60 second grace rather than accepting SIGKILL as graceful.
[Docker stop semantics](https://docs.docker.com/reference/cli/docker/container/stop/) define that distinction.
Review identified fixed-endpoint and hostaddr hazards; reject them before driver connection to bind restart evidence to the actual fixture.
[postgres configuration](https://docs.rs/postgres/0.19.14/postgres/config/struct.Config.html#method.hostaddr) explains the alternative network-address field.
No dependency version changes or production retry-policy implementation were adopted; full Storage and ownership-fence gates remain pending.

## Oracle OCI executor qualification, 2026-10-08

Use rust-oracle 0.6.3 through the existing dedicated-worker executor, with optional independent-consumer dependencies.
[The tagged driver](https://github.com/kubo/rust-oracle/tree/v0.6.3) supports synchronous OCI and external runtime client libraries.
Keep statement, row, and native handles within the worker. Do not introduce a second executor or serialize the ROM ledger.
Use actual explicit commit and rollback. Distinguish ORA-00001 statement rejection from complete transaction rollback.
The [source assessment](oracle-executor-assessment.md) records Oracle's transaction and empty-string semantics.

Select official full Oracle Free and Basic Instant Client artifacts; pin the executed image digest and verify the client SHA-256.
[Official downloads](https://www.oracle.com/database/technologies/instant-client/linux-x86-64-downloads.html) publish the selected archive's size and digest.
The inspected BASIC_LICENSE controls this archive; do not substitute older webpage terms for the artifact's own notice.
Restore original ZIP symlink metadata and permissions, without changing ELF bytes. Retain all native notices outside Git.
Use protected fixture credentials and fixed loopback connection configuration under [official Podman instructions](https://github.com/oracle/docker-images/blob/main/OracleDatabase/SingleInstance/README.md).
Use separate connection and round-trip budgets according to [Oracle Net parameters](https://docs.oracle.com/en/database/oracle/oracle-database/26/haovw/oracle-net-tns-string-parameters.html).
These are experimental test budgets, not total operation deadlines.

The initial restart assertion required exit 0, but the pinned image returned 143 after its shutdown handler completed.
Inspect the pinned image's actual scripts and retain fresh shutdown logs; do not infer SIGKILL from SIGTERM status.
Require exited 143 plus fresh `ORACLE instance shut down.` confirmation for this exact fixture.
[Oracle's shutdown source](https://github.com/oracle/docker-images/blob/main/OracleDatabase/SingleInstance/dockerfiles/23.26.0/shutDown.sh) uses shutdown immediate.
This is graceful retained-store restart evidence, not a power-loss or replicated durability qualification.

## Initial secrets/KMS host adapter, 2026-10-08

Implement host-owned references, bounded secret bytes, and separate KMS context/AAD contracts under approved Task 7.
The [public ROM host boundary assessment](secrets-kms-assessment.md) excludes plaintext credentials from Resource fields and provenance.
Use approved aliases rather than interpreting Resource text as a URL or provider path.
Select a 64-byte ASCII alias grammar and 256-entry maps as explicit initial host policy, not provider limits.
Use separate positive pinned versions and actual returned version validation.
The [KV v2 API](https://openbao.org/docs/api/secret/kv/kv-v2/) specifies nested data and version selection.

Reuse the existing reqwest 0.13.5, serde_json 1.0.151, serde 1.0.229, Tokio 1.53.1, base64 0.22.1 and zeroize 1.9.1 versions.
The initial incorrect serde_json 1.0.149 pin caused a graph-resolution failure; preserve that failure and use the existing exact pin.
Disable proxies, redirects and client retries. Use one admitted permit through request construction, body reading and decoding.
[ClientBuilder](https://docs.rs/reqwest/0.13.5/reqwest/struct.ClientBuilder.html) and [retry::never](https://docs.rs/reqwest/0.13.5/reqwest/retry/fn.never.html) specify these settings.
Bound streamed success and error bodies before JSON parsing. Keep every public error category free of upstream text.
Serialize borrowed wire fields, avoiding an intermediate plaintext-bearing JSON value.
[Serde serialization](https://docs.rs/serde/1.0.229/serde/trait.Serialize.html) supports borrowed request fields.
Clear owned encoded plaintext buffers through zeroize; do not claim HTTP, parser or host copies are completely erased.

Select the existing derived aes256-gcm96 key profile and separate derivation context from authenticated associated data.
[Transit API](https://openbao.org/docs/api/secret/transit/) specifies both inputs.
The exact OpenBao 2.7.1 fixture accepted this combination and rejected changes to either input through the Rust adapter.
TLS negative acceptance uses the fixture's private CA and an IPv4-mapped IPv6 reference identity absent from its SAN.
[RFC 9525](https://www.rfc-editor.org/rfc/rfc9525.html#section-6.4) requires exact IP identity comparison.
An independent curl probe failed with certificate name mismatch, exit 60; the Rust client rejected the same endpoint.
This is Linux fixture evidence, not a universal routing assertion for IPv4-mapped addresses.

The initial administrative initialization exceeded a five-second caller read budget after committing initialization.
Authoritative GET sys/init reported initialized, but the response containing keys was not retained.
Preserve that stopped container and data. Initialize a separate fresh Raft directory with a 60-second administrative budget.
This budget does not change runtime adapter deadlines or add automatic retries.
The [initialization contract](https://openbao.org/docs/commands/operator/init/) separates initialization and unseal from runtime use.
A later HTTP500 mount failure remains preserved; its cause is unproven.
After inspecting existing mounts, an explicit mount request returned204; setup resumed with its retained known initialization response.

The initial Rust service cases and independent compile checks do not complete rotation, restart, token renewal, native host integration or release acceptance.
Keep those gates, plus separate Vault/AWS/Azure profiles, open in the full goal.

### OpenBao rotation and persistent recovery acceptance

Use fresh test-only KV paths, Transit keys and restricted child tokens for destructive acceptance cases.
The [KV v2 API](https://openbao.org/docs/api/secret/kv/kv-v2/) defines deletion, destruction and version selection.
The [Transit API](https://openbao.org/docs/api/secret/transit/) defines rotation and minimum decryption versions.
The [token API](https://openbao.org/docs/api/auth/token/) defines explicit token revocation.
These cases preserve the baseline fixture and test actual bytes, not only successful response status.

Run service cases serially because persistent recovery stops and starts the shared isolated fixture.
Keep the same Raft directory and perform one explicit unseal with privately retained fixture material.
Use a fresh Rust client to verify persisted KV versions and decrypt a pre-restart ciphertext.
The [Raft contract](https://openbao.org/docs/configuration/storage/raft/) and [unseal API](https://openbao.org/docs/api/system/unseal/) specify this separation.
Read-only readiness polling is test administration; it adds no runtime adapter retries.

Bound Docker subprocess execution separately from its shutdown grace period.
[Coreutils timeout source](https://github.com/coreutils/coreutils/blob/master/src/timeout.c) specifies the outer deadline and subsequent kill interval.
GNU documentation requests failed during research; the upstream source was available.
Require an independent certificate-name rejection, not generic connection failure, alongside the redacted Rust result.
The [curl manual](https://curl.se/docs/manpage.html) documents certificate verification and transfer deadlines.
Use English diagnostic output for this fixed Linux fixture; do not claim cross-platform curl diagnostic compatibility.
The required `check-openbao` gate fails when protected configuration or its real service is absent.

### Host secret preparation, expiry and bounded work

Create a three-second, nonrenewable test token with the same explicit maximum TTL.
Check the issued lease before testing successful resolution followed by expiry denial.
The [token API](https://openbao.org/docs/api/auth/token/) defines these issuance controls.
An update-only token receives Denied for a missing Transit key; administrative reads confirm the key remains absent.
The [Transit capability contract](https://openbao.org/docs/api/secret/transit/) distinguishes update from create/upsert.
Preserve the initial test's incorrect Rejected expectation as failure evidence.

Hold one real read future at Pending before trying another operation against a one-permit adapter.
Require Busy, then complete the original read and verify subsequent admission.
The [Tokio semaphore API](https://docs.rs/tokio/latest/tokio/sync/struct.Semaphore.html) specifies nonblocking admission.
The pinned-version documentation URL was unavailable; inspect the pinned local source as well.
Pause only the isolated real service to verify the 100ms complete-operation deadline.
Use a drop guard and explicit unpause before assertions. Require same-client recovery afterward.
[Podman pause](https://docs.podman.io/en/latest/markdown/podman-pause.1.html) and [unpause](https://docs.podman.io/en/latest/markdown/podman-unpause.1.html) define these process controls.
This Linux test allowance is not a provider latency guarantee.

Use the unchanged public [IdentityProvider host boundary](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-identity/README.md).
Authorize the provider read before secret resolution. Validate its opaque alias through the resolver's approved map.
Re-read the activation after resolution and reject a changed revision or disabled provider.
The example prepares material for a verifier; it does not establish an authenticated actor.
The host must still configure the verifier from the captured activation and follow core's verification/binding checks.
Check SQLite/redb state, journal, receipts, fixed host errors and retained files for fixture markers and the runtime token.
This establishes the tested host path and captures, not universal prevention of an application's explicit secret copies.
Add only pinned rom-identity to the independent consumer; retain previous dependency identities and checksums.

Two serial runs returned HTTP500 while creating a fresh policy after restart; an isolated run succeeded.
Diagnostic runs subsequently passed, so the exact upstream cause remains unproven.
Preserve both failures without adding mutation retries or skips.
After explicit unseal, require default [health status200](https://openbao.org/docs/api/system/health/) before administration proceeds.
The [configuration contract](https://openbao.org/docs/configuration/) permits standby reads by default; a successful read alone is insufficient write readiness.
Service logs also report the fixture's disable_mlock field as unsupported. It has no established effect in this 2.7.1 profile.
Retain the original configuration and make no locked-memory claim.

Package the three family crates with exact versions on internal path dependencies.
[Cargo packaging](https://doc.rust-lang.org/cargo/commands/cargo-package.html) requires versions and rewrites distributed manifests without paths.
Verify the generated archive sources through a separate consumer and local patches before claiming package acceptance.
No registry publication is authorized by this local packaging step.

The initial plain and explicit-registry Cargo package runs could not resolve unpublished rom-secrets.
Preserve both failures. Use local patches only for package assembly, then consume extracted normalized archives through separate local patches.
[Cargo's dependency patch contract](https://doc.rust-lang.org/cargo/reference/overriding-dependencies.html#the-patch-section) supplies local sources for registry names.
The independent consumer executes actual KV resolution and derived Transit round-trip; it does not use the workspace source paths.
Package assembly's no-verify option is followed by this independent compilation, execution and Clippy gate.
This establishes local archive-consumer acceptance, not registry availability or publication.

Preserve raw verification logs byte-for-byte, including Rust test output's trailing spaces.
Apply Git's raw-evidence attributes only to docs/verification/*.log; code and prose keep their whitespace checks.
The [Git attributes contract](https://git-scm.com/docs/gitattributes) defines unset text and whitespace behavior.
The staged raw output initially reported two whitespace diagnostics. After the attributes change, the same staged bytes pass the check.
This metadata check follows the full backend verifier; no runtime input changes or log edits accompany it.

## 2026-10-08: projection protocol and delayed writes

Sources: [OpenSearch Bulk](https://docs.opensearch.org/latest/api-reference/document-apis/bulk/), [delete-version lifetime](https://docs.opensearch.org/latest/api-reference/document-apis/delete-document/), [Qdrant consistency](https://qdrant.tech/documentation/scaling/consistency-guarantees/), [tagged conditional operations](https://github.com/qdrant/qdrant/blob/v1.19.2/lib/collection/src/operations/point_ops.rs).

Select separate backend adapters behind shared durable intent/checkpoint orchestration and current ROM disclosure.
Use persistent tombstone revision state; physical-delete retention cannot establish indefinite stale-replay protection.
Do not accept local single-writer serialization alone as Qdrant's remote revision fence.
A submitted old request can remain active after an HTTP timeout and complete after a newer write.
The conditional check/application boundary needs tagged source inspection and actual concurrency tests before protocol admission.
Strong ordering is an alternative ordering setting, not evidence of ROM revision comparison.
The new spec and plan retain this gate explicitly; no runtime/provider support claim follows from research.
Application bounds are conservative fixture policy, not vendor maxima, and remain subject to measured qualification.
Observed host memory available was 9582 MiB and disk available was 193 GiB; these snapshots do not reserve service capacity.
Observed vm.max_map_count was 1048576, above OpenSearch's documented 262144 requirement. No global setting was changed.

### Executed Qdrant native-fence probe

The [tagged-source report](qdrant-revision-fencing.md) identifies conditional resolution under an exclusive submit lock and queue drain.
Select conditional whole-point upsert, strict lexicographic u32 revision halves, and persistent same-ID tombstones for single-node qualification.
The [actual probe record](../verification/qdrant-fence-probe-2026-10-08.json) confirms indexed and unindexed high-u64 comparison, concurrent inserts, and empty-vector tombstones.
Its physical-delete counterexample confirms that a stale request can reinsert a deleted point.
A completed conditional operation can be a no-op. Require stored identity/revision/content reconciliation before acknowledging the worker page.
The exploratory restart preserved u64::MAX and concurrent highest revisions. It did not qualify tombstone restart or acknowledgement loss.
The official image runs with an experimental 1-CPU/512-MiB fixture allowance. This is not a vendor resource minimum.
No production adapter, ROM checkpoint worker, or support claim follows from this probe.

Independent source review found default Python redirects in the exploratory credential-bearing probe.
Reject redirects explicitly. A controlled loopback redirect test confirmed that the target received zero requests.
The corrected probe and persistent restart checks passed again. Updated evidence hashes identify the executed corrected source.
