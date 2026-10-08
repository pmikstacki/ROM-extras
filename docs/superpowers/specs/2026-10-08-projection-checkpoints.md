# Projection checkpoint transaction contract

Status: checkpoint-store increment implemented; worker and full family acceptance remain pending.
Inspection date: 2026-10-08. This contract supplements [authorized projections](2026-10-08-projections.md).

## Research basis

Use an extras-owned redb 4.3.0 file, not private ROM storage tables.
The [checkpoint research](../../research/projection-checkpoint-store.md) records alternatives, versions, ownership limits, and required native acceptance.
The [exact commit source](https://github.com/cberner/redb/blob/v4.3.0/src/transactions.rs#L1746) requires reconciliation after an uncertain commit.
The [public ROM ownership contract](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-backup/src/native_ownership.rs) supplies process exclusion.

## Public seam

`CheckpointStore` owns its native engine and ownership reservation. It exposes no native transactions, closures, or tables.
Its synchronous methods run on the worker's bounded storage thread. No transaction spans network work.
Each store contains one projection deployment. Multiple deployments use separate files.

| Method | Contract |
| --- | --- |
| `create(path, profile, initial)` | Require an absent file; create private storage; commit metadata/state; synchronize the parent before acknowledgement. |
| `open(path, profile)` | Require an existing file; validate application format before original-file repair; retain ownership through engine closure. |
| `load()` | Return the bounded current state and optional intent. Never interpret a missing marker as a new store. |
| `key_state(key)` | Read one acknowledged revision, tombstone flag, and mapping digest in the current physical generation. |
| `prepare_page(page)` | Compare the expected complete state; persist an immutable intent before dispatch; return its durable transaction identifier. |
| `complete_page(page, observed)` | Require exact pending intent and all reconciled states; atomically publish key states, cursor, and completed transaction identity. |
| `prepare_switch(switch)` | Persist old/new target, checkpoints, and immutable switch identity before alias mutation. |
| `complete_switch(switch, observed_target)` | Require actual alias reconciliation; atomically publish the target and new checkpoint. |
| `reconcile(transaction)` | After reopen, classify the last attempted commit as applied, absent, or inconsistent from durable records. |

These names define planned public Rust operations, not existing methods.
Switch support remains Task 6. Page implementation must reserve a distinct intent variant and reject conflicting switch work.

## Identity and bounds

The profile binds format version, deployment identity, provider, mapping version, optional vector model, and immutable limits.
Changing mapping, model, or disclosure policy requires a new physical generation; it is not an in-place profile overwrite.
Resource identity contains the exact public ROM `Key.kind` and `Key.id`.
Use length-delimited encoding for storage and backend identity derivation. Concatenation with an ambiguous separator is prohibited.
Never expose raw Resource values or credentials in checkpoint diagnostics.

Initial bounds are application policy, not redb or provider maxima:

| Item | Maximum |
| --- | --- |
| Operations and distinct keys per page | 64 |
| Resource kinds per deployment | 64 |
| Resource kind, journal generation, deployment, mapping/model, physical target identifiers | 128 UTF-8 bytes each |
| Resource ID | 1024 UTF-8 bytes |
| Encoded intent or state record | 128 KiB |
| Encoded per-key record | 2 KiB |
| Startup records inspected per bounded segment | 256 |
| Native cache | 8 MiB |
| Original file admitted for automatic private-copy recovery | 256 MiB |

Check lengths before deserialization, copying, or transaction creation. Check integer arithmetic for overflow.
Records outside the declared recovery-file bound require an explicit operator recovery route; never truncate or silently initialize them.
Bound each segment of a startup scan and use continuation. Do not collect the full key table in memory.
Scan all admitted records before worker activation; validate record limits, key identity, revisions, and intent/state consistency.
The worker can cancel between segments. The file-size bound also limits initial recovery I/O.

## Page preparation and completion

A page binds physical generation, expected cursor, returned cursor, mapping/model profile, and ordered operation fingerprints.
Every cursor includes ROM generation, kind, and position. Positions use the full u64 domain.
Require matching cursor generation and kind, nondecreasing position, and operation positions within the inspected interval.
An unchanged cursor is only an empty no-op. Do not create a second receipt for polling the same head.
A filtered page can advance its cursor with zero operations.
Collapse repeated keys to their highest accepted revision before creating the intent, while validating the complete input page.
Equal revisions require equal canonical content digest and tombstone state. Reject zero or decreasing revisions.

Preparation uses compare-and-set on the complete expected state, not only a numeric position.
Only one intent is pending. An identical repeated intent is idempotent; a conflicting intent fails without dispatch.
Derive the immutable intent fingerprint from the complete bounded canonical encoding, including expected and resulting cursor.
The transaction sequence uses checked u64 addition and never wraps.
Retain the last completed intent fingerprint and transaction sequence so immediate response loss can be reconciled.
Older pages fail cursor comparison; the API does not promise unbounded historical receipt retention.

The intent stores identities, revisions, tombstone flags, and canonical mapping digests, not plaintext projected values.
Recovery refetches retained authorized history using the exact cursor and immutable mapping profile.
A missing page or changed mapping/disclosure result cannot reproduce the intent. Return `HistoryGap` or `RebuildRequired` without cursor advancement.

Before completion, retrieve provider state and compare original key, physical generation, profile, revision, digest, and tombstone state.
Require an exact match for every intended operation. A successful HTTP response or native no-op is insufficient.
If remote state has an unexpected higher revision, report divergence and reconcile/rebuild; do not invent a successful exact acknowledgement.
One worker does not admit newer pages while an older page has an unresolved intent.
The Qdrant conditional fence and OpenSearch external versions still protect against already submitted stale requests.

Commit per-key acknowledgements, resulting cursor, intent completion, and last transaction identifier together with explicit Immediate durability.
Validation rejection aborts explicitly and produces no partial key state or cursor change.
Equal per-key revision with a different digest is a conflict, including after restart.

## Commit uncertainty and ownership

Commit success is a durable acknowledgement under the selected local filesystem profile.
A non-poisoned native commit error or unwinding panic during commit has an unknown outcome.
Return a fixed sanitized `Unknown` category, close the engine, and retire the handle while retaining its ownership reservation.
Do not admit another transaction through that handle. Reopen and inspect the exact attempted transaction and resulting records.
Classify applied versus absent only when the retained state proves it. Conflicting records return corruption, not retry permission.
`TransactionPoisoned` establishes transaction rollback; underlying corruption still requires engine retirement and inspection.
Do not retry remote effects solely because a checkpoint operation returned an error.

The native engine field precedes its reservation so engine closure occurs first.
No read guard or transaction escapes a method. Worker shutdown drains storage work before releasing ownership.
Only trusted local Linux directories are admitted. Live replacement, rename, hard-link deployments, and network filesystems are excluded.

## Non-destructive format preflight and dirty recovery

Hold public ROM ownership before any native open. Validate clean files through a read-only redb handle.
Require exact known tables and types, marker version, matching profile, bounded records, and valid state transitions.
Future versions, unknown tables, missing markers, oversized records, and malformed state leave original file bytes unchanged.

If read-only open requires native repair, create an exclusive private temporary copy under the trusted parent.
Copy through a bounded reader; verify original file identity and size under the supported ownership contract.
Repair only the copy, then validate its application format and all records in bounded segments.
Only an accepted copy permits native recovery of the original. Revalidate the original after recovery before admitting work.
Rejected copies never authorize original writes. Preserve the original source and return a fixed diagnostic category.
Cleanup only the implementation's own temporary file after all copy-engine handles close.
There is no live rename or replacement of the active checkpoint file.
Accepted-format unclean recovery is required acceptance; a fail-closed-only implementation does not complete Task 2.

## Acceptance evidence

Tests must distinguish source review, controlled backend I/O fault injection, and real process interruption.
Required cases include private creation, parent synchronization, reopen, process exclusion, supported aliases, and rejected hard links.
Preserve bytes for future/malformed markers, including files that need native repair.
Exercise operation/record/file limits and checked sequence overflow before write admission.
Exercise a single cursor-CAS winner, filtered progress, revision conflicts, and atomic multi-key completion.
Interrupt before intent commit, after intent commit, after remote acceptance, and after completion commit before local acknowledgement.
Verify accepted dirty recovery and unknown-commit handle retirement/reconciliation separately.
Actual provider, native public ROM, packaged consumer, dependency, and full verifier gates remain mandatory in the parent plan.

## Concrete Rust types and result categories

Use public ROM `Key` and `JournalCursor` through the pinned dependency. Do not define competing ROM identity or cursor types.
Validated wrapper fields stay private. Constructors enforce the bounds above and getters expose only their documented metadata.
Debug and error output use fixed classifications, not stored keys, paths, bodies, or mapping values.

The following signatures are the Task 2 target. They are not a compiled API claim:

```rust
pub type Result<T> = std::result::Result<T, Error>;

pub struct ProjectionProfile { /* validated immutable metadata and limits */ }
pub struct Checkpoint { /* physical target, <=64 kind cursors, sequence */ }
pub struct OperationMetadata { /* Key, position, revision, tombstone, digest */ }
pub struct PageIntent { /* complete expected state, next cursor, operations */ }
pub struct SwitchIntent { /* expected state, target, new cursors, identity */ }
pub struct TransactionId { /* sequence, transition tag, intent fingerprint, previous control-state digest */ }
pub struct KeyState { /* revision, tombstone, canonical mapping digest */ }
pub struct ReconciledPage { /* constructed only by core after observation checks */ }
pub struct ReconciledTarget { /* constructed only after actual alias observation */ }
pub struct StoreSnapshot { /* checkpoint, pending intent, last completed identity */ }
pub enum CommitStatus { Applied, Absent }

impl TransactionId {
    pub fn encode(&self) -> [u8; 83];
    pub fn decode(bytes: &[u8]) -> Result<Self>;
}

impl CheckpointStore {
    pub fn create(path: &std::path::Path, profile: &ProjectionProfile,
                  initial: &Checkpoint) -> Result<Self>;
    pub fn open(path: &std::path::Path, profile: &ProjectionProfile) -> Result<Self>;
    pub fn load(&self) -> Result<StoreSnapshot>;
    pub fn key_state(&self, key: &rom::Key) -> Result<Option<KeyState>>;
    pub fn prepare_page(&mut self, page: &PageIntent) -> Result<TransactionId>;
    pub fn complete_page(&mut self, page: &PageIntent,
                         observed: &ReconciledPage) -> Result<TransactionId>;
    pub fn prepare_switch(&mut self, switch: &SwitchIntent) -> Result<TransactionId>;
    pub fn complete_switch(&mut self, switch: &SwitchIntent,
                           observed: &ReconciledTarget) -> Result<TransactionId>;
    pub fn reconcile(&self, transaction: &TransactionId) -> Result<CommitStatus>;
    pub fn reopen(&mut self) -> Result<()>;
}
```

`reopen` closes any retired engine before preflight and retains the same ownership reservation.
It does not reacquire the sidecar through a second handle while the reservation is held.
Preparation and completion each have their own transaction sequence and transition tag.
`prepare_page` produces a token derivable before commit from the expected sequence and intent fingerprint.
An uncertain operation returns `Error::Unknown { transaction: TransactionId }`, so the caller can reconcile that exact transition.
The token contains a canonical digest of the previous durable control state, including checkpoint, pending intent, and retained completed tokens.
Construct this digest before the attempted transaction. Return it even if preparation never persists its intent.
The caller must retain the bounded token across process restart; a fresh handle cannot infer an absent attempt from the store alone.
Reconciliation validates token shape, profile binding, transition sequence, and previous-state digest without lost in-memory bookkeeping.
If the caller also loses its token, load durable state and resume any pending intent; do not classify an unrecorded attempt as absent.
For an uncertain initialization, `UnknownInitialization` requires reopening and validating the complete initial state before acknowledgement.
Sequence overflow is a validation failure before any commit.

`reconcile` accepts only the last attempted next transition or a retained matching applied transition.
Applied requires matching durable identity and transition-specific state, not sequence equality alone.
Absent requires the exact previous control-state digest, unchanged sequence, and that transition absent.
An unrelated, older, or contradictory token returns `Conflict` or `Corrupt`; it never grants blind retry permission.
The method has no historical receipt lookup guarantee beyond the retained transition.

Other error categories are `Cancelled`, `Invalid`, `TooLarge`, `Conflict`, `HistoryGap`, `RebuildRequired`, `Busy`, `Unsupported`, `Corrupt`, and `Storage`.
`Unsupported` covers rejected format/profile/platform; `Busy` covers ownership contention.
The implementation must keep a known rolled-back poisoned transaction separate from `Unknown`, and retire its engine for inspection.
Provider observation errors cannot construct a `ReconciledPage` or advance a cursor.

Adapters implement a bounded observation seam returning original identity, target/profile, exact revision, tombstone, and canonical digest.
The core compares observations against each operation and constructs `ReconciledPage` itself.
This protects orchestration against accidental acknowledgement misuse; it is not authentication of a malicious host or forged backend.
Only a qualified adapter's verified transport supplies trusted observations.

## Encoding and table layout

Use a versioned fixed-field binary encoding for this narrow local format.
This is an extras format, not CBOR, JSON, XDR, or a ROM native storage format.
Select fixed layout over a general-purpose parser to bound allocation before decoding fields.
[RFC 8949 deterministic encoding](https://www.rfc-editor.org/rfc/rfc8949.html#section-4.2) is a viable extensible alternative.
It still requires an exact application schema and bounded parser; that additional surface is not needed for these fixed records.

Encode unsigned integers big-endian at their declared width. Encode strings as u16 byte length followed by exact UTF-8 bytes.
Reject invalid UTF-8, empty required identifiers, embedded NUL, trailing bytes, unknown tags, and noncanonical ordering.
Do not normalize Resource identity strings. Length and identity validation remain distinct.
Encode optional values with one byte: 0 absent, 1 present; reject other tags.
Encode bool with one byte: 0 false, 1 true. Digests are exactly 32 bytes.
Encode bounded counts as u16 and validate the count before allocation or iteration.
Order cursor entries by exact UTF-8 kind bytes. Require unique kinds and matching cursor kinds.
Order collapsed operation entries by exact encoded Resource key bytes, not their original arrival order.
Each operation retains its selected journal position. Input arrival order remains validated before collapse.

Every record starts with `ROMXPRJ`, u16 format version 1, and a one-byte record tag.
Record tags are 1 profile, 2 state, 3 pending page, 4 pending switch, and 5 per-key state.
Version 1 implements page intents only. Tag 4 is reserved and rejected until Task 6 explicitly qualifies an extended format.
Out-of-band reconciliation tokens use tag 6, followed by u64 sequence, one-byte transition tag, and two 32-byte digests.
The digests are the intent fingerprint and previous control-state digest. Including the 10-byte header, the encoding is exactly 83 bytes.
Token decoding rejects a different size, version, record tag, transition tag, or zero sequence before allocating.
Tag 6 is not an additional admitted application-table record.
The profile encodes deployment, provider, mapping, optional model, and every numeric admission limit in the table above.
State encodes physical target, sequence, cursor entries, and optional last completed transition identity.
Pending intents encode the expected state, resulting checkpoint metadata, and bounded operation/switch metadata.
Per-key records encode revision, tombstone, and digest; their table key encodes kind then Resource ID.
Future changes increment the application format version. No unknown field is silently ignored.

Use `TableDefinition<&[u8], &[u8]>` for exactly three tables: `romx_projection_meta`, `romx_projection_state`, and `romx_projection_keys`.
Metadata has exactly the byte key `profile`. State has required `checkpoint` and optional `pending` and `completed_page` keys.
`completed_page` retains one canonical page record with tag 3, not an unbounded receipt history.
Preflight reconstructs the completed checkpoint and compares every affected key against that record before accepting retained tokens.
Reject any other application table, multimap, metadata key, or state key during preflight.
Each current-generation key is distinct and validates against the admitted kinds.
Generation switching must preserve previous-generation key state separately or publish a new store; it cannot reinterpret old keys under a new target.
The final Task 6 storage extension needs explicit format qualification before switch methods become supported.

Derive intent fingerprints with SHA-256, domain `ROM-extras/projection-intent/v1`, then length-delimited profile and canonical intent records.
Use domain `ROM-extras/projection-control-state/v1` for previous-state digests, including the profile fingerprint.
Hash each field as u64 big-endian byte length followed by its bytes.
Control-state fields are profile, checkpoint/token record, optional pending page bytes, and optional last completed page bytes, in that order.
Absent optional page records use zero-length fields, distinct from every valid encoded record.
Use a separate domain for backend IDs and canonical document digests. A hash does not grant authority or prove backend persistence.
[FIPS 180-4](https://csrc.nist.gov/pubs/fips/180-4/upd1/final) defines SHA-256; the existing lockfile resolves `sha2 0.10.9`.
Full exact key comparison remains required for a backend-ID collision and checkpoint-key validation.
Document mapping must define its own deterministic encoding before provider writes; checkpoint fingerprinting does not substitute for that requirement.

For a page expected at sequence `s`, preparation publishes sequence `s + 1` and the unchanged cursors with its pending intent.
Completion requires that exact prepared state and publishes sequence `s + 2`, the next cursor, and the completed page fingerprint.
Both additions must validate before preparation; do not prepare an intent that cannot complete because the counter is exhausted.
Identical repeated preparation returns the same `s + 1` token without another write.
Identical repeated completion returns the retained `s + 2` token without another write.
The completed record retains both transition tokens and their page fingerprint so acknowledgement loss can reconcile either phase.
Its bounded retained page proves resulting cursors and key states; token equality alone is insufficient.
These tokens are not a second unbounded receipt table.

## Implemented cooperative startup seam

`Cancellation` is a shared monotonic request with `new`, `cancel`, and `is_cancelled` methods.
`CheckpointStore::open_cancellable(path, profile, cancel)` and `reopen_cancellable(cancel)` supplement the stable startup paths.
A pre-existing request rejects admission before opening or retiring an engine.
After reopen admission starts, cancellation leaves the engine retired and keeps the same ownership reservation.
Application scans check every 256 records. Private-copy readers check between 64-KiB chunks.
Native repair callbacks request abort, without a fixed latency bound for native I/O or repair internals.
Cancellation never interrupts a checkpoint commit or establishes an absent transaction.
A failed open releases ownership after native handles close. Permitted original-file recovery can change bytes before a later cancellation.
The dedicated thread, bounded worker command admission, shutdown/join, and authorized-history reconstruction remain pending.
