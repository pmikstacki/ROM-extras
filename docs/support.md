# Support and verification status

Date: 2026-10-08.
No database or external-service provider is supported yet.

The initial SQL executor has no external dependencies. It does not implement ROM Storage yet.
The proposed compatibility baseline is public ROM commit `d7ef529040eec60dc869034c2d33130219db85fe`.
Local ROM commit `1a2b7934139f3d7bb4cb04aaf4a90bd9405ab470` is unavailable through the GitHub commit API.
The published baseline does not export incremental Work support or the new diagnostic reader.
SQL Storage and telemetry integration require a later public commit; complete-ledger serialization is prohibited as a substitute.

The workspace declares Rust 1.99 to match the published ROM baseline.
Advisory and license checks become executable when external dependencies are adopted.
Oracle requires a separate native-client and redistribution review.
