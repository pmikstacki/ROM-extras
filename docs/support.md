# Support and verification status

Date: 2026-10-08.
No database or external-service provider is supported yet.

The SQL connection executor is implemented and has no external dependencies. It does not implement ROM Storage yet.
Its 13 tests cover bounded admission, connection affinity, timeout uncertainty, panic retirement, shutdown, and handle lifecycle.
Three executor integration tests passed against PostgreSQL 18.6. These are not ROM Storage conformance tests.
An acknowledged control row survived a server restart. See [the fixture](postgres-fixture.md) and [verification evidence](verification/executor-2026-10-08.md).
The proposed compatibility baseline is public ROM commit `d7ef529040eec60dc869034c2d33130219db85fe`.
Local ROM commit `1a2b7934139f3d7bb4cb04aaf4a90bd9405ab470` is unavailable through the GitHub commit API.
The published baseline does not export incremental Work support or the new diagnostic reader.
SQL Storage and telemetry integration require a later public commit; complete-ledger serialization is prohibited as a substitute.

The workspace declares Rust 1.99 to match the published ROM baseline.
The independent consumer pins postgres 0.19.14 and Tokio 1.53.1 behind its PostgreSQL fixture feature.
Cargo-audit 0.22.2 found no advisories or warnings across its 104 locked packages on 2026-10-08.
This result uses RustSec commit `550efd3d587a29b2e2c2b21b17a440da4fede999` and does not establish future advisory status.
The dependency manifest license inventory is preserved with verification evidence; redistribution review remains required before release.
Oracle requires a separate native-client and redistribution review.
