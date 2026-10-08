# Kafka client assessment

Date: 2026-10-08. Evidence: primary-source inspection only. No Kafka container, native build, security audit, or interoperability test was executed.

## Confirmed dependency facts

`rdkafka` 0.39.0 declares MIT licensing and Rust 1.74. Its dependency permits `rdkafka-sys` 4.10.0. The inspected sys manifest is `4.10.0+2.12.1`, also MIT and Rust 1.74. Resolve and inspect the lockfile before claiming an actual dependency version. See [Rust manifest](https://github.com/fede1024/rust-rdkafka/blob/v0.39.0/Cargo.toml) and [sys manifest](https://github.com/fede1024/rust-rdkafka/blob/v0.39.0/rdkafka-sys/Cargo.toml).

The release's bundled librdkafka submodule points to `e1db7eaa517f0a6438bc846a9c49ede73b9ea211`. The sys package identifies bundled librdkafka as 2.12.1. librdkafka uses a two-clause BSD license, distinct from the Rust wrappers' MIT license. See [submodule metadata](https://api.github.com/repos/fede1024/rust-rdkafka/contents/rdkafka-sys/librdkafka?ref=v0.39.0) and [native license](https://github.com/confluentinc/librdkafka/blob/v2.12.1/LICENSE).

Default Rust features are `libz` and `tokio`; they do not enable TLS. `ssl` enables `openssl-sys`. `ssl-vendored` builds bundled OpenSSL. `gssapi` enables SSL and Cyrus `sasl2-sys`; `sasl` is its deprecated alias. Optional curl, zstd, and external LZ4 introduce further native dependencies. Default builds compile bundled librdkafka. Dynamic linking uses the system library. CMake builds require CMake. See [sys manifest](https://github.com/fede1024/rust-rdkafka/blob/v0.39.0/rdkafka-sys/Cargo.toml) and [build implementation](https://github.com/fede1024/rust-rdkafka/blob/v0.39.0/rdkafka-sys/build.rs).

Rust wrappers and native code contain unsafe operations. A safe ROM adapter API does not eliminate those dependency risks. Record the native compiler, native library version, enabled security mechanisms, and dependency licenses. Check both Rust advisories and native OpenSSL/librdkafka advisories. None were audited here.

## Confirmed producer configuration rules

The following names refer to librdkafka configuration, rather than Java producer defaults.

| Setting | Source-established behavior | Proposed ROM requirement |
| --- | --- | --- |
| `acks` | Alias for `request.required.acks`; `all` / `-1` waits for in-sync replicas | Require `all` / `-1` |
| `message.send.max.retries` / `retries` | Controls producer retries | Require `0` for a single adapter delivery attempt |
| `enable.idempotence` | Requires retries greater than zero; incompatible explicit configuration fails producer creation | Require `false` with the zero-retry profile |
| `transactional.id` | Enables transactional production | Require absent or empty for this nontransactional adapter |
| `queue.buffering.max.messages` | Zero disables the count limit | Require positive bounded count |
| `queue.buffering.max.kbytes` | Bounds aggregate queued payload size; takes priority over count limit | Require positive bounded bytes |
| `message.max.bytes` | Bounds protocol message size | Set an explicit cap and separately limit application payload/header size |
| `message.timeout.ms` / `delivery.timeout.ms` | Zero means infinite local delivery timeout | Require a positive bounded deadline |
| `allow.auto.create.topics` | Producer default permits broker automatic topic creation | Require `false`; pre-create topics |

Source: [librdkafka 2.12.1 configuration](https://github.com/confluentinc/librdkafka/blob/v2.12.1/CONFIGURATION.md).

Do not combine `retries=0` with `enable.idempotence=true`. Kafka producer-session idempotence is different from durable Work identity. If a later profile enables it, document retries separately. A stable Work ID in the record key and header supports consumer deduplication. Kafka does not deduplicate arbitrary matching keys or headers. See [librdkafka transaction and idempotence introduction](https://github.com/confluentinc/librdkafka/blob/v2.12.1/INTRODUCTION.md).

## Effective host producer configuration

`ClientConfig::get` returns an explicitly supplied value. `create_native_config` followed by `NativeClientConfig::get` includes native defaults. These inspect construction configuration, not an arbitrary existing producer. See [configuration implementation](https://github.com/fede1024/rust-rdkafka/blob/v0.39.0/src/config.rs).

The safe inspected API has no existing-producer effective-configuration getter. `Producer::client()` exposes `Client`, whose `native_ptr()` exposes a pointer. Native `rd_kafka_conf` returns the producer's read-only configuration. Native `rd_kafka_conf_get` can query it. These require unsafe FFI. `NativeClientConfig::from_ptr` is crate-private and cannot provide a downstream safe wrapper. See [client implementation](https://github.com/fede1024/rust-rdkafka/blob/v0.39.0/src/client.rs), [producer trait](https://github.com/fede1024/rust-rdkafka/blob/v0.39.0/src/producer/mod.rs), and [native configuration API](https://github.com/confluentinc/librdkafka/blob/v2.12.1/src/rdkafka.h).

Proposed safe approach: the host creates the producer through a validated ROM construction function. Return a wrapper with a private producer field. Read normalized native settings before construction. Set required properties explicitly. Keep construction configuration and producer ownership together. Reject arbitrary producer injection accompanied only by claimed settings.

Alternative: implement a narrowly justified unsafe adapter to inspect effective native settings. Keep the producer alive during inspection. Do not free its borrowed configuration. Query only required nonsecret properties. This requires the repository's unsafe-adapter justification and invariant review.

## Delivery and cancellation

`FutureProducer::send_result` returns queue admission failures immediately. Successful admission returns a delivery future. `send` can retry admission when the native queue is full. Its queue timeout does not bound native delivery after admission. See [future producer implementation](https://github.com/fede1024/rust-rdkafka/blob/v0.39.0/src/producer/future_producer.rs).

Dropping the delivery future drops its oneshot receiver. It does not remove the admitted native record. Consequently, cancellation after admission is an unknown outcome. Keep the producer alive, retain delivery evidence where possible, and permit duplicate records after uncertain Work retries. This follows the inspected delivery callback and channel ownership, rather than a tested cancellation experiment.

Proposed acknowledgement validation requires successful delivery, a nonnegative partition, and a nonnegative offset. `acks=0` supplies no broker acknowledgement and must be rejected at construction. A nonnegative offset alone does not prove `acks=all`. Native idempotent retries can return an invalid offset even for previously delivered records; the proposed zero-retry profile disables idempotence. See [native delivery semantics](https://github.com/confluentinc/librdkafka/blob/v2.12.1/src/rdkafka.h) and [Apache acknowledgement semantics](https://kafka.apache.org/39/configuration/producer-configs/).

## Native Rust alternative

The inspected `rskafka` main manifest identifies version 0.6.0, Rust 1.88, and MIT OR Apache-2.0. Its README explicitly excludes consumer groups, offset tracking, transactions, buffering, aggregation, and linger management. TLS is available through Rustls. This is a mutable-source assessment, not a pinned-release audit. See [manifest](https://github.com/influxdata/rskafka/blob/main/Cargo.toml) and [README](https://github.com/influxdata/rskafka/blob/main/README.md).

Its partition producer uses `acks=-1` and no transactional ID. Production passes through retry logic, including connection failures. That behavior needs explicit assessment against a single-attempt Work boundary. Its narrow per-partition model could fit an explicit-partition publisher, but does not establish general Kafka integration support. See [partition implementation](https://github.com/influxdata/rskafka/blob/main/src/client/partition.rs).

## Proposed backend fixture and evidence limits

Apache publishes `apache/kafka:4.3.1`. Its official quickstart starts that image on port 9092. The downloaded-server route formats storage in standalone mode, without ZooKeeper. Use the JVM image for the initial KRaft fixture. Record its resolved digest. See [official downloads](https://kafka.apache.org/community/downloads/) and [official quickstart](https://kafka.apache.org/quickstart/).

A proposed fixture must explicitly disable broker auto-creation and pre-create its topic. Use persistent broker storage for restart tests. Inspect image defaults before setting custom listener addresses and KRaft quorum configuration.

A single-broker, replication-factor-one fixture cannot prove replicated durability or failover. `acks=all` waits for the current in-sync set; it does not create extra replicas. Add a multi-broker fixture with explicit replication factor and `min.insync.replicas` before claiming replica-loss guarantees. See [librdkafka acknowledgement configuration](https://github.com/confluentinc/librdkafka/blob/v2.12.1/CONFIGURATION.md).

Open checks: effective-config rejection, queue admission limits, oversized records, nonexistent topic, valid delivery offset, admitted-record cancellation, broker restart, replica loss, TLS hostname verification, SASL authentication, native license inventory, and advisory audit. No passing result is implied by this source review.

## Observed CMake failure and alternative build route

The parent executed a CMake build with `tokio`, `cmake-build`, and `ssl-vendored`, with default features disabled. The captured build compiled `rdkafka-sys` 4.10.0+2.12.1 and passed `-DWITH_CURL=0`. Compilation nevertheless failed at `rdkafka_conf.c:60` because `curl/curl.h` was absent. See [local build output](../verification/kafka-config-api-red-2026-10-08.log). This is an inspected local failure, not a reproduced build by this research agent.

Source inspection establishes the cause. The CMake template uses `#cmakedefine01 WITH_OAUTHBEARER_OIDC`. The generated local header contains `#define WITH_OAUTHBEARER_OIDC 0`. However, `rdkafka_conf.c` guards its curl include with `#ifdef WITH_OAUTHBEARER_OIDC`. The preprocessor tests definition presence, so a value of zero still includes curl. See [CMake template](https://github.com/confluentinc/librdkafka/blob/v2.12.1/packaging/cmake/config.h.in) and [native source](https://github.com/confluentinc/librdkafka/blob/v2.12.1/src/rdkafka_conf.c). [Upstream issue 829](https://github.com/fede1024/rust-rdkafka/issues/829) reports the same feature-independent curl dependency.

The generated header inspected here is `target/debug/build/rdkafka-sys-cff94650d105f07c/out/build/generated/config.h`. CMake sets OIDC support only when SSL and curl are enabled, but its zero-valued macro conflicts with this source guard. See [CMake options](https://github.com/confluentinc/librdkafka/blob/v2.12.1/CMakeLists.txt).

The proposed alternative removes `cmake-build` and retains vendored SSL. The sys crate then uses its supported configure/Make route. When curl is not selected, that route passes `--disable-curl`. Its configure script enables the OIDC macro only in the applicable curl-backed path. See [sys build script](https://github.com/fede1024/rust-rdkafka/blob/v0.39.0/rdkafka-sys/build.rs) and [native configure script](https://github.com/confluentinc/librdkafka/blob/v2.12.1/configure.self).

This alternative remains subject to an executed build and tests. It requires Make and a working native compiler toolchain. Preserve the CMake failure as evidence. Do not add unused curl support or modify dependency sources merely to conceal the configuration defect.

## Executed build follow-up

The selected configure/Make build passed with Tokio and SSL features, using external static OpenSSL3.5.9.
The official archive SHA256 is603f5602e2eef00d77fbd429d34dcd5822bb301757a1bc9cdb24c670f1eb859a.
GPG verified release subkeyC46ED3F2CBEFDA1FDAADA44264ED7B1DCCE71CB2 under official primaryB146647E45A7B33947AB226B2A2C87D161692D40.
This verifies the cryptographic signature against the official web-published fingerprint, not a personal web-of-trust certification.
Raw signature/build logs are retained. Cargo audit does not assess this external native OpenSSL installation.
