# RabbitMQ native TLS verification

Date: 2026-10-08. Base: 2da3342ba37ce9e69f03f337a4a135d8d1738bf4.
The production adapter, package dependencies, features, and both lockfiles are unchanged.
This increment adds a required native TLS fixture and independent consumer conformance.

The separate official RabbitMQ4.3.2 Alpine fixture uses the existing pinned image digest.
Container and retained volume: rom-extras-rabbitmq-tls-20261008. Label: rom-extras.fixture=rabbitmq-tls.
Fixed hostname: rom-extras-rabbitmq-tls. Limits: one CPU,512 MiB, two Erlang schedulers, four async threads.
Loopback ports:55447 AMQPS,55448 management. Listener evidence records5671 AMQP/SSL and no plaintext AMQP listener.
The server leaf has CA:FALSE, serverAuth, and IP SAN127.0.0.1. Separate private fixture CAs supply trusted and unrelated roots.
Synthetic certificate validity: 2026-10-08T17:08:35Z through2026-11-07T17:08:35Z.
Private signing keys are not mounted; the broker receives only CA certificate, leaf certificate/key, and configuration.

The first start failed eacces on configuration because ownership assumed UID999.
Executing id in the pinned Alpine image established broker UID100/GID101. Correct ownership retained0600 file permissions.
The initial failed startup log is preserved. No certificate verification was disabled to resolve this fixture error.
Server configuration does not request client certificates; client-side server verification remains enabled.
This profile authenticates passwords over verified server TLS. It is not mTLS.

Required negatives fail for UnknownIssuer, certificate name mismatch, ACCESS_REFUSED, and InvalidContentType on a plaintext listener.
The positive test obtains a real publisher confirmation for persistent false JSON with unchanged ID.
After labelled fixture restart, a fresh CA/hostname-verified connection reads the retained message.
A new explicitly bound adapter confirms another publication. Both message bodies, IDs, content types, and persistence properties are checked.
The shared restart helper preserves the existing plaintext test behavior and validates the dedicated container/label pair before control.
The independent consumer executes public lapin TLS configuration with the unchanged extras adapter.
Native roots remain enabled alongside the added CA. This is additional trust, not exclusive certificate pinning.

Source review found that independent TLS consumption did not require an amqps URI.
The old test incorrectly passed against the plaintext fixture; the preserved before log demonstrates that evidence gap.
The corrected test rejects that same configuration before connecting with the expected AMQPS fixture diagnostic.
The follow-up review confirmed resolution and found no remaining source-review blockers.
The reviewer did not independently execute the full verifier.

Initial check-all passed before that evidence correction. Final ./scripts/check-all terminal exit:0 after correction.
Final evidence: check-all-rabbitmq-tls-final-2026-10-08.log.
The gate includes workspace/isolated checks, HTTPS/Runtime, real PostgreSQL, MSSQL, NATS plaintext/TLS, RabbitMQ plaintext/TLS, and independent consumers.
Toolchain: Rust1.99, GCC13.3, CMake3.30.5, OpenSSL3.5.8, Node22.16.
Post-command source hashes are a witness, not a complete pre/post release fence.
No dependency adoption occurs; existing advisory/license inventory applies to unchanged graphs and versions.
Secret-value checks examine publication candidates without printing values. All fixture data, queues, certificates, and failure logs remain preserved.

The fixture was running and unpaused after the gate. This establishes a single-node native TLS profile with explicit host reconnect.
Clustered failover, certificate rotation, client-certificate authentication, and packaged release remain open.
The complete extras goal and parent delivery task remain active.
