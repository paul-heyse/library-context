# Allocative nightly compatibility backport

Source-reviewed 2026-09-28. Crate archive SHA256:
`d8cf9afc79c83d514444b55df3935d317da54b1ce3b17a133c646889cc260de8`.
 This is crates.io `allocative 0.3.6`, selected through
the root `[patch.crates-io]` for ADR-0136. The duplicate nightly `Allocative for !`
implementation and its now-stable `never_type` feature attribute are removed; on the pinned compiler,
`Infallible` is an alias of `!`. The existing `Infallible` implementation covers both.

This is the exact functional change from the
[upstream fix](https://github.com/facebook/buck2/commit/9711293c6de502d50583cafb12e4a7b764094d3a).
The published crate's manifest, build script and source are copied without other edits.
Upstream license texts are included. Cargo metadata/cache markers and the dependency's
standalone lockfile are not build inputs and are omitted.

Remove this patch when a compatible published release satisfies the existing dependency
constraints. Do not edit the shared Cargo registry cache. This remains outside the workspace
and uses the imported-dependency O3/non-incremental profile.
