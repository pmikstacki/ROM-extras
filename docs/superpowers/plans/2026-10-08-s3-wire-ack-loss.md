# S3 wire acknowledgement loss implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Verify actual lost S3 HTTP success response and explicit BlobService recovery on SQLite and redb.

**Architecture:** A test-only loopback HTTP/1 proxy preserves signed requests, forwards one PUT, observes backend success, and closes without returning final response bytes. The unchanged public adapter remains responsible for create-only writes and error mapping. Shared native lifecycle helpers own staging and drain.

**Tech Stack:** Existing published ROM d7ef529, RustFS fixture, standard-library bounded TCP streams, native SQLite/redb. No new dependency.

**Spec:** Task 7 of [complete extras plan](2026-10-08-rom-extras.md); [source assessment](../../research/s3-wire-ack-loss-assessment.md).

## Global constraints

- Research every project and technology decision; record primary sources.
- Preserve signatures, conditional creation, SDK deadlines, and retry policy.
- Never print requests, signed headers, credentials, URLs, or payloads.
- Retain physical objects and native artifacts; no deletion inferred from Unknown.
- Loopback only; finite framing, I/O, accepted connections, and worker shutdown.

## Review focus

- Exactly one forwarded PUT before Unknown; no concealed transport retry.
- Backend success and independent bytes prove actual publication.
- Pending revision remains unchanged; explicit retry must verify Conflict before Ready.
- Ready repetition makes no additional PUT; reopen persists revision and bytes.
- Unsupported framing fails clearly; no fabricated backend response.

## Task 1: Real wire fault and recovery acceptance

Files: tests/public-consumer/tests/s3_fixture/{mod,profile,wire,framing}.rs, tests/common/blob_lifecycle/{mod,ack_loss}.rs, tests/public-consumer/tests/s3_blob.rs, documentation and verification evidence.

- [x] Add the real-backend recovery scenario and transparent proxy baseline; require Unknown and observe the expected failure before enabling response suppression.
- [x] Implement bounded HTTP/1 framing and one-success response suppression; require actual backend success and exact PUT counts.
- [x] Run native recovery on both stores, repeat Ready upload, drain and reopen.
- [x] Run all S3 cases and warnings-denied Clippy; verify Azure shared-harness compatibility.
- [x] Obtain independent review; freeze runtime inputs, run complete verifier and compare hashes.
- [x] Record sanitized counts, command results and remaining limits; commit and push the working branch.

Response-only Connection: close and hop-by-hop removal follow RFC 9110 section 7.6.1 and RFC 9112 section 9.6; signed request bytes remain unchanged.

The selected fixture protocol supports bounded Content-Length framing and bodyless HEAD responses; unsupported transfer encoding must fail instead of guessing. RFC 9112 section 6 governs message length. Standard TcpStream read/write timeouts bound test-only blocking I/O. This is one local fault profile, not a general HTTP proxy or cloud qualification.

Primary references: [RFC 9112](https://www.rfc-editor.org/rfc/rfc9112.html), [TcpStream](https://doc.rust-lang.org/std/net/struct.TcpStream.html), and the linked adapter/backend assessment.
