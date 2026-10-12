# SurrealKit beta dependency profile

Exact source: crates.io `surrealkit 1.0.0-beta.1`, corresponding to upstream
[`v1.0.0-beta.1`](https://github.com/surrealdb/surrealkit/tree/v1.0.0-beta.1).
`Cargo.toml.orig` and `LICENSE` retain the upstream manifest and license. Source,
templates and tests are unchanged. The workspace lockfile is the sole lock authority.
Registry archive SHA-256: `490f79e0c5b18f4f0be7d2a53c0d3b50a83722d153e82d68f9365d60f120dce7`.
Upstream packaged commit: `3eb32d4d6f144044812804cc421ed73580ff66f6`
(`.cargo_vcs_info.json`).

The upstream changes are confined to the normalized manifest:
disable SDK defaults and select `protocol-http` without `jwks`. The repository's
development component explicitly adds its qualified gRPC/HTTP/Rustls SDK features.
The workspace's patched SDK/RPC 3.3.0 remains selected.
Pin `surrealkit-macros` exactly to `=1.0.0-beta.1`; upstream's compatible
prerelease requirement otherwise selected beta.6 despite the beta.1 tool target.

At SDK 3.3.0, `jwks` enables `surrealdb-engine-local/jwks`, pulling the embedded
engine and complete core even for a remote-only application. SurrealKit's default
features being disabled does not remove that unconditional dependency feature.
The SDK's remote `method/signin.rs` dispatches through `router.engine.signin`;
remote gRPC/HTTP authentication has no `jwks` conditional. JWT validation is the
remote server's responsibility. This change removes an unused embedded execution
closure; it does not alter SurrealKit's public API, client authentication or tester
implementation. No embedded-engine support is claimed for this integration.

Initial compilation with the upstream feature closure failed in diskann 0.56.0
on the pinned toolchain. See managed run `20261012T184026.009Z-74e869` for the
original failure. The narrowed profile must pass its own compilation and actual
owned-remote authentication/schema/test controls before being called Tested.
