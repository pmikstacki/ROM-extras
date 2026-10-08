# NATS post-commit acknowledgement loss and Runtime verification

Date: 2026-10-08. Base source: `e01a2190a08f8ebb01eb6b794987e3cdb3dad5b0`.
This increment adds real broker fault injection and native Runtime conformance. Production adapter code is unchanged.

The test proxy frames server MSG/HMSG bytes according to the official NATS protocol.
It suppresses one actual positive-sequence publish acknowledgement for the selected unique stream.
It forwards client bytes unchanged and never logs CONNECT credentials.
A separate direct broker connection reads the stored false JSON and unchanged ID before delivery completion.
An explicit completion flag checks this ordering; concurrent join alone would not establish it.
The first attempt returns Unknown. The next same-ID attempt receives Accepted with one retained stream message.
This tests the configured 120-second duplicate window, not indefinite or end-to-end exactly-once effects.

The native Runtime test runs against SQLite and redb at the unchanged published ROM pin.
An intent is Pending before delivery and survives shutdown/reopen with zero broker messages.
After dropped PubAck, Unknown and attempt one persist; immediate retry without clock advance performs no work.
After another reopen, retry retains ID and payload, receives Accepted, and persists Done with attempt two.
Source-field disclosure revocation and service revocation persist Denied without extra publication.
A final reopen retains Accepted and both denials; processing performs no further work.
The actual post-commit loss test does not restart the broker. A separate existing test covers broker restart and consumer redelivery.

Initial experiments retain the incorrect RawMessage header assumption, absent proxy failure, missing native test dependencies,
and required lockfile update failure. Successful intermediate logs are not substituted for final gate evidence.
Proxy readiness is bounded to three seconds. Child termination/wait is guarded during success and startup failure.
Required broker targets run sequentially after the restart/pause target to avoid interference.
The dedicated broker remained running and unpaused; streams, volume data, native stores, and proxy witnesses were preserved.

Command: `./scripts/check-all`. Final terminal exit: 0.
Evidence: `check-all-nats-runtime-final-2026-10-08.log`.
Toolchain: Rust1.99, GCC13.3, CMake3.30.5, OpenSSL3.5.8, Node22.16.
Private access scripts supplied PostgreSQL, MSSQL, and NATS credentials without printing them.
The gate includes formatting, Clippy, workspace tests/docs, isolated builds, HTTPS, SQL fixtures, NATS, and independent consumer checks.
Source review found no blocking defects; it identified the ordering proof gap subsequently checked explicitly.
A follow-up source review confirmed the proof gap was resolved, with no new findings.
Review is source inspection, not independently reproduced test execution.

The root locked graph changes only rom-nats test dependency edges to existing SQLite/redb packages.
The independent consumer lockfile is unchanged. A fresh root audit and full-feature package inventory are retained.
Cargo-audit0.22.2 reports 237 packages, zero vulnerabilities, and no warnings.
RustSec commit: `550efd3d587a29b2e2c2b21b17a440da4fede999`. Manifest licenses are present; redistribution review remains open.
Production TLS/reconnect, packaged release acceptance, and the remaining extras families stay open.
The goal remains active. Post-command source hashes are witnesses, not a complete pre/post release fence.
