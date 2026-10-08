# Oracle executor fixture

The Oracle profile exercises the shared SQL executor through rust-oracle 0.6.3 and OCI.
It does not implement the public ROM Storage contract.
See [primary-source research](research/oracle-executor-assessment.md), [execution plan](superpowers/plans/2026-10-08-oracle-executor.md), and [fixture facts](verification/oracle-fixture-2026-10-08.json).

The actual server and Instant Client report 23.26.3.0.0. The image label reports 23.26.0; it is not the server-version evidence.
The official amd64 image is pinned to sha256:c4d9b15a1b2240ce90730c23ce298fcdd34f6793792c1b3849181a080009c322.
The persistent fixture publishes only 127.0.0.1:55458 to port 1521, with experimental two-CPU and 4-GiB limits.
Its shared-memory allowance is 1 GiB. These limits are test settings, not production sizing.
Data remains in the named volume rom-extras-oracle-20261008.

The application user ROM_EXTRAS has CREATE SESSION and CREATE TABLE, with a 128-MiB USERS tablespace quota.
Administrator credentials use a Podman secret. Application credentials remain in protected, ignored host configuration.
The test rejects alternate application users and endpoints before connecting.
Connection setup uses one literal loopback address, transport timeout three seconds, connection timeout ten seconds, and zero retries.
OCI call timeout is ten seconds per round trip; it does not bound the whole transaction or establish rollback.

Run with the protected fixture and native-client environment loaded:

```sh
./scripts/check-oracle
./scripts/check-all
```

Missing configuration fails the gate. Seven tests verify worker execution from Tokio, caller timeout followed by confirmed commit,
panic retirement, full rollback after ORA-00001, RAW byte identity, bounded queue rejection, and acknowledged data after restart.
RAW tests distinguish ASCII case, Unicode normalization forms, and zero-byte sequences.
NUMBER zero survives persistence. Empty VARCHAR text becomes NULL, as Oracle specifies.
DDL remains outside rollback scenarios because Oracle can implicitly commit DDL.

Restart serializes all cases and retains the same container and named volume.
The pinned shell returns exit 143 after handling SIGTERM; the test also requires fresh database shutdown confirmation before starting it again.
Readiness requires FREEPDB1 to report READ WRITE. Commands and readiness waiting have explicit limits.
The first restart test wrongly required exit 0; its failure remains preserved.

Instant Client Basic is an external prerequisite, not a bundled repository artifact.
Its published ZIP size and SHA-256 were checked. Every extracted regular file and symlink target matches the original archive.
The archive's BASIC_LICENSE specifies Free Distribution, Hosting, and Use Terms, version 1.0, dated 28 June 2022.
This differs from older license pages considered during source research.
The native libraries and notices remain unmodified outside Git. Rust dependency audit does not assess native OCI advisories.

Full Storage atomicity, revision and ownership fencing, migrations, TLS, replication, ambiguous commit fault injection,
Transaction Guard, and packaged release acceptance remain pending.

Sources: [official client downloads](https://www.oracle.com/database/technologies/instant-client/linux-x86-64-downloads.html),
[container instructions](https://github.com/oracle/docker-images/blob/main/OracleDatabase/SingleInstance/README.md),
[Oracle Net timeouts](https://docs.oracle.com/en/database/oracle/oracle-database/26/haovw/oracle-net-tns-string-parameters.html),
and [Free database terms](https://www.oracle.com/downloads/licenses/oracle-free-license.html).
