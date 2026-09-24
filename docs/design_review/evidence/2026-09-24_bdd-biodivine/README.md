# biodivine-lib-bdd 0.6.3 as the condition kernel (P3), 2026-09-24

**Question.** Does biodivine-lib-bdd give canonical forms, exact factoring and compatibility, a
size limit and deterministic rendering, at a cost fit for our conditions?

**Run.** `RUSTUP_TOOLCHAIN=1.98.1 CARGO_TARGET_DIR=target cargo run --release` here (a standalone
crate; `Cargo.lock` is the resolved set). The synthetic stress case (random 32×8 over 40 atoms)
was removed after it did not finish in 10 minutes; `src/main.rs` holds the realistic-size version.

**Output (2026-09-24, this machine):**
```
R9 canonical: equal=true size=7
R9 rendering: [!d, !b, a & c]
two sites satisfiable=true one atom satisfiable=false
F&N == c&N (factor out N): true
compatible(F, !a & !b) = false
limit 1 -> None
20 vars, 16x<=8: 500 built 29.7 ms; 499 limited conjunctions 14.5 ms (max nodes 4,051, over the 50k limit 0); 50 optimized DNFs 77.4 ms (max terms 26)
40 vars, 16x<=8: 500 built 112.0 ms; 499 limited conjunctions 422.5 ms (max nodes 48,952, over the 50k limit 31); 50 optimized DNFs 991.6 ms (max terms 16)
deterministic rendering: true
```

**Conclusion.** Fit, with a node limit and a bounded rendering. Only `num-rational` is new to our
lockfile. Cited by decision D-11 and forward plan Stage 3.0.

## Stage 3.0 contract extension (2026-09-24)

**Tested.** `cargo test --manifest-path
docs/design_review/evidence/2026-09-24_bdd-biodivine/Cargo.toml --lib` passed 4/4 on
Rust 1.98.1. The standalone probe now depends on the workspace's `cpg-schema` and tests:
Stage 2's known normal forms under BDD identity; exact factoring and compatibility; two
independent evaluation atoms; a library node-limit hit; and a structural Merkle id unchanged by
construction order or redundant support. The implementation is in `src/lib.rs`. It is a spike,
not the product's condition authority.

The pinned 0.6.3 source was inspected. `binary_op_with_limit` in
`_impl_bdd/_impl_boolean_ops.rs` uses a memoized pair-of-input-nodes worklist and caps result
nodes. The probe preflights input-node product (1,000,000), limits result nodes (50,000) and
support (64). `BddVariableSet::transfer_from` copies nodes only when variable-name order is
compatible; sorted atom identities keep the relative order. Product code must also preflight
transfer work, use a capped path iterator for display and distinguish preflight refusals from
library node-limit hits. These source properties and tests do **not** prove a wall-clock bound.

**Not yet measured:** product integration, the pilot's new `budget_reached` count and compile
time, typed theory, and persisted-node migration. The decision stays Proposed until those gates.
