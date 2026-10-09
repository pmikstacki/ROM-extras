# ROM-extras goal and decision policy

The user approved the implementation plan on 2026-10-08.
Execute it in the current chat. The full goal remains incomplete until implemented extensions pass their required verification.

## Complete scope

- Shared SQL persistence core.
- PostgreSQL, Microsoft SQL Server, MySQL, MariaDB, CockroachDB, and Oracle adapters.
- OpenTelemetry, webhook delivery, and Kafka, NATS, and RabbitMQ integration.
- S3-compatible and Azure Blob extensions.
- OIDC presets, OpenSearch and vector projections, secrets/KMS, and notification connectors.
- Backup, migration, and provenance-preserving import tools.
- Documentation, examples, public consumer compatibility, and real backend conformance.

## User-required decision policy

Every project and technology decision must use web research.
Prefer official documentation, source repositories, specifications, and first-party APIs.
Record source URLs, inspection date, the selected approach, alternatives, and unresolved uncertainty.
Confirm behavior with local tests. Web research alone does not establish implementation correctness.

Keep this policy when resuming or delegating work.
The app's goal API cannot edit an existing objective; this file records the user's amendment without replacing the unfinished goal.

## Completion standard

A skeleton, source review, mock-only test, or proposed design is not a completed integration.
Use shared real-database conformance for each database provider.
Report missing backend access and unverified capabilities explicitly.
Preserve the full scope when a particular integration is unavailable.

## User-approved map-provider extension — 2026-10-09

The active goal also includes generic map providers. Preserve existing APIs, working changes, and test evidence.
Implement separate style/raster/vector tile, geocoding/reverse-geocoding, and routing capabilities in rom-map-core.
Implement host-configured self-hosted TileJSON/MapLibre styles, Nominatim, OSRM, and MapTiler adapters.
Use typed validated longitude/latitude coordinates, bounding boxes, geocoding accuracy/provenance, route geometry, metres and seconds.
Preserve exact public ROM Resource keys. Keep authorization, Resource selection, writes, sessions, and browser approval in the host.
Separate backend credentials from browser descriptors. Browser tokens require an explicit host contract.
Preserve attribution and provider-specific license, usage, rate, retry and cache policies.
No default production demo server, automatic geolocation, or automatic external connection is permitted.
Reuse HTTP transport only where its guarantees fit. Do not weaken projection transport or add mapping operations to SQL/projection ports.
Provide a pnpm frontend example with rom-ui/maps. Keep Svelte and MapLibre outside Rust domain core.
Qualify coordinate/geometry boundaries, empty results, response limits, 429, timeout, cancellation, missing capabilities, and secret sanitization.
Run independent public and packaged consumers plus controlled protocol fixtures. Distinguish fixtures from actual service qualification.
Run affected checks and the full local verifier before integration. Report remaining work; compiling stubs are not acceptance.
The existing goal API cannot edit its objective. This amendment records the expanded objective without replacing its unfinished scope.
