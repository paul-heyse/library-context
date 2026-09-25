# biodivine-lib-bdd 0.6.3 decision APIs the kernel does not use yet, 2026-09-25

**Question.** Can the condition kernel (`crates/cpg-schema/src/condition_kernel.rs`) decide
compatibility and implication without building the result, bound work exactly, shrink declared
support and factor by restriction, using only the pinned 0.6.3 API?

**Consumer.** The [library-fit review](../../reviews/design_review_library-fit-reasoning_2026-09-25.md)
F01–F03; `compatible`/`implies` at `condition_kernel.rs:638-664`, the primitive theory at
`primitive_theory.rs:230-252`.

**Run.** `cargo run --release --offline` in `probe/` (a standalone crate with its own
`[workspace]`; `Cargo.lock` is the resolved set; toolchain 1.98.1). The fixture `left` is the
OR of 16 variable pairs in separated order (the B009 blow-up shape), 131,072 nodes.

**Output (2026-09-25, this machine, rerun by the reviewer):**
```
sizes left=131072 right=4 and=98305
limit1 compatible (sat) -> None
limit 1000 -> None
limit1 unsat -> Some(true)
check_binary_op sat -> Some((true, 98303))
check_binary_op unsat -> Some((false, 131070))
check_binary_op limit 10 -> None
pair product = 524288
implies a=>left limit1 -> Some(true)
implies left=>a limit1 -> None
support_set after x&!x | y = 1 num_vars=32
restrict size 65536 <= 131072
is_clause a=true left=false
most_free_clause left=Some(17)
```

**What it shows.**
- `binary_op_with_limit(1, a, b, and)` returns `None` for a satisfiable pair whose full result
  has 98,305 nodes, and `Some(false)` for a contradiction. A node beyond the terminal is never
  the false function, so `None` proves non-emptiness: compatibility and non-implication are
  decided without building the result. Upstream `Bdd::cmp_implies` uses the same idiom
  (`_impl_sort.rs:37-56` in the pinned source).
- `Bdd::check_binary_op(limit, …)` returns `(non_empty, tasks)` without allocating a result. For
  `left ∧ ¬left` the tasks are 131,070 against an input-node product of about 1.7·10¹⁰, which the
  kernel's `MAX_PAIR_WORK` preflight (1,000,000) would refuse. A task limit of 10 returns `None`.
- `support_set()` of `(x ∧ ¬x) ∨ y` has one variable although the diagram lives in 32.
- `restrict` of `left` under two literals is 65,536 nodes, bounded by the input.

**Controls.** Each positive has its opposite in the same run: satisfiable vs contradictory pair,
implied vs not implied, task limit hit vs not hit.

**Not shown.** Wall-clock bounds; pilot frequency of any case; behaviour inside the kernel's
transfer/union path. These need product integration and the pilot.
