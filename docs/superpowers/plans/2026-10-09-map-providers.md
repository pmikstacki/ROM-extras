# Map providers implementation plan

> **For agentic workers:** Use superpowers:executing-plans task-by-task. User authorization includes execution; preserve previous work and evidence.

**Goal:** Deliver independent, validated map service capabilities with explicit host/browser boundaries.
**Architecture:** rom-map-core defines spatial contracts and ports. Adapter crates normalize actual provider protocols; the host approves disclosure.
**Tech Stack:** Rust 1.99/2024, existing pinned public ROM, existing serde/reqwest/Tokio versions; frontend examples use pnpm.
**Spec:** [Generic map providers](../specs/2026-10-09-map-providers.md).

## Constraints and ownership

The spec bounds and user scope apply to every task. Root owns implementation and examples.
Research agent owns only docs/research/map-providers-2026-10-09.md; research assignment finished before shared edits.
Existing Qdrant/shared-HTTP working changes remain intact. Map ports do not extend SQL or projection contracts.
Follow research for each decision and distinguish protocol fixtures from real provider qualification.

### Task 1: Typed map core

Files: crates/rom-map-core/src/{lib,error,coordinates,units,geocoding,routing,capabilities,context,source}.rs and tests.
- [x] Write and execute failing boundary tests for coordinate order, poles, antimeridian boxes, units and geometry.
- [x] Implement immutable validated spatial values, provenance/accuracy, bounded query/results and exact ROM keys.
- [x] Add optional capability discovery and asynchronous geocoding/reverse/routing contracts with cancellation.
- [x] Verify core alone, public consumer and sanitized diagnostics.

- [x] Add core-owned style/tile contracts and separately selectable styles, raster and vector capabilities.
- [x] Preserve approved origins, native identifiers and attribution; reject incompatible source kinds and unsupported style behavior.

Task 1 has local contract and independent custom-provider evidence. It does not qualify map services.

### Task 2: Safe transport and configured source

Files: crates/rom-map-http/, crates/rom-map-source/, controlled fixtures.
- [ ] Define map-specific fixed endpoint, cancellation, 429/rate and cache policy without weakening projection transport.
- [ ] Test response bounds, timeout, in-flight cancellation, redirects and secret captures before implementation.
- [ ] Implement verified bounded requests and explicit browser source approval with transitive URL checks.
- [ ] Qualify self-hosted TileJSON and MapLibre style protocol fixtures; preserve attribution.

### Task 3: Native API adapters

Files: crates/rom-nominatim/, crates/rom-osrm/, crates/rom-maptiler/.
- [ ] Implement and test search/reverse, route GeoJSON and MapTiler protocols against controlled endpoints.
- [ ] Preserve provenance/accuracy/units, empty results and provider-specific rate/cache/usage rules.
- [ ] Require explicit host endpoints and browser-token approval; never expose backend credentials.
- [ ] Record actual service qualification separately; do not claim production support from fixtures.

### Task 4: Integration and acceptance

Files: examples/maps/, tests/public-consumer/, scripts/check-maps, packaging and docs.
- [ ] Implement a ROM host example that authorizes reads and sends approved data into rom-ui/maps.
- [ ] Use pnpm and verify actual UI exports when available; report any absent export as an integration gap.
- [ ] Run independent public and extracted archive consumers and applicable native-service cases.
- [ ] Freeze sources, run audits/review/affected checks and the full local verifier before integration.
- [ ] Publish exact evidence, working examples and remaining limitations; keep the whole goal active.

## Review focus

Reject coordinate swapping, fabricated accuracy, unknown units, browser credential leakage and transitive external URLs.
An absent capability must return Unsupported, not invoke unrelated ports.
A cancelled request must release bounded admission without continuing hidden retries.
