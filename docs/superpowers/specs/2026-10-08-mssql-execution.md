# SQL Server execution fixture

This increment verifies the existing bounded executor with Tiberius and a real SQL Server.
It is part of the approved SQL provider plan, not a substitute for `SqlStore<D>` or Storage conformance.
The public ROM revision still lacks incremental Work exports on 2026-10-08.

## Researched selection

[Tiberius 0.13.0](https://docs.rs/tiberius/latest/tiberius/) is asynchronous and accepts a runtime-independent stream.
Use one connection and current-thread Tokio runtime per executor worker.
Use Tokio compatibility I/O with explicit TDS 7.3 and Rustls features. Disable native TLS and Windows authentication defaults.
This tests the shared executor with both blocking PostgreSQL and asynchronous TDS drivers.

[Microsoft's container instructions](https://learn.microsoft.com/en-us/sql/linux/install-upgrade/quickstart-install-docker?view=sql-server-ver17) describe SQL Server 2025 on x86-64 Linux.
The local host has that architecture and sufficient available memory.
Use Developer edition for these tests only, 4 GiB container memory, two CPUs, and a persistent named volume.
Pin the executed image digest after pulling; do not rely on the moving discovery tag.

The endpoint is fixed to loopback port 55440. Credentials are randomly generated in ignored mode-0600 configuration files.
The isolated fixture accepts its synthetic server certificate; this is not a production certificate-validation profile.
[Tiberius certificate policy](https://docs.rs/tiberius/latest/tiberius/struct.Config.html#method.trust_cert) documents the validation bypass.
Require encryption for fixture traffic. A future supported profile must verify CA, hostname, and certificate expiry.

## Scenarios

- Configuration absence must fail required tests, without skipping.
- A Tokio host calls the synchronous executor while connection I/O runs on its dedicated worker.
- A caller wait timeout remains unknown; released work later commits and a fresh connection observes it.
- A job panic retires its connection and an open transaction rolls back after disconnection.
- An acknowledged control row survives server restart.

Connection and fixture SQL futures have explicit eight-second I/O deadlines.
A deadline or failed SQL operation panics the fixture job, retiring the connection; this is test scaffolding, not final driver error classification.
Lock waits have a five-second server deadline; delayed durability must be disabled.
Full receipt, revision, journal, Work, owner-generation, and recovery conformance remains pending.
