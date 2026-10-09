# SMTP notification research

Reviewed 2026-10-09. Research only. No packages installed, builds run, messages sent, or services changed.
This note records source research; it does not establish runtime qualification.

## Client selection

The current lettre documentation reports version 0.11.23. Its introductory prose claims Rust 1.74.
However, the tagged 0.11.23 manifest declares edition 2024 and `rust-version = "1.85"`.
Use the tagged manifest's MSRV. Do not promise 1.74 compatibility from the stale prose.
The package license is MIT.
[Current docs](https://docs.rs/lettre/latest/lettre/), [tagged manifest](https://raw.githubusercontent.com/lettre/lettre/v0.11.23/Cargo.toml).

Proposed dependency: exact 0.11.23, `default-features = false`, with `builder`, `smtp-transport`, `tokio1-rustls`, `ring`, and `webpki-roots`.
This omits the default native TLS backend, connection pool, and automatic hostname feature.
Rustls needs a crypto provider and certificate verification backend. Explicit feature selection supplies both.
Use `Tokio1Executor` with Tokio 1. Select one supported trust model; custom fixture roots must remain explicit.
Dependency resolution, transitive licenses, build compatibility, and the effective dependency MSRV still require later qualification.
[Features and manifest](https://raw.githubusercontent.com/lettre/lettre/v0.11.23/Cargo.toml), [feature explanations](https://docs.rs/lettre/latest/lettre/).

### Security review

The maintainer advisory GHSA-4pj9-g833-qx53 describes disabled hostname verification in the Boring TLS backend.
Versions 0.10.1 through 0.11.21 are affected. Versions 0.11.22 and later are patched.
Native TLS and rustls are unaffected by this specific defect.
[Maintainer advisory](https://github.com/lettre/lettre/security/advisories/GHSA-4pj9-g833-qx53).

RustSec also lists SMTP body command injection, patched by 0.10.0-rc.3, and historical sendmail argument injection.
Do not enable the sendmail transport for this increment.
This source review is not a dependency audit of a resolved lockfile.
[SMTP injection](https://rustsec.org/advisories/RUSTSEC-2021-0069.html), [sendmail injection](https://rustsec.org/advisories/RUSTSEC-2020-0069.html), [package advisories](https://rustsec.org/packages/lettre.html).

`AsyncSmtpTransport::relay` selects implicit TLS and validates the server domain.
`starttls_relay` requires a successful TLS upgrade before credentials or mail are sent.
Do not select opportunistic STARTTLS for production submission. Never accept invalid certificates or hostnames.
Tokio `send_raw` returns the message response, then performs best-effort connection abort without pool support.
The async-std implementation instead propagates QUIT errors after message acceptance. Avoid conflating those executor paths.
[Tagged async transport](https://raw.githubusercontent.com/lettre/lettre/v0.11.23/src/transport/smtp/async_transport.rs).

## SMTP acceptance and uncertain outcomes

Prepared UTF-8 messages must include `MIME-Version: 1.0` with their content and transfer-encoding headers.
Lettre's plain `body` builder does not insert this header. Set its typed MIME version header explicitly.
[RFC 2045 section 4](https://www.rfc-editor.org/rfc/rfc2045.html#section-4),
[tagged message builder](https://raw.githubusercontent.com/lettre/lettre/v0.11.23/src/message/mod.rs).

A positive completion after the DATA terminator transfers responsibility to the receiving SMTP server.
It does not prove delivery to a mailbox, inbox placement, or reading by a person.
An explicit transient rejection is different from a missing response after DATA.
SMTP queue retry guidance uses a generally recommended interval of at least 30 minutes and a give-up period of at least 4–5 days.
That guidance describes mail transfer queues. Do not automatically impose it on every ROM submission profile.
[RFC 5321 sections 4.2.5 and 4.5.4.1](https://www.rfc-editor.org/rfc/rfc5321.html).

If the server accepts the message but the client misses the acknowledgment, resubmission can produce duplicates.
RFC 1047 describes this synchronization gap. SMTP does not offer general idempotent submission or query-by-delivery-ID.
[RFC 1047](https://www.rfc-editor.org/rfc/rfc1047.html).

Use one deterministic Message-ID for one frozen delivery identity and content.
Message-ID syntax and uniqueness do not require a receiver to deduplicate submissions.
Do not change Message-ID for each attempt. Do not use a random generated ID as a substitute for `Delivery.id`.
[RFC 5322 section 3.6.4](https://www.rfc-editor.org/rfc/rfc5322.html#section-3.6.4).

Lettre exposes transient/permanent SMTP status, client, TLS, connection, timeout, and shutdown classification.
Its public `Error` does not expose an authoritative SMTP transaction stage or accepted-message receipt.
Thus a broad transport failure cannot establish that DATA was never accepted.
[Tagged error API](https://raw.githubusercontent.com/lettre/lettre/v0.11.23/src/transport/smtp/error.rs), [message transaction](https://raw.githubusercontent.com/lettre/lettre/v0.11.23/src/transport/smtp/client/async_connection.rs).

Proposed initial outcome mapping:

| Evidence | ROM outcome |
| --- | --- |
| Local address/header/content validation fails before I/O | `Permanent` |
| Final positive DATA completion | `Accepted` |
| Explicit temporary SMTP rejection, with no accepted effect | `Retryable` |
| Explicit permanent SMTP rejection, with no accepted effect | `Permanent` |
| Transport failure or cancellation with uncertain acceptance | `Unknown` |

Treat opaque errors conservatively. Do not map every timeout or connection error to `Retryable`.
If richer safe pre-DATA retry classification is required, instrument a public SMTP connection seam rather than parse debug strings.
The first increment can keep opaque failures `Unknown` and leave recovery to trusted host verification.

## Public ROM seam

Read-only review of the public ROM pin `d7ef529040eec60dc869034c2d33130219db85fe` confirmed the supported channel seam.
The authoritative checkout is `/root/.cargo/git/checkouts/rom-93d40b828d518890/d7ef529`.
The dirty `/root/ROM` source is not evidence of published extras APIs.
`Delivery<P>`, `DeliveryOutcome`, `ChannelRegistration<P>`, `DeliveryReconciliation`, and `DeliveryVerification` are exported through the crate root.
`Delivery.id` is stable. `attempt` is separate. `NotAccepted` asserts that the old attempt cannot succeed later.
`Channel::delivery_profile`, registration `verifier` and `verification_timeout`, and `RuntimeBuilder::channel_with` exist at this pin.
The callback consumes `Delivery<P>` and returns a Send future yielding `DeliveryOutcome`; `P` must implement `Input`.
Registration requires a service actor.
[Published exports](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/lib.rs),
[channel exports](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/channels.rs),
[delivery model](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/channels/model.rs),
[registration](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/channels/registration.rs),
[builder](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/execution/builder.rs).

`Channel::intent_at` and channel `not_before` scheduling are absent at this pin.
Do not design the SMTP increment around those unpublished additions.
`PreparedDelivery` bounds serialized JSON; SMTP MIME bytes require their own final byte limit.
That preparation helper is an extras source observation at `/root/ROM-extras/crates/rom-delivery-core/src/message.rs`, not a ROM export.

Propose `SmtpDelivery` with host-owned transport and an explicit, immutable SMTP submission profile.
Consume a typed `Delivery<EmailNotification>` through `RuntimeBuilder::channel_with`.
Keep current authorization, durable Work/intent state, attempts, retry budgets, and recovery in ROM Runtime.
[Published notification execution](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/channels/execution.rs),
[persisted notification intentions](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/channels/intentions.rs).
Do not create a notification outbox, alternate receipt ledger, or internal retry loop.

Start with one envelope recipient per delivery. This avoids partial-recipient acceptance complexity.
Keep envelope sender/recipient separate from display headers. Freeze both before external I/O.
Bound subject, address, body, and final MIME sizes. Build typed headers and reject injection inputs.
Use a validated host Message-ID domain and a deterministic digest of the stable delivery identity.
The existing delivery identity can be 2048 bytes; do not insert it unbounded into Message-ID.

Server credentials stay host-owned and redacted. No implicit relay discovery, MX delivery, default external endpoint, or browser credentials.
One callback makes one SMTP submission attempt. Runtime timeout/cancellation must preserve uncertainty.
SMTP itself cannot implement a portable `NotAccepted` verifier. A missing message in a fixture API is insufficient.
Provide `Unresolved` unless trusted host evidence proves acceptance or impossibility of later acceptance.

## Mailpit fixture

The official latest release resolved to v1.31.4. The tagged project license is MIT.
Pin the release and retain image digest/version evidence when the fixture is later provisioned.
[Release](https://github.com/axllent/mailpit/releases/tag/v1.31.4), [license](https://raw.githubusercontent.com/axllent/mailpit/v1.31.4/LICENSE).

Mailpit defaults to SMTP port 1025 without encryption or authentication.
Certificates enable optional STARTTLS. `MP_SMTP_REQUIRE_STARTTLS=true` makes upgrade mandatory.
`MP_SMTP_REQUIRE_TLS=true` selects implicit TLS and disables STARTTLS on that listener.
`MP_SMTP_AUTH_FILE` supports PLAIN/LOGIN. Authentication normally requires TLS.
Keep insecure-auth and accept-any switches disabled for qualification.
[SMTP controls](https://mailpit.axllent.org/docs/configuration/smtp/).

Set explicit loopback/private bind addresses. Disable version checks and reverse DNS for a deterministic local fixture.
Use `MP_DATABASE` with a preserved local SQLite file; default temporary storage disappears on exit.
Disable auto-deletion with `MP_MAX_MESSAGES=0`; leave max-age unset.
Keep `MP_IGNORE_DUPLICATE_IDS=false`, its default, so duplicate submissions remain observable.
Do not configure relay, forwarding, or webhook destinations. Do not enable external message inspection features.
Set strict RFC headers and recipient restrictions deliberately. Keep silent rejected-recipient acceptance disabled.
[Runtime controls](https://mailpit.axllent.org/docs/configuration/runtime-options/).

Mailpit supplies a REST API and interactive API documentation for fixture observation.
Use its recorded stored message and raw headers as test evidence, not proof of internet mail delivery.
Its database is provider fixture state, not a second ROM delivery ledger.
[Mailpit API](https://mailpit.axllent.org/docs/api-v1/).

## Later qualification plan and limits

Require separate plaintext local protocol tests and authenticated TLS tests.
Include wrong-hostname, untrusted-CA, wrong-password, missing STARTTLS, malformed-header, oversized-content, and recipient rejection cases.
Capture a real final DATA acknowledgment for `Accepted`.
Use an isolated SMTP-aware proxy to suppress that final acknowledgment after Mailpit stores the message.
Verify Runtime records an uncertain outcome and does not blindly retry after restart.
Keep Message-ID dedup disabled when demonstrating duplicate risk.

Test native Runtime/store reopen before delivery, after uncertainty, and after acceptance.
Test current authorization revocation before external submission.
Inspect counts and frozen headers/content using fixture APIs; preserve the database and logs for each run.
Do not delete or reset historical evidence.

Source review does not prove the proposed crate compiles, dependency policy passes, or live TLS/auth behavior works.
No Mailpit instance, proxy, or SMTP endpoint was contacted in this task.
No production SMTP provider, deliverability, bounce processing, DKIM/SPF/DMARC, multi-recipient behavior, or exactly-once guarantee is qualified.

## Transport read bounds

The selected connection seam is public `AsyncSmtpConnection::connect_with_transport`.
Lettre checks reply size after `read_line`; an unterminated line can allocate before those checks.
The adapter must bound decrypted reads before the parser, and use a whole-attempt timeout.
It will establish verified implicit TLS on a literal host-approved SocketAddr, then provide an opaque bounded stream.
Lettre marks an externally supplied stream as TCP; the adapter owns TLS proof and never performs a downgrade or upgrade.
Enable only SMTP/Tokio features for its protocol client. Never enable tracing of AUTH or message bytes.
[Tagged reader](https://raw.githubusercontent.com/lettre/lettre/v0.11.23/src/transport/smtp/client/async_connection.rs),
[tagged public stream](https://raw.githubusercontent.com/lettre/lettre/v0.11.23/src/transport/smtp/client/async_net.rs).

A total decrypted reply budget of 1024–65536 bytes limits parser allocation before a missing newline.
Host supplies a 1ms–60s total attempt budget and 0–86400s dispatch interval. One shared transport admits one attempt.
Require greeting 220, AUTH 235 and final DATA 250; a different positive completion does not prove message acceptance.
Do not wait for QUIT after accepted DATA. A QUIT failure must not erase established acceptance.
These are selected adapter limits, not universal SMTP service capacities. Native and hostile-peer tests must qualify them.

## Implemented transport decisions

Native DATA capture showed that transport framing appends CRLF to an unencoded text part.
The additive `prepare_smtp` method uses Base64 to preserve logical text and final empty lines.
The original preparation method remains stable.
Encoding rules follow [RFC 2045](https://www.rfc-editor.org/rfc/rfc2045.html).

The adapter uses explicit public MAIL, RCPT, DATA and message commands instead of the library's `send` helper.
It checks `354` before sending content and `250` after content, as required by [RFC 5321](https://www.rfc-editor.org/rfc/rfc5321.html).
The bounded stream refuses cleanup QUIT locally and closes locally when the enclosing attempt drops it.
This prevents EHLO initialization cleanup waits from masking known rejections.
The selected PLAIN mechanism does not support challenges; unexpected `334` remains Unknown without message submission.
Tagged [lettre connection source](https://raw.githubusercontent.com/lettre/lettre/v0.11.23/src/transport/smtp/client/async_connection.rs) exposes those cleanup waits.
Independent failing regressions confirmed the behavior before the correction.

The later implementation has actual native evidence, separate from the initial research-only statements above.
See [SMTP qualification and limitations](../smtp.md).

The Node protocol fixture disables Nagle batching for short replies without changing attempt budgets.
See [Node socket.setNoDelay](https://nodejs.org/api/net.html#socketsetnodelaynodelay).
