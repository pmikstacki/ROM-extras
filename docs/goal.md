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
