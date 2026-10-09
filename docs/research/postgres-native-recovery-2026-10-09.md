# PostgreSQL native recovery decisions

Inspected on 2026-10-09. This research supports the maintained ownership recovery profile; it is not runtime evidence.

## Fault selection

Use SIGKILL on the exact retained fixture container. Verify PostgreSQL is PID1 before interruption.
Verify exited137 without OOM before restarting the same container and persistent volume.
A normal restart can use orderly shutdown and cannot establish this fault profile.
PostgreSQL discourages direct SIGKILL for operator shutdown because it bypasses cleanup and can leave child processes.
The fixture runner requires container exit before restart. This is an isolated qualification, not an operator recommendation.
Sources: [PostgreSQL18 shutdown](https://www.postgresql.org/docs/18/server-shutdown.html), [Docker kill](https://docs.docker.com/reference/cli/docker/container/kill/).

SIGQUIT immediate shutdown would also require recovery, but it is a different fault mode.
No-op readiness signalling is the negative control. The actual postmaster start timestamp must change.

## Native durability observations

Require fsync, full_page_writes and synchronous_commit ON. Check the actual protected write session as well as reopened fixture settings.
Use permanent logged control/data tables. Preserve acknowledged ownership values in the consumer, rather than reconstructing them from inspected rows.
Sources: [PostgreSQL18 WAL settings](https://www.postgresql.org/docs/18/runtime-config-wal.html), [server information](https://www.postgresql.org/docs/18/functions-info.html).

Compare pg_control_system system_identifier before and after to exclude a different initialized cluster.
Source: [system administration functions](https://www.postgresql.org/docs/18/functions-admin.html).

Process death leaves the host kernel and storage caches running. It cannot qualify OS death, power loss or hardware flush correctness.
Source: [PostgreSQL18 reliability](https://www.postgresql.org/docs/18/wal-reliability.html).

## Independent crash witness

Provision an UNLOGGED table with value17 before interruption. Require zero rows afterward.
PostgreSQL truncates these tables after a crash or unclean shutdown; they are not suitable for durable ownership.
A mutation that skips the timestamp comparison must still reject a no-op through this witness.
Source: [PostgreSQL18 CREATE TABLE](https://www.postgresql.org/docs/18/sql-createtable.html).

The pinned fixture's startup log must report interrupted shutdown, WAL redo and readiness.
Capture only a selected bounded private window. Retain boolean observations, not native statements or credentials.
These literal messages qualify this exact PostgreSQL18.6 fixture; they are not a universal provider API requirement.

## Open work

This increment does not qualify in-flight server interruption, uncertain owner acquisition, deadlock victims, TLS or full ROM Storage.
No production dependency, API or retry policy changes are required.
See the separate [execution record](../verification/postgres-native-recovery-2026-10-09.json) for actual results.

## Startup witness correction

The second full run completed native state checks but rejected its selected startup window.
The optional introductory interrupted-state message was absent; the automatic crash-recovery, redo and readiness messages were present.
Require the explicit improperly-shut-down automatic-recovery message instead of the introductory state message.
Keep redo start, redo completion, UNLOGGED truncation and every exact-state assertion required.
The corrected predicate passes the same preserved native window; it does not infer recovery from availability alone.
Source: [PostgreSQL18 startup implementation](https://github.com/postgres/postgres/blob/REL_18_STABLE/src/backend/access/transam/xlog.c).
