# Projection checkpoint store

Inspection date: 2026-10-08. Evidence category: source review only.

This report recommends an extras-owned redb store for projection intent and checkpoint transactions. No runtime code, graph, service, or probe was changed. Native conformance remains required; source review does not complete this increment.

## Selection and dependency scope

The existing lockfile resolves `redb 4.3.0`, checksum `fb338a6c67830a61bed824b78c2bb034ab1ff401a20616bd7957524f4fdc2f22`. Its source declares Rust 1.90 and MIT/Apache-2.0. Default feature is `std`. Linux dependencies include libc. Experimental multi-process concurrency is optional and is not needed for this store. Pin the existing graph and standard exclusive writer mode. [Exact manifest](https://github.com/cberner/redb/blob/v4.3.0/Cargo.toml), [published API](https://docs.rs/redb/4.3.0/redb/).

Use a separate database file and extras-owned table names. Public ROM `Redb` and `Sqlite` adapters expose Storage operations, not arbitrary checkpoint transactions. Their native handles and tables are private. A Resource action is not the right substitute for a host-local transaction that atomically updates page progress, intent, and per-key revisions. [Published redb facade](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-redb/src/lib.rs), [SQLite handle boundary](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-sqlite/src/store.rs).

Direct SQLite is a viable later alternative. `rusqlite 0.40.2` is already resolved, so it need not add a new graph. It would require its own schema, native transaction, busy policy, and exact synchronous/journal settings. SQLite permits cooperating multi-process readers and writers through its lock protocol; it does not provide an application's exclusive projection-worker authority by itself. Prefer redb for the first narrow key-value checkpoint seam. [SQLite locking](https://sqlite.org/lockingv3.html), [SQLite synchronous settings](https://sqlite.org/pragma.html#pragma_synchronous).

No dependency audit was executed during this research. A web search without a matching advisory is not a no-advisory finding. Run the repository's exact graph/license/advisory/MSRV gates before adoption. Do not infer native filesystem safety or engine correctness from a Rust advisory scan.

## Ownership and lifetime

Redb's normal engine obtains an exclusive storage lock. Its Linux path uses open-file-description byte-range locks where supported, with whole-file lock fallback. Lock contention maps to `DatabaseAlreadyOpen`. An unsupported lock implementation can allow opening rather than rejecting it. Therefore, scope the initial profile to validated local Linux storage and reuse an explicit host ownership reservation. [Native lock acquisition](https://github.com/cberner/redb/blob/v4.3.0/src/tree_store/page_store/page_manager.rs#L717), [Linux range locks](https://github.com/cberner/redb/blob/v4.3.0/src/tree_store/page_store/file_backend/range_lock.rs#L384), [whole-file fallback](https://github.com/cberner/redb/blob/v4.3.0/src/tree_store/page_store/file_backend/optimized.rs#L234).

Locks apply to opened file identity, not just a path string. Existing symlink or hard-link aliases can refer to the same inode, but pathname replacement creates another identity. Canonical strings alone cannot prevent rename/replacement races. Ordinary advisory locks also do not stop noncooperating writers. Do not claim universal file exclusion.

Published ROM provides `rom_backup::NativeOwnership::acquire` and `NativeAccess`. Reuse this public primitive around the complete preflight/open/engine lifetime. It canonicalizes supported paths, keeps a persistent private `.rom-owner` sidecar, rejects hard-link deployments, and requires trusted parent directories. Its contract excludes live renames, replacement, and network filesystems. Never delete the sidecar to recover ownership. [Exact public ownership contract](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-backup/src/native_ownership.rs).

Declare the redb engine before the ownership guard so it closes first. A redb transaction can outlive its Database handle and keep its storage open. Do not expose native transactions or guards through the checkpoint API. Drain the bounded worker and drop every transaction/read guard before releasing ownership. Two processes and supported path aliases need actual exclusion tests. [Native transaction lifetime](https://github.com/cberner/redb/blob/v4.3.0/src/db.rs#L1754), [public ROM lifetime ordering](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-redb/src/store.rs).

## Durability and unknown outcomes

Only one redb write transaction proceeds at a time. `begin_write` can block while another write is active. Run it on a dedicated bounded worker; do not hold a transaction across HTTP calls or await network work. Each transaction reads its expected state, performs bounded validation and changes, then commits. [Exact write API](https://github.com/cberner/redb/blob/v4.3.0/src/db.rs#L1754).

Set `Durability::Immediate` explicitly for initialization, intent publication, checkpoint completion, and switch records. Redb documents persistence when commit returns successfully. None durability does not provide that acknowledgement boundary. Native file synchronization uses the engine backend's sync operations. Storage devices and filesystems must honor them. [Durability API](https://docs.rs/redb/4.3.0/redb/enum.Durability.html), [backend synchronization](https://github.com/cberner/redb/blob/v4.3.0/src/tree_store/page_store/file_backend/optimized.rs).

Redb 4.3.0 explicitly distinguishes poisoned transactions from other commit failures. `TransactionPoisoned` rolls back its transaction, though underlying corruption can remain. Other commit errors leave atomic changes either fully applied or absent, and those changes may already be durable. The engine then refuses further write transactions; closing and reopening repairs internal state. Do not translate these errors into confirmed rollback. [Exact commit contract](https://github.com/cberner/redb/blob/v4.3.0/src/transactions.rs#L1746), [commit error variants](https://github.com/cberner/redb/blob/v4.3.0/src/error.rs#L594).

Return fixed `Unknown` for an uncertain commit or panic during commit. Retire the store handle. Reopen under retained/reacquired ownership and inspect the durable transaction identifier and state. Do not repeat external effects based only on a local commit error. Ordinary uncommitted transaction drop can abort, but Drop does not report cleanup errors and has panic/storage-failure exceptions. Prefer explicit abort for recoverable validation failures. [Drop implementation](https://github.com/cberner/redb/blob/v4.3.0/src/transactions.rs#L2744).

A successfully synced file does not by itself prove a newly created directory entry survives power loss. Create fresh files with private permissions under a trusted directory. Sync the parent directory after initialization/publication before returning the initial store acknowledgement. Do not publish by replacing an active checkpoint file. This is a required host profile, not a claim about redb's automatic parent-directory handling. [Rust file synchronization](https://doc.rust-lang.org/std/fs/struct.File.html#method.sync_all).

## Format preflight without source writes

Use an application marker distinct from redb's native format. Include family format version, projection identity, mapping/model version, and configured limits. Missing markers in a nonempty store, unknown tables, mismatched types, and future versions are unsupported; they are not empty new stores.

A writable redb open can repair native recovery state before the caller reads its application marker. Thus “open writable, then reject future marker” does not satisfy unchanged-source rejection. Redb read-only opening returns `RepairAborted` when recovery is required. [Exact native open recovery](https://github.com/cberner/redb/blob/v4.3.0/src/tree_store/page_store/page_manager.rs#L863), [error categories](https://github.com/cberner/redb/blob/v4.3.0/src/error.rs#L292).

Hold public NativeOwnership during read-only preflight and subsequent writable open. Read and validate application metadata with bounded scans before creating tables or beginning a write. If the source needs native repair, initially fail closed with a specific repair-required category. This is the narrowest implementation that preserves the source. A later recovery path can inspect a bounded private copy, repair only that copy, validate it, then permit source recovery. Keep original bytes unchanged on rejected future or malformed formats.

Public ROM uses private-copy preflight for its own format, but those routines are private and cannot be called by extras. Reuse public ownership/staging primitives where suitable; implement only extras' distinct format validation. Do not copy private ROM tables or its production adapter. [Published format preflight](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-redb/src/preflight.rs).

Automatic source recovery after an unclean accepted-format close is a later explicit capability if the narrow fail-closed implementation is selected. Do not mark crash/reopen acceptance complete until that required recovery path actually exists and passes native tests.

## Proposed transaction seam

Keep storage operations narrow; do not expose a generic closure capable of arbitrary table writes. Proposed methods are `load`, `prepare_page`, `complete_page`, `prepare_switch`, `complete_switch`, and bounded recovery inspection. Names require the final family spec.

| Record | Required bounded content |
| --- | --- |
| Store metadata | Exact format, projection/profile identity, mapping/model identity, limits. |
| Projection state | Physical generation, ROM generation/kind cursor, phase, committed transaction identifier. |
| Page intent | Unique operation identity, expected old cursor, intended new cursor, bounded operation identities/digests. |
| Per-key state | Stable Resource key, acknowledged revision, tombstone state, deterministic mapping digest. |
| Switch intent | Old/new physical targets, expected checkpoints, phase, immutable switch identity. |

Use redb tables with typed byte/string keys and bounded byte values. Check raw byte length before deserialization or copying into owned buffers. Reject oversized input before opening a write. Bound number of operations, distinct keys, aggregate encoded bytes, startup records, and scan continuation. Redb's engine key/value maxima are not application limits. Keep cache size explicit and avoid unbounded `collect` over table ranges. [Native table insertion bounds](https://github.com/cberner/redb/blob/v4.3.0/src/table.rs#L324), [builder and read API](https://docs.rs/redb/4.3.0/redb/struct.Builder.html).

`prepare_page` atomically checks the expected cursor and absence of a conflicting intent, then publishes the intent. Commit it before network dispatch. Repeating the identical intent is idempotent; a different intent under the same identity is a conflict. Store no credentials. Prefer metadata/digests instead of plaintext Resource payload in intents. If replay requires source values, refetch the retained journal with its exact cursor and mapping identity.

`complete_page` requires provider-specific acknowledged or reconciled outcomes for every operation. In one Immediate transaction, compare the exact pending intent, update per-key acknowledged revisions monotonically, advance the page cursor, and remove/finish the intent. Filtered journal positions can advance without a backend document. Reject mismatched generations, backwards cursors, and equal-revision differing digests. Preserve completed transaction identity for unknown-commit reconciliation.

The local transaction cannot prove a supplied network acknowledgement is authentic or current. Keep transport outcome construction private to the worker where practical. Qdrant needs its native conditional revision fence; OpenSearch needs persistent tombstones or equivalent fencing. The local ledger cannot stop an already forwarded stale HTTP request. [Qdrant fencing assessment](qdrant-revision-fencing.md), [provider assessment](projections-assessment.md).

`prepare_switch` persists intent before changing a provider alias. `complete_switch` records the actual confirmed target and generation checkpoint atomically. Local store commit and provider alias change remain separate commits. On restart, inspect actual alias state and reconcile the pending switch. Do not claim an atomic transaction spanning redb and either provider.

## Recovery and acceptance

After recovery, resume pending pages before admitting conflicting generations. If retained journal data cannot reconstruct a pending operation, report HistoryGap and rebuild; do not invent success. If the backend was restored behind its recorded checkpoint, invalidate continuity and rebuild/reconcile explicitly. A local checkpoint alone cannot prove remote persistence.

Required real store evidence includes private fresh-file creation, full close/reopen, cross-process exclusion, supported path aliases, rejected hard links, invalid/future markers with unchanged source bytes, and bounded oversized/corrupt record rejection. Exercise competing expected-cursor updates and verify one winner.

Exercise process interruption before intent commit, after intent before backend dispatch, after backend success before completion, and after completion commit before caller acknowledgement. Verify atomic checkpoint/per-key/intent state on reopen. Inject native commit I/O faults using a controlled backend where feasible; distinguish that evidence from real filesystem crash/restart. Verify Unknown retirement and transaction-identity reconciliation.

Test deliberate format rejection before source repair, and separately qualify accepted-format dirty recovery. Test page and switch reconciliation against the already qualified real OpenSearch/Qdrant profiles. Run packaged consumers and the full local verifier after implementation. No source-only or mock result replaces these gates.
