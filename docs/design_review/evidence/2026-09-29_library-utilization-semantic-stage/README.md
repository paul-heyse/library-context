# Library utilization: the semantic stage (S2)

**Consumer:** `docs/library-utilization.md` (stage S2); `tools/lu-resolve` and
`scripts/library_semantic.py`.

**Question.** Which library item does each workspace reference resolve to, without building the
workspace, on a tree that is mid-refactor? Options weighed: `rust-analyzer scip` (installed), and a
small tool on the `ra_ap_*` crates the rust-code-model skill indexes.

## What was run (2026-09-29, HEAD a738da7 to 818e6a2, working tree changing)

1. `rust-analyzer scip . --output index.scip --config-path ra.json --exclude-vendored-libraries`
   with `{"cargo":{"buildScripts":{"enable":false}},"procMacro":{"enable":false}}`. The run **ignored
   the config**: it ran build scripts (proc-macro2, serde, libc, ...), built proc macros and waited on
   cargo's package-cache lock before indexing. Stopped; not used. `scip` forces
   `load_out_dirs_from_check`, which is a `cargo check`, and indexes every token of every crate.
2. `tools/lu-resolve` (ra_ap 0.0.352, nightly-2026-09-13, salsa held at 0.28.2, unicode-ident at
   1.0.22): `load_workspace_at` with `load_out_dirs_from_check: false`, `ProcMacroServerChoice::None`,
   `prefill_caches: false`, `CargoFeatures::All`, `set_test: true`; then `Semantics::resolve_path` on
   every path node and `resolve_method_call` on every method call in the workspace's own files.

## Result

- 313 files, about 192k nodes, 4,815 (file, library item) records; 80-110 s wall clock, about 5 GB
  resident; no build of the workspace. A run with a method-name filter from the catalog was not
  faster (2:58 versus 1:51 in that pair of runs) and was dropped.
- Findings that changed the tool: `set_test` defaults to `false` in `CargoConfig::default()` and
  leaves every `#[cfg(test)]` body unresolved (locals and local functions came back unresolved in
  a test fn until it was set); `canonical_path` starts below the crate root and keeps `r#` on
  keyword segments (`arrow_ipc::r#gen`); resolution needs `ra_ap_hir::attach_db` around the walk; a
  method that implements a trait method is reported as both `Type::method` and `Trait::method`.
- 250 of 334 catalog items matched exactly at first; after the fixes and evidence-backed
  normalisation the unresolved remainder is proc-macro-only use (`serde_arrow` derive output,
  `sqlx::FromRow`) and a few items the code no longer references.
- Not visible by design: references inside macro expansions and derive output (proc-macro server
  off), and files outside the module tree (`crates/cpg-core/tests/dormant/`).

## Reproduce

```
cd tools/lu-resolve && cargo build --release --offline
cd ../.. && tools/lu-resolve/target/release/lu-resolve . > resolved.jsonl
LU_DEBUG_FILE=<path suffix> tools/lu-resolve/target/release/lu-resolve .   # per-node trace
```
