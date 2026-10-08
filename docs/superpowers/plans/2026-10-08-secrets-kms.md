# Secrets/KMS execution plan

Execute the [family spec](../specs/2026-10-08-secrets-kms.md) under the already approved full ROM-extras plan.

1. Add failing public-contract tests for validated references, versions, bounded secret buffers and redacted errors.
2. Implement rom-secrets and rom-kms host contracts in named modules with stable facades.
3. Inspect and pin the official OpenBao 2.7.1 image. Prepare private TLS and persistent Raft fixture.
4. Initialize/unseal through private administrative configuration. Provision KV versions, derived Transit key and limited runtime token.
5. Add rom-openbao using existing pinned reqwest, Tokio, base64 and zeroize versions; review dependency delta.
6. Implement bounded transport, approved alias maps, versioned KV reads, and separate context/AAD encryption inputs.
7. Run the real-service acceptance cases from the spec. Preserve failures and exact service configuration.
8. Integrate the public host example and independent consumer. Check no plaintext enters integration-controlled Resource paths.
9. Add required verifier gate, review all changes, freeze runtime sources, and run full verifier.
10. Publish exact scope and evidence on the work branch. Keep other provider profiles and the full goal active.

All decisions use [primary research](../../research/secrets-kms-assessment.md).
Limits are experimental family policy, not vendor limits. The zeroization boundary follows [zeroize source](https://github.com/RustCrypto/utils/tree/zeroize-v1.9.1/zeroize).
