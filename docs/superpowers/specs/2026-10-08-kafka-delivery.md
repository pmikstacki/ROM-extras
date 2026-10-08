# Kafka delivery adapter

This extends Task5 of the approved extras plan through published ROM Delivery and the shared bounded JSON preparation core.
Runtime retains durable intents, authority, stable ID, and retry budgets. Kafka acceptance does not establish consumer effects.

Use rdkafka0.39.0's safe public producer surface. Assess native dependency features, licenses, advisories, and MSRV before publication.
The adapter constructs and privately owns its producer from a supplied ClientConfig. It does not accept an unverifiable existing producer.
[ClientConfig](https://raw.githubusercontent.com/fede1024/rust-rdkafka/v0.39.0/src/config.rs) permits safe native configuration validation before construction.
An existing producer does not expose a safe effective configuration getter. Avoid custom unsafe FFI in extras.

Enforce acks=all, no protocol message retries, nontransactional/non-idempotent production, bounded native queue, and disabled automatic topic creation.
Remove contradictory aliases before installing the adapter's fixed delivery profile. Refuse transactional.id rather than silently changing transactional semantics.
Host configuration retains brokers, SASL/TLS, certificates, and authentication. Do not log its values or configuration errors containing credentials.
The broker administrator owns topic provisioning, replication, retention, min.insync.replicas, flush policy, and ACLs.
[librdkafka configuration](https://raw.githubusercontent.com/confluentinc/librdkafka/v2.12.1/CONFIGURATION.md) distinguishes acknowledgement and retry guarantees.
Enabling producer-session idempotence conflicts with retries=0. Stable ROM ID is not Kafka's producer-session sequence identity.

Publish the exact prepared JSON body. Preserve the unchanged ROM delivery ID as the record key.
Use an explicit exact topic and JSON content-type header. Do not append actor, Resource, secret, or internal metadata.
Fixed serial admission and a finite deadline bound transport work. Shared body limits and identity checks precede native enqueue.
Immediate QueueFull is Retryable before enqueue. Preparation rejection is Permanent. Uncertain delivery results and timeouts are Unknown.
Accepted requires an actual successful delivery report with a nonnegative partition and offset under the enforced acknowledgement profile.
[FutureProducer](https://docs.rs/rdkafka/0.39.0/rdkafka/producer/struct.FutureProducer.html) separates queue admission from delivery completion.
Use send_result for immediate enqueue refusal; await its delivery future without another adapter retry loop.
After uncertain admitted work or cancellation, retire the adapter. Native queued work can still publish; retirement does not establish rollback.
Host replacement retains Runtime identity. Receivers must deduplicate durable effects; no cross-Resource or global ordering is claimed.

Required real conformance includes actual offset/body/key/header, same-ID duplicates, restart persistence, native queue refusal,
preparation refusal, unknown publication and retirement, missing configuration failure, and an independent public consumer.
Further acceptance requires actual post-commit acknowledgement loss, native Runtime reopen/current authority, verified TLS/SASL, and packaged release.
Initial implementation evidence cannot mark the complete Kafka provider or parent delivery family complete.
