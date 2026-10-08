# RustFS conditional create assessment

Evidence category: primary-source review, 2026-10-08. This research agent did not start a server or execute conformance tests.

## Decision and identity

RustFS 1.0.1 is suitable for an isolated single-node qualification attempt. Its inspected normal PUT path checks write preconditions under a namespace guard retained through publication. This finding does not establish the public ROM adapter's three-second or eight-writer acceptance results.

The official latest-release API identifies non-prerelease `1.0.1`, published 2026-10-03. Annotated tag object `01fa3eced9c1a957a2a31405b09993fa8224b275` resolves to commit `6de965ae3c965a78ff819fbcd7acd4aa44177d92`. [Release](https://github.com/rustfs/rustfs/releases/tag/1.0.1), [tag metadata](https://api.github.com/repos/rustfs/rustfs/git/tags/01fa3eced9c1a957a2a31405b09993fa8224b275)

The parent separately pulled `docker.io/rustfs/rustfs:1.0.1`. An isolated version command reported version 1.0.1 and the same commit. This is parent-reported artifact identity evidence, not server qualification. The research agent did not verify its digest.

## Conditional write path

The S3 implementation delegates PUT to the object usecase. The usecase builds write options and calls `put_object_with_old_current_size`. The options helper parses `If-None-Match` into `http_preconditions`. Normal options retain `no_lock=false`, because `ObjectOptions` derives `Default` and the helper does not override this boolean. [S3 dispatch](https://github.com/rustfs/rustfs/blob/6de965ae3c965a78ff819fbcd7acd4aa44177d92/rustfs/src/storage/ecfs.rs#L1533), [usecase](https://github.com/rustfs/rustfs/blob/6de965ae3c965a78ff819fbcd7acd4aa44177d92/rustfs/src/app/object/put.rs#L1626), [header helper](https://github.com/rustfs/rustfs/blob/6de965ae3c965a78ff819fbcd7acd4aa44177d92/rustfs/src/storage/options.rs#L316), [default options](https://github.com/rustfs/rustfs/blob/6de965ae3c965a78ff819fbcd7acd4aa44177d92/rustfs/src/storage/options.rs#L639), [option type](https://github.com/rustfs/rustfs/blob/6de965ae3c965a78ff819fbcd7acd4aa44177d92/crates/ecstore/src/object_api/types.rs#L912)

`SetDisks` first performs an advisory check before reading the full body. This early check alone is insufficient. After staging, the normal path acquires the `put_object_commit` namespace guard and repeats the precondition check under that guard. [Commit lock and recheck](https://github.com/rustfs/rustfs/blob/6de965ae3c965a78ff819fbcd7acd4aa44177d92/crates/ecstore/src/set_disk/ops/object.rs#L4030)

The checker reads object metadata without acquiring another lock. Existing objects that match the condition return `PreconditionFailed`. `If-None-Match: *` matches an existing object's ETag. A missing object permits the write when `If-Match` is absent. Ordinary writes may proceed past a delete marker. [Checker](https://github.com/rustfs/rustfs/blob/6de965ae3c965a78ff819fbcd7acd4aa44177d92/crates/ecstore/src/set_disk/core/io_primitives.rs#L6857), [condition evaluator](https://github.com/rustfs/rustfs/blob/6de965ae3c965a78ff819fbcd7acd4aa44177d92/crates/ecstore/src/set_disk/mod.rs#L7190)

The guard moves into the commit closure rather than being dropped after the recheck. Publication calls `rename_data_owned_with_fence` while the closure owns the guard. A pending rename tail can receive the guard through a later handoff. [Guard transfer](https://github.com/rustfs/rustfs/blob/6de965ae3c965a78ff819fbcd7acd4aa44177d92/crates/ecstore/src/set_disk/ops/object.rs#L4375), [commit closure](https://github.com/rustfs/rustfs/blob/6de965ae3c965a78ff819fbcd7acd4aa44177d92/crates/ecstore/src/set_disk/ops/object.rs#L4414), [publication](https://github.com/rustfs/rustfs/blob/6de965ae3c965a78ff819fbcd7acd4aa44177d92/crates/ecstore/src/set_disk/ops/object.rs#L4603)

The source includes a commit-time conditional recheck test. This review did not execute it. A historical multi-endpoint race report is closed; that status does not qualify distributed operation. [Source test](https://github.com/rustfs/rustfs/blob/6de965ae3c965a78ff819fbcd7acd4aa44177d92/crates/ecstore/src/set_disk/ops/object.rs#L20306), [historical issue](https://github.com/rustfs/rustfs/issues/1659)

## Proposed isolated profile

Official testing guidance specifies at least one CPU core and 1 GB memory for one node. Use one CPU and at least 1 GiB for the proposed container. A 512 MiB cap is below this documented minimum. [Hardware requirements](https://docs.rustfs.com/en/installation/requirement/checklists/hardware-selection)

The official Docker documentation supports one persistent `/data` volume and the `rustfs/rustfs` image. It documents runtime UID 10001. Prepare only the fixture's private data and log directories for that user. Pin the already-inspected version or its separately verified digest. [Docker deployment](https://docs.rustfs.com/en/installation/container/docker), [versioned compose](https://github.com/rustfs/rustfs/blob/1.0.1/docker-compose-simple.yml)

Publish only `127.0.0.1:55456:9000`. Set `RUSTFS_CONSOLE_ENABLE=false`. Do not publish a console or admin port. Use region `us-east-1` to match the proposed host profile. The parent inspected binary help for `/usr/bin/rustfs server --address 0.0.0.0:9000 --region us-east-1 /data`; this report does not claim that server command ran.

Provide generated fixture credentials through a private environment file or readable read-only secret mounts. The exact release supports `RUSTFS_ACCESS_KEY`, `RUSTFS_SECRET_KEY`, and their `_FILE` alternatives. Restrict local files and avoid command-line secret values. Container administrators can inspect environment values; an environment file is not a secret vault. [CLI source](https://github.com/rustfs/rustfs/blob/6de965ae3c965a78ff819fbcd7acd4aa44177d92/rustfs/src/config/cli.rs#L1546)

## Remaining gates

First verify sequential create-only rejection and unchanged content. Then run the unchanged public ROM adapter's strict eight-writer race. Require one accepted result and seven conflicts within the existing deadline. Retain unknown outcomes as unknown. Verify bounded read, head, delete, restart persistence, and lifecycle behavior separately.

The single-node test will not establish distributed locking, cloud TLS, acknowledgement-loss recovery, or crash durability beyond its executed conditions. Keep the SeaweedFS failed profile and Garage source incompatibility evidence. Dependency, image license, and advisory audits remain separate adoption gates; this source review is not a clean advisory assessment.
