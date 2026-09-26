# Design review: explicit keyword recursive control

**2026-09-26 · change/conformance · scoped author review.** Core 3.0,
code-intelligence profile 1.1 and the library-context binding in
`standard.toml` apply. The subject is the local-call seed relation, the
existing ADR-0053 finite value worklist, the shared validator and FORMAT 8
serving. [Plan W12](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)
owns general recursive argument substitution.

## Contract and scenario

For `return f(value, stop=True)` and
`return f(value=value, stop=True)`, Ruff supplies two explicit arguments,
ty ties the returned source value to the first argument, and Pass B maps
both arguments definitely to distinct callee formals. The second argument
is an exact boolean literal, and the callee's direct entry-value test link
cites the guard to specialize. The same bounded BDD restriction, ordered
proof identity and SCC worklist as the two-positional case apply. The
opposing `stop=False` variants cannot reuse the conditional base path;
their recursive origins retain `call_transfer` unknowns. An unpacked
argument does not meet the explicit argument mapping.

The real fixture must expose the guarded function as a public path:
entry-value links are intentionally formed only for public operations.
The initial fixture omitted its `__all__` entries, so it correctly had no
guard link and no recursive proof. Declaring those entries restored the
evidence rather than weakening the admission rule.

| Gate | Scoped judgment |
|---|---|
| G1–G3, CI-G1 | Satisfied: distinct source, argument, formal, literal, guard-link and callee-summary identities remain cited. The shared validator reconstructs the positive proof. |
| G4–G6 | Satisfied: one DataFusion selection extension feeds the existing bounded worklist and Delta/native path. No second keyword-specific evaluator was added. |
| G7, CI-G2 | Satisfied for path-local conditional inspection; an opposing guard and unpacking remain unknown, never refuted merely by a missing proof. |
| G8 | Satisfied: existing Ruff/ty/Pass B, DataFusion, BDD, Arrow and native contracts cover this exact form without an adapter. |
| CI-G3 | Unaffected: the gold reference is not a compiler input. |

FP-01–06 and applicable DP-01/02/03/04/08/11/16/18/19/21 and
CI-01–04/06 are satisfied within this scoped form. General argument
evaluation, nonliteral substitutions, multi-path condition conjunction,
other summary channels and production cap traces remain open in
[plan orders 1 and 6](../../plans/behavioral-model-forward-plan_2026-09-24.md#3-stage-3-execution-queue).
There is no new architectural finding or §B decision.

**Tested 2026-09-26:** `RUST_MIN_STACK=16777216 INSTA_UPDATE=no cargo test
-p cpg-core --test compile nested_returns_need_an_uncontrolled_exit_before_becoming_value_summaries
--quiet` passed the real positive/opposing cases and publication validation;
`RUST_MIN_STACK=16777216 INSTA_UPDATE=no cargo test -p cpg-core --test bundle
finite_depth_and_unsupported_refusals_reach_the_native_response --quiet`
passed both keyword shapes through Delta/native serving. `just fmt`,
`just test-all` and `just pilot` are **not_run** pending functional completion.

**A1 satisfied, A2 satisfied, A3 satisfied. Accept scoped** at Tested
strength. Enclosing Stage 3 completion and integrated qualification remain open.
