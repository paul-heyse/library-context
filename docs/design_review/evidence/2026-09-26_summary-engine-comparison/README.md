# W12 finite-base engine comparison

**2026-09-26 · Tested for the bounded relation below.** This isolated probe
compares pinned `ascent 0.8.1`, `datafrog 2.0.1` and `petgraph 0.8.3` on the
same attributed caller→callee state. It does not read `.claude/skills/` at
runtime or join the product Cargo workspace. Ascent's unused default parallel
feature is disabled in the probe manifest.

The state has a finite cited base at function 1, a `1 ↔ 2` recursive SCC,
caller 0, a base-free self-recursive function 3 and its caller 4, plus an
independent base at 5. All three engines derive exactly
`{(0,101),(1,101),(2,101),(5,105)}` as `(function, base origin)`. Reversing
both input vectors leaves the result unchanged; removing every base leaves
the relation empty. The absence for 3/4 is **unknown `call_transfer`** under
the declared candidate state, not a refutation. A deliberately exhausted
SCC pair-work cap produces **unknown `budget_reached`**, not an empty proof.

Run from the repository root:

```sh
CARGO_TARGET_DIR=/home/paul/library-context/build/evidence-target cargo run --locked --manifest-path docs/design_review/evidence/2026-09-26_summary-engine-comparison/Cargo.toml --quiet
```

**Passed 2026-09-26:** `agreement={(0, 101), (1, 101), (2, 101), (5, 105)}
full_work=4 capped_work=1 cap=budget_reached`. The isolated target is under
ignored `build/`; no binary or environment is committed.

The probe isolates one finite-base reachability relation. Ascent's declarative
rule and datafrog's semi-naive keyed join compute that relation with little
code, but neither result carries the product's ordered source proof, BDD
conjunction, per-origin refusal or work accounting. The petgraph SCC-local
worklist can place those checks at each transfer and stop before publishing a
false positive. This supports a **narrow native choice for the first value
channel**, with reevaluation when effect, exception and role channels share a
genuine multi-relation recursive rule. No runtime performance comparison is
claimed from this tiny state; the fresh pilot remains the cost test.

API basis: the repository's pinned `rust-reasoning` skill, Ascent's
[`ascent_run!` documentation](https://github.com/s-arash/ascent/blob/master/README.MD),
and the checked-in exact-version source used by the isolated Cargo lock.
Context7 resolved Ascent as `/s-arash/ascent` on 2026-09-26; its Datafrog search
did not resolve the Rust crate, so the pinned local source supplied that API.
