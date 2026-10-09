# SMTP notifications

Implement the notification family in the approved complete ROM-extras goal. Keep existing delivery APIs stable.

## Boundary

Use public ROM pin `d7ef529040eec60dc869034c2d33130219db85fe` and public Delivery/Work registration.
Runtime owns durable intentions, authorization, attempts, retry budgets and reconciliation. No alternate Work ledger is permitted.

A typed `EmailNotification` contains one exact ASCII mailbox, UTF-8 subject, plain-text body and immutable Unix creation seconds.
Host policy approves recipient addresses, a fixed sender and a Message-ID domain. Persisted payloads contain personal data; host storage policy applies.
The initial mailbox profile excludes display names, SMTPUTF8 and multiple recipients. It does not establish internet deliverability.

`rom-email-core` prepares RFC message bytes without transport features or network I/O. Reuse delivery identity validation from `rom-delivery-core`.
Subject is 1–512 UTF-8 bytes without controls. Text is at most 262144 UTF-8 bytes; only tabs and valid line endings are permitted controls.
Mailboxes are at most 254 ASCII bytes, with local part at most 64 bytes, and must pass lettre address parsing.
Creation time is 0–4102444800 Unix seconds. Dates and Message-ID must not depend on current time or attempt number.
Host wire-byte limit is 1–1048576 inclusive. Enforce it after MIME encoding as well as the bounded input limits before encoding.
Message-ID uses a SHA-256 digest of the stable identity and the host domain. It does not promise receiver deduplication.
Debug and errors omit addresses, subject, body, identity and credentials. Explicit message/envelope access remains privileged host data.

## Transport acceptance

A later `rom-smtp` transport uses authenticated implicit TLS to one host-selected relay; required STARTTLS can follow as a separate qualified profile.
No external endpoint, MX discovery, sendmail, browser disclosure, automatic retry or opportunistic TLS is allowed.
One delivery attempt has bounded admission, timeout and rate policy. Plain SMTP is restricted to controlled protocol fixtures.
Positive DATA completion is `Accepted`, not proof of mailbox delivery. Explicit temporary/permanent rejection maps to Retryable/Permanent.
Opaque I/O failure after admission is Unknown. Cancellation must not assert rollback. SMTP cannot prove portable NotAccepted reconciliation.

## Qualification

The pure preparation increment must pass boundary tests, an independent public consumer, dependency checks and the full local verifier.
Complete SMTP acceptance additionally requires controlled protocol and native Mailpit TLS/auth/persistence/restart tests.
Use an isolated response-loss proxy after native storage acceptance. Disable Message-ID deduplication to observe duplicate risk.
Register through public Runtime on SQLite and redb, and verify revocation and uncertain outcomes across reopen.
Do not send messages to real people. Owned fixtures have no forwarding, relay, webhook or external inspection.
Preserve all historical databases, logs and failed attempts. Production SMTP accounts, bounces and DKIM/SPF/DMARC remain separate acceptance work.

## Research

See [official-source research](../../research/notification-smtp-2026-10-09.md).
Lettre 0.11.23 is MIT and declares Rust 1.85. The workspace remains Rust 1.99.
Enable only lettre `builder` in the core. Resolve and inspect transitive dependencies before adopting the transport features.
