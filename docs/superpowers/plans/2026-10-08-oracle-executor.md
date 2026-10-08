# Oracle shared-executor qualification

Status: in progress. This is an executor qualification, not a ROM Storage adapter qualification.

## Sources and decisions

Inspected on 2026-10-08. Use the [source assessment](../../research/oracle-executor-assessment.md) for driver and transaction semantics.

Use the official full Free image, pinned to the inspected amd64 manifest digest.
[Oracle's registry](https://container-registry.oracle.com/ords/ocr/ba/database/free) publishes full and Lite variants.
The fixture does not need Lite's reduced capabilities.

Use Instant Client Basic 23.26.3.0.0, downloaded from [Oracle's Linux client page](https://www.oracle.com/database/technologies/instant-client/linux-x86-64-downloads.html).
Verify its published size and SHA-256 before extraction.
The downloaded `BASIC_LICENSE` specifies Free Distribution, Hosting, and Use Terms, version 1.0, dated 28 June 2022.
This artifact's terms differ from the older distribution-license webpage considered during source review.
Keep the archive, extracted libraries, and original notices outside Git. Use the libraries unmodified.

Use a Podman secret mounted as `oracle_pwd`, as specified in the [official container instructions](https://github.com/oracle/docker-images/blob/main/OracleDatabase/SingleInstance/README.md).
Keep the administrator password outside command arguments and published logs.
Bind only `127.0.0.1:55458` to database port 1521. Retain data in a named volume.
The fixture allowance is two CPUs and 4 GiB container memory, with 1 GiB shared memory.
These are experimental limits, not a claimed vendor minimum or production configuration.
[Oracle Free terms](https://www.oracle.com/downloads/licenses/oracle-free-license.html) describe internal development and testing use.
Native artifact installation does not establish a production license or security review.

## Implementation sequence

1. Verify the downloaded image and client archive. Inspect actual image scripts and client dependencies.
2. Start the isolated persistent fixture. Confirm its actual database version and PDB readiness.
3. Create a fixture user with session, table, and bounded tablespace privileges. Retain private connection configuration.
4. Add optional `oracle = 0.6.3` to the independent consumer. Review the resulting dependency delta.
5. Exercise OCI connections through the existing dedicated-worker executor. Keep statement and row handles within the worker.
6. Verify bound RAW identifiers, integer NUMBER values, false/zero, empty-string NULL behavior, and distinct binary keys.
7. Verify ORA-00001 leaves earlier writes pending until explicit transaction rollback. Confirm absence through a fresh connection.
8. Verify a worker panic retires the connection and leaves no committed write visible through a fresh connection.
9. Verify a caller wait timeout leaves the same ticket available to resolve a later confirmed commit.
10. Exercise bounded admission and restart recovery with the retained database files.
11. Run affected tests and Clippy. Review source independently. Freeze runtime inputs before the full verifier.
12. Publish exact commands, versions, source hashes, and outcomes. Retain failures and fixture state.

## Claim limits

OCI call timeout bounds a round trip, not the whole transaction.
A commit error or caller timeout does not prove rollback. Do not replay an unknown commit automatically.
DDL stays outside transaction rollback scenarios because Oracle can implicitly commit DDL.
This fixture does not qualify TLS, replication, Transaction Guard, or a complete ROM Storage implementation.

The test connection uses one literal loopback address and no retries.
Its transport timeout is 3 seconds; its connection timeout is 10 seconds.
These are experimental test limits using [Oracle Net parameters](https://docs.oracle.com/en/database/oracle/oracle-database/26/haovw/oracle-net-tns-string-parameters.html).
Set OCI call timeout to 10 seconds per round trip. This does not bound the total operation.
