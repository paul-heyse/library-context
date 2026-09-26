# Primitive predicate Python-lowering control

**2026-09-26 · Tested.** `probe.py` executes a small, fixed matrix of literal
predicates under the repository's CPython 3.14.7 uv environment and writes
`observations.jsonl`. Reproduce with:

```bash
uv run --no-sync python docs/design_review/evidence/2026-09-26_primitive-python-lowering/probe.py > docs/design_review/evidence/2026-09-26_primitive-python-lowering/observations.jsonl
cargo test -p cpg-schema --lib finite_primitive_lowering_matches_recorded_cpython_314_predicates --quiet
```

The Rust test reads those independent CPython observations and checks the
production exact-input atom evaluator. It covers string membership and
equality, non-bool integer equality, truthiness, `is None`, and exact builtin
`type(x) is str`. Both controls passed on 2026-09-26. The two `lowered: false`
rows record deliberate unknowns: CPython evaluates `True in (1,)` and
`True == 1` as true, but the narrow theory does not claim bool/int coercion.

This probes the finite literal lowering only. It does not establish that a
source operand is the queried entry formal, that a generation serves the
right path, or that Q09 is answered operation-wide. Those require the
separate cited-link and served-path controls in the Stage 3 plan.

`native_probe.rs` constructs two evaluated atoms and their content-addressed
catalog rows through production `Diagram` and `IdHasher`. Its output feeds
`test_native_string_membership_and_integer_equality_stay_path_local`, which
loads a synthetic native executor with two independently cited paths. The
targeted PyO3 test passed on 2026-09-26: `stdio` refutes only membership,
integer `3` refutes only equality, `http` remains compatible only on
membership, and `True` leaves both paths unknown. This tests native path-local
semantics, not a Delta generation or structured MCP response. Reproduce the
rows by temporarily copying `native_probe.rs` to
`crates/cpg-schema/examples/native_probe.rs` and running
`cargo run -p cpg-schema --example native_probe --quiet`; remove the temporary
example afterward.
