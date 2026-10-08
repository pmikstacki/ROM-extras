# MySQL and MariaDB native execution verification

Base: `379dadfe7e565051c5030133a8ac9deb283c3ecd`. Uncommitted increment executed on 2026-10-08.
Compiler: Rust 1.99.0 (`b940084d7`). The Linux native build uses maintained external static OpenSSL 3.5.9.
No production executor code, public API or workspace lockfile changed.
The independent consumer adds a feature-gated native MySQL dependency and real-server test target.

## Reproduced profiles

MySQL 8.4.11 and MariaDB 11.8.9 each passed five tests under `scripts/check-mysql`.
Final native-backend output is preserved in `mysql-mariadb-native-gate-2026-10-08.log`.
The servers run separate pinned official images, labelled containers, loopback ports and persistent volumes.
Selected actual versions, settings, authentication plugins, bindings and volume mounts are in `mysql-mariadb-fixtures-2026-10-08.json`.
Both report InnoDB flush-at-commit 1, sync-binlog 1 and enabled binary logging.

- A Tokio caller uses a dedicated native executor connection and checks the selected server profile.
- Admitted transaction work writes a row, then blocks before commit. Caller observation returns Unknown.
- Releasing that same work completes native commit. A fresh connection observes value 41.
- A synthetic panic retires the connection. A fresh connection observes zero uncommitted rows.
- Duplicate-key error 1062 leaves the earlier row visible inside the transaction. Explicit rollback leaves zero rows.
- Five binary identities (`A`, `a`, UTF-8 `ż`, empty bytes and a zero byte) remain distinct.
- The acknowledged five-row transaction survives labelled server restart and finite connection readiness.

These are native executor experiments, not ROM Storage conformance.
Reconnection occurs in the same test process. No process-crash or power-loss guarantee is established.
Socket deadlines are per-I/O, not total transaction deadlines. Native job release and readiness have finite waits.

## Adoption and preserved failures

The initial build failed because the command omitted the available OpenSSL/pkg-config build environment.
A subsequent missing-profile run failed all four then-existing tests for required configuration absence.
The first profile gate passed backend tests but failed Clippy `let_and_return`; the redundant binding was removed.
The first full-gate invocation omitted the OpenSSL CLI path and refused to start. Its output is preserved.
The next full gate passed with the initial Rustls dependency selection.

That initial graph had 396 packages and an unmaintained `rustls-pemfile` warning, RUSTSEC-2025-0134.
Its license-file exception for webpki was inspected; the initial inventory and audit remain preserved.
The final `native-tls` graph has 403 packages, with no reported vulnerabilities or warnings.
All packages declare licenses. All 371 preceding package identities and checksums remain unchanged.
The scan used cargo-audit 0.22.2 and RustSec revision `550efd3d587a29b2e2c2b21b17a440da4fede999`.
Cargo audit does not cover the external OpenSSL installation or future advisory changes.
Its separate signed-source verification is documented in the earlier Kafka build evidence.

Source review found a restart-evidence gap: another loopback server could be queried while the labelled fixture restarted.
The final configuration requires its exact profile IP, port and database before connecting.
A wrong-IP invocation fails the intended endpoint assertion; output is preserved in `mysql-wrong-endpoint-2026-10-08.log`.
Review is source inspection, not an independent live reproduction.

## Remaining gates

The final full local verifier is recorded in `mysql-mariadb-native-full-2026-10-08.log`; terminal exit is 0.
All 87 Rust, manifest, lockfile and verifier source hashes match before and after execution.
The full gate includes workspace checks, all required real backends, broker TLS profiles and independent consumer checks.
Verified CA/name TLS, owner fencing, commit-ack loss, atomic ROM bundles, receipts and incremental Work remain pending.
The shared `SqlStore<D>` and packaged release are not implemented by this increment.
Public ROM main still points to `d7ef529040eec60dc869034c2d33130219db85fe`, without required incremental Work exports.
The whole ROM-extras goal remains active.
