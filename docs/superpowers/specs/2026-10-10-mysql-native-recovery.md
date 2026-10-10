# MySQL and MariaDB maintained native recovery

The full goal requires durable SQL providers, including native restart recovery.
Public ROM still lacks the required incremental Work bridge; this increment does not redefine full Storage completion.

Run both explicit retained fixtures independently: MySQL8.4.11 and MariaDB11.8.9.
Keep original acknowledged guards in one consumer process across SIGKILL and same-container/image/volume restart.
Close the maintained native client without releasing ownership. Hold a separate uncommitted native write through process death.
Observe exact generation2/data17 after recovery, absence of the uncommitted value, stale guard rejection and successful generation3/data42.
Validate durable InnoDB settings, exact server version and native Uptime reset independently from Docker lifecycle observations.
A no-op restart signal must fail the native restart witness.
No write or ownership transition is retried. Explicit connection readiness polls are fixture-only and bounded.
Preserve every table, volume, failure log and run directory. Never signal an arbitrary endpoint or recreate the database.
Run direct and normalized archive consumers, affected checks, final source review and the full local verifier before integration.

## Boundaries and alternatives

Retain the existing production API; only qualification fixtures and their documented evidence change.
Use the shared MySQL driver with distinct vendor observations, not two copied production drivers.
A graceful restart does not qualify process crash. Fresh guards after restart cannot prove retained guard validity.
Docker exited137/changed start and native Uptime reset qualify process restart, not OS/storage power loss or corruption.
An uncommitted observer connection supplies a native rollback witness; it is trusted fixture setup, not a ROM application operation.
Host credentials remain private. Reports include finite lifecycle observations, never raw inspect configuration or native logs.
Deadlock victims, uncertain owner acquisition, production TLS and full Storage remain separate required gates.
