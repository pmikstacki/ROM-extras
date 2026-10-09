# SQL Server encrypted COMMIT response loss

Inspection date: 2026-10-09. Evidence: primary source review only. No fixture execution or maintained changes occurred.

## Protocol facts

MS-TDS Transaction Manager requests use message type 0x14. Request type 7 is TM_COMMIT_XACT. The request includes the transaction descriptor and commit options; piggyback begin is optional. A transparent TCP proxy must not assume one TCP read equals one TDS request. See [Transaction Manager request specification](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-tds/0fb28ba5-ddcb-4d02-95c3-aa5b05ec6092).

The transaction response includes environment changes and completion tokens. ENVCHANGE type 9 means commit and clears the transaction descriptor. Completion-token presence alone must not hide an ERROR token or incomplete response. See [ENVCHANGE](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-tds/2b3eb7e5-d43d-4d1b-bf4d-76b9e3afc791).

Inspected locked Tiberius 0.13.0 source is `src/client.rs`, `src/tds/codec/transaction_manager.rs`, and `src/tds/codec/token/token_env_change.rs`. `commit_transaction` builds the native request. `send_transaction_manager_request` first flushes previous response state, sends the transaction-manager packet, then waits through `TokenStream::flush_done`. Success is returned afterward. This is source evidence, not an executed wire capture. See [versioned Client](https://github.com/tiberius-rs/tiberius/blob/v0.13.0/src/client.rs) and [request codec](https://github.com/tiberius-rs/tiberius/blob/v0.13.0/src/tds/codec/transaction_manager.rs).

## TLS constraints

With full encryption, TDS 7.x encapsulates subsequent TDS traffic in TLS. The initial handshake itself uses PRELOGIN encapsulation. A byte-transparent proxy cannot identify encrypted COMMIT or DONE by searching for TDS bytes. See [PRELOGIN](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-tds/60f56408-0188-4cd5-8b90-25c6f2423868) and [TLS negotiation state](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-tds/d62e225b-d865-4ccc-8f73-de1ef49e30d4).

Preserve every original request byte and TLS negotiation byte. Do not terminate TLS, forge DONE, change encryption settings, parse ciphertext as plaintext, or substitute a different trust profile. The proxy changes only routing and response delivery. Authentication and certificate verification remain between Tiberius and SQL Server. Existing loopback-only self-signed fixture behavior remains its existing profile, not a production TLS claim.

TLS record lengths are not proof of a COMMIT request. Multiple records, fragmentation, retransmission and post-handshake traffic make byte-count heuristics insufficient. Host sequencing plus independent database state provides the acceptance evidence.

## Proposed bounded test proxy

Use one test-only loopback listener, one admitted client, and one fixed backend. The listener must not be a general forwarding service. Relay both directions with small fixed buffers and bounded total bytes/deadline. Keep no payload/header dumps.

1. Connect the maintained driver through the transparent proxy. Complete login, transaction begin, ownership lock and prepared writes normally.
2. Fully consume the last precommit driver response. Use a unique disposable sentinel identity bound in that transaction.
3. Send an arm command through a bounded fixture control channel. Wait for an explicit proxy acknowledgement.
4. After acknowledgement, invoke unchanged native `OwnerTransaction::commit` immediately. Submit no other request on this connection.
5. Continue forwarding client-to-server bytes. Discard all server-to-client bytes after arming.
6. Keep the backend socket open while an independent direct connection verifies the committed sentinel.
7. Require the caller to return OwnerError::Unknown, never success. Confirm subsequent use of the retired maintained connection fails.
8. Shut down the proxy with a bounded join after evidence collection.

The proxy's arm acknowledgement must be a delivery barrier. One owner task must coordinate server reads, client writes and commands. If a server-to-client write is in progress, finish it before acknowledging arm. After that acknowledgement, no task may forward a previously buffered or newly received backend byte. A shared atomic flag checked before asynchronous write is insufficient: the write could already be in flight.

Prefer drop/discard over accumulating suppressed responses. Counters record only forwarded/suppressed byte totals and phase changes. Ensure the forwarding task does not close the backend merely because it has received a response. Premature closure before COMMIT reaches SQL Server can cause rollback and a false claimed ACK-loss test.

Do not immediately reset both sockets when arming. Suppressing responses while allowing requests gives SQL Server a chance to execute COMMIT. The independent reader can prove execution before the proxy closes the backend. Driver I/O timeout and retirement remain bounded; no driver API change is needed.

## Placement with existing public methods

For an owner-protected prepared write, use existing `with_owner`. Its fixture closure stages only fixed native SQL, then performs the bounded proxy arm handshake as test instrumentation. The helper immediately calls commit after the closure returns. No production policy callback or fault hook is added.

For a generation transition, the public OwnerTransaction trait also permits explicit lock/read/write/commit sequencing in a fixture. Apply the same state checks as the tested shared transition and arm after `write_owner` returns. This qualifies the native transaction response-loss boundary; it is not independent proof that a differently sequenced shared helper was faulted at exactly the same point.

The initial scenario should use a unique sentinel and protected counter to avoid confusing owner acquisition with reconciliation. A later owner-transition variant can prove the exact generation/token persisted after Unknown. Do not grant a usable Ownership solely because COMMIT was submitted.

## Independent proof and false-positive rejection

Read the unique sentinel through a new direct database connection. Use committed isolation, never NOLOCK/READ UNCOMMITTED. Verify exact bytes and protected state, not merely row count. Only the faulted transaction may write that identity. No observer may create or repair the row.

Require the sentinel to be absent before the transaction. Verify that its insert and all expected control changes use the same transaction. After arming, the proxy permits only that final request phase. A matching committed read establishes that SQL Server committed, even though the encrypted proxy cannot identify the response's semantic status.

An independent read may block until commit under locking READ COMMITTED, or temporarily report absence under versioning. Poll absence only within a fixed deadline. Presence is positive evidence; a timeout/absence is inconclusive and must fail the ACK-loss acceptance. Do not retry the writer to manufacture the expected row.

The strongest accepted claim is: after the last precommit response was fully consumed and the arm barrier acknowledged, no server response bytes reached the caller; the caller returned Unknown; a fresh committed read found the unique transaction's writes. Do not claim a parsed wire COMMIT success token, because the proxy sees ciphertext.

Reject these false positives:

- Arming before lock_owner/write responses complete can prevent COMMIT from being sent.
- Closing the backend before forwarding the complete encrypted request can roll back.
- Dropping only some response bytes can let Tiberius observe partial transaction ENVCHANGE, rather than the requested all-response-loss profile.
- Reading dirty state or a reused sentinel can falsely imply commit.
- Read-before-commit absence does not establish non-commit.
- A caller ticket timeout alone does not prove any wire bytes were lost.

Require at least one suppressed backend byte as supporting wire evidence, but not as proof of success. TLS control traffic could contribute bytes. Independent exact committed state remains decisive.

## Recovery and limits

The maintained transaction commit path maps uncertainty to Unknown and retires its client. A fresh inspection must reconcile the durable generation/token or unique sentinel. Explicit takeover must revalidate the exact observed state. Any old surviving connection must reject writes after takeover through the native fence.

Check delayed durability is DISABLED when testing acknowledged durable behavior. Optional same-disk restart after reconciliation can strengthen persistence evidence. A committed read alone does not prove survival of power failure or storage loss. See [SQL Server durability](https://learn.microsoft.com/en-us/sql/relational-databases/logs/control-transaction-durability?view=sql-server-ver17).

Keep timeout and retirement distinct from query cancellation. Tiberius documents that dropped futures do not stop server execution; cancellation can desynchronize the connection. Its Attention API does not retract committed writes. See [Client cancellation source](https://github.com/tiberius-rs/tiberius/blob/v0.13.0/src/client.rs).

Record source revision, native version, deadline profile, arm acknowledged, post-arm delivered bytes=0, suppressed bytes>0, finite Unknown result, exact direct-read match and retired-client rejection. Record only sanitized counters and booleans. This is native ownership transaction qualification, not full ROM Storage support or exactly-once external delivery.

## Selected qualification

The bounded consumer uses one loopback listener and the fixed existing native server, without TLS termination.
One synchronous pump owns both directions and acknowledges arm only between completed bounded forwarding operations.
The arm follows the prepared UPDATE response; shared with_owner invokes unchanged native COMMIT next.
All later server bytes are discarded, client bytes still reach the server, and no payload is retained.
The relay has a 25-second lifetime, 8MiB total traffic, 16KiB buffer, and bounded socket writes.
A fresh READCOMMITTEDLOCK read must observe the unique table's counter1 before the caller returns Unknown.
The caller's native connection must retire. Inspection must preserve the original acknowledged owner.
An explicit takeover fences the old guard; only the successor performs the later value42 write.
Pass-through is a negative control: success, delivered response bytes, no suppression.
Closing the relay before COMMIT is a counterexample: Unknown with rolled-back value0.
This rules out treating Unknown alone or encrypted byte counts as proof of native commit.

Alternatives: TLS termination would expose credentials and change the transport profile; parsing ciphertext cannot identify TDS tokens.
A caller waiting timeout alone cannot establish wire response loss. The selected test needs neither production fault hooks nor new dependencies.
Owner acquisition response loss, power loss, during-COMMIT crash and distributed profiles remain outside this qualification.
