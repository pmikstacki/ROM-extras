# Shared native maintenance tools implementation plan

> For agentic workers: use superpowers:executing-plans. Root owns maintained files; one final reviewer owns a named ignored note.

**Goal:** Deliver working backup/inspection/restore/typed-migration tools over native public ROM contracts while retaining full SQL maintenance scope.
**Architecture:** Sealed native backup sources; shared validated archive/report core; optional provider dispatch and count-only operator CLI.
**Tech stack:** Exact existing ROM0.0.3 pin, Rust1.99; existing serde_json/Tokio; no new registry package selection.
**Spec:** ../specs/2026-10-10-maintenance-native-tools.md

## Global constraints

Preserve public ROMd7ef529040eec60dc869034c2d33130219db85fe, APIs and every history/evidence artifact.
Native archives remain SQLite/redb only; no external SQL disguise or complete-backup claim.
No library unsafe, duplicate archive/ownership/publication protocol, implicit retries or production fault callbacks.
Host owns authorization, callback purity and deployment cutover.

## Review focus

- Backup must not create a missing source or replace an archive; accept native handles only.
- Preview must not publish files or certify a later changed archive.
- Unknown publication stays Unknown; count-only outputs must omit private diagnostics.
- Restore claim/cursor fencing does not stop the old deployment.
- Feature refusal must occur before touching a nonexistent path.

### Task1: Native facade and operator boundary

Files: crates/rom-extras-maintenance/{Cargo.toml,src/lib.rs,src/error.rs,src/limits.rs,src/report.rs,src/archive.rs,src/native.rs,src/backup.rs,src/bin/rom-extras-maintenance.rs,src/cli.rs}, tests/maintenance-public-consumer/.

- [x] Observe missing-public-API consumer RED.
- [x] Implement bounded limits, safe error mapping, full archive inspection, pure migration preview and feature dispatch.
- [x] Add sealed native backup adapters and count-only CLI inspect/restore.
- [x] Execute real SQLite/redb backup/restore/migration/reopen and failure boundaries through public APIs.
- [x] Execute live-backup coherence, replay/attribution, cursor/claim fencing and converter/Work negatives.

### Task2: Independent packages and integration

Files: scripts/check-maintenance, scripts/prepare-maintenance-package.mjs, docs/maintenance.md, docs/support.md, README.md, scripts/check-all, docs/verification/maintenance-native-tools-2026-10-10.json.

- [x] Verify no-native feature core and consumer plus source/archive native consumers, CLI and upstream interruption fixture.
- [x] Audit exact graphs, licenses/checksums/compiler; retain existing paste warning without suppression.
- [x] Run final source review; fix substantive findings with observed regression proof.
- [x] Freeze source and execute affected checks/full local verifier.
- [x] Record actual native/CLI evidence and remaining SQL/external-object/uncertainty/cutover gaps, then publish verified increment.
