# CockroachDB connection executor assessment

Evidence category: primary-source review, 2026-10-08. No image was pulled and no CockroachDB server or conformance test was executed by this agent.

## Scope and source baseline

Approved plan Task 4 includes CockroachDB transaction retry, recovery, representations, and ownership fencing. The current increment can qualify the existing public `rom_sql_core::Executor<postgres::Client>` against an actual server. It cannot establish a complete ROM Storage adapter or satisfy the ownership-fence gate.

The inspected consumer lockfile resolves `postgres` 0.19.14 and `tokio-postgres` 0.7.18. The existing PostgreSQL test constructs the synchronous client inside the executor worker. It exercises Tokio caller compatibility, waiting timeout followed by confirmed commit, and panic rollback. It does not implement Cockroach-specific retry policy. Local sources: `tests/public-consumer/tests/postgres_executor.rs`, `tests/public-consumer/Cargo.lock`, `crates/rom-sql-core/src/ticket.rs`.

## Current server candidate

The official release page lists v26.3.2, released 2026-09-18, with image `cockroachdb/cockroach:v26.3.2`. v26.3 is an Innovation release; v26.4 is upcoming. This review did not verify a registry digest or executable identity. The public GitHub tag endpoint for v26.3.2 returned 404, so no matching source commit is claimed. Record binary build metadata after a separately authorized artifact inspection. [Release notes](https://www.cockroachlabs.com/docs/releases/v26.3#v26-3-2), [release overview](https://www.cockroachlabs.com/docs/releases/)

CockroachDB supports PostgreSQL's wire protocol and most PostgreSQL drivers, with documented SQL differences. This supports a reuse hypothesis, not verified compatibility with these exact Rust driver versions. Do not reuse PostgreSQL-only `fsync` or `synchronous_commit` assertions. Probe actual server version, transaction isolation, supported startup options, parameter encoding, and result types. [v26.3 PostgreSQL compatibility](https://www.cockroachlabs.com/docs/v26.3/postgresql-compatibility)

## Transaction outcomes

Use explicit `SERIALIZABLE` transactions for the profile. A `40001` retry signal requires restarting the complete database unit of work, including reads that determined writes. Bound attempts, elapsed time, and backoff. Do not replay external effects inside this loop. Keep a typed driver error distinct from an executor waiting timeout. [Retry reference](https://www.cockroachlabs.com/docs/v26.3/transaction-retry-error-reference), [BEGIN](https://www.cockroachlabs.com/docs/v26.2/begin-transaction)

CockroachDB reports ambiguous outcomes with SQLSTATE `40003`. Network failure, node failure, or timeout can leave commit status unknown. Do not retry an ambiguous non-idempotent transaction blindly. Reconcile through a stable application operation identifier using a fresh connection. This executor profile must not claim durable ROM receipts it does not implement. [Ambiguous errors](https://www.cockroachlabs.com/docs/v26.2/common-errors#result-is-ambiguous)

The synchronous Rust transaction wrapper consumes its underlying transaction before awaiting COMMIT. Its Drop path attempts rollback only while the underlying transaction remains present, and ignores rollback errors. Explicit `rollback()` reports errors. Thus Drop is not proof of confirmed rollback, and a failed COMMIT is not automatically repaired by Drop. [postgres 0.19.14 transaction source](https://github.com/sfackler/rust-postgres/blob/postgres-v0.19.14/postgres/src/transaction.rs)

Driver cancellation is inherently racy and the server gives no confirmation that cancellation succeeded. An executor ticket timeout does not cancel admitted work; the same ticket can later return its terminal result. Keep cancellation, waiting timeout, serialization rejection, and ambiguous commit as separate scenarios. [tokio-postgres cancellation](https://github.com/sfackler/rust-postgres/blob/tokio-postgres-v0.7.18/tokio-postgres/src/cancel_token.rs)

Treat duplicate SQLSTATE `23505` as an application constraint outcome, not generic `40001` retry. Test the actual failed explicit transaction before recovery. Require explicit rollback, verify that preceding writes did not survive, and prove a fresh transaction succeeds. Do not assume COMMIT success after an earlier error means the writes committed. Exact v26.3.2 abort-state behavior remains a live gate; code mappings from v26.2.7 are only supporting source evidence. [Versioned SQLSTATE definitions](https://github.com/cockroachdb/cockroach/blob/v26.2.7/pkg/sql/pgwire/pgcode/codes.go)

## Proposed local fixture

Use `start-single-node` with a disk-backed persistent `--store`, never an in-memory store. Publish only the SQL port on literal loopback. Keep the HTTP administration port unpublished. An insecure profile is restricted to this isolated development fixture. Validate exact v26.3.2 help before selecting flags.

The official command disables replication and is intended for development, not production or performance testing. Repeat the same startup flags on restart. Default cache is 256 MiB; local SQL memory can be reduced to 128 MiB. Cache, SQL memory, and TSDB memory together should stay below 75% of available memory. [v26.3 single-node command](https://www.cockroachlabs.com/docs/v26.3/cockroach-start-single-node)

This review did not find a definitive official minimum CPU/RAM for a tiny single-node fixture. A proposed two-CPU, 2 GiB profile is an experiment, not a vendor-certified minimum. Production guidance recommends substantially larger nodes; do not present a 512 MiB limit as supported. [Official sizing discussion](https://www.cockroachlabs.com/blog/unlocking-scale-cockroachdb-azure-part-4/)

Gracefully stop and restart the same persistent store, then reconnect with a new executor. Verify previously committed rows and absent rolled-back rows. This establishes only the executed restart profile. It does not establish quorum availability, power-loss durability, failover, or replicated ownership fencing.

## Required executor scenarios

1. Execute a typed BIGINT parameter/read inside a current-thread Tokio caller through the worker-owned synchronous client.
2. Start a transaction, insert, pause before commit, time out the ticket, release it, and confirm commit through the same ticket and a fresh connection.
3. Panic before commit, require connection retirement, and verify no inserted row survives.
4. Trigger a duplicate constraint after a preceding insert; explicitly rollback and verify atomic absence plus fresh transaction usability.
5. Use two independent worker connections to create controlled SERIALIZABLE contention. Require actual `40001`, then bounded complete-transaction retry with a fresh read.
6. Verify committed rows after a real server restart with the same disk store.

The retry test must fail when it never observes the intended retry condition. Ordinary contention is not proof that the client handled `40001`. An acknowledgement-loss test, if added later, must retain unknown outcomes and reconcile independently; a synthetic wait timeout is not lost wire acknowledgement.

## Adoption and unfinished gates

Current binaries use the CockroachDB Software License. Official licensing FAQs exempt internal non-production development using `start-single-node` from license keys and throttling. This exemption does not authorize general production deployment or redistribution. Free enterprise eligibility and telemetry obligations require a separate deployment-specific review. [Licensing FAQs](https://www.cockroachlabs.com/docs/stable/licensing-faqs), [release licensing](https://www.cockroachlabs.com/docs/releases/)

No PostgreSQL advisory-lock equivalence is assumed. Full Storage support still needs an independently proven ownership fence, state/event/receipt atomicity, representation and collation checks, bounded I/O, interruption reconciliation, migration rejection, packaged-consumer evidence, license review, and image/driver advisory assessment. Existing driver reuse does not waive these gates.
