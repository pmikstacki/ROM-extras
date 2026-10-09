# Sourced import transport and recovery research

Date: 2026-10-09. Source review only. No builds, provisioning, or fixture changes occurred.

The public ROM authority is commit `d7ef529040eec60dc869034c2d33130219db85fe`. The reqwest authority is version 0.13.5, source commit `de55373434f07f42926599dbb5a88550d8e55112`, identified by its published crate metadata.

## Conditional HTTPS transport

Prepare a fixed host-supplied URL. Reject non-HTTPS schemes, userinfo, fragments, and absent hosts. Do not discover URLs from input documents. Keep credentials in explicit host-only headers. Request preparation must not fetch or activate from environment configuration.

The async client supports `https_only(true)`, `no_proxy()`, `redirect(Policy::none())`, finite `connect_timeout`, `timeout`, and `read_timeout`. Disable gzip, brotli, zstd, and deflate with their `no_*` methods. These calls exist even when their features are disabled. They defend against feature unification enabling decompression. Select HTTP/1 with `http1_only` when simplifying qualification. [Exact client source](https://raw.githubusercontent.com/seanmonstar/reqwest/de55373434f07f42926599dbb5a88550d8e55112/src/async_impl/client.rs).

Disable retry with `retry(reqwest::retry::never())`. This disables the retry policy rather than assuming GET is harmless or that defaults never retry. [Exact retry source](https://raw.githubusercontent.com/seanmonstar/reqwest/de55373434f07f42926599dbb5a88550d8e55112/src/retry.rs).

Use an optional explicit `If-Match` header containing one strong quoted entity-tag. Reject weak tags, `*`, multiple tags, and control characters for this narrower host contract. RFC `If-Match` itself permits a list or wildcard; that broader syntax is not needed here. Strong comparison evaluates representation identity. It does not authenticate provenance. [RFC 9110 If-Match](https://www.rfc-editor.org/rfc/rfc9110.html#section-13.1.1).

Only actual HTTP 200 supplies a candidate document. Reject 204, 206, 304, redirects, and all error statuses. Do not use `is_success` or `error_for_status` as the complete acceptance condition. Check status before reading error bodies. Send `Accept-Encoding: identity`; also reject any unsupported nonidentity Content-Encoding. Digest exact body bytes after HTTP framing removal. Do not use `text()` or automatic charset conversion. [Response methods](https://raw.githubusercontent.com/seanmonstar/reqwest/de55373434f07f42926599dbb5a88550d8e55112/src/async_impl/response.rs), [HTTP status semantics](https://www.rfc-editor.org/rfc/rfc9110.html#section-15).

For body admission, stream `Response::chunk()` and reject checked aggregate length before appending. `Content-Length` provides an early rejection only; actual streamed bytes remain authoritative. A chunk already exists before the application examines it. Therefore a Vec limit does not establish a strict cap on every internal transport allocation. Avoid unbounded `bytes()` collection.

`http1_max_headers(n)` limits HTTP/1 header COUNT; its documented default is 100. `http2_max_header_list_size(bytes)` is available with HTTP/2. No public HTTP/1 response-header byte-cap builder was found in this version. Checking header bytes after receipt limits retained/accepted metadata, not parser allocation. Do not claim an arbitrary hostile-response total-memory cap from these APIs. [Client header limits](https://raw.githubusercontent.com/seanmonstar/reqwest/de55373434f07f42926599dbb5a88550d8e55112/src/async_impl/client.rs).

Wrap request plus streaming body in one explicit operation deadline/cancellation scope. Per-read timeout alone resets between reads. Dropping an async future cancels the caller's wait; it does not prove every DNS or operating-system task stops immediately. Bound concurrent operations independently. Classify errors into static safe variants. Do not return reqwest Display strings, URLs, headers, or response bodies. `Error::without_url` removes the URL, but a finite error classification gives a stronger disclosure contract. [Error source](https://github.com/seanmonstar/reqwest/blob/de55373434f07f42926599dbb5a88550d8e55112/src/error.rs).

The blocking client is available, but its construction panics inside an async runtime. Its Response Read implementation applies wait timeouts on reads. Use the async client for cancellation-aware HTTP instead of mixing blocking reads into an async operation. [Blocking client](https://raw.githubusercontent.com/seanmonstar/reqwest/de55373434f07f42926599dbb5a88550d8e55112/src/blocking/client.rs), [blocking response](https://github.com/seanmonstar/reqwest/blob/de55373434f07f42926599dbb5a88550d8e55112/src/blocking/response.rs).

## Reuse and module boundary

Local `rom-map-http` owns map URLs, provider intervals, map RequestContext, and special HTTP400 admission. Its public reads do not expose conditional headers or an exact HTTP200 contract. Forcing imports through it would change guarantees or import map policy into a maintenance package. Projection write transport must remain unchanged.

A small import transport module can directly own its narrower read contract. If multiple consumers later share identical guarantees, extract a neutral bounded HTTP reader with explicit request/status admission. Do not introduce a generic abstraction merely to share builder lines. This follows the adopted [ROM module and DRY rules](/root/ROM/docs/quality.md). This is a design recommendation from source review, not an implemented refactor.

## Host-provided regular file

Accept an already opened `std::fs::File`. Check `file.metadata()?.is_file()` before reading. Refuse directories, pipes, and devices. This checks the opened handle; it requires no path discovery, unsafe code, or dependency. Opening a FIFO can already block in the host before this helper receives it. [File metadata](https://doc.rust-lang.org/std/fs/struct.File.html#method.metadata), [regular-file classification](https://doc.rust-lang.org/std/fs/struct.Metadata.html#method.is_file).

Rewind the cursor with `Seek::rewind()` before reading. Prefer owned File to reduce ambiguous shared-cursor use. Cloning a File can share its underlying cursor; it does not create immutable input. [Seek rewind](https://doc.rust-lang.org/std/io/trait.Seek.html#method.rewind), [File clone](https://doc.rust-lang.org/std/fs/struct.File.html#method.try_clone).

Compute checked `limit + 1`; reject arithmetic overflow. Read through `Read::take` and reject a result longer than `limit`. Compare an explicitly supplied expected digest when the host requires immutable content. Read failures, shorter content, or changed digest must not apply a candidate. Metadata length alone cannot prove content completeness. [Bounded Read adapter](https://doc.rust-lang.org/std/io/trait.Read.html#method.take).

An open File does not prevent concurrent modification. Before/after size or modification-time checks can detect some changes, but not all equal-length modifications. A digest binds the bytes actually read. It does not prove a coherent snapshot unless matched against a trusted expected digest or supplied immutable host snapshot. [File mutation warning](https://doc.rust-lang.org/std/fs/struct.File.html).

Regular file I/O is synchronous. Standard File exposes no portable hard syscall deadline. A worker and cancellation flag can check between reads; timeout cannot stop a syscall already blocked on filesystem I/O. `take` bounds bytes, not elapsed time. Document this limit instead of claiming cancellation forcibly terminates the filesystem operation. No new dependency or unsafe code is needed for the bounded-byte contract.

## Public ROM Unknown and restart qualification

`Sqlite::inject_fault` is public behind `test-support`. Points 1 through 4 fail before native commit with NotCommitted. Point 5 commits successfully and loses acknowledgement, returning Unknown. `on_commit` observes write ordinals, zero before commit, and usize::MAX after commit. Use point 5 for acknowledged-unknown exact retry; use a child process interruption for real process-restart evidence. [Fault API](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-sqlite/src/store.rs), [actual commit behavior](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom-sqlite/src/persistence.rs).

A generic `Storage` wrapper can return Unknown before delegating or after a successful inner commit. Label these different fixture cases accurately. Before delegation establishes no commit for that fixture; Unknown itself conveys no such guarantee to the caller. After delegation simulates lost acknowledgement. It must not write another receipt or ledger.

The minimal required trait methods are capabilities, load, snapshot, receipt, and commit. Transparent wrappers must also forward acquire_owner, retry_epochs, register, query_read, reaction support/update/records, operator support/snapshot/control, and journal support/head/read. Defaults can silently turn a capable inner adapter into unsupported or legacy behavior. Forward all public Storage methods to preserve the contract. [Exact Storage trait](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/persistence.rs).

For child-process restart, persist trusted original action input and public host binding before submission. Include target expected revision, operation/action name, idempotency, original retry epoch, complete provenance, dependency, expiry, and stable actor scope. Store this record privately in a trusted fixture directory with bounded encoding and restrictive permissions. It is recovery material, not a browser-deserializable grant. On restart, the trusted host validates it and reconstructs SourcePermit through `trusted`; it still applies current authorization. Do not serialize private SourcePermit fields for introspection. [Public source seam](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/source.rs).

Exact retry must use the prepared Invocation and original provenance. Do not refetch or derive a replacement identity after Unknown. Verify one durable change, one receipt, surviving provenance, and no duplicate effects after reopening the real storage file. If current authority rejects replay, preserve the unknown classification; rejection does not prove rollback. [Identity](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/invocation.rs), [replay](https://github.com/pmikstacki/ROM/blob/d7ef529040eec60dc869034c2d33130219db85fe/crates/rom/src/replay.rs).

## Qualification gaps

Required native cases include strong-validator mismatch, 200/204/206/304 distinction, redirect refusal, untrusted TLS, explicit authentication, oversized/chunked/truncated bodies, encoded responses, timeout, cancellation, and header-count failure. File cases include nonzero starting cursor, directory/pipe refusal, growth, truncation, expected digest mismatch, and invalid UTF-8. A stable byte digest alone does not establish source authenticity or input-to-output attribution. These are proposed checks; none ran during this research.
