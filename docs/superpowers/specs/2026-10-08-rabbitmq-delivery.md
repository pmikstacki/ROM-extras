# RabbitMQ confirmed delivery

Use public ROM Delivery/DeliveryOutcome and shared bounded JSON preparation. Runtime owns intents, authorization, retry, and identity.
Select lapin4.12.0 with explicit Tokio, Rustls/AWS-LC, and native-root features. Its manifest declares MIT and MSRV1.88.
The host supplies a Connection with authentication/TLS and recovery policy. Default lapin properties disable automatic recovery and reconnect retry.
Bind an existing queue through the default exchange, using its exact queue name as routing key.
The adapter creates a private dedicated publisher channel, probes queue existence passively, and enables publisher confirms.
Passive AMQP declaration does not establish queue durability or type. The host must provision and maintain a durable queue.
The required fixture provisions durable quorum queues. Replicas/topology, policies, retention, and ACLs remain host-owned.

Use mandatory publication, persistent delivery_mode2, application/json, and unchanged message_id.
AMQP message_id is a ShortString: reject IDs over255 bytes before payload encoding. Do not truncate or normalize them.
Queue names use ASCII alphanumeric, dot, hyphen, underscore, at most255 bytes, and exclude reserved amq. names.
Keep one admitted publication per adapter/channel, a positive deadline at most300 seconds, and shared body limits.
Accepted requires Ack without a returned message. Broker enqueue alone and NotRequested are not Accepted.
A confirmed NO_ROUTE return is Retryable without an external queue effect. Other uncertain confirms/errors/deadlines are Unknown.
Oversized or invalid preparation is Permanent before publication. Immediate overload is Retryable before external admission.
No message retry or second ledger is implemented. RabbitMQ AMQP does not deduplicate message_id; receivers must deduplicate durable effects.

After uncertain publication or cancellation, retire the adapter from further publication. The host must explicitly bind a new adapter.
Queued writes may still occur after timeout/cancellation. Retirement does not establish rollback or stop all connection buffers.
Dedicated serial publication plus retirement avoids reusing a channel with an unresolved acknowledgement/returned-message association.
Do not log payloads or implement Debug on the adapter. The selected client's returned-message tracing can include payloads:
the host must filter that target when collecting production logs. This known dependency behavior is not a silent-library guarantee.

Require real confirms, false JSON/properties, mandatory unroutable return, preparation refusal, queue persistence/restart,
unacknowledged consumer redelivery, overload/uncertain retirement, independent public consumer, audits, and full local verifier.
Extend with actual post-commit confirmation loss, native Runtime reopen/authority/retry, and verified TLS host profiles.
The Runtime increment now verifies actual confirm loss and native SQLite/redb reopen, authority, and retry.
Its fixture receiver commits one durable action receipt, closes without consumer ACK, and deduplicates real redelivery across reopening.
The receiver is a test application. Process-crash, power-loss, verified TLS, and packaged release remain open.
No production provider or complete delivery family is claimed by initial publication tests.

Sources: [RabbitMQ confirms](https://www.rabbitmq.com/docs/confirms),
[quorum queues](https://www.rabbitmq.com/docs/quorum-queues),
[lapin confirmation](https://docs.rs/lapin/latest/lapin/struct.PublisherConfirm.html),
[pinned lapin source](https://github.com/amqp-rs/lapin/tree/5226cfb1d05b965369d87f75f505a76943274b6a).
