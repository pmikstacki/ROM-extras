# Research-backed decisions

Inspection date: 2026-10-08. Tests remain separate evidence from source review.

| Decision | Source and consequence |
| --- | --- |
| Create a separate public repository | The approved plan follows ROM's public open-source intent. [GitHub CLI](https://cli.github.com/manual/gh_repo_create) provides explicit public repository creation. |
| Disable GitHub Actions | ROM quality policy requires local verification. [GitHub API](https://docs.github.com/en/rest/actions/permissions#set-github-actions-permissions-for-a-repository) exposes the repository setting; GET verified `enabled: false`. |
| Use a published immutable ROM revision | [GitHub commit API](https://docs.github.com/en/rest/commits/commits#get-a-commit) rejected the local-only SHA. Public `d7ef529040eec60dc869034c2d33130219db85fe` is the compatibility candidate; unsupported exports remain blocked. |
| Start independent connection execution while incremental APIs await publication | [Postgres implementation](https://docs.rs/postgres/0.19.14/postgres/#implementation) runs a Tokio-backed synchronous client. [Tiberius](https://docs.rs/tiberius/0.13.0/tiberius/) uses async I/O. Both need adapter-owned execution. |
| Use one worker per connection with a bounded standard-library queue | [Rust sync_channel](https://doc.rust-lang.org/std/sync/mpsc/fn.sync_channel.html) provides FIFO bounded admission. [try_send](https://doc.rust-lang.org/std/sync/mpsc/struct.SyncSender.html#method.try_send) rejects a full buffer without queue waiting. No third-party channel dependency is necessary initially. |
| Keep post-admission timeouts uncertain | [recv_timeout](https://doc.rust-lang.org/std/sync/mpsc/struct.Receiver.html#method.recv_timeout) only limits waiting. It does not cancel the submitted operation. Keep the ticket for later outcome inspection. |
| Retire a connection after an unwinding job panic | [catch_unwind](https://doc.rust-lang.org/std/panic/fn.catch_unwind.html) catches unwinding panics only. Connection invariants are not established after an arbitrary driver panic; queued operations must not reuse it. Abort panics remain process failures. |
| Explicit shutdown drains admitted jobs | [Rust channel disconnection](https://doc.rust-lang.org/std/sync/mpsc/struct.Receiver.html#method.recv) permits buffered messages to be read after senders close. [JoinHandle](https://doc.rust-lang.org/std/thread/struct.JoinHandle.html) joins explicitly; dropping the last handle must not imply cancellation. |
