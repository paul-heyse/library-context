# Design review: summary condition-cap publication

**2026-09-26 · change/conformance · scoped review.** Core standard 3.0,
code-intelligence profile 1.1 and library-context binding. Subject: the
finite summary producer's typed refusal when a stored return condition or a
predecessor compatibility conjunction reaches a BDD cap. The
[forward plan](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)
owns W5's remaining limits and origin identity.

## 1–5. Ownership, contracts and scenario

The bounded condition kernel owns the `atom_limit` cause and the catalog
records it against a condition id. `cpg-core` hydrates that catalog and maps
the cause to the append-only `condition_atom_limit` boundary. The pure finite
producer owns the decision to admit or refuse a direct summary and returns
its refusal with the source key. The publisher and shared validator reconstruct
the boundary; FORMAT 8 carries it to the native reader. No server-side
condition reinterpretation is needed.

The change scenario is a direct `return value` guarded by 129 independent
Boolean parameters. Before the fix, the condition catalog correctly recorded
`atom_limit`, but the producer asked for predecessor compatibility first.
Because the return diagram could not be hydrated, it fell into
`unsupported_control_flow` and discarded the more specific cause. The
producer now checks the stored return condition before predecessors. A
second pure case composes a 128-atom return condition with one additional
predecessor atom. The bounded `and` failure becomes the same typed refusal;
it can no longer fall through to a normal-completion witness. A control with
an unsupported predecessor still retains `unsupported_control_flow`.

| Contract | Authority | Refusal behavior |
|---|---|---|
| `analysis_conditions` | Structural BDD root or named kernel boundary | A missing diagram with `atom_limit` is unknown with a specific cause |
| `FiniteSummaryOutcome` | One pure flow/step/refusal/boundary decision | No positive flow for either cap; no generic control fallback overwrites it |
| `summary_boundaries` / native value paths | Validated published projection | `condition_atom_limit` survives Delta, FORMAT 8 and native query |

The correction uses the existing `Diagram::and` and its typed `KernelBoundary`;
no separate BDD interpretation, new adapter or library choice is warranted.
It changes decision order within the accepted W5 contract, not a binding §B
choice, so no ADR is needed. The 129-atom fixture is deliberately over the
declared 128-atom limit and is an analysis input, never executed.

## 6–8. Judgments, gates and decision

A1–A3 and FP-01–FP-06 are **satisfied for this slice**: the condition kernel
retains semantic authority, the pure producer uses its cause, and the
published/native consumer sees one validated boundary. Applicable
DP-01/02/03/08/11/13/15/16/21/23/24 and CI-01/02/04/06/08/11 are satisfied
at this tested boundary. G1–G8 and CI-G1/CI-G2 **pass for the atom-cap and
control cases**: no positive/negative claim follows from the cap and the
specific refusal survives serving. CI-G3 is unchanged; gold is not an
analysis input. Node/work-cap publication, multiple origins sharing a raw
key and full Stage 3 qualification are **unresolved**. W5 tracks them; this
review does not certify the enclosing architecture.

**Tested 2026-09-26:** `cargo test -p lctx-analytics
direct_return_preserves_stored_and_predecessor_atom_caps --quiet` passed
both pure cap cases. `RUST_MIN_STACK=16777216 cargo test -p cpg-core --test
bundle finite_depth_and_unsupported_refusals_reach_the_native_response
--quiet` passed the real 129-atom, unsupported-control and depth cases through
Delta/FORMAT 8/native, including absence of a positive cap summary. `just
fmt`, `just test-all`, fresh `just pilot` and complete Stage 3 evaluation are
`not_run`.

**Decision:** accept this scoped correction. W5 and Stage 3 remain open at
the explicit boundaries above.
