# S3-compatible local fixture assessment

Date: 2026-10-08. Evidence: primary source and published ROM code inspection only. No server launch, image pull, registry digest resolution, dependency build, or conformance test was executed by this research agent.

## Public adapter contract

ROM revision `d7ef529` supplies `rom_blob_object_store::Adapter::s3`. Use that public constructor and `BlobStore`; do not copy its implementation or access private fields. Its dependency pins `object_store` 0.14.2 with AWS support. See [public constructor](https://github.com/pmikstacki/ROM/blob/d7ef529/crates/rom-blob-object-store/src/configuration.rs) and [dependency manifest](https://github.com/pmikstacki/ROM/blob/d7ef529/crates/rom-blob-object-store/Cargo.toml).

The constructor fixes path-style addressing, ETag conditional creation, zero retries, a three-second request timeout, and a one-second connect timeout. `LoopbackTestOnly` permits HTTP only for literal loopback IPv4/IPv6 addresses. DNS `localhost` does not satisfy that policy. It rejects embedded endpoint credentials, queries, and fragments. The buffer limit must be positive and no greater than 16 MiB. Use `http://127.0.0.1:<port>` for the proposed local fixture. See [configuration validation](https://github.com/pmikstacki/ROM/blob/d7ef529/crates/rom-blob-object-store/src/configuration.rs).

Create uses `PutMode::Create`. In `object_store` 0.14.2, the S3 ETagMatch implementation sends `If-None-Match: *`. A compatible server must enforce that precondition atomically. A successful ordinary PUT does not establish this contract. See [ROM create](https://github.com/pmikstacki/ROM/blob/d7ef529/crates/rom-blob-object-store/src/adapter.rs) and [S3 conditional implementation](https://github.com/apache/arrow-rs-object-store/blob/v0.14.2/src/aws/mod.rs).

Get checks reported size, then bounds accumulated bytes and stream chunk count. Head returns byte count. Delete treats missing as success and is restricted by the public port to trusted maintenance with detachment, grace, and quiescence. Writes map precondition failures to Conflict and uncertain provider failures to Unknown. See [adapter operations](https://github.com/pmikstacki/ROM/blob/d7ef529/crates/rom-blob-object-store/src/adapter.rs), [error mapping](https://github.com/pmikstacki/ROM/blob/d7ef529/crates/rom-blob-object-store/src/errors.rs), and [maintenance contract](https://github.com/pmikstacki/ROM/blob/d7ef529/crates/rom-blob/src/storage.rs).

## At most three fixture candidates

| Candidate | Current primary-source finding | License | Recommendation |
| --- | --- | --- | --- |
| SeaweedFS | Official release metadata reports 4.48, published 2026-09-28; project documents an authenticated one-node S3 launch | Apache-2.0 | First candidate to qualify through the complete ROM contract suite |
| Garage | Project documents fixed-tag `dxflrs/garage:v2.4.1` deployment, persistent directories, SigV4, and path-style APIs | AGPL-3.0 license text | Alternative if exact-version conditional creation passes |
| MinIO community | README says no longer maintained and source-only; repository metadata marks it archived | AGPL-3.0 | Preserve historical evidence; do not adopt as a maintained fixture |

Sources: [SeaweedFS release metadata](https://api.github.com/repos/seaweedfs/seaweedfs/releases/latest), [SeaweedFS launch](https://github.com/seaweedfs/seaweedfs/blob/master/README.md), [SeaweedFS license](https://github.com/seaweedfs/seaweedfs/blob/master/LICENSE), [Garage deployment](https://github.com/deuxfleurs-org/garage/blob/main-v2/doc/book/cookbook/real-world.md), [Garage compatibility](https://github.com/deuxfleurs-org/garage/blob/main-v2/doc/book/reference-manual/s3-compatibility.md), [Garage license](https://github.com/deuxfleurs-org/garage/blob/main-v2/LICENSE), [MinIO README](https://github.com/minio/minio/blob/master/README.md), and [MinIO repository metadata](https://api.github.com/repos/minio/minio).

These source facts do not prove a registry tag or image digest. No digest is pinned here. Resolve the chosen official project image and record its digest before fixture execution. Keep server license assessment separate from Rust client dependency licensing.

## SeaweedFS candidate details

The official README documents `weed mini -dir=...` and the project image `chrislusf/seaweedfs`. The launch accepts `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, and `S3_BUCKET`; `/data` is the documented Docker persistence mount. Supply generated private fixture credentials and a dedicated bucket. Publish only the S3 port on loopback. Do not expose control or admin ports. See [project quickstart](https://github.com/seaweedfs/seaweedfs/blob/master/README.md).

The inspected 4.48 PUT implementation validates conditional headers. Its finalization path also uses routed write conditions or an object write lock. That is stronger source evidence than a standalone HEAD-then-PUT check, but requires concurrent execution tests. See [versioned PUT implementation](https://github.com/seaweedfs/seaweedfs/blob/4.48/weed/s3api/s3api_object_handlers_put.go).

A historical issue reports conditional-write problems when versioning and object lock are enabled. It is not proof that 4.48 fails. Avoid implying versioned-bucket support from an unversioned fixture. Record the exact bucket configuration and test additional modes separately. See [project issue 8073](https://github.com/seaweedfs/seaweedfs/issues/8073).

The inspected launch documentation is mutable main source. Confirm the exact 4.48 command and image entrypoint before using its simplified launch. If the documented image tag is unavailable, build the exact release source with recorded toolchain and checksum. Do not silently substitute an unrelated community image.

## Garage and MinIO limitations

Garage documents separate persistent metadata and object-data directories and access-key-per-bucket permissions. Its compatibility matrix reports core PUT/GET/HEAD/delete support, but that alone does not establish `If-None-Match:*` semantics. Qualify conditional creation explicitly before selecting it. Its fixed Docker tag is a documentation example, not a verified pull in this assessment. See [Garage deployment](https://github.com/deuxfleurs-org/garage/blob/main-v2/doc/book/cookbook/real-world.md) and [compatibility matrix](https://github.com/deuxfleurs-org/garage/blob/main-v2/doc/book/reference-manual/s3-compatibility.md).

MinIO's current README directs users to AIStor alternatives and states historical binaries receive no updates. A free AIStor license is different from AGPL community licensing and requires separate terms review. Do not infer maintained open community distribution from an old `minio/minio` image. Keep ROM's historical MinIO verifier intact as historical evidence. See [current MinIO distribution statement](https://github.com/minio/minio/blob/master/README.md) and [existing ROM verifier](https://github.com/pmikstacki/ROM/blob/d7ef529/crates/rom-blob-object-store/verify-s3).

## Required qualification and remaining gates

Run public-consumer tests for create, conflicting duplicate, concurrent same-key creation, bounded read, HEAD size, missing object, deletion, invalid credentials, and restart with unchanged storage. For concurrent creation, exactly one operation may publish; all others must conflict without overwriting the winning bytes. Keep tests on the published adapter rather than reproducing its logic.

Record the exact server version, image digest or build checksum, bucket modes, client lockfile, published ROM revision, command, and result. Never print secret environment values, signed requests, credential-bearing URLs, or object payloads in diagnostics. These are proposed evidence controls, not executed results.

An HTTP loopback test does not prove cloud TLS verification. A normal restart does not prove crash durability or lost PUT acknowledgement recovery. A successful create does not prove BlobService attachment, authorization revocation, orphan grace, or safe cleanup. Those remain separate required gates.

Use the existing public `BlobService` for lifecycle scenarios with SQLite and redb. Preserve create-only content publication and conditional Resource attachment as separate operations. Real local S3-compatible qualification supports only the exact tested product/profile; it does not establish Amazon S3 or all compatible services.


## Root candidate artifact check

After this source assessment, the root pulled `docker.io/chrislusf/seaweedfs:4.48` successfully.
The isolated version command used no network and returned exit 0 with `version 30GB 4.48 530be3e37 linux amd64`.
[Selected artifact facts](../verification/seaweedfs-candidate-2026-10-08.json) record image ID, repository digests, actual selected digest, and command.
This checks the downloaded artifact's reported version only. No S3 server, bucket, credentials, persistence, or conformance was exercised.
Exact mini startup and provider qualification remain the next acceptance steps.
