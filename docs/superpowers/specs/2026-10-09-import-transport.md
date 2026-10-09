# Sourced import acquisition and recovery

This implements Task 2 of the sourced-import plan. The approved Task9 maintenance scope remains unchanged.

## Architecture

Add `rom-import-transport`, with default regular-file admission and optional `http` feature.
Reuse `rom-import` as the only JSON, digest, source permit and request builder.
Expose `ActionPlan::max_document_bytes()` so acquisition honors the host's tighter byte budget before parsing.
Do not force imports through map errors/capabilities or weaken projection HTTP writes.
Their response/status contracts differ. Sharing only client builder lines does not justify a generic facade.

`prepare_file(plan, File)` consumes a host-opened regular file, checks metadata, rewinds and reads at most limit+1 bytes.
It refuses nonregular handles and oversized documents. Exact expected SHA binds actual bytes to the approved representation.
The host owns path selection, opening and authorization. There is no discovery or implicit source deletion.
Standard filesystem calls can block; this API has no hard syscall deadline or cancellation guarantee.
A descriptor can refer to mutable content; the expected digest is required, not a snapshot inferred from metadata.

`HttpConfig::new(endpoint, agent, interval, concurrency)` validates one fixed HTTPS URL without userinfo, query or fragment.
It rejects control/whitespace, backslashes and encoded/dot path ambiguities. No browser descriptor is returned.
An explicit bearer header and bounded private CA may be configured. Construction performs no network request.
`HttpSource::new` uses reqwest0.13.5 rustls, no redirects/proxy/retry/decompression, HTTP1 only, 32 header-count maximum, 3-second connection and 60-second client timeout.
Each `fetch(plan, optional StrongEtag, RequestContext)` requires a 1ms–60s total operation budget and host cancellation watch receiver.
True cancellation or a closed host channel cancels admission, rate waiting, sending and body collection.
Concurrency admission is fail-fast, 1–8 operations. The configured minimum request-start interval is 0–60 seconds.
There is no automatic retry, cache or environment-selected source.

Only HTTP200 can supply a candidate. Refuse 204/206/304/redirect/error statuses without consuming error bodies.
404 is Missing; 412 is PreconditionFailed; 429 has an optional bounded integer Retry-After hint, up to 3600 seconds.
5xx is Unavailable; other statuses are Rejected. No response message, URI or credential enters errors.
Accept only application/json, optionally a UTF-8 charset. Refuse duplicate relevant headers and nonidentity Content-Encoding.
Send Accept-Encoding:identity and never reserialize or transcode before core digest checking.
If-Match supports exactly one visible-ASCII strong quoted tag, at most256 bytes. Wildcard, weak/list/control syntax is refused.
For conditional imports require the same single ETag echoed by the response. This is a conservative import profile.
ETag does not authenticate the producer. The expected digest remains necessary.

Check declared and streamed body lengths before accumulation. Retained response-header admission is8KiB after parsing.
Header-count and postparse-byte checks do not prove a strict all-allocation cap in reqwest/hyper.
Dropping an async request cancels the caller's wait; it does not prove immediate termination of all resolver/system work.
Debug and errors are finite and redacted; explicit request/permit data still requires host protection.

## Qualification

Admission tests cover regular files, rewind, tighter limits, oversized/truncated/changed/invalid bytes, directory/device/socket refusal and private diagnostics.
An independent consumer exercises actual authored TLS HTTP framing, authentication, validators, status distinctions, redirection refusal, compressed/chunked/truncated bodies, header limits, timeout, cancellation, pacing and fail-fast concurrency.
A native Nginx profile independently checks static JSON, native strong validators, missing/auth-denied files and restart persistence.
Authored protocol fixtures are separate evidence from native Nginx and actual storage.

Use public SQLite test-support fault5 and redb after-commit observer for actual committed-then-Unknown attempts.
Persist the original host-authorized bytes and binding privately before submission. Reconstruct through public trusted constructors after actual child-process exit.
Never deserialize private SourcePermit fields or refetch after Unknown. Verify unchanged native row/event/receipt counts on exact replay and surviving provenance.
Fault injection is qualification-only, not a production API dependency. Unknown recovery remains subject to current authority and grant checks.

SQL archives, migrations and external object manifests remain pending Task3. No production service or Nominatim deployment is authorized by this increment.
