# RabbitMQ confirmed delivery

`rom-rabbitmq::RabbitMqDelivery` binds a host-provisioned queue through the AMQP default exchange and exact queue-name routing key.
The host supplies a lapin Connection with authentication, TLS, and recovery policy. Payloads cannot select the connection or queue.
Binding creates a dedicated publisher channel, passively probes queue existence, and enables publisher confirms.
Passive AMQP lookup does not verify durability, queue type, policies, or replication. The host must maintain a durable queue.
The fixture provisions durable quorum queues; its single-node topology does not establish replicated failover.

Queue names contain 1..=255 ASCII alphanumeric/dot/hyphen/underscore bytes and exclude the reserved `amq.` prefix.
The finite operation deadline is positive and at most300 seconds. One publication is admitted per adapter at a time.
Overload returns Retryable before preparation or publication. Shared preparation bounds exact JSON bytes.
AMQP `message_id` must fit ShortString: IDs over255 bytes return Permanent before encoding. IDs are never truncated or normalized.
Published ROM's64-byte SHA256 intent IDs fit this profile. Custom Input encoding work remains outside the output-byte limit.

Publications are mandatory, use persistent delivery_mode2, application/json, and unchanged message_id.
Accepted requires publisher Ack without a returned message. It establishes broker acceptance, not consumer processing.
An Ack carrying confirmed NO_ROUTE is Retryable; unroutable publication is not accepted even when the broker sends Ack.
Other uncertain confirmations, errors, and deadlines return Unknown. Preparation rejection is Permanent before publication.

RabbitMQ AMQP does not deduplicate message_id. A same-ID retry can create another broker message.
Consumers must deduplicate durable effects and acknowledge only after their own durable acceptance contract.
Runtime owns durable intents, current authority, budgets, and stable identity. The adapter has no message retry or second ledger.

After Unknown or cancellation during admitted publication, the adapter retires. Later calls return Retryable without publishing.
The host must explicitly bind another adapter, retaining Runtime identity. Timeout/cancellation does not prove rollback.
Already-buffered commands can publish later. Retirement prevents reusing the same confirm channel for new attempts.
Default lapin4.12 properties disable automatic recovery and reconnect retry. Explicit host recovery must be reviewed separately.
The adapter has no Debug or payload logging. The selected client's returned-message tracing can include payloads.
Hosts collecting production traces must filter that dependency target, for example `lapin::returned_messages=off`.
This is a known client source behavior, not a guarantee that every third-party trace omits payloads.

## Required fixture

The labelled container and preserved volume are `rom-extras-rabbitmq-20261008`, with `rom-extras.fixture=rabbitmq`.
Official RabbitMQ4.3.2 management Alpine image, executed digest:

```text
docker.io/library/rabbitmq@sha256:0753b75ce99094c385483d89449d532a0544fb85e4942a478b21cc497ab66d33
```

The hostname is fixed to preserve the Erlang node's data identity. AMQP/management bind only to loopback,55445/55446.
The fixture has one CPU and512 MiB memory, with two Erlang schedulers and four async threads.
Its isolated user/vhost is rom_extras, with a random private password. Do not print its env/access files or connection URI.
The local fixture is plaintext; production TLS is not verified by these tests.

```sh
source .superpowers/rabbitmq-access.sh
./scripts/check-rabbitmq
```

`check-all` requires this profile alongside PostgreSQL, MSSQL, plaintext NATS, and TLS NATS.
Missing configuration fails instead of skipping. Restart/pause verify the dedicated fixture label first.
Pause is restored during ordinary completion or unwinding. The test retains all message-bearing queues and volume data.
Only a newly created empty routing-test queue is deleted and recreated to exercise mandatory NO_ROUTE.
After restart, readiness checks the RabbitMQ application; runtime ping alone does not establish AMQP availability.

One live test checks confirms, properties, duplicate messages, persistence/restart, unacknowledged redelivery,
unroutable return/reprovision, invalid configuration, preparation refusal, overload, timeout/cancellation retirement, and eventual buffered writes.
An independent public consumer receives actual confirmation and reads persistent false JSON and the unchanged ID.
The required Runtime test drops a real basic.ack after broker acceptance. A separate connection reads the message before timeout.
SQLite/redb reopening preserves Pending, Unknown, identity, and attempt count. Explicit host rebinding permits the Runtime-owned second attempt.
That retry creates two queue messages with the same ID. RabbitMQ does not deduplicate these publications.
A fixture receiver uses ROM's durable action receipt to commit one effect for both messages.
After the first effect commits, its consumer connection closes without ACK. RabbitMQ automatically redelivers the unacknowledged message.
The reopened receiver replays its receipt before acknowledging duplicates. A further reopening verifies exactly one retained counter increment.
Current source disclosure denial and service revocation stop work without further publication.
This receiver is a test application, not a universal production receiver or distributed exactly-once guarantee.
The following native TLS profile supplies CA/hostname/password and host-reconnection evidence. Packaged release remains open.
No complete production RabbitMQ provider support is claimed by this increment.

## Native TLS fixture and host connection

The separate container and volume are `rom-extras-rabbitmq-tls-20261008`, with label `rom-extras.fixture=rabbitmq-tls`.
It uses the same pinned official image, one CPU,512 MiB, and a fixed hostname `rom-extras-rabbitmq-tls`.
AMQPS binds only to loopback55447; management binds to loopback55448. Regular AMQP listeners are disabled.
The broker terminates TLS directly. Its leaf certificate has CA:FALSE, serverAuth, and IP SAN127.0.0.1.
Separate fixture CAs sign the leaf and supply the unrelated trust negative case. Synthetic certificates expire after30 days.
The broker runs as UID100/GID101 in the Alpine image. Mounted configuration and leaf key are readable only by that owner.
Private CA signing keys are not mounted. Keys and access files remain ignored; never print connection URIs.
The broker does not request a client certificate. Password authentication follows verified server TLS; this is not mTLS.

Hosts can use lapin's public `Connection::connect_with_config`, `tcp::OwnedTLSConfig`, and `runtime::default_runtime`.
Supply the CA PEM through cert_chain and retain an amqps URI. Native trust roots remain enabled alongside the added CA.
This configuration adds trust; it does not pin trust exclusively to that fixture CA. Do not log TLS identity configuration.
The extras adapter accepts the established Connection and does not bypass its certificate verification.

```sh
source .superpowers/rabbitmq-access.sh
source .superpowers/rabbitmq-tls-access.sh
./scripts/check-rabbitmq
```

Required negatives verify UnknownIssuer, name mismatch, ACCESS_REFUSED, and TLS InvalidContentType on the plaintext fixture port.
The TLS broker test obtains an actual publisher confirmation, restarts the labelled fixture, and establishes a fresh verified connection.
It reads the retained persistent false JSON and ID. An explicitly rebound adapter then confirms a new publication.
This is host-managed reconnect; automatic client recovery is not enabled or claimed.
The independent consumer executes public TLS configuration and reads confirmed persistent false JSON.
Both TLS and plaintext fixture configuration are required by check-all. Missing values fail instead of skipping.
Clustered failover, certificate rotation, client-certificate authentication, and packaged release remain open.
