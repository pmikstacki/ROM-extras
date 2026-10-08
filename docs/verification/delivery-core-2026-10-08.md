# Delivery preparation verification

Date: 2026-10-08. This verifies payload preparation and webhook signatures, not an external delivery provider.

The initial red test failed because the preparation and signing public APIs did not exist.
Implementation passed five preparation tests, two signing tests, and one doctest.
Preparation tests distinguish null, false, zero, and empty values; preserve retry identity; and count exact JSON bytes.
They cover escaping and UTF-8, reject invalid IDs before custom encoding, and keep payload values out of Debug output.
Signing tests compare a Rust signature against an independently computed Node.js HMAC-SHA256 vector.
They check bounded current/previous-key rotation and timestamp binding.
The exact vector is preserved in `delivery-signature-vector-2026-10-08.json`.

The independent consumer imports and uses the new public preparation API.
Source review found no actionable defect; its reviewer did not independently rerun tests.
`CARGO_BUILD_JOBS=2 ./scripts/check-all` passed with Rust 1.99.0 and both existing SQL fixtures.
Formatting, Clippy, workspace tests, doctests, documentation, no-default core, MSRV, and independent consumer gates passed.

Cargo-audit 0.22.2 reported zero vulnerabilities and warnings in 113 workspace and 209 consumer packages.
RustSec commit: `550efd3d587a29b2e2c2b21b17a440da4fede999`.
Manifest license inventories are recorded separately; release redistribution review remains open.
Executed source and lockfile hashes are in `delivery-source-2026-10-08.json`.

HTTPS transport, destination policy, independent HTTP receiver, durable intent reopen, authorization revocation, and provider retry/reconciliation remain required.
The full delivery goal is incomplete until those scenarios and real broker/notification profiles pass.
