# Email preparation

`rom-email-core` prepares one plain-text message from public `Delivery<EmailNotification>` without SMTP transport or network I/O.
It is the first implementation step for the [SMTP notification design](superpowers/specs/2026-10-09-notification-smtp-design.md).
The separate [SMTP adapter](smtp.md) handles authenticated TLS submission and native qualification.

## Host ownership

The host approves a fixed sender, exact recipient addresses, a Message-ID domain and the final message byte limit.
Only an exact approved recipient can pass preparation. Address case and spelling are not normalized.
The host must retain the same profile and immutable payload across retries and recovery.
Runtime owns authorization, durable Work, delivery attempts and reconciliation. No new outbox or receipt ledger is created.

`EmailNotification` implements public ROM `Input`. Its codec revalidates restored fields and rejects missing or extra members.
The payload contains personal data. The host must apply its storage and disclosure policy to addresses, subject and text.
Safe errors and Debug omit those fields; explicit access to prepared bytes exposes them to the authorized host.

## Limits

| Field | Accepted profile |
| --- | --- |
| Recipient or sender | One ASCII mailbox, at most 254 bytes; local part at most 64 bytes; lettre address validation |
| Approved recipients | 1–1024 unique exact addresses |
| Subject | 1–512 UTF-8 bytes; no control characters |
| Text | 0–262144 UTF-8 bytes; tabs, LF and CRLF permitted; other controls and bare CR rejected |
| Creation timestamp | 0–4102444800 Unix seconds, inclusive; supplied by the host when the intent is created |
| Message-ID domain | ASCII DNS labels, at least two labels; maximum 253 bytes; no DNS lookup |
| Final wire limit | Host selects 1–1048576 bytes, inclusive; checked after MIME encoding |
| Delivery identity | Existing shared 1–2048 ASCII letter/digit/hyphen/underscore profile |

The immutable creation timestamp supplies Date. A SHA-256 digest of the exact delivery identity supplies Message-ID.
Attempt number does not change bytes, envelope, Date or Message-ID.
The message includes typed MIME-Version, UTF-8 plain-text content type and transfer encoding from lettre.
`prepare_smtp` adds explicit Base64 text encoding for SMTP framing without changing `prepare`.
Its encoded output must also fit the host wire limit.
Logical text and terminal line breaks survive transport framing; LF is canonicalized to CRLF.
SMTP does not require a receiver to deduplicate that Message-ID. Do not infer exactly-once delivery.
This profile excludes display names, multiple recipients, SMTPUTF8, HTML and attachments.

## Public example

```rust
use rom::Delivery;
use rom_email_core::{EmailNotification, EmailProfile};

let profile = EmailProfile::new(
    "sender@example.invalid",
    &["Exact+Tag@example.invalid"],
    "host.example.invalid",
    1_048_576,
)?;
let payload = EmailNotification::new(
    "Exact+Tag@example.invalid", "Hello", "Approved message", 1_800_000_000,
)?;
let prepared = profile.prepare(Delivery {
    id: "work-17".into(), attempt: 1, payload,
})?;
assert_eq!(prepared.to(), "Exact+Tag@example.invalid");
// Pass these frozen bytes and envelope to a separately qualified transport.
# Ok::<(), rom_email_core::EmailError>(())
```

## Verification scope

`./scripts/check-email` runs boundary tests, Clippy and the independent public Rust consumer.
Set `ROM_EXTRAS_PYTHON` to an explicit Python 3 executable for independent RFC/MIME parsing.
Python checks decoded Unicode subject/text, MIME version, exact headers, date, digest identity and absence of Bcc/Cc.
No message is submitted to a server by this gate. Preparation checks do not qualify SMTP acceptance, TLS or production deliverability.

This core declares only lettre 0.11.23 builder support. Its independent consumer resolves no transport features.
Workspace builds can unify SMTP features from the separate adapter; the core performs no network I/O.
See [source research and dependency requirements](research/notification-smtp-2026-10-09.md).
The affected checks and full local verifier passed on unchanged source before integration.
See [the scoped verification record](verification/email-preparation-2026-10-09.json).
