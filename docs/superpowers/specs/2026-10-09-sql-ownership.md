# Shared SQL ownership transaction foundation

Date: 2026-10-09. Part of the approved SQL-core plan; not full Storage acceptance.

## Outcome and authority

Implement portable ownership transitions in `rom-sql-core` and execute them against native PostgreSQL transactions.
Preserve existing executor APIs and all fixtures. Public ROM remains `d7ef529040eec60dc869034c2d33130219db85fe`.
Incremental Work preparation remains unpublished. Do not copy private policy or serialize a complete Work ledger.

## Contract

A native singleton control row contains a monotonic generation and an optional exact binary owner token.
Generation zero is valid only for a never-claimed idle row. Supported generations stop at `i64::MAX` without wrapping.
Owner tokens contain exactly 32 bytes and cannot be all zero. The trusted host supplies unpredictable fresh tokens.
Their Debug output is redacted. The core performs no randomness, credential discovery, SQL or network I/O.

`OwnerTransaction` locks and reads the control row, writes its new state, and commits or rolls back explicitly.
Its driver must retain the exclusive lock through commit and roll back on Drop after any unsuccessful operation.
Driver errors are finite. Failure to establish commit or rollback remains Unknown.
Transaction initialization, statement and lock deadlines remain driver responsibilities.

`claim_owner` changes an idle row to a new generation and token.
`takeover_owner` requires the exact host-observed previous state and advances the generation under the same lock.
Takeover is an explicitly authorized administrative operation, never automatic timeout-based liveness inference.
`release_owner` requires the exact current ownership and clears its token without resetting generation.
`with_owner` verifies ownership under the row lock before invoking native writes and commits them before returning their value.
The callback contains driver persistence work, never application actions, external effects or author-defined codecs.

A stale token or generation must fail before protected writes.
The same row lock orders owner changes and protected transactions, including work on a surviving separate connection.
An in-flight old transaction can commit before takeover obtains that lock. Takeover is established only after its commit.
No atomic release callback is added to ROM's public `StorageOwner`; future adapter integration must solve that lifetime separately.

## Errors and recovery

Use Invalid, Busy, Stale, Exhausted, Unavailable and Unknown distinctions.
No automatic retry is permitted. A caller timeout after admission does not establish rollback.
If a claim or takeover commit is unknown, do not return usable ownership.
The host must inspect authoritative state or authorize a subsequent exact-state transition.
A failed protected operation must roll back all staged native writes.

## Qualification

Pure transaction fixtures test idle/busy transitions, exact state, token/generation bounds, rollback and unknown commit.
Native PostgreSQL tests use dedicated retained tables and separate connections.
They test competing claims, held row locks, post-takeover stale writers, stale release, rollback and generation exhaustion.
They test a transaction held before takeover and the ordered final state after release.
They simulate loss of the committed acknowledgement and inspect the actual persisted state without claiming provider receipts.
An independent consumer and the same consumer from the normalized archive must pass.
Run affected checks and the full verifier before integration.

These tests qualify the ownership foundation only. Native Runtime ownership, canonical bundle writes, journal/Work and all six full Storage profiles remain required.

## Decision sources

PostgreSQL row locks are retained until transaction end and order competing `SELECT FOR UPDATE` operations.
Session advisory locks alone do not protect a pooled writer after takeover.
Select the persisted-row protocol for this foundation; optional native session ownership can be composed later.
See [PostgreSQL18 locks](https://www.postgresql.org/docs/18/explicit-locking.html),
[transaction isolation](https://www.postgresql.org/docs/18/transaction-iso.html), and the accompanying source research.

