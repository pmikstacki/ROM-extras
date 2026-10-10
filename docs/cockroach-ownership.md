# Maintained CockroachDB ownership

`rom-cockroach` implements `rom-sql-core::OwnerTransaction` over the shared `rom-pgwire` transport.
The core owns claim, takeover, release and stale-owner arbitration.
The adapter does not implement full ROM Storage, journal publication or incremental Work.
The public ROM pin remains d7ef529040eec60dc869034c2d33130219db85fe; required incremental Work ports remain unpublished.

## Configure the host

Supply `Config` with one explicit TCP endpoint and host-owned credentials.
`Connection::connect(config, tls, deadlines)` requires encryption; the host connector must verify server identity and trust.
`connect_loopback` admits plaintext only for one literal loopback IP.
There is no default service, automatic reconnect, retry, provisioning or ownership release.
The qualified local profile uses CockroachDB CCL26.3.2. Admission requires the26.3 family and the following effective settings:

| Setting | Required value |
| --- | --- |
| Transaction isolation | Explicit SERIALIZABLE at each begin |
| `enable_durable_locking_for_serializable` | `on` |
| `optimizer_use_lock_op_for_serializable` | `on` |
| `statement_timeout` | Host statement limit, set with explicit milliseconds |
| `lock_timeout` | Host lock limit, set with explicit milliseconds |
| `idle_in_transaction_session_timeout` | Host statement limit |

These are settings of the owned connection. The adapter does not change cluster settings.
PostgreSQL continues to use its separate READ COMMITTED and synchronous-commit profile.
CockroachDB's compatibility `synchronous_commit` setting is not durability evidence.

`Deadlines::new(connect, io, statement, lock)` requires positive bounds, lock < statement < I/O, and connect/I/O through60 seconds.
Statement and lock limits must use whole milliseconds.
The private runtime bounds complete connection and operation futures, including response consumption.
Blocking calls refuse an active Tokio runtime. Use a host-owned bounded blocking bridge.
The host must bound total statements, parameter payloads and CPU work between calls; no cumulative transaction deadline is claimed.

## Provision native ownership

Provision a singleton separately in one native column family. Keep the table identity immutable.
Use exact INT4/INT8 and BYTES wire types; the adapter does not coerce strings or floats.

```sql
CREATE TABLE public.rom_extras_owner (
    singleton_key INT4 PRIMARY KEY CHECK(singleton_key=1),
    format_version INT4 NOT NULL CHECK(format_version=1),
    store_identity BYTES NOT NULL CHECK(octet_length(store_identity)=32),
    generation INT8 NOT NULL CHECK(generation>=0),
    owner_token BYTES,
    CHECK(owner_token IS NULL OR octet_length(owner_token)=32),
    CHECK(generation>0 OR owner_token IS NULL),
    FAMILY owner (singleton_key,format_version,store_identity,generation,owner_token)
);
-- Bind the exact32-byte host identity during explicit provisioning:
INSERT INTO public.rom_extras_owner VALUES(1,1,$1,0,NULL);
```

Construct `ControlTable::new(schema, table, exact_identity)` with portable validated identifiers.
Generation uses0..=i64::MAX; it is distinct from ROM's full-u64 Resource revisions.
Identity and token bytes are preserved exactly. Inspection alone does not grant ownership.
FOR UPDATE and bounded CASE/length projections read at most2 rows and5 columns.
The shared validator rejects malformed, missing, duplicate, unsupported or exhausted state without repair.
Do not add triggers, replace the schema or change column families during application use.

Use the shared `claim_owner`, `takeover_owner`, `release_owner` and `with_owner` operations.
`Transaction::execute(sql, params)` accepts trusted fixed persistence SQL through16KiB and64 parameters.
Never supply DDL, session/transaction control, rowsets or external effects.
All protected writes retain the same native control-row lock through commit or rollback acknowledgement.
Caught errors poison the transaction. Reuse requires confirmed full rollback.

## Preserve uncertainty

40001 requires a fresh complete host operation after confirmed rollback and renewed ownership validation.
The adapter never automatically replays a statement or a host `FnOnce` callback.
The current shared error type reports ordinary native rejection as `Unavailable`; it does not label every such error retryable.
40003 from reads, writes or terminal operations produces `Unknown` and permanently retires the native transport.
Any failed COMMIT acknowledgement is conservatively `Unknown`, including a confirmed native commit rejection.
Lost or incomplete responses also retire the transport. Inspection must not mint replacement ownership after `Unknown`.

Adapter errors and Debug omit credentials, endpoints, SQL and native diagnostic messages.
PGwire retains only valid five-character uppercase/digit SQLSTATE codes; malformed codes are discarded and retire the transport.
SDK tracing and native query logs remain the host's responsibility.
Metadata budgets bound retained decoded results; they do not bound the SDK's allocation of one incoming frame.
Fixed bounded projections remain required. Unexpected prepared rowsets retire the transport.

## Run the examples and qualification

The independent consumer in `tests/cockroach-owner-consumer` contains working ownership, rollback, retry and restart examples.
Run it only with the explicitly configured controlled fixtures:

```sh
./scripts/check-cockroach-owner
```

The gate requires CockroachDB and PostgreSQL DSNs and an explicit Python executable; it cannot silently skip native tests.
It tests source and normalized archive consumers independently of workspace feature unification.
Profiles cover stale clients, contention, replicated SELECT locks before writes, native40001, rollback, malformed state and generation exhaustion.
They also cover native lock/statement timeouts, startup blackhole, limits, wrong-provider/TLS denial and async-handoff socket closure.
Opaque relay tests suppress or dribble actual committed responses and verify `Unknown`, absolute I/O bounds and peer closure before cleanup.
Same-image/same-volume SIGKILL recovery retains acknowledged generation2 guards in memory, rejects generation1, then acknowledges generation3.

Native developer functions inject40001,40003 and a malformed protocol code in explicit test sessions.
Only these test sessions enable `allow_unsafe_internals`; production adapter admission never enables it.
Injected40003 establishes error mapping, not an actual distributed ambiguous commit.
The replicated-lock witness has a negative control with durable locking disabled.

Production TLS, lease transfers, range splits, quorum loss, multi-region behavior and power loss remain unqualified.
The local single-node fixture does not establish these guarantees or full ROM Storage support.
CockroachDB's server license is separate from the MIT extras crates; determine deployment eligibility under the [official licensing rules](https://www.cockroachlabs.com/docs/v26.3/licensing-faqs).
See [primary research](research/cockroach-maintained-owner-2026-10-09.md) and the [maintained verification record](verification/cockroach-maintained-owner-2026-10-09.json).
