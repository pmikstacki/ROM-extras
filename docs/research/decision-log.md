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

## 2026-10-08: executed projection fault and OpenSearch fixture profiles

Sources: [Qdrant tagged conditional tests](https://github.com/qdrant/qdrant/blob/v1.19.2/tests/openapi/test_conditional_update.py), [Python fixture server](https://docs.python.org/3.12/library/http.server.html), [OpenSearch TLS](https://docs.opensearch.org/latest/security/configuration/tls/), [certificate authentication](https://docs.opensearch.org/latest/security/authentication-backends/client-auth/), [initialization tool](https://docs.opensearch.org/latest/security/configuration/security-admin/), [disk settings](https://docs.opensearch.org/latest/install-and-configure/configuring-opensearch/cluster-settings/), [audit fields](https://docs.opensearch.org/latest/security/audit-logs/field-reference/).

Select an isolated TLS forwarding proxy for actual Qdrant delayed-request and response-loss qualification.
The held old request reaches the backend only after its client times out and newer state is accepted.
Use different unit vectors for old and new writes. This establishes vector retention as well as revision/payload retention.
Drop one real completed write response, retrieve its state directly, then verify newer state and stale replay.
Repeat indexed and unindexed conditions and persistent restart, including u64::MAX tombstones.
These are executed native protocol cases, not a ROM checkpoint implementation.

Select OpenSearch certificate authentication for the first local fixture instead of demo certificates or shared admin passwords.
Use distinct writer/reader/operator certificates and a private CA. Limit writer index access to `rom_extras_*`.
Keep CA and client private keys outside the server-mounted TLS directory after verified bootstrap.
Retain request-body audit logging disabled; the official field reference identifies its disclosure surface.
This is an explicit fixture policy, not a universal assertion about all upstream logs.

Pin the official amd64 OpenSearch 3.9.0 image at `sha256:13487e0953520edf6cc866dfc672fd67a70ab122aa7d84d4a6c97106f84d3f84`.
Use an experimental 2-CPU/2-GiB allowance with 512-MiB heap. Actual startup and bounded native probes succeeded; production sizing remains unqualified.
The high-disk watermark initially blocked the security index. Actual filesystem availability was 198621110272 bytes when inspected.
Use documented absolute reserves 20/10/5 GiB for this fixture, retaining disk protection. Percentage thresholds were an unsuitable local alternative.
The immediate retry still saw the previous asynchronous block. Logs and settings then confirmed automatic release before further initialization.
Single-type bootstrap uploaded config but waited for missing types. Complete bootstrap loaded all nine expected types successfully.
Preserve failed output, rather than reporting those attempts as successful setup.

Actual OpenSearch evidence covers strict native versions, signed overflow rejection, HTTP-200 Bulk partial failures, scoped credentials, live/tombstone filtering, and restart.
A future adapter must reconcile equal-revision conflicts and unknown outcomes against stored identity/content; conflict alone is not replay success.
Add the real-service probes as a required `check-all` gate. A missing Python executable, private fixture, or backend is a failure.
Standard-library socket timeouts do not establish the future adapter's total request deadline.

The full verifier, including the new required native probe gate, exited successfully with 168 frozen input files unchanged.
The [execution record](../verification/projection-probes-full-verifier-2026-10-08.json) retains exact source hashes, service profiles, raw output, and incomplete requirements.
Public ROM HEAD was independently rechecked and remains `d7ef529040eec60dc869034c2d33130219db85fe`.
The referenced architecture chat remains active on release qualification; its private changes are not dependencies of this increment.

## 2026-10-08: projection checkpoint transactions and format

Sources: [checkpoint-store assessment](projection-checkpoint-store.md), [redb 4.3.0 commit contract](https://github.com/cberner/redb/blob/v4.3.0/src/transactions.rs#L1746), [native builder and read-only open](https://github.com/cberner/redb/blob/v4.3.0/src/db.rs), [public ROM ownership](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-backup/src/native_ownership.rs), [deterministic CBOR alternative](https://www.rfc-editor.org/rfc/rfc8949.html#section-4.2), [SHA-256 standard](https://csrc.nist.gov/pubs/fips/180-4/upd1/final).

Select extras-owned redb 4.3.0 with explicit Immediate commits, a bounded cache, and public ROM native ownership.
Direct rusqlite is a viable later checkpoint implementation; private ROM engine handles and Resource actions are unsuitable substitutes.
Select read-only application preflight and bounded private-copy repair before recovering an unclean original.
Writable-open-then-marker-check can modify a future-format source before rejection. Reject that ordering.
Fail-closed repair-required behavior is useful during implementation, but does not complete required accepted-format crash recovery.

Select intent-before-dispatch and exact remote metadata reconciliation before atomic cursor/key publication.
Reject unexpected higher remote revisions as divergence instead of manufacturing an exact acknowledgement.
Reserve generation switching for Task 6; local checkpoint and remote alias commits cannot be one atomic operation.
Retain the last page's preparation/completion tokens for immediate unknown-commit reconciliation, not unbounded historical receipts.
An uncertain native commit retires its engine while retaining ownership. Its error carries the exact token needed after reopen.

Select a versioned fixed-field binary schema with length-delimited UTF-8 keys and preallocation bounds.
General JSON/CBOR codecs are alternatives; they still need exact canonicalization and bounded schema validation.
The fixed record set does not need their additional extensibility. This is a private extras format, not a claim of CBOR conformance.
Select domain-separated SHA-256 using the existing resolved dependency; exact original-key checks remain mandatory for collisions.
The [concrete contract](../superpowers/specs/2026-10-08-projection-checkpoints.md) states signatures, format, two-phase sequences, and acceptance requirements.
Admission values are initial application policy and need measured tests; research does not establish those tests passed.

Independent source-only review found no actionable recovery issue in the initial contract.
Concrete encoding and result categories were subsequently added. Runtime tests, dirty recovery, native I/O faults, and full verifier acceptance remain pending.
Further independent review found that a sequence and intent digest cannot prove an absent preparation after process restart.
Add the previous control-state digest to the token and bind it to the profile; require caller retention across restart.
A fresh handle uses durable state plus that token, not lost in-memory attempt bookkeeping.
If both token and uncommitted intent are lost, no absent-attempt classification is available.
The architecture reference was read again. Its active private release work is not an extras dependency.

## 2026-10-08: implemented checkpoint-store increment and archive consumers

Sources: [redb tagged native commit and flush](https://github.com/cberner/redb/blob/v4.3.0/src/tree_store/page_store/page_manager.rs#L1783), [native StorageBackend contract](https://github.com/cberner/redb/blob/v4.3.0/src/db.rs), [Cargo archive normalization](https://doc.rust-lang.org/cargo/commands/cargo-package.html), [multiple dependency locations](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#multiple-locations), [consumer patches](https://doc.rust-lang.org/cargo/reference/overriding-dependencies.html#the-patch-section).

Implement fixed-schema local metadata and page transactions in a separate crate behind existing public ROM ownership.
Keep actual delivery and authorized source reconstruction out of the store; they remain required worker/adapter tasks.
Native write faults and faults after real file synchronization exercise the same production commit path through a controlled test-only file backend.
These distinguish absent versus applied transitions after reopen; they are not simulated HTTP or arbitrary hardware power-loss claims.
An unwinding native commit panic also retires the handle and retains the reconciled durable intent in the executed case.

Independent review found that token equality alone did not establish completed cursor/key state.
Retain one bounded completed page, validate its resulting checkpoint and each affected key, and include it in control-state hashing and CAS.
Actual redb regressions changed the completed cursor and deleted an acknowledged key; both failed before the correction and passed afterward.
The targeted independent source recheck resolved the finding.

Declare ROM dependency versions alongside immutable git revisions so Cargo can normalize the archive manifest.
Use an archive consumer with explicit patches back to immutable public ROM, rather than private source copies or a registry-publication claim.
Share archive extraction and consumer setup with the existing secrets gate; both families consume generated normalized Cargo archives.
The new package consumer exercises actual local checkpoint close/reopen and token reconciliation independently of workspace feature unification.
The first public-consumer attempt lacked the documented native OpenSSL environment. Preserve that failure; the configured native environment passed.
The root and public-consumer lockfile changes add only the new local crate; registry versions and checksums remain unchanged.
Dependency inspection and complete verification remain separate acceptance evidence, not conclusions from those lockfile diffs.

Full local verification passed for the frozen 189-file runtime set, including both archive-consumer gates and real backend restart probes.
The [execution record](../verification/projection-checkpoint-full-verifier-2026-10-08.json) identifies source hashes, command, results, and limits.
All three audited graphs contain zero known vulnerabilities at inspection time.
The independent public consumer reports informational [RUSTSEC-2024-0436](https://rustsec.org/advisories/RUSTSEC-2024-0436.html) for unmaintained paste 1.0.15 through oracle 0.6.3.
Preserve the warning; do not change an existing transitive driver dependency solely to suppress it.
The new archive-consumer graph has no advisory warnings. A maintained-fork substitution requires separate driver qualification.
This completes the checkpoint-store increment, not Task 2 or the full ROM-extras goal.

## 2026-10-08: cooperative checkpoint startup cancellation

Sources: [Tokio blocking-task cancellation and dedicated threads](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html), [Rust atomic synchronization](https://doc.rust-lang.org/std/sync/atomic/), [thread join ownership](https://doc.rust-lang.org/stable/std/thread/struct.JoinHandle.html), [redb 4.3.0 repair callbacks](https://github.com/cberner/redb/blob/v4.3.0/src/db.rs).

Select a monotonic shared atomic cancellation token rather than interpreting task abort as native I/O termination.
Use release/acquire for the request; no projected values or diagnostics travel through the flag.
Check admission, every 256 scanned key records, and each 64-KiB private-copy chunk; bound the remaining control scans through existing record limits.
Use redb's repair callback to request abort at native callback points, without a latency guarantee for I/O or repair internals.
Keep existing open/reopen paths compatible through unrequested tokens. Never interrupt commits or infer rollback from cancellation.
A pre-cancelled reopen preserves the live engine; cancellation after retirement keeps ownership until retry or close.
Tests use actual files and deterministic segment boundaries. They do not qualify a hard shutdown deadline.
The first combined test run found two independently compiled temporary-directory counters with the same process prefix.
Share one unit-test fixture module so both test families allocate from one counter; preserve the failing test output.
Remaining alternatives include a host cancellation trait or Tokio token. Neither is required for a single irreversible flag without a new dependency.
The worker's dedicated thread and shutdown join remain pending; source research does not establish their implementation.

## 2026-10-08: pending-intent reconstruction source selection

The [worker recovery research](projection-worker-recovery.md) selects the pinned public authorized journal, not direct rows or current reads.
Recovery covers the immutable old interval even when a replay page now extends beyond its endpoint.
Full intent equality is required after current authorization and immutable mapping; lost continuity and changed disclosure remain separate failures.
Typed commands on a bounded dedicated storage thread preserve native transaction and ownership boundaries.
These choices prepare the next implementation. No compiled worker or native history acceptance is claimed here.

The cancellation increment passed the full local verifier, both archive consumers, and real service restart probes.
The [execution record](../verification/projection-cancellation-full-verifier-2026-10-08.json) retains the 192-file source fence, failures, unchanged-lock audit references, and limits.
Independent source review reported no actionable findings; it did not run tests.

## 2026-10-08: typed bounded storage-owner execution

Sources: [nonblocking bounded admission](https://doc.rust-lang.org/std/sync/mpsc/struct.SyncSender.html), [join versus detach](https://doc.rust-lang.org/std/thread/struct.JoinHandle.html), [Tokio 1.53.1 oneshot across synchronous threads](https://docs.rs/tokio/1.53.1/tokio/sync/oneshot/index.html), [pinned Tokio license/features/MSRV](https://raw.githubusercontent.com/tokio-rs/tokio/tokio-1.53.1/tokio/Cargo.toml), [Cargo archive patches](https://doc.rust-lang.org/cargo/reference/overriding-dependencies.html).

Reuse the existing shared executor behind a non-cloneable storage owner, rather than duplicating its queue/health/exception handling.
Restrict public submissions to bounded typed checkpoint operations. The generic closure helper remains private.
Select one queued operation plus active work; zero capacity would make try-send admission depend on the receiver's rendezvous timing.
Larger queues increase retained payloads without helping the current single-intent protocol.
Join on explicit shutdown and owner drop; do not inherit the generic executor's detach-on-drop behavior.
Use oneshot responses for asynchronous waits. A dropped response never implies cancellation of an accepted command.
Initialization, shutdown and drop remain blocking and require the host's bounded blocking context; no hard deadline is asserted.
Preserve checkpoint failures/tokens; treat lost admitted responses and unexpected job panics as unclassified, not rolled back.
Tokio remains pinned to the existing 1.53.1 lockfile version: MIT, declared Rust 1.71, default features disabled, direct production feature sync.
Tests and archive consumers explicitly request rt. Public consumer's named feature is required by the executable gate, without backend absence skips.
Root and independent-consumer registry package versions/checksums remain unchanged; only local projection dependency edges changed.
The normalized package gate now extracts the executor archive as well. This is archive acceptance, not registry publication.
Initial independent-consumer compilation exposed a missing feature/import, then a duplicate drop. Preserve both failures; corrected compilation passed.
Worker history and provider orchestration remain incomplete; this increment qualifies the native storage owner only.

## 2026-10-08: CockroachDB fixture graceful-stop budget

Source: [CockroachDB 26.3 node shutdown and termination grace](https://www.cockroachlabs.com/docs/v26.3/node-shutdown#termination-grace-period-on-kubernetes).
The first storage-worker full verifier failed the unchanged Cockroach restart gate: stop exceeded its outer limit, and the remaining fixture tests saw a poisoned mutex.
The stopped container had exit 137, not OOM. Native logs still showed SQL draining and descriptor leases near the 60-second grace limit.
Current shutdown settings were inspected: initial/connections wait 0s, transactions timeout 10s, lease-transfer iteration timeout 5s.
Select the vendor's five-minute grace recommendation for this persistent fixture; retain a finite 315-second outer command budget.
Keep the exact exited-0 assertion and same-disk persistence checks. Do not accept SIGKILL or skip the restart test.
This changes fixture termination admission, not driver operation deadlines or projection shutdown guarantees.
The selected budget does not explain every source of drain delay; resource pressure and native lease cleanup remain possible contributors.
Preserve the original failed verifier, source fence, stopped state, and native drain tail. No persistent store or historical table is deleted.

The revised full verifier passed with all 195 frozen runtime hashes unchanged, including seven actual Cockroach cases and backend restart probes.
The [storage-owner execution record](../verification/projection-storage-worker-full-verifier-2026-10-08.json) preserves both attempts, normalized archives, source fences and audit results.
Independent source review found no actionable storage-owner issues; it executed no tests.
All audited graphs contain zero known vulnerabilities at inspection time; the public consumer retains its unsuppressed informational paste warning.

## 2026-10-08: bounded pending-history reconstruction

Sources: [pinned authorized journal](https://raw.githubusercontent.com/pmikstacki/ROM/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/journal.rs), [historical disclosure and tombstones](https://raw.githubusercontent.com/pmikstacki/ROM/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/projection.rs), [global inspected positions and retention](https://raw.githubusercontent.com/pmikstacki/ROM/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/storage_state.rs).

Use a bounded incremental reconstruction component before full worker dispatch. Accept only authorized public journal batches through the trusted host seam.
Validate the complete batch's kind, generation, cursor and event ordering before clamping to the immutable pending endpoint.
Map only events inside that interval; reject mapper changes to original key, position, revision or tombstone status.
Collapse repeated keys across batches, preserving revision monotonicity and equal-revision content identity. Compare the complete reconstructed intent.
Use fixed limits of 64 fetches, 4096 returned views and 4096 global inspected positions, including final overrun.
The global position span measures inspected facts, including unrelated kinds and denied events. It is deliberately more conservative than returned-view count.
Reject exhausted bounds without local publication. These recovery admission limits do not change the durable checkpoint format or profile encoding.
Use a terminal failed state after a rejected batch; do not reuse a partially mapped interval or rerun a stateful mapper after failure.
Pre-cancellation preserves reconstruction state. Cancellation during inspection makes reconstruction terminal, without publishing partial metadata or inferring rollback.
The pending durable intent stays unchanged.
Alternatives: current reads lose historical tombstones; matching only cursors misses changed disclosure; unlimited replay gives no finite work bound.
This component retains metadata only. It cannot establish mapping determinism, authorization, remote persistence or native feed qualification by itself.
Actual public SQLite/redb integration, immutable approved documents and full page-worker orchestration remain required.

The [history execution record](../verification/projection-history-full-verifier-2026-10-09.json) identifies the actual source and remaining acceptance requirements.
Eleven history cases and 36 existing focused cases passed, with one compile-fail doctest. Initial seven-case RED and mapper-unwind RED remain preserved.
The full verifier passed with all 197 frozen runtime hashes unchanged. Public and extracted-archive consumers exercised exact interval reconstruction.
Source review found no actionable runtime issue; the cancellation prose clarification was applied. The reviewer executed no tests.
Fresh audits reported no known vulnerabilities. The independent public consumer retains its informational unmaintained paste advisory.
This accepts the metadata component, not the full worker or native history/provider integration.

## 2026-10-09: approved document mapping and content identity

Sources: [JCS number and string constraints](https://www.rfc-editor.org/rfc/rfc8785), [public JSON value representation](https://docs.rs/serde_json/1.0.150/serde_json/enum.Value.html), [Rust floating-point bit conversion](https://doc.rust-lang.org/std/primitive.f32.html#method.to_bits), [Qdrant payload](https://qdrant.tech/documentation/manage-data/payload/).

Select a versioned binary content identity, not a JCS compliance claim. JCS's IEEE-754 restriction cannot preserve every ROM u64 value.
Keep nonnegative integers as u64, negative integers as i64 and finite floats as normalized-zero IEEE-754 bits with separate tags.
Sort object keys by UTF-8 bytes at every level; retain array order and exact Unicode without normalization.
Bind profile, immutable sorted field selection, fixed vector dimension, original key, revision, tombstone, selected values and finite vector bits.
Exclude journal position from the content digest so identical equal-revision content at another event position retains its identity.
The first mapping selects named top-level fields from an already authorized historical view; it cannot add unprojected values.
Missing selected fields stay absent. Null, false, zero, empty strings, arrays and objects stay distinct.
Use at most 64 selected fields,128 bytes per selected name,4096 value nodes,32 recursive levels and16KiB canonical selected-value bytes.
Treat the vector as a separate Qdrant-style sidecar with1..4096 finite f32 dimensions, at most16KiB binary values.
This clarifies document versus vector admission; it does not relax the 1 MiB complete encoded request/response bound required of future providers.
Full wire-size, backend numeric support and generation metric checks remain provider/worker requirements. Some64-operation pages will exceed wire bounds and must be rejected.
Tombstones contain neither selected values nor vectors. Live vector mappings require an exact dimension and profile model identity.
Immutable private fields and sanitized Debug prevent mutation or diagnostic disclosure. Constructing a document does not grant export authority.
Alternatives include hashing arbitrary serialized JSON or trusting host-supplied digests; neither defines a stable exact-value contract across map features.
Native backend round-trips and public SQLite/redb authorization tests remain required; core construction alone does not qualify providers.

Derive the durable mapping identifier from the original profile and complete selector/dimension configuration with SHA-256 and a separate mapping domain.
Use `DocumentMapping::profile()` for checkpoint creation/opening and provider configuration. Reusing the unbound input profile is not the document contract.
A changed selector therefore fails native checkpoint profile admission, even when the host reuses its human mapping version label.
A targeted test observed the old same-profile behavior fail before this correction. This prevents an in-place mapping change without altering checkpoint format.

Source follow-up: [serde_json 1.0.151 Number implementation](https://raw.githubusercontent.com/serde-rs/json/v1.0.151/src/number.rs) matches the actually locked version.
The initial documentation lookup used 1.0.150; no dependency downgrade is selected. Tests and package consumers use existing 1.0.151.
Source review found arbitrary-precision integers could silently fall through to rounded f64 identities.
An independent arbitrary_precision consumer reproduced the intended failure; the initial missing native pkg-config environment failure is also retained.
Reject unsupported integers and noncanonical/non-roundtripping arbitrary-precision floats; never retain more precise numbers under a rounded digest.
Bound numeric formatting to 64 bytes before feature-dependent numeric conversion, without allocating a raw-number string.
Independent consumers enable arbitrary_precision and preserve_order; standard-feature workspace tests remain separate.
No new production dependency is added to projection core. Fresh archive graph audits must cover the additional test feature dependencies.

The [document execution record](../verification/projection-documents-full-verifier-2026-10-09.json) identifies both standard and feature-enabled consumer acceptance.
The full verifier passed with200 frozen runtime files unchanged,55 focused cases and one compile-fail doctest.
Source review's arbitrary-precision P2 was resolved through an observed independent-consumer failure and successful correction. Final source review found no remaining actionable precision issue.
Root lock bytes and public registry versions/checksums are unchanged. The public consumer adds only the serde_json dependency edge to existing indexmap.
The40-package archive graph includes the additional feature dependencies; licenses are MIT/Apache-2.0 and their highest MSRV is1.85.
Fresh audits report no known vulnerabilities; the public consumer retains its unsuppressed informational paste warning.
This accepts immutable core construction and independent consumers, not native feed authorization, provider round-trips or the full worker.

## 2026-10-09: full core page orchestration

Sources: [public authorized journal and inspected cursors](https://raw.githubusercontent.com/pmikstacki/ROM/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/journal.rs), [Tokio1.53.1 oneshot receiver lifecycle](https://docs.rs/tokio/1.53.1/tokio/sync/oneshot/struct.Receiver.html), [Rust return-position futures in traits](https://doc.rust-lang.org/reference/items/traits.html#async-functions-and-impl-trait-in-traits).

Compose the accepted storage owner, approved documents and pending-history reconstruction into Worker::apply_page/recover.
Use static Send future contracts rather than a new async-trait dependency or boxed arbitrary backend commands.
A trusted target prepares an owned bounded request before durable preparation; only apply performs backend I/O and returns actual stored-state observations.
Request preparation must enforce provider numeric, metric, origin/profile and complete wire bounds. A synthetic target is only core protocol evidence.
Require target/source/document profiles to match the actual storage-owner profile. Cache that profile from successful native initialization, not host assertion.
Collapse approved documents using the same exact-key order as immutable intent; dispatch only the selected complete representations.
An empty filtered interval needs no backend request. Publish it only through the same durable intent/completion transactions.
Await preparation before dispatch. Preserve native uncertain tokens unchanged; do not automatically retry, reopen or infer rollback.
Recovery first reconstructs the complete old authorized interval and its latest approved documents; compare full intent before backend I/O.
Reject later page admission while any durable intent remains pending. Higher/different observations cannot establish exact completion.
Cancellation checks are cooperative phase boundaries. Dropping a future does not establish native rollback or remote nonacceptance.
Once local completion is admitted, wait for its classified outcome; a late cancellation flag cannot retract a confirmed commit.
Constructor/error/drop and shutdown still use blocking owner lifecycle. The bounded asynchronous lifecycle bridge remains required.
Native public SQLite/redb feed tests and Rust OpenSearch/Qdrant transports remain separate mandatory acceptance scopes.

Worker review found an admission/recovery mismatch: an interval exceeding 4096 positions could become permanently unreconstructable.
Reuse PendingHistory::new before intent admission; do not duplicate the bound. The regression first reached Target(Unknown), then passed after correction.
The fetch-budget regression first made 65 source calls. Check the shared budget before fetch to enforce 64 actual calls.
Cancellation/drop tests use controlled synthetic target acceptance and actual checkpoint files; they do not prove native remote acknowledgement loss.
The Tokio receiver contract does not establish rollback of admitted work. Preserve pending recovery and exact native failure classification.

The [worker execution record](../verification/projection-worker-full-verifier-2026-10-09.json) identifies the verified source and scope.
The full verifier passed with 205 unchanged runtime files, 67 focused cases and one compile-fail doctest.
Independent public and normalized archive consumers passed synthetic-target restart recovery.
Source review found one admission/recovery P2; the regression failed before correction and passed afterward. Final source review found no actionable issue.
Both lockfiles are byte-identical to baseline. Fresh root, consumer and 40-package archive audits report no known vulnerabilities.
The public consumer retains the unsuppressed paste informational warning. Task 2 and the full goal remain incomplete.

## 2026-10-09: bounded asynchronous lifecycle bridge

Sources: [Tokio1.53.1 blocking work](https://docs.rs/tokio/1.53.1/tokio/task/fn.spawn_blocking.html), [Rust retained thread join](https://doc.rust-lang.org/std/thread/struct.JoinHandle.html), [bounded channel](https://doc.rust-lang.org/std/sync/mpsc/fn.sync_channel.html), [retained watch result](https://docs.rs/tokio/1.53.1/tokio/sync/watch/struct.Sender.html#method.send_replace), [watch borrow/update](https://docs.rs/tokio/1.53.1/tokio/sync/watch/struct.Receiver.html#method.borrow_and_update).

Use one persistent StorageLifecycle supervisor rather than spawning a new blocking task for every start or shutdown.
A host admits one active owner, including initialization and cleanup. Another start returns Overloaded without waiting.
Bound queued startup paths to 4096 encoded bytes. Captures are typed; arbitrary factory injection exists only in tests.
The supervisor retains an executor owner clone through native destruction and join. Managed Drop closes admission and requests cleanup without joining native I/O.
Dropping a startup response cancels cooperatively and requests cleanup; it does not roll back completed file creation.
Shutdown futures await a retained terminal result. Use send_replace so a later subscriber observes completion after an earlier waiter disappeared.
Keep watch borrows outside await. Release the host slot only after native shutdown/join completes.
Host setup, final shutdown and Drop require a blocking embedding context. The host retains its supervisor JoinHandle instead of detaching.
Do not claim a process-wide thread limit: the fixed one-owner bound applies to each explicitly created host.
No hard native I/O deadline is supplied. Native feeds, authorization and Rust provider transports remain mandatory separate acceptance.
No production dependency or Tokio feature is added; watch and oneshot use the already-enabled sync feature.

An adversarial wake test found startup failure acknowledgement before slot release.
Delay failed startup responses until native cleanup, slot release and retained terminal publication have completed.
The actual Busy regression failed before correction. Successful startup must still publish its managed owner before waiting for close.

The [lifecycle execution record](../verification/projection-lifecycle-full-verifier-2026-10-09.json) records the source and actual scope.
The full verifier passed with 209 unchanged runtime files, 79 focused cases and one compile-fail doctest.
Twelve lifecycle cases cover real files and controlled native startup/active-job phases.
The actual Busy wake-order regression failed before correction and passed afterward. Other initial failures identify missing API/test seams, not behavioral evidence.
Final source review found no actionable issue. Independent public and normalized archive consumers passed managed-worker restart recovery.
Root, public-consumer and archive lockfiles are byte-identical to their prior graphs. Fresh audits report no known vulnerabilities.
The public consumer retains the unsuppressed paste informational warning. Native feed/authorization and Rust provider transport acceptance remain required.

## 2026-10-09: authorized native history for projection recovery

Use public ROM 0.0.3 at immutable revision d7ef529040eec60dc869034c2d33130219db85fe.
RuntimeHistory owns a fixed Runtime, Actor, Resource kind and derived mapping profile.
Read history through Runtime::journal; do not reconstruct disclosure from private storage or partial Resource decoding.
Current row and field policy can suppress historical values. Recovery must compare the complete pending intent before target dispatch.
Denied or expired identity requires rebuilding; source operational errors remain sanitized SourceUnavailable.

Sources inspected on 2026-10-09: [public journal](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/journal.rs), [historical disclosure](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/projection.rs), [identity policy](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/policy.rs), [command identity](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/resource/command.rs), [runtime limits](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/execution/lifecycle.rs).

Use a trusted bounded HostVectors callback over validated selected field references.
Reject model changes before and after computation. Tombstones do not invoke the callback.
Canonical validation precedes callback invocation and cloning. This does not qualify any embedding model or prove arbitrary host callback bounds.
Native Runtime limits remain host configuration; projection bounds do not establish an arbitrary runtime's peak memory bound.
Authorization applies at each source fetch; continuous remote revocation fencing remains unimplemented.

Archive tests consume unchanged normalized manifests outside the parent workspace.
The extracted library's own manifest selects its integration tests and development dependencies.
Share immutable public-ROM CLI patches between packaging and extracted tests.
First resolve the extracted graph offline; repeat tests with its resulting lockfile locked.
Sources: [Cargo test selection](https://doc.rust-lang.org/cargo/commands/cargo-test.html), [workspace discovery](https://doc.rust-lang.org/cargo/reference/workspaces.html), [configuration patches](https://doc.rust-lang.org/cargo/reference/config.html), [dependency overrides](https://doc.rust-lang.org/cargo/reference/overriding-dependencies.html).

Actual SQLite and redb cases passed tombstone restart, old endpoint clamping, revoked row/field recovery, generation/identity/closed source errors, selected vector inputs, denied empty pages and oversized values.
Targets are controlled adapters, not OpenSearch or Qdrant transports.
Initial missing API, fixture identity, fixture command limit and Cargo workspace-routing failures are retained separately from passing evidence.

The native archive consumer expands from 40 to 95 resolved packages because it directly tests public SQLite and redb.
The 55 additional package versions already exist in the root graph. No production dependency is added to projection core.
Declared licenses include MIT/Apache-2.0, Zlib, Unicode-3.0 and Unlicense alternatives; only the temporary consumer lacks package license metadata.
Declared package Rust requirements remain within the tested 1.99 toolchain. Native source dependencies have no optional features enabled.
Root and archive audits report no known vulnerabilities or warnings. The public consumer retains its unsuppressed paste maintenance warning.

## 2026-10-09: native Rust OpenSearch write adapter (qualification in progress)

Consume the accepted ProjectionTarget seam; do not replace durable worker or public native history orchestration.
Use strict external versions, not external_gte. Equal-version conflicts require exact real-time stored-source equality.
Retain same-ID tombstone documents; never physically delete the revision fence.
Use SHA-256 IDs over framed original kind and ID, then verify the original key in stored source.
Reject revisions above i64::MAX before intent or network dispatch.
Keep complete approved selected values in an unindexed object; index only explicitly named string/null text fields.
This avoids truncating unsigned JSON values through native long mappings.
Require explicit strict mappings, actual mapping equality and request translog durability. Refresh visibility remains separate.
Keep a fixed index UUID during adapter lifetime. Durable generation/UUID manifest recovery remains Task6 work.

Sources inspected on 2026-10-09: [Bulk versions and item responses](https://docs.opensearch.org/latest/api-reference/document-apis/bulk/), [real-time stored source](https://docs.opensearch.org/latest/api-reference/document-apis/get-documents/), [explicit index creation](https://docs.opensearch.org/latest/api-reference/index-apis/create-index/), [object mapping and disabled indexing](https://docs.opensearch.org/latest/mappings/supported-field-types/object/), [native settings](https://docs.opensearch.org/latest/api-reference/index-apis/get-settings/), [reqwest 0.13.5 verified TLS and finite transport configuration](https://docs.rs/reqwest/0.13.5/reqwest/struct.ClientBuilder.html).

The scoped fixture initially rejected mapping/settings reads. Preserve the failure and inspect native actions rather than widening to admin.
Add only indices:admin/mappings/get and indices:monitor/settings/get to the existing rom_extras_* writer pattern.
Tagged sources: [GetMappingsAction](https://github.com/opensearch-project/OpenSearch/blob/3.9.0/server/src/main/java/org/opensearch/action/admin/indices/mapping/get/GetMappingsAction.java), [GetSettingsAction](https://github.com/opensearch-project/OpenSearch/blob/3.9.0/server/src/main/java/org/opensearch/action/admin/indices/settings/get/GetSettingsAction.java).
Original and changed fixture roles are retained privately. Client keys and credential material remain private.

Use a test-only verified-mTLS relay to discard actual Bulk responses after the upstream response ends.
Rust target observes Unknown; real SQLite/redb source and actual native checkpoint reopen recover the pending old interval.
These cases do not establish service restart, delayed stale requests, bounded hostile responses, or independent archive compatibility.
First missing API and compilation failures are not behavioral RED evidence. Initial native mapping/permission rejection is an executed behavior failure.
The initial native Runtime test omitted its field policy; its Denied results are retained as fixture failures.
Task3 and the full projections family remain incomplete pending the remaining real-service and package acceptance.

### OpenSearch delayed request qualification

Hold an actual bounded request at the verified TLS relay before upstream submission.
Publish a private barrier after receiving the complete body; submit it seven seconds later, beyond the five-second client deadline.
After the client observes Unknown, dispatch a newer live or tombstone representation directly.
The late native Bulk response reports HTTP200 with item409; exact stored-source replay confirms the newer representation remains unchanged.
Use temporary-file rename for barrier publication, so readers cannot observe incomplete status JSON.
Sources: [strict external Bulk comparison](https://docs.opensearch.org/latest/api-reference/document-apis/bulk/), [Node verified HTTPS request](https://nodejs.org/api/https.html#httpsrequesturl-options-callback), [Node file rename](https://nodejs.org/api/fs.html#fsrenamesyncoldpath-newpath).
The actual late-request test passed both representations. This does not establish backend restart or distributed durability.

### Restart and independent OpenSearch consumers

Inspect actual stored sources before replay after a persistent native server restart.
A replay could recreate missing data; it is insufficient evidence that a restart preserved prior state.
Add inspect_prepared as a bounded read-only operation over an already validated immutable request.
Share stored-source validation with apply instead of duplicating key, revision, digest and complete selected-value comparisons.
The actual restart case preserves live i64::MAX, original index UUID and persistent tombstone; a stale resurrection attempt still fails.
Sources: [native stored-source GET](https://docs.opensearch.org/latest/api-reference/document-apis/get-documents/), [native index settings](https://docs.opensearch.org/latest/install-and-configure/configuring-opensearch/index-settings/).

Add the same public-only conformance case to the independent consumer and the normalized three-archive consumer.
The independent consumer enables serde_json arbitrary_precision and preserve_order; both cases passed actual native source equality.
Reuse the immutable public ROM patches and extras archive paths; do not rewrite normalized manifests.
Source: [Cargo package normalization](https://doc.rust-lang.org/cargo/commands/cargo-package.html).
Missing inspect_prepared initially fails compilation; classify it separately from native behavior.
Native restart, the complete 13-case adapter suite, public consumer and Clippy passed.
The archive consumer run and Clippy passed; extracted core native-history tests passed twice, including locked replay.
Hostile transport qualification, final source review, fresh graph audits and the full frozen verifier remain required before integration.


### Transport review corrections and uncertain initialization

Use one operation deadline for generation creation and subsequent metadata reads. Do not renew it between requests.
The controlled TLS regression reproduced success after the configured deadline. The corrected case returns Unknown.
The first fixture matched a raw path with a trailing query marker. Preserve that fixture failure separately from behavioral RED.
Source: [Tokio timeout cancellation](https://docs.rs/tokio/1.53.1/tokio/time/fn.timeout.html).

Own each fixture child before readiness. Bound the readiness read, then kill, wait and join on failure or drop.
Silent, malformed, oversized and valid readiness cases confirm process cleanup.
Controlled mTLS sockets qualify redirect rejection, response bounds, malformed responses, deadlines and untrusted roots.
These sockets supplement actual OpenSearch tests; they do not replace native acceptance.

The first hostile required gate failed during two generation creations with Unknown. Native settings reads confirmed both indexes existed.
Reconcile uncertain test initialization through bounded read-only verification. Never repeat creation after Unknown.
Keep the production deadline unchanged. The observed result does not establish the cause of native latency.
Source: [OpenSearch index creation acknowledgement](https://docs.opensearch.org/latest/api-reference/index-apis/create-index/).

The corrected required gate passed 19 adapter cases, one public case and both Clippy checks.
The corrected normalized three-archive consumer passed, including 14 native history cases twice.
Final review identified empty indexed fields as an admitted mapping boundary requiring native qualification.
Full frozen verification and fresh dependency audits remain pending. Task3 remains incomplete.

The empty indexed-field native regression failed during generation creation. Emit an explicit object type without empty properties for this case.
Native creation, exact verification, write and read-only inspection now pass with all selected values stored but no text fields indexed.
Source: [OpenSearch object mapping](https://docs.opensearch.org/latest/mappings/supported-field-types/object/).

The first full verifier failed at one late-request test's generation setup with Rejected; seven other service cases passed.
A scoped diagnostic creation succeeded. All eight service cases passed on reproduction after adding the index name to failure diagnostics.
The cause remains unresolved. Preserve both results and rerun the full verifier with a new source fence.
Do not convert the earlier failure into success evidence or retry rejected creation inside a test.

Later full retries failed in unchanged NATS acknowledgement witnessing and core lifecycle test timeouts.
The unchanged NATS case passed alone. All seventeen core unit cases passed with one harness thread.
Use RUST_TEST_THREADS=1 for the next full qualification. Preserve internal test concurrency, existing deadlines and every required gate.
Default parallelism uses available hardware concurrency. Serial harness execution reduces competing native fixture work.
This execution choice does not establish the cause of each earlier failure or production capacity.
Sources: [Rust test execution](https://doc.rust-lang.org/rustc/tests/index.html), [parallel test isolation](https://doc.rust-lang.org/book/ch11-02-running-tests.html).

The serial full verifier exited zero with all 236 runtime hashes unchanged. Twenty adapter cases and independent consumers passed.
The [write qualification record](../verification/opensearch-rust-write-2026-10-09.md) preserves earlier failures, source review and fresh dependency inventories.
Typed search, Qdrant Rust transport, current query authorization and durable generation switching remain incomplete.


## 2026-10-09: typed search and current public hydration

Choose a core-owned approval token over a public unauthenticated candidate API or raw backend DSL.
An asynchronous trusted host SearchPolicy approves fixed kind, field and mode before provider I/O and again before publication.
Public Runtime::read_projected supplies current authorized values; backend candidates supply only original keys and revisions.
Reject candidates lacking the current visible query field, even if other Resource fields remain readable.
Bound one candidate request rather than unbounded search pagination. Short results are permitted when candidates are stale or denied.
Support explicit all-terms/any-terms match modes. Query text has a conservative 4096-byte application bound, not a vendor maximum.
No Debug formatter exposes query text or candidate keys. Current hydration is not an atomic global authorization snapshot.

Sources inspected 2026-10-09: [public pinned ROM projection](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/projection.rs), [OpenSearch match semantics](https://docs.opensearch.org/latest/query-dsl/full-text/match/), [finite search and partial-result controls](https://docs.opensearch.org/latest/api-reference/search-apis/search/), [keyword doc values](https://docs.opensearch.org/latest/mappings/supported-field-types/keyword/).
Native keyword-filter behavior still requires qualification; do not change the accepted write mapping based only on inference.

## 2026-10-09: native OpenSearch typed candidates

Native OpenSearch 3.9.0 qualifies existing index:false keyword doc-value filters for kind/profile, AND/OR matching and metadata-only source selection.
The private probe preserves its index; no mapping or service permission was changed. The accepted write mapping remains compatible.
Implement SearchTarget with one total deadline around generation inspection, search and final UUID inspection.
Use finite size, track_total_hits:false, explicit match operator and zero_terms_query:none. Disable partial results in the request.
Reject timed-out/failed/incomplete shards, foreign generation/profile/kind, inconsistent original-key hashes or versions and duplicate/oversized pages.
Return only validated keys/revisions to the accepted core, which supplies current Resource disclosure.
Reuse the existing write-key hash; do not introduce a second key encoding.
Move the existing pinned public ROM dependency from development to production for Key construction. This introduces no new package version.
The two native SQLite/redb cases pass current hydration, stale/deleted filtering and original Unicode identity. They do not qualify paused-response revocation yet.

Sources inspected 2026-10-09: [Search API](https://docs.opensearch.org/latest/api-reference/search-apis/search/), [match semantics](https://docs.opensearch.org/latest/query-dsl/full-text/match/), [keyword doc values](https://docs.opensearch.org/latest/mappings/supported-field-types/keyword/), [Cargo dependency scopes and commit pins](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html).

## 2026-10-09: actual search-response authorization barrier

Retain the actual bounded native HTTP200 response in a verified-mTLS relay; do not fabricate candidates for revocation qualification.
An atomic held marker proves four native candidates exist before test authority changes. A release marker sends the retained bytes unchanged.
Test current query, row and title-field revocation independently on both public storage backends, with Resource revisions still one.
A permitted note remains visible after title revocation. Hidden title membership still yields no query result.
A fresh relay request counter proves protected-field scope denial sends zero HTTP requests. A real two-candidate budget also executes before mutation.
Extract one shared test permission registry rather than duplicate the isolation logic across core and native provider fixtures.
Reuse the bounded child process owner; embed the relay script in shared test helpers so extracted consumers can execute it.
Declare Tokio macros explicitly in the archive fixture because join! multiplexes the query and revocation barrier on one task.
This is a test dependency feature; the production adapter feature profile remains unchanged.

Sources inspected 2026-10-09: [Node22 verified HTTPS forwarding](https://github.com/nodejs/node/blob/v22.16.0/doc/api/https.md), [Node22 TLS verification](https://github.com/nodejs/node/blob/v22.16.0/doc/api/tls.md), [Tokio1.53.1 join](https://docs.rs/tokio/1.53.1/tokio/macro.join.html), [OpenSearch finite search controls](https://docs.opensearch.org/latest/api-reference/search-apis/search/).

### Native OpenSearch test setup: wait for the target primary shard

The full search qualification and an isolated restart→recovery sequence both failed in the first redb recovery case with `TargetFailure::Unknown`. The service log placed primary-shard activation after that case failed. Mapping/settings inspection had already succeeded. This evidence identifies a setup readiness gap; it does not justify changing production deadlines or recovery semantics.

[Create Index](https://docs.opensearch.org/latest/api-reference/index-apis/create-index/) distinguishes metadata acknowledgement from shard acknowledgement and explains that a timed-out creation can still create the index. [Cluster Health](https://docs.opensearch.org/latest/api-reference/cluster-api/cluster-health/) supports an exact-index health request and reports active primary, initializing, relocating and unassigned shards.

The shared native test fixture therefore performs a read-only, verified mTLS health wait for its own physical index after either successful creation or metadata reconciliation. The setup deadline is 45 seconds, each HTTP request is bounded, response bytes are capped, and only sanitized first/last health summaries are retained privately. Creation is never repeated. Production writer permissions, request deadlines and public APIs are unchanged. Independent and extracted consumers use the same setup helper with an explicitly scoped reqwest dependency.

The first readiness probe exposed HTTP 408 for an unmet one-second wait. OpenSearch 3.9 [ClusterHealthResponse.status](https://github.com/opensearch-project/OpenSearch/blob/3.9/server/src/main/java/org/opensearch/action/admin/cluster/health/ClusterHealthResponse.java) explicitly maps `timed_out` to `REQUEST_TIMEOUT`. The helper now parses bounded health bodies for both 200 and 408, retaining the latter as a pending observation; only `timed_out=false` plus the exact ready-shard conditions permits test execution.

The corrected restart→recovery sequence passed both native stores. Its first redb readiness observation had zero active primaries and one unassigned shard; the final observation had one active primary and no pending shards after 16,339 ms. The full verifier then detected archive-only Rust module resolution: a bare child module in a relocated source file searches a child directory. The fixture now names the copied sibling file explicitly; archive acceptance remains required.
The sibling `path` attribute follows the [Rust Reference module-source rules](https://doc.rust-lang.org/reference/items/modules.html#the-path-attribute): an explicit non-inline module path is relative to the containing source directory. This keeps native and relocated consumer fixtures aligned.

### Held native search: total deadline and mapping drift

Inspection date: 2026-10-09. The [Search API](https://docs.opensearch.org/latest/api-reference/search-apis/search/) defines shard timeout and partial-result behavior. [Tokio timeout](https://docs.rs/tokio/1.53.1/tokio/time/fn.timeout.html) bounds the complete asynchronous provider operation. Tests retain a real successful native response without releasing it and require `Unknown` at the configured five-second total deadline. Exact request counts distinguish one retrieval from retries or final inspection.

The [Update Mappings API](https://docs.opensearch.org/latest/api-reference/index-apis/put-mapping/) permits adding a field to an existing index without recreating it. A separate case adds an unindexed field to its newly created, PID-scoped test index while a real search reply is held. The actual bytes are then released. Final mapping inspection must reject those candidates. Historical indexes and stored documents remain intact. This qualifies mapping drift, not UUID replacement, alias switching or a continuous generation fence.

The tests share one verified administrative fixture client with the existing read-only readiness helper. Production writer credentials and transport are unchanged. Actual SQLite/redb focused pairs pass; independent consumers, archives and full acceptance remain required.

### Qdrant REST acceptance follow-up

Inspection date: 2026-10-09. Rechecked the executed version's [conditional-update tests](https://github.com/qdrant/qdrant/blob/v1.19.2/tests/openapi/test_conditional_update.py) and [REST schema](https://github.com/qdrant/qdrant/blob/v1.19.2/lib/api/src/rest/schema.rs), plus official [upsert](https://api.qdrant.tech/api-reference/points/upsert-points) and [retrieve](https://api.qdrant.tech/api-reference/points/get-points) APIs.

The planned Rust target must inspect actual retrieved identity, revision and deterministic content after completed conditional writes. HTTP success alone cannot prove replacement. Keep the accepted u32-half revision predicate, same-ID tombstones and host-provided finite named vectors. These sources guide implementation; current Python probes do not establish a Rust adapter, public Resource integration or an embedding-model guarantee. Shared HTTP boundaries and fixed-dimension query contracts still require detailed design and actual-service tests.

### Qdrant metric normalization must be explicit

Inspection date: 2026-10-09. Official [collection documentation](https://qdrant.tech/documentation/manage-data/collections/) states that Cosine upload normalizes vectors. The pinned [metric implementation](https://github.com/qdrant/qdrant/blob/v1.19.2/lib/segment/src/spaces/simple.rs) distinguishes preprocessing by metric. Therefore raw host-vector byte equality is not a valid universal native reconciliation rule.

Before implementing vector reconciliation, define and qualify the native representation for each supported metric. Preserve the core's deterministic original content identity. Do not silently switch to Dot merely to avoid Cosine acceptance work, and do not mistake a normalized stored vector for the original sidecar. Existing unit-vector probes do not establish arbitrary-vector normalization, zero-vector, extreme finite-value or cross-metric behavior. This is an implementation requirement, not an accepted Rust metric implementation.

A verified-TLS actual Qdrant1.19.2 experiment then created four fresh persistent collections, one per metric, using named float32 vectors. Each accepted four points: non-unit, zero, large finite and small finite. Read-only retrieval established actual stored values. Cosine changed `[3,4,0]` to `[0.6,0.8,0]` and changed the tested `[3e38,0,0]` float32 input to zeros. Dot, Euclid and Manhattan retained the tested float32 representations. These are observed cases, not a universal numeric guarantee. Keep the exact private script, result and source report under `.superpowers/qdrant-metric-observation-2026-10-09.*`; no previous collection or index was removed.

Consequently the Rust adapter needs an explicit, tested numeric contract for original host-side identity and native stored vectors. Silent clipping, truncation or interpreting stored zeros as the original large vector is not acceptable. Metric-specific preparation/reconciliation remains required before claiming support.

### Native process interruption after remote acceptance

Inspection date: 2026-10-09. [Rust process::exit](https://doc.rust-lang.org/std/process/fn.exit.html) does not run Rust destructors. Use a bounded child invocation with a dedicated exit code after actual remote-state inspection, without worker/runtime shutdown. The parent owns and joins the verified acknowledgement-loss relay.

The child commits real public Resource mutations on SQLite or redb, persists a checkpoint intent, loses the actual Bulk response, and confirms the exact newer document and tombstone through GET-only inspection. The parent restarts only the owned persistent OpenSearch service, opens Resource and checkpoint files, and inspects native state before replay. It requires pending intent and the original cursor before recovery, then exact endpoint publication and cleared intent.

[OpenSearch GET](https://docs.opensearch.org/latest/api-reference/document-apis/get-documents/) supports real-time retrieval. Public [ROM projection source](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/projection.rs) returns Denied for a current tombstone. Verify tombstones through authorized history and native inspection, while asserting denied current disclosure.

Initial fixture negatives assumed four disclosed historical events (actual three) and current tombstone read success (contract requires Denied). Both remain preserved. Verify authoritative journal head and exact final native documents instead. No production behavior changes. This qualifies process exit without Rust cleanup, not power failure.

Follow-up source review confirms that Task2 key-state lookup uses length-framed original kind/id bytes, not digest-derived keys. The [accepted codec source](https://github.com/pmikstacki/ROM-extras/blob/5da2a332686793ece46e109d8f554cf716463235/crates/rom-projection-core/src/codec.rs) and ambiguous-separator test establish that boundary. A forced hash collision is therefore not applicable to this core table. Qdrant digest-derived native IDs still require collision qualification.

The combined recovery test now cleanly closes and opens the checkpoint once more after publication. It verifies the persisted cursor, absence of pending intent, and both original keys' revision, tombstone and digest state through public storage operations. Source-only review found no actionable issue. Independent native/archive execution and full acceptance remain separate requirements.

### Retained native fixture recovery and shared relay process owner

Inspection date: 2026-10-09. Archive run 39940 completed its native cases, then Clippy rejected duplicate inclusion of the process helper. [Clippy duplicate_mod](https://rust-lang.github.io/rust-clippy/rust-1.99.0/index.html#duplicate_mod) identifies repeated module compilation. Each test binary now declares the helper once; both relay modules use that root declaration. Root and public-consumer Clippy passed.

Archive run 56665 then failed the existing 45-second per-index readiness gate after a process-crash case restarted the persistent fixture. Retained evidence shows zero active primaries for the new index throughout that gate. Service logs show recovery of 465 retained indices and eventual GREEN at 04:06:54.802 UTC, approximately 65 seconds after node startup. No OOM or index deletion was observed.

[OpenSearch Cluster Health](https://docs.opensearch.org/latest/api-reference/cluster-api/cluster-health/) distinguishes index health from cluster allocation state. After this scenario's owned restart, wait for all selected fixture indices to become GREEN before continuing. The test-only recovery budget is 90 seconds, based on the observed recovery interval. Existing per-index setup and production request deadlines remain unchanged. Retain first/last health observations for each restart wait. This fixture synchronization does not qualify a universal recovery-time or capacity guarantee.

### Qdrant canonical numeric preparation remains a separate contract

Inspection date: 2026-10-09. Pinned [metric dispatch](https://github.com/qdrant/qdrant/blob/v1.19.2/lib/segment/src/spaces/simple.rs), [normalization threshold](https://github.com/qdrant/qdrant/blob/v1.19.2/lib/segment/src/spaces/tools.rs), and [AVX arithmetic](https://github.com/qdrant/qdrant/blob/v1.19.2/lib/segment/src/spaces/simple_avx.rs) establish CPU-dependent floating-point accumulation. The [query API](https://api.qdrant.tech/api-reference/search/query-points) distinguishes named vectors, payload selection and result limits.

Actual verified-HTTPS experiments wrote eighty points across four metrics and dimensions 1/16/32/4096, then retrieved and queried them. Candidate float64 host normalization initially produced exact float32 retrieval for all tested Cosine inputs. Twelve additional dominant-component inputs disproved that as a general bypass: two dimension4096 values were normalized again. Dot large-finite queries returned null scores in successful native responses. Original vectors were hashed separately from prepared values.

Reject the unqualified assumption that host-normalized bytes always equal native stored bytes. Do not silently reduce the final profile to another metric or smaller dimensions. Resolve exact reconciliation and native overflow query semantics before Rust acceptance. The public numeric research record preserves hashes and observations. All nineteen new collections and previous service data remain intact. No production adapter or full-family completion follows from these experiments.

### Generation setup must wait after any preceding fixture restart

Full verifier4032 failed during redb generation setup, after both process-interruption cases and the preceding native restart case passed. The new index retained zero active primaries throughout its 45-second setup budget. The owned service recovered 504 retained indices. It started at 04:23:21.166 UTC and reached GREEN at 04:24:13.621 UTC.

The preceding restart test had confirmed its own index, which did not establish completion of other index recovery. Apply the same researched Cluster Health preflight before every native test generation creation. Keep the existing 90-second aggregate recovery budget and 45-second exact-index setup budget. Preserve the failed full log and its original frozen source manifest. Re-run the actual restart followed by recovery sequence before another full verifier. Production transport and durability contracts remain unchanged.

### README rendering and status navigation

Inspection date: 2026-10-09. The user requested correction of the README's continuous rendered paragraph. [GitHub table documentation](https://docs.github.com/en/get-started/writing-on-github/working-with-advanced-formatting/organizing-information-with-tables) specifies table separation and Markdown structure. Replace the continuous status paragraph with database links, integration/projection tables and roadmap navigation. Preserve existing link destinations, command blocks and incomplete-provider qualifications.

[GitHub's Markdown API](https://docs.github.com/en/rest/markdown/markdown#render-a-markdown-document) rendered the exact README in gfm mode. Both tables and all six navigation/verification sections appeared in returned HTML. Local link validation and the unchanged255-file runtime freeze passed. README-only commit279356fe995677bc4c51bb74b8af0e6cf3ac7219 was published; the remote default-branch README blob matches the local blob. This documentation publication does not qualify pending runtime changes.

### Exact tagged Cosine arithmetic reference

Inspection date: 2026-10-09. Tagged SSE/AVX/NEON sources, Arm intrinsic reduction documentation and Rust's fused-rounding guarantee establish explicit arithmetic sequences. A safe standalone Rust reference used typed float32 operations, without unsafe code. GET-only comparison matched all32 existing Cosine points, including the two4096-dimensional counterexamples, which matched AVX only.

Select exact tagged arithmetic reconciliation as the next implementation method to qualify. Preserve original approved host-document identity; compare native normalized values against eligible complete-vector reference results. Do not replace this with approximate component tolerances, a universal host-normalization bypass, another metric or fewer dimensions. The comparison record is candidate protocol evidence, not production Rust adapter or native ARM acceptance. Raw-vector, CPU-boundary, conditional write/recovery and query cases remain required.

## 2026-10-09 — Qdrant Rust conditional wire preparation

Select a whole-point conditional upsert with strict revision comparison and original Resource kind/id predicates.
Comparing only revision halves permits a higher-revision colliding key to replace another Resource.
The original-key predicates remain inside the native condition, including concurrent first insertion.
The tagged [conditional operation](https://github.com/qdrant/qdrant/blob/v1.19.2/lib/shard/src/operations/point_ops.rs) applies its condition to existing points.
The [REST contract](https://api.qdrant.tech/api-reference/points/upsert-points) supplies wait and write-ordering controls.
Neither an HTTP success nor a completed condition no-op establishes exact content acceptance.

Derive length-framed SHA-256 point identities, retaining 122 digest bits in UUIDv8 form.
The [UUID standard](https://www.rfc-editor.org/rfc/rfc9562.html#section-5.8) permits application-defined UUIDv8 content.
Retain and verify original keys; digest uniqueness is not an acceptance assumption.
Keep u64 revisions in two exact scalar u32 halves. Never send full revisions as floating-point filter bounds.

Retain selected approved JSON as a JSON string in payload, separately from its core-derived digest.
This preserves original JSON number representations instead of relying on native payload numeric conversion.
Actual Rust-generated requests retained u64::MAX selected values, fenced stale and equal-revision changes, and retained an empty-vector tombstone.
Eight concurrent initially missing forced-ID collision rounds retained their first accepted original key.
The native fixture uses verified HTTPS and existing private write/read API keys through the probe client.
This qualifies generated wire and the native predicate, not a production Rust transport, exact vector reconciliation, checkpoint recovery, or the complete adapter.
The local negative tests first failed against missing preparation; all collections and failure logs remain preserved.

## 2026-10-09 — Shared bounded projection HTTP transport

Move the qualified fixed-origin OpenSearch HTTP machinery into a transport crate used by both projection adapters.
Keep domain orchestration independent of reqwest. Preserve the public OpenSearch TlsConfig constructor and path.
Retain verified TLS, no proxies, no redirects, no implicit retries, streamed 1-MiB bounds, and finite total/connect deadlines.
Add a separately validated opaque API-key header for Qdrant.
The [Qdrant upsert contract](https://api.qdrant.tech/api-reference/points/upsert-points) specifies API-key authentication.
The pinned [reqwest ClientBuilder](https://docs.rs/reqwest/0.13.5/reqwest/struct.ClientBuilder.html) documents these controls and sensitive default headers.
Mark the API-key HeaderValue sensitive. Do not expose origins, credentials, requests or response bodies through errors or Debug.
This shares transport guarantees only. Provider-native acknowledgement and revision reconciliation remain separate.

## 2026-10-09 — Generic map core and host boundaries

Extend the full goal with the user-requested map providers; preserve existing SQL and projection work.
Select separate map capabilities, not additional operations on SQL or projection ports.
Use WGS84 longitude, latitude ordering and antimeridian-aware west, south, east, north boxes.
The [GeoJSON specification](https://www.rfc-editor.org/rfc/rfc7946.html) defines coordinates, line geometry and antimeridian bounds.
Reject malformed and excessive geometry instead of silently repairing or truncating it.
Route distance and estimated duration use explicit metres and seconds, matching the [tagged OSRM protocol](https://raw.githubusercontent.com/Project-OSRM/osrm-backend/v26.10.0/docs/http.md).

Preserve public ROM Key directly. Do not derive Resource identity from native map result IDs.
Use explicit native provenance and required plain-text attribution with controlled credit links.
Nominatim importance and MapTiler relevance do not establish measured positional accuracy; retain Unknown unless the native service reports it.
The [provider research](map-providers-2026-10-09.md) links those API contracts and license/usage policies.

Select optional asynchronous geocoding, reverse-geocoding and routing ports with no default network provider.
Use a monotonic watch cancellation token and a total operation budget covering rate/admission waits and I/O.
The pinned [Tokio select contract](https://docs.rs/tokio/1.53.1/tokio/macro.select.html) documents cancellation by dropping losing futures.
The pinned [watch wait_for contract](https://docs.rs/tokio/1.53.1/tokio/sync/watch/struct.Receiver.html#method.wait_for) retains current state across subscription.
Cancellation before dispatch must not poll the request future. In-flight cancellation and timeout are tested separately.
This token has asynchronous wait semantics; the projection startup token has a different cooperative native-operation contract.
Do not couple map capabilities to projection orchestration merely to share an atomic flag.

Keep map transport separate where projection HTTP cannot represent required 429/rate policy, identifying User-Agent, query authentication and cancellation.
Do not weaken projection transport or reinterpret its unknown outcomes as map rate limits.
The current rom-ui package has no maps export. This is an open example integration gap, not permission to fabricate imports or move UI code into Rust.

## Map browser boundary: reviewed source requirements, implementation pending

Date: 2026-10-09. This decision extends the map plan; it does not qualify a browser adapter.

A style URL is not a complete network disclosure boundary. MapLibre styles reference sources, sprites, glyphs and font faces.
The configured-source adapter must inspect supported resource references before the host approves a browser descriptor.
Reject unsupported network-bearing constructs rather than claim that checking the top-level origin covers them.
Keep backend credentials outside this descriptor. Require a separate explicit host grant for a browser token and its approved origins.
Keep required credit links and dataset notices separate from untrusted provider HTML.

MapTiler map content must not gain a server cache or proxy through a generic default.
Its Cloud terms require written permission for proxy use and restrict server-side caching.
The adapter must distinguish browser map access from backend geocoding and apply the selected operator's permissions separately.
A controlled fixture cannot establish commercial permission or production service qualification.

Sources: [MapLibre style resources](https://maplibre.org/maplibre-style-spec/root/),
[MapLibre source types](https://maplibre.org/maplibre-style-spec/sources/),
[MapTiler Cloud terms](https://www.maptiler.com/terms/cloud/).

## Map HTTP semantics remain separate from projection write outcomes

Date: 2026-10-09. Map transport implementation remains pending.

The extracted projection transport returns Rejected or Unknown target outcomes. It requires projection-core and fixed TLS authentication.
Map operations need RateLimited, Retry-After, host cancellation, identified geocoder requests and operator-specific admission rules.
Do not translate a map rate limit into an unknown committed write. Do not extend SQL or projection ports to add these semantics.
Reuse the pinned HTTP library and verified security settings where appropriate. Preserve projection redirects, proxy and retry restrictions.
The map contract will require explicit endpoints, bounded streaming, verified TLS and a total context budget that includes admission.
It will perform no implicit retries. A rate-limit response must preserve bounded retry guidance without disclosing response bodies or credentials.

Sources: [HTTP 429](https://www.rfc-editor.org/rfc/rfc6585.html),
[reqwest client controls](https://docs.rs/reqwest/0.13.5/reqwest/struct.ClientBuilder.html).

## Retain completed fixture indices without keeping every historical shard active

Date: 2026-10-09. This changes the controlled fixture state, not adapter code or production deadlines.

The full verifier failed three public-consumer cases at the fixture's 90-second aggregate readiness gate.
Two failure records retained 190 and 386 unassigned shards after restart. Both actual acceptance barrier files existed.
The fixture had 625 owned open indices, plus five other indices.

Use the reversible OpenSearch close operation for owned historical indices created before the frozen runtime inputs.
Keep current-run indices open. Keep other indices unselected. Do not delete indices, documents, logs or test records.
The operation closed 566 historical indices with unchanged UUIDs and retained 59 current-run indices open.
A closed sample was reopened and read: its five documents retained the same exact data digest. It was then closed again.
The private retention manifest records original UUIDs, document counts and each acknowledged batch for later reopening.
This maintenance does not qualify production capacity or relax the adapter's operation limits.
Rerun the failed cases and the full verifier before integration; preserve the original failed result.

Sources: [Close Index API](https://docs.opensearch.org/latest/api-reference/index-apis/close-index/),
[Open Index API](https://docs.opensearch.org/latest/api-reference/index-apis/open-index/).

The first retained-fixture rerun still failed its two process-restart readiness cases; retain that failed result too.
A controlled comparison found 633 indices in the implicit health query and 66 with explicit `expand_wildcards=open`.
Require that parameter in fixture readiness. Keep the existing 90-second aggregate and 45-second individual readiness limits.
Every current open index still requires green status, active primaries and zero initializing, relocating or unassigned shards.
Current-index persistence, exact observation, checkpoint recovery and authorization assertions remain unchanged.
This is a fixture query correction. It does not change production transport or target behavior.
Source: [Cluster Health API](https://docs.opensearch.org/latest/api-reference/cluster-api/cluster-health/).

Explicit open-index readiness alone still timed out: two open shards remained unassigned at 90 seconds.
A bounded read-only allocation diagnostic identified an older `top_queries` index throttled behind four initial primary recoveries.
Archived closed indices still participated in backend recovery. Closing them did not by itself remove this scheduling pressure.
Set recovery priority to zero only on the 566 UUID-verified historical closed fixture indices.
Record each original priority for restoration. Leave current open indices and other indices unchanged.
OpenSearch allocates higher-priority indices first, then orders equal priorities by creation date.
The focused rerun must confirm this fixture scheduling change before the full verifier runs again.
Source: [OpenSearch index recovery priority](https://docs.opensearch.org/latest/install-and-configure/configuring-opensearch/index-settings/).

The priority change did not resolve every readiness timeout. The user selected a separate qualification fixture.
Preserve the previous container, persistent data and failed test evidence as an archive.
The new fixture uses the same image, verified TLS configuration, security roles, CPU and memory limits.
Its persistent data directory is separate. Restart tests select the fixture through validated host configuration.
Both process-interruption cases and the SQLite query-revocation case passed on this fixture.
The first full rerun then stopped at CockroachDB initialization: its retained container had exited with code 8.
Six later failures were poisoned-lock consequences of the first failure. This result does not qualify the full increment.
Restart the same CockroachDB container and verify its SQL readiness before another full run.
Keep disk-stall protection enabled. CockroachDB documents disk-stall detection and process termination as availability safeguards.
Source: [CockroachDB availability runbook](https://github.com/cockroachlabs/cockroachdb-runbook-template/blob/main/system-overview/data-availability.md).

Keep pure style and tile metadata contracts in rom-map-core, alongside the other map capabilities.
Expose styles, raster metadata and vector metadata as independent optional ports.
Typed raster/vector wrappers reject the other source kind. They do not activate browser or server connections.
Preserve MapLibre source-option precedence over resolved TileJSON metadata through the same bounded validation.
Retain inherited dataset credits when a style adds its own attribution.
Reject unimplemented rendering properties explicitly instead of silently changing the style.
Source: [MapLibre root specification](https://maplibre.org/maplibre-style-spec/root/),
[MapLibre TileJSON loading implementation](https://raw.githubusercontent.com/maplibre/maplibre-gl-js/main/src/source/load_tilejson.ts).

The isolated full rerun passed OpenSearch but failed Qdrant's existing 15-second post-restart fixture readiness check.
The retained Qdrant fixture contains 105 collections and later accepted verified TLS requests.
Measure startup before changing the fixture or its readiness budget. Do not relax adapter request deadlines or certificate validation.
Qdrant distinguishes readiness from a container merely running.
Source: [Qdrant monitoring endpoints](https://qdrant.tech/documentation/ops-monitoring/monitoring/).
