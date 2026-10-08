# NATS JetStream delivery

`rom-nats::JetStreamDelivery` binds an existing file-backed stream and one explicitly listed subject.
The host supplies a JetStream context configured for authentication, TLS, reconnect, and bounded client buffers.
The adapter does not create or modify streams. It rejects memory storage, disabled publication acknowledgements, and wildcard binding.
Names and subject tokens use ASCII alphanumeric, hyphen, and underscore characters. Dots separate subject tokens.
Stream names are at most 128 bytes; subjects are at most 256 bytes. These are this adapter's conservative configuration limits.

`bind` validates positive admission of 1..=1024 and a positive deadline of at most 300 seconds.
The same finite deadline bounds the stream lookup and each publication/acknowledgement attempt.
`deliver` uses the shared bounded JSON preparation. Custom Input encoding work remains outside the output-byte limit.
The unchanged Runtime ID becomes `Nats-Msg-Id`; `Nats-Expected-Stream` pins the expected stream name.
Accepted requires both client publication and a broker acknowledgement naming the expected stream with a positive sequence.

Immediate overload returns Retryable before external admission. Payload preparation failure returns Permanent before publication.
All publication errors, uncertain acknowledgements, and deadlines return Unknown.
A client-buffered command can publish after the attempt times out. Neither dropping the future nor timeout proves rollback.
The adapter creates no retry ledger and performs no message retry. ROM owns durable intents, retry budgets, and authorization.
Host-owned client reconnect and buffer behavior remain outside the adapter's semaphore bound.

Broker deduplication has a configured finite window. Stable IDs do not guarantee indefinite or end-to-end exactly-once effects.
The host owns replica topology, retention, duplicate window, fsync configuration, and permissions.
Later stream configuration changes can invalidate the profile checked at binding.

## Required local fixture

The dedicated fixture is `rom-extras-nats-20261008`, with label `rom-extras.fixture=nats`.
It uses official NATS 2.15.0, executed image digest:

```text
docker.io/library/nats@sha256:cd3fcd4ecdda44e3a66728a5334af0a959bc3979b32810e033d1c547241cd0f4
```

The preserved named volume is `rom-extras-nats-20261008`, mounted at `/data`.
The container has one CPU and 256 MiB memory. Broker and monitoring ports bind only to loopback, 55441 and 55442.
The private configuration contains a random authentication token and `sync_interval=always`.
Runtime monitoring confirms `sync_always=true`, maximum memory 64 MiB, and maximum file storage 256 MiB.
Do not print `.superpowers/nats.conf` or `.superpowers/nats-access.sh`.
The plaintext loopback fixture is not a production TLS profile.

With the fixture running, execute:

```sh
source .superpowers/nats-access.sh
./scripts/check-nats
```

`check-all` now requires this fixture alongside PostgreSQL and MSSQL. Missing fixture variables fail instead of skipping tests.
The broker test verifies the loopback URL and container label before restarting or pausing the dedicated container.
It retains fixture streams and volume data. It restores a paused broker on normal return or unwinding failure.

The live broker test checks exact false JSON, stable-ID deduplication before/after restart, and retained file messages.
An unacknowledged durable consumer receives the same stream sequence after restart, then confirms its acknowledgement.
A paused broker test produces Unknown while publication is buffered; resuming proves later persistence and duplicate acceptance.
This is timeout-before-persistence evidence, not direct acknowledgement loss after persistence.
It also checks overload, oversized preparation, invalid configuration, and memory-stream rejection.
An independent public consumer compiles and obtains an actual JetStream acknowledgement.

Separate proxy tests discard the first actual JetStream publish acknowledgement after its positive stream sequence arrives.
An independent direct broker connection reads the committed message before the adapter deadline expires.
The adapter returns Unknown; the same-ID retry receives Accepted, with one stored message within the duplicate window.
The proxy forwards client bytes unchanged and never logs authentication credentials. It parses server MSG/HMSG framing only.

Native SQLite and redb tests persist an intent before external delivery, then close and reopen ROM.
They persist Unknown after the dropped acknowledgement, reopen again, and retry the unchanged ID and false payload.
Accepted and attempt count two survive a further reopen. Current source-field denial and service revocation prevent publication.
The fixture proxy does not restart the broker; broker restart evidence comes from the separate broker test.
Tests run sequentially with the restart/pause test to avoid interfering with the same dedicated broker.

Production TLS/reconnect checks, a public packaged-consumer release, and the production topology profile remain open.
No complete production NATS provider profile is supported yet. Kafka, RabbitMQ, and notification connectors remain pending.
