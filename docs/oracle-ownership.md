# Maintained Oracle ownership foundation

`rom-oracle` implements the shared `rom-sql-core::OwnerTransaction` port over one private OCI worker.
Shared claim, takeover, release and stale-owner arbitration remain in SQL core.
This foundation does not implement full ROM Storage, journal publication, incremental Work or complete Resource codec coverage.
The public ROM pin remains d7ef529040eec60dc869034c2d33130219db85fe.

## Configure the host

Construct `Config::new(host, port, service, user, password)` with an explicit endpoint and protected credentials.
`Connection::connect(config, wallet, bounds)` generates native TCPS configuration and requires exact server certificate DN matching.
Construct `TlsWallet` with a host-owned absolute wallet directory and exact certificate DN.
No verification bypass, administrator role, arbitrary descriptor, initializer, session pool or raw native handle is exposed.
Native wallet files, Oracle Net settings and SDK/native logging remain host responsibilities.
The TCPS constructor is implemented, but real TCPS trust/identity qualification remains pending. Do not advertise its production qualification.

`Connection::connect_loopback` permits plaintext only for one literal loopback IP.
There is no endpoint, service, wallet, retry or reconnect default.
The executed fixture is Oracle23.26.3.0.0 with external Instant Client23.26.3.0.0 on loopback55458.
Native admission verifies the23 server family, disabled autocommit and effective OCI call timeout.
One generated Oracle Net ADDRESS has explicit connect and transport bounds with retry count zero.
DNS expansion, listener redirection and host native configuration remain operational limits; no endpoint-redirection containment is claimed.

`Deadlines::new(connect, transport, roundtrip, lock)` requires positive values through60seconds.
Connect, transport and roundtrip use whole milliseconds. Transport must be shorter than connect.
Lock uses whole seconds and must be shorter than roundtrip.
OCI call timeout bounds each round trip. It does not bound a complete statement, operation, transaction or physical socket retirement.
Blocking calls reject an active Tokio runtime. The host must supply a bounded blocking bridge and bound total statements and work.
No ticket timeout is presented as cancellation of an admitted mutation.

## Provision ownership separately

Use an ordinary extras-owned table with a native indexed singleton. Do not change its schema, triggers or routines during use.
Quoted schema/table names retain exact case; identities remain exact binary bytes.
Provision native DDL separately because Oracle can implicitly commit DDL.

```sql
CREATE TABLE ROM_EXTRAS.ROM_OWNER (
    singleton_key NUMBER(1,0) PRIMARY KEY CHECK(singleton_key=1),
    format_version NUMBER(19,0) NOT NULL CHECK(format_version=1),
    store_identity RAW(32) NOT NULL CHECK(UTL_RAW.LENGTH(store_identity)=32),
    generation NUMBER(19,0) NOT NULL CHECK(generation BETWEEN 0 AND 9223372036854775807),
    owner_token RAW(32),
    CHECK(owner_token IS NULL OR UTL_RAW.LENGTH(owner_token)=32),
    CHECK(generation>0 OR owner_token IS NULL)
);
-- Bind the exact host identity; commit explicit provisioning separately:
INSERT INTO ROM_EXTRAS.ROM_OWNER VALUES(1,1,:1,0,NULL);
COMMIT;
```

Construct `ControlTable::new("ROM_EXTRAS", "ROM_OWNER", exact_identity)` after provisioning.
Generation uses0..=i64::MAX and differs from full-u64 Resource revisions.
A new native transaction uses READ COMMITTED and indexed `ROWNUM<=2 FOR UPDATE WAIT n`.
The adapter never uses SKIP LOCKED, automatic takeover or Oracle FETCH FIRST with FOR UPDATE.
The native lock remains held through complete commit/rollback acknowledgement.
Small fetch/prefetch settings and CASE projections bound the ownership result to two rows and seven columns.
A zero-row description first checks original NUMBER/RAW columns; bounded projection metadata and integral/range predicates precede potentially coercing getters.
The host must keep schema stable; concurrent administrative DDL remains outside this boundary.
The shared validator rejects malformed identities, tokens, format, empty/duplicate state and exhaustion without repair.
These projections do not qualify malicious native frames, arbitrary LOB schemas or total SDK allocation.

## Execute protected persistence

Use shared `claim_owner`, `takeover_owner`, `release_owner` and `with_owner` with host-selected cryptographic owner tokens.
`Transaction::execute` accepts owned nullable integer, RAW, text and finite double values.
SQL is capped16KiB, binds64, variable values32767bytes each and total bind payload64KiB.
Empty RAW/text follows Oracle NULL semantics; this API does not establish every ROM field mapping.
Native statement metadata admits only DML without RETURNING before execution.
DDL, PL/SQL, rowsets, transaction control and session commands are rejected.
This admission is not an untrusted SQL sandbox. Host-owned triggers/routines must not introduce external effects or implicit transaction control.

Every failed prepared operation poisons the transaction, including caught errors.
ORA-00001 can leave earlier writes pending; complete rollback must be acknowledged before reuse.
Drop attempts full rollback on the worker. Native handles remain inside that worker through cleanup.
Only a small set of known native statement rejection codes receives an ordinary error after confirmed rollback.
Unclassified failures, failed COMMIT acknowledgements and failed rollback return `Unknown` and permanently retire native use.
A later rollback does not disprove an earlier commit. There is no callback replay or automatic reconnect.
Errors and Debug omit credentials, endpoints, SQL, wallet paths, bound bytes and native diagnostic text.
Native close can itself require OCI work; logical retirement is distinct from an absolute physical teardown guarantee.

## Run independent qualification

The working consumer in `tests/oracle-owner-consumer` uses only public adapter contracts and a separate native observer.
Load the protected local fixture/native-client environment before running:

```sh
./scripts/check-oracle-owner
./scripts/check-all
```

The gate rejects missing native configuration. It runs source and normalized archive consumers independently of workspace feature unification.
Executed cases cover exact ownership, surviving stale clients, rollback after duplicate rejection, caught-error poison, Drop and same-value updates.
They cover native SELECT locking before writes, WAIT expiry, malformed RAW/NUMBER, fractional values, overflow and exact i64::MAX exhaustion.
Forbidden native statement kinds cannot commit a preceding write or create a new table.
Descriptor/runtime/bind limits and diagnostic canaries are checked separately.
A real blocked DML round trip reaches the configured OCI timeout and permanently retires native use.
This observation does not establish an absolute operation deadline.
Same-image/same-volume graceful restart retains acknowledged generation2 guards in memory, rejects generation1 and acknowledges generation3.
Independent native database identity, changed startup time and FREEPDB1 READ WRITE readiness establish the restart witness.

Actual COMMIT wire loss, absolute operation/cancellation containment, SIGKILL/power-loss recovery, TCPS trust/hostname negatives and native security review remain unqualified.
These gaps remain required for broader Oracle acceptance. Rust advisory scans do not assess native OCI security.
Instant Client and database binaries remain external artifacts under their separate terms; see [existing fixture facts](oracle-fixture.md).
See [primary research](research/oracle-maintained-owner-2026-10-10.md) and [executed verification](verification/oracle-maintained-owner-2026-10-10.json).
Source, archive and full local verifier passed against1395 frozen maintained files without changes during execution.

Dependency audits report zero known Rust vulnerabilities and one unmaintained warning: paste1.0.15, required by rust-oracle0.6.3.
The warning is retained without suppression; see [the adoption ruling](research/oracle-maintained-owner-2026-10-10.md#dependency-adoption-ruling).
This warning and the separate native OCI review remain relevant to broader acceptance.
