# Controlled OIDC host example

This example qualifies a local Keycloak service. It is not a production session implementation.
The Rust preset performs no HTTP requests. The host approves the issuer and acquires its public keys.

`flow.py` uses an explicit literal loopback HTTPS origin and a trusted CA.
This controlled example rejects DNS names and external IP addresses before acquisition.
It does not follow redirects automatically. It accepts only the exact callback URL and retained state.
The host consumes state before it exchanges the authorization code.
A rejected exchange requires a new login. Requests have a 60-second operation deadline and a 24-request budget.
Each request has a five-second inactivity timeout.
A deadline timer interrupts blocked socket reads, including trickled HTTP headers. JSON responses are limited to 65536 bytes; login HTML is limited to 262144 bytes.
There are no automatic retries. A 429 response fails the operation; the host must apply its provider's rate policy.

The example retains tokens only in private fixture files. A production host must supply its own protected session storage.
It must implement logout, session expiry, cancellation, refresh and concurrency control.
It must not accept an Actor, key endpoint or provider configuration from browser input.
The callback fixture has no browser application and does not qualify browser security or production sessions.

## Run the qualification

Configure an isolated Keycloak 26.8.0 fixture at `https://127.0.0.1:55475`.
Use the pinned image `quay.io/keycloak/keycloak@sha256:d79bc4bf1c54e802735ef91926b5c003de1fbb50b1a93382611972277219c9ad`.
The controlled fixture uses persistent H2 data, a trusted test CA, a one-CPU limit and a 1 GiB memory limit.
Its container name must be `rom-extras-keycloak-oidc-20261010` for the guarded restart test.
Publish only `127.0.0.1:55475:8443`. Do not expose the development HTTP listener.
This profile is local qualification, not a production deployment.

Import realm `rom-fixture` with a fixture human user and a public client named `rom-web`.
Enable the client's standard authorization-code flow. Disable direct access grants.
Require `pkce.code.challenge.method=S256`.
Permit only `https://127.0.0.1:55476/callback` as its redirect URI.
Set the realm access-token lifespan to 300 seconds.

The private fixture directory must contain `tls/ca.crt` and a mode-0600 `credentials.json`.
That file contains `admin_username`, `admin_password`, `user_username` and `user_password`.
Use unrelated generated fixture passwords. Do not copy production credentials.
The administrative password grant performs fixture setup only; human qualification uses authorization code and S256-PKCE.

```bash
export ROM_EXTRAS_KEYCLOAK_FIXTURE=/absolute/private/keycloak-fixture
export ROM_EXTRAS_PYTHON=/absolute/path/to/python3
./scripts/check-oidc-native
```

The gate mutates only this dedicated fixture.
It generates a new signing key and disables previous signing keys without deleting their stored configuration.
It restarts the container, confirms unchanged public keys, and performs another code login.
It exercises independent source and normalized archive Rust consumers, with separate retained run directories.

The consumers use `ProviderActivation`, `IdentityGate`, explicit `IdentityLink` and `User` Resources.
They check exact Unicode Resource IDs, denied unlinked identities, forged Actors, expiry and current User disable.
They reopen native SQLite/redb stores and check that credentials did not enter the persisted database.
Injected transport tests are separate from actual Keycloak protocol evidence.
