# Native schema development worker

Development-only Rust integration with SurrealKit `1.0.0-beta.1` and the patched
SurrealDB SDK/RPC `3.3.0`. The product does not depend on this component. The
[dependency profile](../../third_party/surrealkit/README.lctx.md) explains the
manifest-only removal of unused embedded-engine features.

The native Rust owners produce the schema. `schema/` contains derived assets for
the beta macro, in native installation order. Never edit those files by hand.

```sh
cargo run --release --locked -p lctx-surreal-dev -- generate
cargo run --release --locked -p lctx-surreal-dev -- generate --check
cargo nextest run --release --locked -p lctx-surreal-dev
```

The generator compares complete file membership and bytes. Its ownership marker
limits regeneration/removal to this component's derived directory. `build.rs`
tracks that directory, so additions and removals rebuild macro embedding.
Normal embedded Sync requires neither a writable checkout nor introspection to
recover source authority.

The internal `lctx-surreal-dev build-info` response needs no binding or service
connection. It reports the resolved beta/SDK/RPC versions and the selected
workspace patch declarations from the build's lockfile and manifests. Patch
manifest/documentation digests identify declared build inputs; they do not
qualify the patches or identify a running server. The compiled generated-schema
profile identity uses the same ordered embedded inventory as test projections,
and the response refuses if those bytes differ from compiled native authority.

Before launching a worker, the host captures this response with that executable's
SHA-256 and a timestamp in its existing development ledger. Passive inspection
reads the ledger and current managed-run cleanup; it never executes a worker or
connects to a database. Older ledgers show missing provenance, and a mismatched
worker digest shows unknown provenance. Command exit, capture, child cleanup,
credential restoration, remote uncertainty and whole-target disposal remain
separate observations. Server generation/capabilities come from the existing
installation descriptor; `just service status` supplies current host observation.

## Parent-owned operations

The host service/run adapters authenticate the exact lifecycle campaign and own
its executable, process, datastore, worker, project and report before invoking
the worker. They issue a private binding; the Rust worker has no endpoint or
credential fallback. Its directly owned Any session checks the actual validation
generation before selecting a fresh `lifecycle_…` database. It never applies
schema to `main` or `validation`.

With `LCTX_SURREAL_SERVICE_CONFIG` selecting the owned campaign's installation
file, use the existing host entrypoint:

```sh
just surreal-dev sync --installation ID
just surreal-dev test --installation ID
just surreal-dev test --installation ID --project /absolute/test-input-project
just surreal-dev inspect --installation ID
just surreal-dev inspect --installation ID --indexes
```

The host invokes `test --prepare-project`, adding `--sample-suite` when no test
input project is supplied. The worker's direct binding interface is internal to
that owner; standalone `project` only prepares schema files in an already owned
empty project and grants no target authority.

`project` writes the beta tester's default-module projection, including all
ordered dependencies, and a source/inventory manifest. `--modules` selects
`control`, `canonical`, `search`, `compiler` or `completion`; omission means all.
Unknown names and stale projections refuse before test execution. The sample
requires `completion` and its dependencies (or the default full inventory). It
exercises actual native physical fields, nested functions, type refusal,
record permissions and server-issued token authentication. Its token input is
private. Before substitution, the worker durably records the exact original
marker input in a private recovery journal. It restores the input on normal and
early-error exits; the parent restores it after confirmed child drainage,
including cancellation. Checked campaign disposal repeats that recovery through
the existing development ledger before confirming credential removal. Reports
and source projects are preserved; journal-absent projects are not rewritten.

| Operation | Targeting and evidence |
|---|---|
| Generated embedded Sync | Explicit responsibility/dependency selection; pruning/fail-fast explicit; independent readback of every required native definition. |
| Public beta declarative tester | Default-module projection only; explicit remote root configuration; ordinary available suite parallelism; `fail_fast=false`, `keep_db=true`; JSON report and required-definition postflight. |
| Native product installation | Existing native installer/maintenance owner; this worker grants no installation, publication or admission authority. |
| Watch, seed, typegen, rollout, API suites | Not exposed by this adapter. Their distinct targeting and lifecycle contracts require a concrete consumer. |
| Embedded engines, cloud/TLS endpoints | Not supported by this remote campaign profile. |

The beta tester privately creates sessions and random suite databases. Its exit
and JSON report establish test outcome, not their drainage or disposal. The host
owns the entire service inventory even when setup fails before a report exists.
Only confirmed whole-target drainage and the existing storage owner permit
datastore discard. Logical DROP is deliberately not attempted (`keep_db=true`);
only whole-target disposal retires these campaign databases. No shared service is a scratch fallback.

These controls establish schema/tooling behavior. They do not establish product
publication, arbitrary network-interruption finality, measured performance or
operator adoption. The [Catalog execution coordinator](../../docs/plans/catalog-creation-plan_2026-10-14.md)
owns current acceptance evidence. [Negative input projects](tests/fixtures/README.md)
exercise malformed input, failed assertions, refused actor authentication and
parent-owned cancellation without borrowing another target.
