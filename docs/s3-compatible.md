# S3-compatible qualification

ROM-extras reuses the published `rom_blob_object_store::Adapter::s3` constructor.
It adds no copy of ROM's provider or BlobService implementation.
The current local candidate is SeaweedFS 4.48; this is not an Amazon S3 cloud test.

## Fixture profile

The fixed image digest and launch configuration are recorded in [fixture facts](verification/seaweedfs-fixture-2026-10-08.json).
The server has a persistent named volume, one CPU, 512 MiB memory, and only a loopback S3 port.
Generated credentials remain in ignored mode-600 files. The dedicated bucket is `rom-extras`.
Admin UI and WebDAV are disabled. Internal server components are not published to the host.
This is a single-node unversioned fixture, without a disk quota or production topology claim.

The adapter fixes path-style addressing, conditional create, zero retries, and a three-second request timeout.
Its connect timeout is one second. The configured object limit is 16 bytes for conformance.
See [the pinned public configuration](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-blob-object-store/src/configuration.rs).

## Results and qualification status

| Scenario | Executed result |
| --- | --- |
| Published blob contract, including two-way create arbitration | Passed |
| Wrong secret denied; correct account verifies no created object | Passed |
| Acknowledged false bytes and conflict behavior after server restart | Passed |
| Eight concurrent creates with uncertainty and explicit reconciliation | Passed; one Accepted, four Unknown, three Conflict in the recorded run |
| Eight concurrent creates, one acknowledged winner and seven confirmed conflicts within the fixed deadline | Failed with Unknown |

The uncertainty test collects all caller outcomes and checks retained bytes against an acknowledged or uncertain candidate.
A confirmed-conflict candidate cannot be the stored winner. Eight subsequent creates conflict without changing the observed bytes.
Uncertain objects remain preserved: cancelled waiters do not prove provider quiescence or permit deletion.
This does not prove wire acknowledgement loss, global exactly-once writes, or every later provider-side completion.

Independent native SDK diagnostics found typed HTTP timeouts near three seconds.
Versioned server source identifies lock contention as a possible latency mechanism; the exact server path remains untraced.
The strict burst test is an additional operational qualification, not proof that create-only correctness is broken.
It stays separate from the valid Unknown/reconciliation contract. No outcome is relabelled to make it pass.

Run the required profile with the five private ROM_EXTRAS_S3 variables set:

```sh
./scripts/check-s3
```

Missing variables fail. The current profile fails its strict burst qualification and is included in `check-all`.
Do not interpret previous green verifier runs as green results for this newly added gate.
See [current evidence](verification/s3-qualification-2026-10-08.md).

## Remaining work

SeaweedFS does not yet meet the complete operational profile. Provider-route tracing and alternative fixture qualification remain available next steps.
Actual post-write acknowledgement loss, BlobService lifecycle, verified TLS, cloud authorization, and packaged release remain pending.
Use the public BlobService for lifecycle work. Preserve trusted cleanup requirements for detachment, grace, and quiescence.
Other S3-compatible services require their own versioned conformance; this fixture does not certify them.
