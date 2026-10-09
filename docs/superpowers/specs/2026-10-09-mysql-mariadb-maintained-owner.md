# Shared maintained MySQL and MariaDB ownership

Implement the next user-approved SQL ownership mappings over rom-sql-core. This does not implement full ROM Storage or copy private ROM ports.
Use one rom-mysql protocol implementation and separate MySQL8.4/MariaDB11.8 native profiles.
Root owns maintained files. Research/review agents own named ignored notes only.

## Architecture

The public Connection owns a private rom-sql-core Executor with capacity1. Do not expose or clone its executor.
One worker owns an unpooled mysql_async0.37.1 Conn and current-thread Tokio runtime.
Bound connect and complete I/O futures, including native result draining. Use the shared positive connect/I/O limits up to60s.
Construct and destroy Conn on its worker outside active Tokio contexts. Retire the entire private runtime after uncertain I/O/connect.
Never use SDK Transaction, pool, automatic reconnect or write retry. Native terminal commands are explicit.
Connection and transaction operations refuse active Tokio callers. Drop can join an idle worker after async handoff.
Keep common arbitration in rom-sql-core and its public API unchanged. Retain PostgreSQL/MSSQL and historical executor evidence.

## Configuration and public API

Export Connection, Transaction, Config, Profile, ControlTable, Deadlines, Value and SslOpts.
Config constructs only an explicit TCP endpoint, credentials, database and profile. Do not accept arbitrary SDK Opts.
Reject empty/oversized/NUL endpoint/user/database values, port0 and unverified TLS flags or hostname overrides.
Normal connect requires approved TLS. Explicit connect_loopback admits only a literal loopback IP and no TLS.
Use no callbacks, init/setup SQL, compression, socket fallback, resolved-address overrides, pools, local infile handler or cleartext plugin.
Set packet cap65536 bytes, statement cache32 and CLIENT_FOUND_ROWS explicitly. SDK still advertises LOCAL_FILES; refuse and retire unexpected requests.
Read a bounded native version during connect; reject a server that does not match the explicit vendor profile.
ControlTable uses shared portable validated names and native backtick quoting; preserve immutable32-byte identity.
Deadlines requires a whole-second native lock timeout with 0<lock<io. It is not a universal server statement timeout.

## Native transaction and result contract

Start READ COMMITTED and set innodb_lock_wait_timeout. Begin partial failures are Unknown and retire the client.
Lock exactly one singleton with SELECT LIMIT2 FOR UPDATE. Project oversized identity/token to NULL with separate token length.
Require format1, exact identity32, signed nonnegative generation and shared OwnerState invariants.
Require the locked control table to use InnoDB; host provisioning must maintain its indexed immutable schema.
Fixed metadata results have exact row/column bounds. Reject malformed native types without panic, coercion, repair or token loss.
Prepared trusted persistence accepts SQL at most16384bytes, at most64 positional parameters, and total parameter bytes at most61440.
The host must supply fixed non-DDL statements without session/transaction control, rowsets or external effects.
Inspect native result columns and further results; rowsets are not silently accepted. Complete native acknowledgement precedes success.
Poison after every rejected/prepared statement error, including errors the caller catches. Only confirmed complete rollback permits reuse.
Any commit failure is Unknown with permanent client/runtime retirement. No result means rollback without authoritative acknowledgement.
Uncertain cleanup is Unknown. Drop performs bounded cleanup, then closes the worker on connection destruction.
Public Debug/errors omit credentials, SQL, endpoint and native error strings.

## Qualification

Run identical independent public and normalized Cargo archive consumers separately on both labelled native fixtures55452/55453.
Verify exact transitions, competing claims, ordered takeover, surviving stale clients, prefix rollback, caught-error poison and lock timeout.
Exercise independently malformed/oversized state, duplicate/missing rows, exhausted generation, invalid configuration and parameter limits.
A controlled opaque relay must qualify actual COMMIT response loss with a fresh committed read before Unknown and native peer closure before cleanup.
A dribbling/held native response must not extend the whole-I/O bound. Constructor timeout and async-handoff Drop must close owned sockets.
SDK/source/advisory/license/MSRV audit precedes adoption. Preserve prior dependency identities and checksums.
Run affected gates, final source review and the full local verifier on frozen sources before publication.
Do not infer MySQL support from MariaDB results or vice versa. Do not advertise full Storage, production TLS or power-loss support from these cases.
