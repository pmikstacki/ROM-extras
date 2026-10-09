# SMTP notifications

`rom-smtp` submits one host-approved message through authenticated implicit TLS.
It consumes public ROM `Delivery<EmailNotification>` and returns `DeliveryOutcome`.
The host owns authorization, sessions, resources, durable Work and retry policy.
The adapter creates no outbox, receipt ledger or internal retry loop.

## Configuration

The host supplies a literal `SocketAddr`, TLS server identity and EHLO domain through `Endpoint::new`.
The adapter performs no DNS lookup, endpoint discovery, proxy selection or alternate-relay fallback.
There is no default relay. Private addresses require explicit host configuration.
The host must approve network destinations before constructing an endpoint.

`Credentials::new` accepts a username and password, each 1–1024 UTF-8 bytes without control characters.
Credentials remain on the server. Do not place them in Resource payloads, browser descriptors, logs or frontend environment variables.
Credential Debug is opaque. Owned credential strings use zeroizing storage.
The protocol library creates temporary copies; complete zeroization of those copies is not guaranteed.

`Smtp::new` uses public WebPKI roots.
`Smtp::with_private_root` replaces them with one host-supplied DER root, limited to 65536 bytes.
Both paths verify the server certificate and configured identity.
They permit TLS 1.2 and TLS 1.3 without a plaintext fallback.
Certificate revocation checking is not qualified.
STARTTLS, client certificates and other authentication mechanisms are outside this profile.

`EmailProfile` fixes the sender, exact approved recipient addresses, Message-ID domain and final MIME byte limit.
The adapter uses additive `prepare_smtp` with Base64 text encoding.
The existing `prepare` behavior remains available for independent RFC preparation.
Both preserve immutable Date and Message-ID across attempts.
The [email core guide](email-core.md) defines input and wire limits.

## Bounds and outcomes

| Host policy | Accepted values |
| --- | --- |
| Whole attempt timeout | 1 millisecond–60 seconds |
| Total decrypted reply bytes | 1024–65536 bytes |
| Minimum dispatch interval | 0–86400 seconds |
| Concurrent admission | One attempt per shared adapter; no waiting queue |
| Encoded message | Profile limit, at most 1048576 bytes |

Share one adapter through `Arc` to enforce aggregate admission and rate limits.
Separate instances have separate limits. Limits do not persist across process restart.
Runtime and host policy must bound retries and their rate across restarts.

`Accepted` requires final DATA acknowledgment `250`.
It proves relay acceptance, not inbox delivery.
Known `4xx` and `5xx` rejections produce `Retryable` and `Permanent` respectively.
Invalid preparation produces `Permanent` before network I/O.
Admission, rate refusal and cancellation before dispatch produce `Retryable`.
Opaque I/O, invalid positive replies, deadline expiry and cancellation after dispatch produce `Unknown`.

The adapter requires `220` greeting, `235` authentication, `250` MAIL, `250` or `251` RCPT, and `354` before message bytes.
It checks the final acknowledgment separately.
Cleanup does not wait for QUIT or TLS shutdown after an established outcome.
Reply bytes are bounded before the parser can allocate an unterminated line.
Provider replies and credentials do not enter public errors or payloads.

SMTP provides no portable idempotency key or Message-ID deduplication guarantee.
The native fixture retained two messages when the same identity was intentionally submitted twice.
An absent message cannot prove `NotAccepted` after uncertainty.
Select an explicit Runtime retry policy that accepts duplicate risk, or reconcile through a qualified provider contract.
Do not infer delivery verification from this adapter.

## Host integration

Construct `Smtp` from host configuration and share it with a public Runtime channel handler:

```rust
// CHANNEL is a host-declared Channel<EmailNotification>.
// adapter is Arc<Smtp>; service is the host's authorized service actor.
builder.channel(CHANNEL, service, move |delivery| {
    let adapter = adapter.clone();
    async move { adapter.deliver(delivery).await }
});
```

The host action emits the intent after selecting its exact Resource and authorized recipient.
Runtime commits the intent with the resource mutation and rechecks authority before delivery.
The [independent public consumer](../tests/smtp-public-consumer/src/main.rs) constructs the transport.
Its [Runtime example](../tests/smtp-public-consumer/src/runtime.rs) uses SQLite and redb without private ROM APIs.
These examples send only to the selected local capture fixture.

## Local verification

The authored TLS protocol receiver tests malformed responses, explicit rejections, bounded reads, cancellation and uncertainty:

```sh
node scripts/check-smtp-protocol.mjs
```

Those results are controlled protocol evidence, not native-service qualification.
The native gate requires an owned, persistent Mailpit fixture with no external forwarding.
The fixture directory contains `setup.json`, private `credentials.json`, and a `tls` directory.
Setup records `id`, exact image ID and `tag`.
Credentials contain `username`, `smtp_password` and `api_password`.
Protect that file and all TLS keys with host file permissions; never commit their contents.
TLS files are `ca.pem`, `server.pem` and `server.key`, with `mailpit.fixture.test` in the server certificate.
Configure authenticated implicit SMTP on 1025 and authenticated HTTPS API on 8025.
Allow API Host `localhost:55470`. Disable forwarding, deduplication and automatic database deletion.

Select the fixture explicitly:

```sh
export ROM_EXTRAS_SMTP_NATIVE_FIXTURE=/absolute/path/to/owned/mailpit-fixture
export ROM_EXTRAS_SMTP_NATIVE_CONTAINER=rom-extras-mailpit-native-YOUR-RUN
export ROM_EXTRAS_PYTHON=/absolute/path/to/python3
./scripts/check-smtp
```

The native gate verifies container and image identity before restarting the selected container.
It preserves messages, keys, raw MIME, logs and Runtime databases.
After restart, the host runner explicitly refreshes the container's literal address.
The adapter itself never rotates destinations.
The fault proxy verifies native TLS and suppresses one observed final DATA acknowledgment per Runtime case.
Native API reads independently verify stored content and duplicate identity.
The gate repeats native and Runtime checks with consumers built from normalized Cargo archives.

## Qualification limits

The qualified native profile uses Mailpit 1.31.4 with private TLS, authentication and persistent storage.
It covers relay acknowledgment, wrong credentials, wrong identity, untrusted roots, restart, duplicates and SQLite/redb Runtime recovery.
Source review, authored protocol tests, native service tests and packaged consumption are separate evidence categories.
Production relay accounts, inbox deliverability, bounces, SPF/DKIM/DMARC and certificate rotation remain unqualified.
No external message delivery is enabled by these examples or checks.

See the [scoped verification record](verification/smtp-native-runtime-2026-10-09.json) for executed source and commands.
