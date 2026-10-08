# CockroachDB executor fixture

This profile qualifies the shared SQL executor with the existing PostgreSQL wire driver. It is not a complete ROM Storage adapter.
[Primary research](research/cockroachdb-executor-assessment.md) distinguishes SQLSTATE 40001 retries, ambiguous 40003 outcomes, and caller waiting deadlines.

The executed server is CockroachDB CCL v26.3.2, built from reported commit 8db696c79524f16c5fd046165e52a2e8c7eff326.
The official image digest is sha256:84f0c929fdd4284b40a0c4afb1f284573fb281abc0323f8329e4e5f1650a3e00.
The public matching source tag was unavailable during research; binary build metadata does not establish inspected matching source.
See [fixture facts](verification/cockroach-fixture-2026-10-08.json).

The persistent single-node fixture has experimental limits of two CPUs and 2 GiB, cache 256 MiB and SQL memory 128 MiB.
These are local test parameters, not an official minimum or production sizing recommendation.
RPC binds container loopback 127.0.0.1:26257; a separate SQL listener uses 26259.
Only SQL is published to host 127.0.0.1:55457. The administration port is not published.
All database tables, the named disk volume, rejected initial containers and failed logs remain preserved.
The fixture has no hard disk quota or replication.

The NoTls profile requires exact host 127.0.0.1, port 55457, database rom_extras, user root and explicit disable mode.
It rejects alternate hostaddr overrides before connecting. The shared helper also prevents this override in PostgreSQL's loopback profile.
Generated fixture access configuration is kept in ignored mode-0600 .superpowers/cockroach-access.sh.

Run the required gate through protected host configuration:

```sh
./scripts/check-cockroach
./scripts/check-all
```

Missing backend configuration fails rather than skips the gate.
The consumer feature cockroach-conformance reuses pinned postgres 0.19.14 and tokio-postgres 0.7.18 without dependency version changes.
TCP connection establishment has a three-second configured timeout. Cockroach SQL statements have a ten-second session timeout; PostgreSQL retains three seconds.
The initial cold DDL exceeded three seconds after an abrupt restart; this remains recorded failure evidence.
A statement timeout is cancellation, not confirmed rollback or a known commit result.

Seven real-server cases cover typed execution from Tokio, waiting timeout then later commit, panic retirement and absent uncommitted row,
23505 explicit rollback and fresh transaction, controlled40001 with complete transaction reread, byte-key identity, and acknowledged restart retention.
Restart requires an exited 0 graceful stop before starting the same container and store; readiness is bounded.
See [verification](verification/cockroach-executor-2026-10-08.md).

The official licensing FAQ exempts internal nonproduction start-single-node development from license keys and throttling.
That scope does not establish production or redistribution authorization.
TLS,40003 wire faults, ownership fencing, state/event/receipt atomicity, migration conformance, quorum and packaged consumer acceptance remain pending.

Sources: [v26.3.2 release](https://www.cockroachlabs.com/docs/releases/v26.3#v26-3-2),
[single-node command](https://www.cockroachlabs.com/docs/v26.3/cockroach-start-single-node),
[session configuration](https://www.cockroachlabs.com/docs/v26.3/set-vars),
[licensing FAQ](https://www.cockroachlabs.com/docs/stable/licensing-faqs),
and [container stop semantics](https://docs.docker.com/reference/cli/docker/container/stop/).
