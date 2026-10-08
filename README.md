# ROM-extras

Database providers and optional integrations for [ROM](https://github.com/pmikstacki/ROM).
ROM-extras preserves the Resource → action → atomic commit → event contract.

Development has started. No database provider is supported yet.
Implemented increments include a bounded SQL connection executor and [explicit OIDC issuer presets](docs/oidc-presets.md).
The delivery core prepares bounded JSON and Standard Webhooks signatures.
[HTTPS transport](docs/webhooks.md) has real receiver tests and durable Runtime integration on SQLite and redb.
See [the complete goal](docs/goal.md), [the implementation plan](docs/superpowers/plans/2026-10-08-rom-extras.md), and [support status](docs/support.md).

## Verify locally

Use Rust 1.99, a native linker, CMake, OpenSSL, and Node.js. OpenSSL generates ephemeral synthetic test signing keys.
Run:

```sh
./scripts/check
```

GitHub hosts source code. GitHub Actions is disabled.
