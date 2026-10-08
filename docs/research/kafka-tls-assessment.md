# Kafka native TLS and SASL assessment

Date: 2026-10-08. Evidence category: primary-source review. No TLS fixture or security test was executed by this research agent.

The parent reports the current client uses `rdkafka` 0.39.0 with Tokio and SSL, bundled librdkafka 2.12.1 through configure/Make, and external static OpenSSL 3.5.9. This report assesses that profile. It does not independently establish the executed binary's native versions.

## Confirmed client controls

| Property | Required secure fixture value | Source-established meaning |
| --- | --- | --- |
| `security.protocol` | `ssl` or `sasl_ssl` | Native encrypted broker connection |
| `ssl.ca.location` | Fixture CA PEM file | Trust roots for broker certificate verification |
| `enable.ssl.certificate.verification` | `true` | OpenSSL server certificate verification |
| `ssl.endpoint.identification.algorithm` | `https` | Verify broker hostname against its certificate |
| `sasl.mechanism` | `PLAIN` for SASL profile | Select one SASL mechanism |
| `sasl.username` / `sasl.password` | Host-owned fixture credentials | PLAIN authentication credentials |

These properties and defaults come from [librdkafka 2.12.1 configuration](https://github.com/confluentinc/librdkafka/blob/v2.12.1/CONFIGURATION.md). Keep verification and hostname identification enabled in both positive and negative tests.

The native SASL implementation selects its PLAIN provider without a Cyrus/GSSAPI compile guard. GSSAPI is separately guarded. Therefore the inspected SSL-enabled native profile can support PLAIN without adding the Rust `gssapi` feature. An actual SASL handshake remains required evidence. See [native provider selection](https://github.com/confluentinc/librdkafka/blob/v2.12.1/src/rdkafka_sasl.c) and [Rust native features](https://github.com/fede1024/rust-rdkafka/blob/v0.39.0/rdkafka-sys/Cargo.toml).

Apache recommends PLAIN only over encrypted transport. Use SASL_SSL, not SASL_PLAINTEXT, for password authentication. Broker JAAS credentials configure users separately from the broker's outbound identity. See [Apache SASL guide](https://kafka.apache.org/43/security/authentication-using-sasl/).

CA validation does not imply certificate or SPKI pinning. Server-authenticated TLS does not imply mutual TLS. Native client certificate and key settings exist, but the proposed password profile does not supply them. Kafka producer-session idempotence also remains distinct from durable Work identity. See [native SSL properties](https://github.com/confluentinc/librdkafka/blob/v2.12.1/CONFIGURATION.md).

## Apache image configuration, inspected at 4.3.1

The official image's JVM launch script runs `kafka.docker.KafkaDockerWrapper setup`. The wrapper reads mounted files from `/mnt/shared/config` and writes final files under `/opt/kafka/config`. If a mounted `server.properties` exists, environment-derived properties are appended. If no mounted file exists, nonempty environment properties replace the default file. Therefore a fixture cannot assume partially supplied environment values inherit all image defaults. See [JVM launch](https://github.com/apache/kafka/blob/4.3.1/docker/jvm/launch) and [wrapper implementation](https://github.com/apache/kafka/blob/4.3.1/core/src/main/scala/kafka/docker/KafkaDockerWrapper.scala).

The wrapper removes `KAFKA_`, converts names to lowercase, converts underscores to dots, then converts three dots to hyphens and two dots to underscores. This is confirmed Apache mapping, not a Confluent image assumption. For example, `KAFKA_LISTENER_NAME_SASL__SSL_PLAIN_SASL_JAAS_CONFIG` maps to `listener.name.sasl_ssl.plain.sasl.jaas.config`. See [environment mapping](https://github.com/apache/kafka/blob/4.3.1/core/src/main/scala/kafka/docker/KafkaDockerWrapper.scala).

The image's configure script detects `SSL://` inside advertised listeners, including `SASL_SSL://`. It then requires these filenames beneath `/etc/kafka/secrets`:

- `KAFKA_SSL_KEYSTORE_FILENAME`
- `KAFKA_SSL_KEY_CREDENTIALS`
- `KAFKA_SSL_KEYSTORE_CREDENTIALS`

It reads password files and exports the corresponding location/password properties. Truststore filename and credential checks are conditional on requested or required client authentication. The fixture still needs a truststore for broker/controller outbound TLS verification. See [Apache configure script](https://github.com/apache/kafka/blob/4.3.1/docker/resources/common-scripts/configure).

For SASL advertised listeners, the script requires `KAFKA_OPTS`. It warns if the value lacks `java.security.auth.login.config`. Mount a test JAAS file and point that JVM property to it. Avoid debug tracing: Apache's shell configuration explicitly warns that tracing can expose credentials. See [configure](https://github.com/apache/kafka/blob/4.3.1/docker/resources/common-scripts/configure) and [shell configuration](https://github.com/apache/kafka/blob/4.3.1/docker/resources/common-scripts/bash-config).

## Proposed minimal direct TLS fixture

Use the official JVM image `apache/kafka:4.3.1` with its resolved digest recorded. Use a combined broker/controller only for local test evidence. Bind all test endpoints to loopback. On Linux, host networking simplifies matching advertised loopback endpoints; assess that choice against the available container runtime.

Use one broker listener named `SASL_SSL`, mapped to `SASL_SSL`. Use a separate `CONTROLLER` listener mapped to `SSL`. Configure no PLAINTEXT broker or controller listener. Set `inter.broker.listener.name=SASL_SSL`, `controller.listener.names=CONTROLLER`, and a consistent static controller voter address. Apache requires separate broker and controller listener names. See [listener guide](https://kafka.apache.org/43/security/listener-configuration/) and [image storage formatting](https://github.com/apache/kafka/blob/4.3.1/core/src/main/scala/kafka/docker/KafkaDockerWrapper.scala).

Use a local test CA and a broker certificate with an explicit `localhost` DNS SAN. Connect positive clients to `localhost`. Configure a truststore for outbound Java TLS. Require hostname checks on Java outbound connections as well as librdkafka clients. Choose PKCS12 stores explicitly. See [Apache TLS guide](https://kafka.apache.org/43/security/encryption-and-authentication-using-ssl/) and [broker properties](https://kafka.apache.org/43/configuration/broker-configs/).

Set `ssl.client.auth=none` for this server-authenticated password profile. Enable only PLAIN. Mount a JAAS file containing an outbound broker identity and the accepted test user. Keep credentials outside captured commands, configuration dumps, and test output. Use the established adapter construction path for adapter delivery tests. The parent confirms that it privately constructs a `DefaultClientContext` producer. It does not accept an injected custom context.

Pre-create the test topic. Disable broker and client automatic topic creation. Persist the exact broker log directory across restart. Keep the same cluster ID. A fresh host producer must reconnect using the same verified CA and advertised hostname after restart.

For a direct TLS-only profile, run a separate fixture configuration using an SSL broker listener without SASL. Do not make the password test pass by falling back to SSL-only transport.

## Cause-specific negative evidence

`ClientContext::error` receives `KafkaError` and a native reason string. `KafkaError::rdkafka_error_code` exposes typed native error codes. The inspected enum includes `SSL`, `Authentication`, and `SaslAuthenticationFailed`. Delivery alone can ultimately return a timeout, so capture global callback codes as well. See [client callbacks](https://github.com/fede1024/rust-rdkafka/blob/v0.39.0/src/client.rs), [error extraction](https://github.com/fede1024/rust-rdkafka/blob/v0.39.0/src/error.rs), and [native error enum](https://github.com/fede1024/rust-rdkafka/blob/v0.39.0/rdkafka-sys/src/types.rs).

For an independent native conformance client, override its test context's `error` and `log` callbacks. Record only allowlisted typed codes and scenario identifiers in bounded thread-safe state. Do not print native reason strings, configuration values, payloads, returned messages, usernames, or passwords. Default callbacks log native text. Keep debug disabled. The independent client can inspect typed causes without changing the adapter API. It cannot prove that the adapter exposes those causes. Adding such inspection to the adapter requires a separately reviewed internal context change or public API design.

| Scenario | Controlled change | Required evidence |
| --- | --- | --- |
| Wrong CA | Supply an unrelated CA; keep hostname verification enabled | SSL callback code, failed delivery, no matching published record |
| Wrong hostname | Connect using an address absent from the certificate SAN | SSL callback code, failed delivery, no matching published record |
| Wrong password | Retain trusted CA and correct hostname; change only password | Authentication-related callback code, failed delivery, no matching published record |
| TLS client on plaintext peer | Use a separate isolated negative target; never add plaintext to the secure fixture | Failed TLS/protocol connection and no valid acknowledgement |
| Restart | Restart the broker with unchanged persistent storage; create a fresh host producer | Valid delivery offset and independently consumed payload, key, and JSON content-type header |

Wrong CA and hostname both map to the SSL category; the safe typed API does not establish distinct subcodes for them. Controlled one-variable scenarios provide the distinction. Native reason text may aid local diagnosis but is not required in saved evidence. TLS-on-plaintext may report SSL or transport errors; do not assert a single exact code without execution.

If Kafka itself must be the plaintext negative peer, use a separate loopback-only fixture with no SASL credentials. Never send PLAIN passwords to it. This negative setup is separate from the positive fixture's all-TLS listener contract.

## Evidence limits and remaining checks

Require success before and after restart with the same exact payload bytes, stable Work key, and matching JSON content-type header. Use the existing fixture's exact JSON `false` payload. Kafka storage persistence is a separate fact; the adapter has no `persistent` payload field. The Work key does not establish Kafka deduplication.

Use an independent verified TLS consumer to inspect output and absence checks. Record test binary versions, image digest, certificate SANs, selected nonsecret security properties, native versions, command, and results. Do not capture credential-bearing container inspection output.

A single-broker restart fixture proves only its tested restart conditions. It does not establish replica-loss durability, mutual TLS, certificate pinning, rotation, ACL enforcement, SCRAM, GSSAPI, or exactly-once processing. This report establishes source requirements and a proposed evidence profile. Runtime verification remains outstanding.
