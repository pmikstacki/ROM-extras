# Oracle executor verification

Date: 2026-10-08. Baseline: 5f4ff81d2cdb465020431229ffb70441ecc3a8fc.
The source changes are the optional consumer driver, seven integration cases, fixture lifecycle helper, and required verifier gate.
Production SQL executor APIs remain unchanged. [Runtime input hashes](oracle-source-inputs-2026-10-08.json) identify the executed sources.

The real Oracle server and Instant Client were 23.26.3.0.0. rust-oracle was pinned to 0.6.3, with odpic-sys 0.1.1.
See [fixture configuration](oracle-fixture-2026-10-08.json) and [dependency delta](oracle-dependencies-2026-10-08.json).
The independent consumer changed from 440 to 450 locked packages; all previous identities and checksums remain unchanged.
The root workspace lockfile remains unchanged. The RustSec audit reported zero vulnerabilities; it does not assess native OCI security.

## Executed results

| Command or check | Result | Evidence |
| --- | --- | --- |
| Six initial Oracle cases | Exit 0; six passed | [raw log](oracle-six-cases-2026-10-08.log) |
| Seven cases with initial restart assertion | Exit 101; one passed, six failed | [preserved failure](oracle-seven-cases-2026-10-08.log) |
| `./scripts/check-oracle` | Exit 0; seven passed and Clippy passed | [gate log](oracle-graceful-gate-2026-10-08.log) |
| First full-verifier attempt | Exit 2; missing OpenSSL executable in PATH | [environment failure](oracle-full-verifier-2026-10-08.log) |
| Configured `./scripts/check-all` | Exit 0; all configured gates passed | [full log](oracle-configured-full-verifier-2026-10-08.log) |
| Frozen runtime input comparison | All 120 hashes unchanged | [source record](oracle-source-inputs-2026-10-08.json) |
| Client archive integrity | Published size and SHA-256 match; all extracted bytes and symlink targets match | [artifact facts](oracle-fixture-2026-10-08.json) |

The initial restart failure was an incorrect test assumption: the pinned shell returns 143 after its SIGTERM shutdown handler.
Fresh logs confirmed `ORACLE instance shut down.`; the container was not forcibly killed.
The failed restart poisoned the test mutex, causing the five subsequent cases to fail before execution.
The corrected assertion requires both exact exited 143 and fresh shutdown confirmation, followed by bounded PDB readiness.
The same persistent container and volume were restarted and reused.

Initial build failures also remain preserved: [missing pkg-config](oracle-initial-test-2026-10-08.log),
[incorrect factory error type](oracle-configured-build-2026-10-08.log), and [client symlink extraction failure](oracle-worker-smoke-2026-10-08.log).
Restoring original ZIP symlink metadata produced the [passing OCI smoke test](oracle-worker-symlink-smoke-2026-10-08.log).
No Oracle ELF bytes or original notices were changed.

## Scope and review

Independent source review inspected the connection configuration, queue assertions, transaction boundaries, restart helper, and documentation.
It found no actionable defect. The reviewer did not run the backend cases; those results are local execution evidence.
Seven passing cases do not establish ROM Storage conformance, Transaction Guard, TLS, replication, or ambiguous commit recovery.
The whole ROM-extras goal remains active.
