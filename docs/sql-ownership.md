# Shared SQL ownership foundation

`rom-sql-core` now supplies ownership arbitration as well as the bounded connection executor.
It still does not implement ROM `Storage`.
The public ROM pin remains `d7ef529040eec60dc869034c2d33130219db85fe`.
Its required incremental Work preparation boundary remains unpublished.

## Native driver contract

Implement `OwnerTransaction` on an already-opened native transaction.
Lock the store's singleton control row before reading ownership.
Validate the immutable store identity and supported format under that lock.
Retain the lock through commit or confirmed rollback.
Every protected transaction and takeover must use this same arbitration row.
An unlocked check followed by writes is invalid.

The driver supplies bounded reads, connection and statement deadlines, and exact single-row update checks.
Normal open cannot provision, migrate or reset the database.
The driver must roll back when an unsuccessful transaction drops.
Map uncertain commit and rollback results to `OwnerError::Unknown`.
Do not include native messages, SQL, URLs or credentials in public errors.

The singleton contains a generation and optional binary token.
`OwnerState::new` accepts generations through `i64::MAX`; generation zero must be idle.
`OwnerToken::new` accepts a 32-byte token whose entire value is not zero.
Individual bytes may be zero.
The host supplies fresh unpredictable tokens. The core does not generate them or claim complete memory zeroization.
These generations are independent of ROM's full-u64 Resource revisions.

| Operation | Required state and result |
| --- | --- |
| `claim_owner` | Require idle state, advance generation and install the token. Busy state changes nothing. |
| `takeover_owner` | Require exact host-observed state, advance generation and install a new token. The host explicitly authorizes takeover. |
| `release_owner` | Require exact acknowledged ownership, clear token and retain generation. |
| `with_owner` | Verify exact ownership under the lock, execute prepared native writes and commit before returning their result. |

`with_owner` is a driver persistence boundary.
Do not call application actions, author codecs, external effects or network services inside its callback.
Prepare canonical data before taking locks when the public ROM preparation boundary becomes available.

## Takeover and uncertainty

The core does not infer owner death from a timeout or connection loss.
A crashed owner can leave a Busy control row. Recovery needs explicit host inspection and authorization.
An old transaction that already holds the row lock can commit before takeover acquires it.
Once takeover commits, surviving connections with the previous ownership must fail before protected writes.
A stale release cannot clear the new owner.

An unknown acquisition cannot return usable `Ownership`.
Inspect authoritative state before deciding whether to perform another transition.
No operation is retried automatically.
The executor's waiting deadline does not cancel admitted work or prove rollback.
This foundation creates no receipt ledger or historic replay guarantee.

Native ownership lifetime is not yet integrated with ROM's opaque `StorageOwner` guard.
That guard has no public native release callback at the current pin.
Do not infer automatic native release from Runtime shutdown or a dropped adapter handle.

## Executed qualification

Supply the protected loopback `ROM_EXTRAS_POSTGRES_DSN`, then run:

```sh
./scripts/check-sql-owner
```

The [independent consumer](../tests/sql-owner-consumer/src/main.rs) uses PostgreSQL18.6 and postgres0.19.14.
Its plaintext connection is restricted to explicit loopback IPs. This is not a qualified production TLS profile.
It creates uniquely named logged tables and retains them as evidence.
It checks `fsync=on` and `synchronous_commit=on` before qualification.
The same cases execute against the normalized `rom-sql-core` Cargo archive.
Owned qualification processes have a 30-second deadline; a failed second-worker setup must return within five seconds.
Readiness and release use bounded channels. The failure probe supplements the native race rather than qualifying a service failure.

Eight groups check competing claims, stale surviving connections, exact release and takeover, ordering, rollback, uncertainty, deadlines and invalid metadata.
The ordering case observes PostgreSQL's actual lock wait before allowing the old transaction to commit.
The statement-error case confirms rollback of previously staged writes.
The lock-timeout case confirms rollback and subsequent explicit use.
The executor-deadline case confirms that native work can commit after the caller's waiting timeout.
Another case suppresses acknowledgement after SDK commit success; this is a simulated acknowledgement fault over an actual native commit.
It is not evidence of a dropped wire response or receipt replay.
Reopened connections verify acknowledged state; this increment does not execute a PostgreSQL server restart.

Five pure transaction tests supplement native tests. They do not replace native qualification.
MSSQL, MySQL, MariaDB, CockroachDB and Oracle ownership mappings remain unimplemented and unqualified.
Canonical Resource bundles, journal publication, incremental Work and full Storage conformance remain required.
See [source research](research/sql-ownership-2026-10-09.md) and [the implementation plan](superpowers/plans/2026-10-09-sql-ownership.md).

Affected checks and the full local verifier passed on 1,227 unchanged source files.
See [the verification record](verification/sql-ownership-2026-10-09.json) for exact logs and evidence limits.
