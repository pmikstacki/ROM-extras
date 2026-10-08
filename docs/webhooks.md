# HTTPS webhook transport

`rom-webhook` sends signed JSON through one host-configured HTTPS endpoint.
It uses `rom-delivery-core` for bounded serialization and Standard Webhooks signatures.
Runtime supplies the stable delivery ID and attempt. The host supplies signing keys and trusted Unix seconds.

Resolve the destination outside the payload path. Construct `Destination::public_resolved` with all resolved IP addresses.
Every address must pass the public profile. Mixed public/private answers are rejected.
The transport pins those addresses for its lifetime; it does not resolve again during connection.
Rebuild the transport when host resolution changes. DNS acquisition and its deadline belong to host configuration.
IP literal destinations must match their supplied address exactly.
HTTPS certificate and hostname verification remain enabled.

The public profile rejects every special-purpose prefix in the reviewed IANA registries.
It also rejects multicast, reserved IPv4 space, and IPv6 outside `2000::/3`.
Globally reachable special-purpose allocations are deliberately excluded. This is stricter than general IP routability.
Recheck registry changes before release. See [the decision log](research/decision-log.md).

The transport disables redirects, proxy discovery, HTTP retries, cookies, and connection pooling.
It uses HTTP/1 with at most 64 response headers. It does not inspect or retain response body bytes.
HTTP parser and TLS buffer allocations remain dependency-owned; this is not a total process-memory bound.
Destination URLs and signing keys have no Debug or serialization API.

`TransportLimits` accepts 1..=1024 concurrent attempts and positive deadlines of at most 300 seconds.
The connect deadline must not exceed the request deadline. Defaults are eight attempts, two seconds, and four seconds.
Admission is immediate. Local overload returns `Retryable` before payload preparation or network I/O.
Payload validation failure returns `Permanent` before network I/O.
Custom `Input::encode` computation and allocation remain outside the serialized-output byte bound.

| Receiver result | ROM outcome |
| --- | --- |
| 2xx | Accepted, under the receiver's durable-acknowledgement contract |
| 3xx or ordinary 4xx | Permanent rejection |
| 408, 425, 429, or 5xx | Unknown |
| TLS/network failure or request deadline | Unknown |

HTTP status alone cannot prove application rollback. Receivers must honor the documented acknowledgement contract.
The adapter makes one attempt. Runtime owns retries and authorization; receivers must durably deduplicate stable IDs.
Dropping an admitted delivery future does not establish that the external effect was rolled back.

## Verification scope

The explicit `loopback-fixture` feature permits loopback destinations and a configured fixture CA.
It never disables certificate or hostname verification. It is absent from default features.
Tests generate a separate CA and server certificate with OpenSSL, then start an independent Node HTTPS receiver.
Fixture keys stay in a private temporary directory and are not committed.
The receiver checks exact body bytes, identity, timestamp, and signature with Node crypto.
Tests cover acceptance, permanent rejection, redirects, server error, lost acknowledgement, deadline, overload, and duplicate identity.
Receiver counters check one network attempt and no redirect follow-up. An untrusted CA fails.
Oversized payload preparation produces no receiver request.

Three Runtime tests cover native SQLite and redb intent reopening, persisted outcomes, authorization revocation, and bounded retries.
The independent receiver commits dedup IDs and body bytes in SQLite WAL/FULL before acknowledgement.
A killed/restarted receiver accepts the retried stable ID without a second stored effect.
Its test-only Node 22.16 SQLite API is experimental; its warnings are retained.
This establishes process-restart recovery, not power-loss behavior or a production receiver profile.
No supported production receiver or broker profile is claimed yet.
