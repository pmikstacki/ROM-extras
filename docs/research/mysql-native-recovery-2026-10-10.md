# MySQL and MariaDB ownership recovery gate

Inspection date: 2026-10-10. Primary-source review only. Target retained native profiles: MySQL8.4.11 and MariaDB11.8.9. No builds, lifecycle operations or maintained edits occurred. Read extras writing and quality rules in this research session.

## Durability profile

Before mutation, record actual server version, table engine and effective global/session durability settings. Require ordinary InnoDB tables and `innodb_flush_log_at_trx_commit=1`. Record `log_bin`; when enabled, require `sync_binlog=1`. Record doublewrite configuration and reject forced recovery. A value of2 leaves OS-cache durability exposure; a process-kill test cannot establish power-loss safety. Device and OS flush honesty remain environmental assumptions. Source: [MySQL8.4 InnoDB variables](https://dev.mysql.com/doc/refman/8.4/en/innodb-parameters.html#sysvar_innodb_flush_log_at_trx_commit).

MariaDB documents group-commit dependencies between durable InnoDB data and binary logs. Use both flush=1 and sync_binlog=1 when binary logging is enabled. Do not apply newer MariaDB12.3 InnoDB-based-binlog semantics to11.8. A disabled binlog profile proves local database recovery only, not replication consistency. Sources: [MariaDB group commit durability](https://mariadb.com/docs/server/server-management/server-monitoring-logs/binary-log/group-commit-for-the-binary-log), [MariaDB InnoDB variables](https://mariadb.com/docs/server/server-usage/storage-engines/innodb/innodb-system-variables).

## Recommended two separate tests

First qualify acknowledged owner recovery. Claim or take over through the maintained public driver. Retain the actual Ownership guard in host memory. Close native client executors without releasing owner state. Record exact committed generation and identity privately, then crash and restart the same retained container. After bounded readiness, prove exact owner state survived. Fresh claim must return Busy. Prove the retained guard can perform a fenced operation before takeover. Take over once; prove old guard becomes Stale and cannot change the sentinel. No application Work policy belongs in this test.

Second qualify incomplete transaction recovery. Use a controlled independent native connection to perform an uncommitted sentinel update and retain its transaction until process death. Confirm the statement completed before kill; do not issue COMMIT or ROLLBACK. After recovery, prove sentinel remains at its last acknowledged committed value. If driver Drop performs rollback, do not drop that transaction before kill. Distinguish this test from closing all clients before an acknowledged-only recovery test.

MySQL recovery applies redo and rolls back incomplete non-XA-prepared transactions. The server may accept connections before background rollback finishes, so readiness alone does not prove recovery locks have cleared. Bound a real indexed sentinel/fence query separately; retain diagnostics if it remains locked. Do not enable innodb_force_recovery to make this qualification pass. Source: [MySQL8.4 recovery](https://dev.mysql.com/doc/refman/8.4/en/innodb-recovery.html).

MariaDB needs its own executed result. Shared InnoDB terminology does not prove MySQL's exact log wording or readiness timing occurs on MariaDB11.8. Keep version-specific log recognition optional supporting evidence; require exact native state assertions for both profiles.

## Process-death witness

Record container ID, image digest, selected data-volume identity, state PID and StartedAt before the fault. Never print the complete inspect object: it can contain secrets. Confirm PID1 is the database executable or an entrypoint that execs it. Docker kill targets the container's main process; shell-form launch can defeat intended signal forwarding. Use an explicit SIGKILL crash, observe stopped state and exit137, then start the same container. Do not recreate it, change volumes or invoke initialization scripts. Source: [Docker kill](https://docs.docker.com/reference/cli/docker/container/kill/).

After start, require the same container ID/image/mount and a new StartedAt. PID alone is insufficient because identifiers can be reused. Require native version/readiness, and retain a sanitized recovery-log interval bounded by the recorded start timestamp. Record whether logs show crash recovery/redo/incomplete rollback without publishing credentials, row contents or private paths. Source: [Docker inspect](https://docs.docker.com/reference/cli/docker/container/inspect/).

If restart policy automatically starts the container, disable that behavior only within the isolated fixture setup or explicitly record automatic restart timing. Do not confuse a missed stopped observation with proof no crash occurred. Preserve historical evidence and same-volume data.

## Limits and alternatives

Graceful stop/start qualifies reopen after ordinary shutdown. SIGKILL qualifies unexpected database-process death with the host OS and storage still running. Neither establishes host power loss, lost fsync, torn writes, replication failover or COMMIT acknowledgement-loss behavior. A lost response remains Unknown even if a later independent reader proves commit; this requires its own proxy qualification.

A child-process fixture can avoid Docker-specific observation, but it must retain the same data directory and identify the actual database process. A VM or storage fault profile can address power-loss behavior later; it is a separate gate. No native recovery claim follows from this source note.

## Executed fixture adjustments

The default Docker exec user could not inspect the database executable through proc.
Explicit database-user execution succeeded without elevated privileges. See [Docker exec user selection](https://docs.docker.com/reference/cli/docker/container/exec/).
Native restart checks use the Uptime status variable, independently from Docker start time.
Both vendors define it as time since server startup: [MySQL Uptime](https://dev.mysql.com/doc/refman/8.4/en/server-status-variables.html#statvar_Uptime), [MariaDB status variables](https://mariadb.com/docs/server/server-management/variables-and-modes/server-status-variables#uptime).

The first full verifier stopped when its isolated OpenSearch fixture blocked creation below its 10GiB high watermark.
A measured 9.83GB was available; 694 primaries were green, below the 1000-shard limit.
Fixture-only byte thresholds changed from 20/10/5GiB to 8/6/5GiB after admission required more than 8GiB available.
The 5GiB flood floor and disk threshold protection remained enabled. No index was deleted.
This is a controlled fixture capacity profile, not a production recommendation.
Source: [OpenSearch dynamic disk settings](https://docs.opensearch.org/latest/install-and-configure/configuring-opensearch/cluster-settings/).

A completed PostgreSQL packaged-consumer incremental cache was preserved in a compressed archive, with every regular file checked byte-for-byte.
Its working cache was replaced only after verification; the final binary checksum remained unchanged.
This recovered working space while retaining source, packages, binaries and logs. Later compiler cache reconstruction can take longer.
Source: [Cargo distinguishes intermediate compiler cache from final artifacts](https://doc.rust-lang.org/cargo/reference/build-cache.html).
See the verification record for actual observations, failure evidence and archive inventories.
