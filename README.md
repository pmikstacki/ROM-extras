# ROM-extras

Database providers and optional integrations for [ROM](https://github.com/pmikstacki/ROM).
ROM-extras preserves the Resource → action → atomic commit → event contract.

Development has started. No database provider is supported yet.
Implemented increments include a bounded SQL connection executor and [explicit OIDC issuer presets](docs/oidc-presets.md).
The delivery core also prepares bounded JSON payloads and Standard Webhooks signatures; network delivery remains pending.
See [the complete goal](docs/goal.md), [the implementation plan](docs/superpowers/plans/2026-10-08-rom-extras.md), and [support status](docs/support.md).

## Verify locally

Use Rust 1.99, a native linker, CMake, and OpenSSL. OpenSSL generates ephemeral synthetic test signing keys.
Run:

```sh
./scripts/check
```

GitHub hosts source code. GitHub Actions is disabled.
