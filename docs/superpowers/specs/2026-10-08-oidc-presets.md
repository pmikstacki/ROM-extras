# OIDC issuer presets

This family increment uses published ROM `d7ef529040eec60dc869034c2d33130219db85fe`.
It provides configuration presets, not a second token verifier or a browser login implementation.

`IssuerPreset` retains an exact issuer string. `configure` delegates to `rom_auth::OidcIdTokenAdapter::configured`.
The host supplies an authority namespace, client ID, and issuer-bound TrustedKeys source.
No token or request can select the issuer, client ID, or key endpoint.

## Researched choices

[OIDC Discovery](https://openid.net/specs/openid-connect-discovery-1_0.html) requires HTTPS issuer URLs without query or fragment components and identical issuer binding.
Validate structure with [url 2.5.8](https://docs.rs/url/2.5.8/url/struct.Url.html), but retain the original issuer bytes for ROM's exact comparison.
Reject whitespace, controls, backslashes, credentials, and nonabsolute authority syntax before accepting a URL.
Limit issuer length to ROM's 2048-byte bound.

[Keycloak](https://www.keycloak.org/securing-apps/oidc-layers) uses the realm path under its configured deployment base.
Preserve the base context path. Append one realm segment; initially accept ASCII unreserved realm names up to 256 bytes, excluding dot segments.
Other realm names can use an explicit exact issuer rather than implicit encoding.

[Microsoft Entra](https://learn.microsoft.com/en-us/entra/identity-platform/v2-protocols-oidc) has tenant-specific v2 issuers.
Accept one concrete GUID tenant for the public cloud preset. Reject multi-tenant aliases.
Use explicit exact issuers for sovereign clouds and other providers, including Dex and Auth0.
This deliberately narrower preset does not implement multi-tenant issuer substitution.

## Verification and scope

Test exact-string preservation, invalid URL forms, bounded input, context paths, concrete tenants, and no key fetch during configuration.
Then use signed synthetic ID tokens through the returned ROM adapter to check expiry, issuer/audience/nonce mismatch, rotation, and human principal kind.
Do not claim live identity-provider support from synthetic tests.
Live provider login and host state/nonce consumption remain whole-goal acceptance work.
ROM owns cryptographic verification and its fixed RS256 authorization-code human profile.
The host still owns discovery trust, bounded acquisition, state/code/nonce one-use, actor expiry, and current authorization.
