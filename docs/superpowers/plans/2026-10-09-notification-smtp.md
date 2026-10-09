# SMTP notifications Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans. Root owns implementation files. Steps use checkbox syntax.

**Goal:** Deliver bounded, host-approved notifications through public ROM Delivery/Work and authenticated SMTP.

**Architecture:** Separate pure RFC message preparation from one-attempt SMTP submission. Runtime owns persistence, authority and recovery.

**Tech Stack:** Rust 1.99, public ROM d7ef529, lettre 0.11.23, SHA-256; native Mailpit 1.31.4 for later qualification.

**Spec:** ../specs/2026-10-09-notification-smtp-design.md

## Global Constraints

- Preserve existing API paths and all historical evidence.
- No production SMTP support from preparation or mock results.
- Input, byte limits and safe diagnostics follow the exact values in the spec.
- No external mail, alternate ledger, internal retry or unsafe TLS.

## Review Focus

- Restored payloads must revalidate all fields and reject unknown members.
- Unicode/line-ending MIME expansion must count toward the final wire limit.
- Retry must preserve exact date, Message-ID, envelope and body bytes.
- Approved recipient matching must preserve exact mailbox spelling.
- DATA acknowledgment loss must preserve uncertainty across Runtime restart.

### Task 1: Pure preparation

Files: create crates/rom-email-core/{Cargo.toml,src/lib.rs,src/error.rs,src/payload.rs,src/profile.rs,src/prepared.rs,tests/preparation.rs}; add shared identity validation in rom-delivery-core.
Interfaces: EmailNotification::new(to,subject,text,created_unix_seconds), EmailProfile::new(sender,recipients,message_id_domain,wire_limit), profile.prepare(Delivery<EmailNotification>) -> Result<PreparedEmail,EmailError>.

- [x] Write failing public tests for roundtrip, injection, exact recipient policy, stable retry bytes, Date/Message-ID, input/wire limits and redacted Debug.
- [x] Run the tests and retain the expected missing-API failure.
- [x] Implement pure preparation with only lettre builder enabled; no network APIs.
- [x] Run focused tests, Clippy, docs, no-default-feature and resolved dependency checks.
- [x] Add independent public consumer and gate; verify actual RFC bytes and restored payload behavior.
- [x] Request source review; fix confirmed findings with regressions.
- [x] Freeze source, run full local verifier and record scope before integration.

### Task 2: SMTP transport

Files: create crates/rom-smtp and controlled SMTP protocol consumers; add scripts/check-smtp.
Interfaces: consumes PreparedEmail and host endpoint/TLS/auth configuration; produces DeliveryOutcome with documented uncertainty.

- [ ] Add failing controlled protocol tests for acknowledgment, explicit rejection, loss, deadlines and cancellation.
- [ ] Implement one-attempt TLS submission, bounded admission and safe errors; no internal retry.
- [ ] Test private credentials and certificates never enter diagnostics or payloads.

### Task 3: Native and Runtime acceptance

Files: owned Mailpit fixture runner, public consumers, SQLite/redb Runtime tests, verification records and docs.

- [ ] Provision isolated authenticated TLS Mailpit with persistent database and no external forwarding.
- [ ] Observe native acceptance, authentication/TLS rejection, restart and intentional duplicate submissions.
- [ ] Suppress final DATA acknowledgment after storage; verify Unknown and public Runtime recovery on both stores.
- [ ] Check authorization revocation before external submission and independent/package consumers.
- [ ] Run affected checks and frozen full verifier; publish only the qualified scope.
