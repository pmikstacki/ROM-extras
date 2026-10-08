# Garage conditional create assessment

Evidence category: primary-source review, 2026-10-08. No fixture was launched by this research agent.

## Decision

Do not select Garage v2.4.1 for the unchanged public ROM S3 adapter's conditional-create qualification. The inspected PUT path does not enforce `If-None-Match: *`. This is a source-based incompatibility finding, not an executed failure.

## Exact source

The annotated tag `v2.4.1` resolves to commit `268334bd2530fa99f8b06c7383b2e9f776691edd`. The tag object is `cdc4ca791f1b0cdc8c93d2ba5a9db797c10143ad`, dated 2026-09-08. Its release note describes a startup discovery panic fix. The API reports an unknown signing key; this review did not verify the signature. [Tag metadata](https://api.github.com/repos/deuxfleurs-org/garage/git/tags/cdc4ca791f1b0cdc8c93d2ba5a9db797c10143ad)

The server authenticates requests, resolves the bucket, checks authorization, and processes CORS before endpoint dispatch. `Endpoint::PutObject` calls `handle_put` directly. This inspected dispatch has no conditional-create guard. [Dispatch source](https://github.com/deuxfleurs-org/garage/blob/268334bd2530fa99f8b06c7383b2e9f776691edd/src/api/s3/api_server.rs#L106)

`handle_put` processes metadata, checksums, encryption, and the request body. It then calls `save_stream`. Neither function parses `If-None-Match` or rejects an existing object on that condition. `extract_metadata_headers` preserves content metadata and selected `x-amz-*` headers; it does not enforce write preconditions. [PUT source](https://github.com/deuxfleurs-org/garage/blob/268334bd2530fa99f8b06c7383b2e9f776691edd/src/api/s3/put.rs#L62), [metadata helper](https://github.com/deuxfleurs-org/garage/blob/268334bd2530fa99f8b06c7383b2e9f776691edd/src/api/s3/put.rs#L666)

`save_stream` reads the existing object and generates a new version timestamp. Both inline and block-backed paths insert object versions without a create-if-absent condition. Thus this path does not establish atomic conditional creation, even before testing concurrency. [Storage path](https://github.com/deuxfleurs-org/garage/blob/268334bd2530fa99f8b06c7383b2e9f776691edd/src/api/s3/put.rs#L147)

## Image, license, and evidence limits

The official deployment instructions identify `dxflrs/garage:v2.4.1`. The image uses `FROM scratch` and includes `/garage`. The crate declares `AGPL-3.0`. These facts do not establish conditional-create compatibility. [Deployment](https://github.com/deuxfleurs-org/garage/blob/v2.4.1/doc/book/cookbook/real-world.md), [Dockerfile](https://github.com/deuxfleurs-org/garage/blob/v2.4.1/Dockerfile), [Manifest](https://github.com/deuxfleurs-org/garage/blob/v2.4.1/src/garage/Cargo.toml)

The official mirror was not archived and reported activity on 2026-10-08 during inspection. This is limited maintenance evidence, not a support guarantee. [Repository metadata](https://api.github.com/repos/deuxfleurs-org/garage)

The parent separately completed an image pull. This report does not record its digest or treat that pull as runtime qualification. No fixture commands are proposed for acceptance. Preserve the separate SeaweedFS failed profile and its evidence.

## Next source audit

RustFS is a candidate for research only. Its official compatibility matrix and current replication-check documentation describe conditional writes. A historical multi-endpoint conditional PUT issue was closed on 2026-02-10. Closure alone does not verify a released fix. Trace a pinned release's lock, precondition check, and commit before selecting a fixture. [Compatibility matrix](https://docs.rustfs.com/en/reference/s3-compatibility), [Replication check](https://github.com/rustfs/rustfs/blob/main/docs/operations/replication-check.md), [Issue 1659](https://github.com/rustfs/rustfs/issues/1659)
