# Maintained Oracle ownership foundation

## Intent and scope

Implement native Oracle ownership under the shared SQL contract. Preserve every existing provider API and historical fixture.
This is an architectural increment toward the full approved ROM-extras goal, not replacement acceptance for complete Oracle Storage.
The public ROM pin remains d7ef529040eec60dc869034c2d33130219db85fe; missing incremental Work ports remain a separate gap.

## Native boundary

`rom-oracle` uses exact rust-oracle0.6.3 and external Instant Client23.26.3.0.0 on one private capacity-one SQL executor worker.
No raw connection, statement, row, session pool, administrator role, arbitrary initializer or executor callback escapes this boundary.
Expose `Connection`, `Config`, `Deadlines`, `ControlTable`, `Transaction`, typed owned `Value`, and explicit `TlsWallet` configuration.
Blocking calls reject active Tokio callers. The host supplies a bounded blocking bridge; the worker owns all OCI handles.
Only explicit literal loopback admits plaintext. Production construction requires TCPS, a host-owned wallet and explicit certificate DN matching.
Validate descriptor fields before native connection. No service, endpoint, retry or wallet defaults exist.
Connect/transport bounds use one explicit ADDRESS and zero retries. DNS, listener redirects and host Oracle Net configuration remain operational limits.
Native admission requires the23 server family, disabled autocommit and the effective configured OCI call timeout.

## Timeout and uncertainty

`Deadlines::new(connect, transport, roundtrip, lock)` accepts positive bounds through60seconds.
Connect/transport/roundtrip use whole milliseconds; transport<connect. Lock uses whole seconds and lock<roundtrip.
OCI bounds each round trip. Neither this bound nor executor waiting proves an absolute whole-operation or transaction deadline.
Do not add a timed-out ticket that leaves an admitted mutation running as if it were canceled.
On unknown native outcome, clear admission and retire the owned native connection after all bounded handles are destroyed.
Native close/drop can consume additional OCI work. Physical socket destruction at an absolute deadline remains unqualified.
All COMMIT or rollback failures produce Unknown and permanent retirement. Never automatically retry callbacks or reconnect a transaction.
Only an explicit small list of known native statement rejections remains ordinarily reusable after confirmed full rollback.
Native diagnostic messages, endpoints, credentials, SQL and bound bytes stay out of errors and Debug.

## Ownership and prepared persistence

Begin clean READ COMMITTED using native transaction control. Provision native schema outside application transactions.
An immutable quoted control-table identity holds a native singleton index, format NUMBER, exact RAW32 identity, NUMBER generation and nullable RAW32 token.
Lock the indexed singleton with `ROWNUM<=2 FOR UPDATE WAIT n`; never SKIP LOCKED or FETCH FIRST with FOR UPDATE.
Fetch at most two rows with explicit small fetch/prefetch sizes and bounded CASE projections.
Require native NUMBER/RAW metadata. Check integral signed range before native conversion; reject string, fractional or overflowing state.
Use shared `validate_owner_row` and claim/takeover/release/with_owner arbitration. Preserve identity bytes exactly; do not normalize names or Resource keys.
Protected writes retain the same native row lock through acknowledged commit/rollback. Caught failures poison the transaction.
Prepared SQL is trusted fixed host persistence, capped16KiB,64 binds,32767bytes per variable bind and64KiB total bind bytes.
Owned values cover nullable integer, RAW, text and finite double. They do not advertise full ROM codec coverage.
Before execute, native statement metadata must admit only DML and deny RETURNING. Reject queries, DDL, PL/SQL and session control.
Exactly one ownership UPDATE is required. Drop attempts complete rollback on the worker; failed cleanup retires.

## Qualification

An independent public consumer and normalized archive consumer execute the same native owner cases without workspace feature unification.
Cases include claim races, ordered takeover, surviving stale clients, exact release, same-value writes, prefix rollback after ORA-00001,
Drop rollback, empty/duplicate state, malformed RAW/NUMBER, unsupported format, generation exhaustion, lock waiting, type/SQL/bind limits and safe errors.
A retained-generation example proves same-image/same-volume graceful restart and rejects stale ownership after reopening.
Real COMMIT response suppression, native cancellation containment, SIGKILL recovery, TCPS trust/hostname negatives and native security review remain required later.
A compiling crate, mock or source review is insufficient. Run affected gates and full local verifier before integration.
Record compiler, exact graphs/checksums/licenses, archive prerequisites, source hashes, observed native cases, preserved failures and open gaps.
