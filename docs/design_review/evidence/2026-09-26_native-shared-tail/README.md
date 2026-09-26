# Native condition catalog shared-tail probe

**2026-09-26 · Tested.** `catalog_probe.rs` constructs two bounded BDDs from
`truthy(a) & truthy(z)` and `truthy(b) & truthy(z)`. To reproduce, copy it to
`crates/cpg-schema/examples/catalog_probe.rs`, run
`cargo run -p cpg-schema --example catalog_probe --quiet`, then remove that
temporary example. The resulting content-addressed rows are embedded in
`python/lctx_mcp/tests/test_native_semantics.py`.

Both conditions retain the same `truthy(z)` node. Their union has three stored
nonterminal rows, while native hydration owns four nonterminal nodes across
the two condition roots. The targeted native test passed with
`uv run --no-sync pytest python/lctx_mcp/tests/test_native_semantics.py::test_native_catalog_distinguishes_stored_from_retained_shared_tail -q`.
This tests the exposed diagnostics for a shared tail; the aggregate admission
limit itself is covered by the Rust catalog-limit test, not by this tiny probe.
