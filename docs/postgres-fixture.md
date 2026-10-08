# PostgreSQL executor fixture

These tests verify native connection execution. They do not implement or certify a ROM PostgreSQL Storage adapter.
Use a dedicated disposable database. Tests create uniquely named tables and preserve them as evidence.
Keep authentication secrets outside Git.

Inspected image: `postgres:18.6-alpine3.23`.
Executed digest: `docker.io/library/postgres@sha256:3928680cee9028902891672c0a6ce84de58ed3eb1b987588e7c9424d8f08d21d`.
Bind the server only to loopback. The fixture accepts literal loopback addresses and uses `NoTls` exclusively for local tests.
This does not establish TLS or production deployment support.

Set `ROM_EXTRAS_POSTGRES_DSN` through protected host configuration. Run:

```sh
./scripts/check-all
```

The three real-server checks cover a Tokio caller, post-timeout commit, and connection retirement after an unwinding panic.
The fixture verifies `fsync=on` and `synchronous_commit=on`.
Power-loss durability, ROM receipt recovery, journal ordering, and ownership fencing remain unverified.

For the current local fixture, credentials are in ignored, mode-0600 `.superpowers/postgres-access.sh`.
Its container is `rom-extras-postgres-20261008`, with host port `55439` and a persistent database volume.
Preserve the container and tables until their verification evidence is no longer needed.

Sources: [official PostgreSQL image](https://github.com/docker-library/docs/blob/master/postgres/README.md),
[WAL settings](https://www.postgresql.org/docs/18/runtime-config-wal.html),
and [Postgres driver execution](https://docs.rs/postgres/0.19.14/postgres/#implementation).
