# Delivery and webhook family

This implements Task 5 of the approved ROM-extras plan through published ROM `d7ef529040eec60dc869034c2d33130219db85fe`.
Runtime owns durable channel intents, retry budgets, current authorization, and the stable `Delivery.id`.
Adapters perform external delivery and return `DeliveryOutcome`. They do not create a second retry ledger or bypass Runtime authorization.

## Shared preparation increment

`rom-delivery-core` prepares exact JSON bytes from the typed ROM payload.
It preserves the delivery identity and attempt supplied by Runtime.
Limit JSON output during serialization, including quotes, escapes, and UTF-8 bytes.
The default limit is 64 KiB; host configuration accepts 1..=1 MiB. This is an explicit operational profile, not a ROM value restriction.
Reject invalid identities before calling the payload encoder.
Identity length is 1..=2048 bytes; accept ASCII letters, digits, hyphen, and underscore.
The selected ROM notification implementation emits 64 hexadecimal bytes and fits this profile.
Do not normalize or replace identity values. Future incompatible ID formats need compatibility review.
Custom `Input::encode` allocation or computation is outside the serialized-byte limit.

[serde_json::to_writer](https://docs.rs/serde_json/1.0.151/serde_json/fn.to_writer.html) supports a bounded output writer.
Preserve null, false, zero, and empty values. Do not add actor, Resource, secret, or internal metadata to the payload.
Debug output omits payload values. Signing keys have no Debug or serialization API.

## Webhook signing and transport

Use the [Standard Webhooks specification](https://github.com/standard-webhooks/standard-webhooks/blob/main/spec/standard-webhooks.md).
Sign exact ID, trusted host attempt timestamp, and exact body bytes with HMAC-SHA256.
Emit `webhook-id`, `webhook-timestamp`, and `webhook-signature`, with standard base64 `v1` signatures.
Support one current and at most one previous 32-byte key for bounded rotation.
[RustCrypto HMAC](https://docs.rs/hmac/0.12.1/hmac/) provides the MAC primitive; [zeroize](https://docs.rs/zeroize/latest/zeroize/) clears stored key buffers on drop.
Do not claim that every cryptographic intermediate is erased.
Host configuration supplies entropy, secret acquisition, and trusted time. Receivers enforce timestamp tolerance and durable ID deduplication.

The transport increment will use [reqwest](https://docs.rs/reqwest/0.13.5/reqwest/struct.ClientBuilder.html) with verified HTTPS and no automatic redirects or implicit proxy.
Destinations belong to host configuration, never delivery payloads. Pin validated resolution results to avoid DNS rebinding between validation and connection.
Reject resolved private, loopback, link-local, special-use, and multicast addresses for the public destination profile.
Use [IANA IPv4](https://www.iana.org/assignments/iana-ipv4-special-registry) and [IPv6](https://www.iana.org/assignments/iana-ipv6-special-registry) registries for address decisions.
Provide a separate explicit local-fixture profile; it must not weaken the production default.
Limit concurrency, connect and request deadlines, outbound body, and inspected response data.
Retries belong to Runtime. A lost acknowledgement remains unknown; it does not prove that the receiver rejected the message.

## Required acceptance still open

Implement HTTPS transport with destination policy and explicit status/outcome mapping.
Verify signed requests with an independently implemented real HTTP receiver.
Test redirects, private resolution, body limits, permanent rejection, overload, duplicates, and missing acknowledgement.
Create committed channel intents in a durable ROM backend, reopen, deliver, and inspect persisted outcomes.
Exercise source authorization revocation before send. Test bounded Runtime retry and receiver deduplication.
Kafka, NATS, RabbitMQ, and notifications each require real provider acknowledgement and restart/redelivery tests.
Shared preparation and synthetic signing tests alone do not establish webhook or broker support.
