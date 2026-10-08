# S3-compatible qualification

ROM-extras reuses the published `rom_blob_object_store::Adapter::s3` constructor.
It adds no copy of ROM's provider or BlobService implementation.
SeaweedFS 4.48 and RustFS 1.0.1 have separate local fixture evidence. Neither is an Amazon S3 cloud test.

## Fixture profiles

| Profile | Fixed endpoint | Persistence and limits | Qualification |
| --- | --- | --- | --- |
| seaweedfs | http://127.0.0.1:55455 | Named volume, one CPU, 512 MiB | Strict burst fails with Unknown |
| rustfs | http://127.0.0.1:55456 | Dedicated data/log directories, one CPU, 1 GiB | Four public-port tests pass, including strict burst |

[SeaweedFS facts](verification/seaweedfs-fixture-2026-10-08.json) and [RustFS facts](verification/rustfs-fixture-2026-10-08.json) identify the fixed images and actual configuration.
RustFS follows its [documented testing memory minimum](research/rustfs-conditional-create-assessment.md#proposed-isolated-profile); the hardware limits differ between these profiles.
Both publish only a loopback S3 port. Generated credentials remain in ignored mode-600 files.
Both use the dedicated unversioned `rom-extras` bucket and region `us-east-1`.
SeaweedFS admin UI and WebDAV are disabled. RustFS console is disabled.
These single-node fixtures have no disk quota, redundancy, or production topology claim.

The adapter fixes path-style addressing, conditional create, zero retries, and a three-second request timeout.
Its connect timeout is one second. The configured object limit is 16 bytes for port conformance and 1024 bytes for lifecycle tests.
See [the pinned public configuration](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-blob-object-store/src/configuration.rs).

## Results and qualification status

| Scenario | SeaweedFS | RustFS |
| --- | --- | --- |
| Published blob contract, including two-way create arbitration | Passed | Passed |
| Wrong secret denied; correct account verifies no created object | Passed | Passed |
| Acknowledged false bytes and conflict behavior after server restart | Passed | Passed |
| Eight concurrent creates with outcome classification and explicit reconciliation | Passed; recorded Unknown outcomes | Passed; does not require an Unknown outcome |
| Sixteen fresh keys with eight simultaneous creates each; one acknowledged winner and seven conflicts | Failed with Unknown | Passed |
| BlobService staging, private reads, detachment, and native SQLite/redb reopen | Passed | Passed |
| Deleted reservation after physical publication, retained Unattached receipt, and native reopen | Passed | Passed |

The strict test reads each winner's bytes, verifies subsequent Conflict without overwrite, and deletes only after all outcomes are confirmed.
The separate reconciliation test permits Unknown and retains bytes without cleanup.
It checks that a confirmed-conflict candidate cannot be the stored winner.
A passing run without an injected fault does not prove actual lost acknowledgement recovery.

SeaweedFS native diagnostics found typed HTTP timeouts near three seconds.
A separate verbose reproduction confirms distributed lock use despite prior S3 owner-ring delivery.
Exact source also locks recovery after routed precondition rejection; individual request attribution remains limited.
See [the recovery-path assessment](research/seaweedfs-race-assessment.md#recovery-locks-after-routed-rejection).
The strict failure does not prove create-only correctness is broken. No outcome is relabelled to make it pass.

## Run one exact profile

Set the five private ROM_EXTRAS_S3 variables for the dedicated fixture, then select its matching profile:

```sh
ROM_EXTRAS_S3_PROFILE=rustfs ./scripts/check-s3
```

Use `seaweedfs` to reproduce the retained SeaweedFS profile.
An unset profile preserves the previous SeaweedFS selection. Unknown profiles and mismatched endpoints fail before provider access.
Restart tests verify the selected container's fixture label before issuing restart.
The required s3-lifecycle feature runs all four port cases and two shared native BlobService cases, plus Clippy.
The public constructor and strict scenario remain identical.

`check-all` runs S3 qualification for the configured profile. Selecting RustFS does not establish SeaweedFS qualification.
See [SeaweedFS evidence](verification/s3-qualification-2026-10-08.md) and [RustFS evidence](verification/rustfs-qualification-2026-10-08.md).

## Remaining work

[Garage 2.4.1 source review](research/garage-conditional-create-assessment.md) finds no conditional-create enforcement in its inspected PUT path.
RustFS passes local public-port qualification only. SeaweedFS remains unqualified for the strict operational profile.
Basic native BlobService lifecycle now passes through the same scenario functions used for Azure.
See [shared lifecycle evidence](verification/blob-lifecycle-shared-2026-10-08.md).
Actual post-write acknowledgement loss, corrupt provider reads, current revocation during I/O, cancellation/drain faults, verified TLS, cloud authorization, and packaged release remain pending.
Use the public BlobService for lifecycle work. Preserve trusted cleanup requirements for detachment, grace, and quiescence.
Other S3-compatible services require their own versioned conformance; these fixtures do not certify them.
