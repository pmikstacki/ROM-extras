# SQL Server native owner recovery qualification

Qualify the existing maintained rom-mssql driver against an actual deadlock and an explicit SIGKILL restart of the retained native fixture.
Preserve public API, shared arbitration, protected native tables, prior fixtures and evidence.
Do not expose native error text or add a production API only to identify a test session.

Create a real two-session lock cycle between an owner control row and a separate auxiliary row.
An explicitly provisioned native fixture aggressor uses HIGH deadlock priority; the maintained driver retains its default priority.
Capture native Extended Events with bounded memory. Require error1205 for the observed driver session and matching deadlock victim evidence.
A native1222 lock timeout, outer deadline or ordinary Unavailable result is not deadlock evidence.
Verify rollback of the victim's staged write. Then authorize takeover, reject the old owner on explicit retry and admit the current owner.
Native writes remain prepared driver operations; fixture coordination is bounded and absent from maintained runtime modules.

Keep acknowledged Ownership values in the consumer's memory across restart; never reconstruct an acknowledged owner from inspected state.
Close native connections before a controlled SIGKILL restart of the same SQL Server container and logged data volume.
Verify persisted exact state, prepared data, stale old owner rejection, current owner use, release and monotonic reclaim.
Bind restart orchestration to the explicit existing isolated fixture and verify the same container/volume after restart.
Owned child processes and Docker commands have deadlines and bounded cleanup. Preserve evidence on every failure.
Repeat both profiles through direct public imports and normalized Cargo archives in the required gate.
Run affected checks, source review and the full verifier before publication.

This tested SIGKILL after acknowledged commit is not power-loss or during-commit qualification. Actual lost COMMIT wire responses, production trust chains and full ROM Storage remain open.
