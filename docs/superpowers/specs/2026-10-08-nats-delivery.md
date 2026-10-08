# NATS JetStream delivery

Task 5 consumes published ROM Delivery and DeliveryOutcome. Runtime owns intent persistence, authorization, retry, and stable IDs.
The adapter prepares exact JSON through rom-delivery-core. It does not implement a second Work ledger.

Select async-nats 0.50.0 with explicit JetStream, AWS-LC, and NUID features; disable unrelated default capabilities.
The host supplies an authenticated/TLS-configured JetStream context. No payload-derived connection or destination is allowed.
Bind an existing file-backed stream and one explicitly listed subject. Reject memory storage and invalid/empty names.
Wildcard stream binding is outside the initial profile; do not silently accept broader destinations.
The host owns stream creation, limits, permissions, replica topology, fsync, and duplicate-window configuration.

Publish with Nats-Msg-Id equal to the unchanged Runtime ID and Nats-Expected-Stream equal to the bound stream.
Wait for both client publication and PublishAckFuture. Accepted requires the expected stream and positive sequence.
A duplicate acknowledgement is acceptance within the broker's configured duplicate window; it is not indefinite idempotency.
Do not advertise end-to-end exactly-once processing.

Bound local admission, prepared body bytes, binding lookup, and publication/acknowledgement time.
Overload is Retryable before external admission; invalid payload is Permanent before publication.
Network/acknowledgement errors and timeouts remain Unknown. They do not prove that buffered commands cannot later publish.
Host connection reconnect/buffer behavior belongs to async-nats and host client configuration.
Do not implement internal message retries; Runtime retry must retain stable identity.

The required fixture uses official NATS 2.15.0 pinned by executed digest, a named volume, authentication, and loopback ports.
Single-node JetStream uses sync_interval=always, file storage, bounded stream size, and a 120-second duplicate window.
Verify server runtime sync_always, not only configuration text.
Required tests must fail when fixture configuration is missing.
Require exact false payload, actual server acknowledgement, duplicate publication, broker restart, and retained file messages.
Then extend acceptance with consumer redelivery, lost acknowledgement injection, native ROM reopen/authorization/retry,
production TLS/reconnect contracts, independent public consumer, audit, and full local verifier.

Sources: [JetStream](https://docs.nats.io/concepts/jetstream),
[client publication](https://docs.rs/async-nats/0.50.0/async_nats/jetstream/context/struct.Context.html),
[deduplication headers](https://github.com/nats-io/nats.docs/blob/master/nats-concepts/jetstream/headers.md),
[fsync configuration](https://github.com/nats-io/nats.docs/blob/master/running-a-nats-service/configuration/README.md).
