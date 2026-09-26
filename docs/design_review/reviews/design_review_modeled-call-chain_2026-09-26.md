# Design review: exact nested modeled identity calls

**2026-09-26 · change/conformance · scoped author review.** Core 3.0,
code-intelligence profile 1.1 and the library-context binding in
`standard.toml` apply. [Plan orders 1, 3 and 4](../../plans/behavioral-model-forward-plan_2026-09-24.md#32-functional-completion)
own the remaining evaluation and composition work.

## Scope, owners and change scenario

This review covers returned parameter value paths crossing two or more nested,
closed, definite, normal-returning identity model calls. The source observation is
the ordered Ruff-derived `flow_value_calls` path and its bound argument links;
the pinned model application and Pysa target facts supply the transfer and
completion assertions. `cpg-schema` owns the DataFusion
`modeled_chain_arguments` projection. The pure `lctx-analytics` finite producer
owns dense-chain admission, source-ordered proof construction and budgets.
`cpg-core` acquires the same typed inputs for publication and reconstruction;
FORMAT 8 native serving reads the validated proof.

The immediate extension scenario is a third `typing.cast` around the same
formal. It adds one source call and two argument rows without a new seed type,
SQL relation or native decoder. A raising sibling in the inner call gives no
normal-evaluation witness, so the chain yields a `call_transfer` unknown. An
unmatched, open or nonidentity step likewise cannot borrow its neighbor's
normal-return assertion. This is a source and model proof, not a conclusion from
nesting spans alone.

| Fact or relation | Fidelity and identity | Coverage and consumer |
|---|---|---|
| `flow_value_calls` / links | Extracted ordered raw call step, parent flow fact, Ruff span and bound argument fact | One use-to-return path; missing/unbound steps remain unknown |
| model application / transfer site | Pinned target/model assertions and explicit argument endpoint | Sole closed definite identity and normal return only |
| `modeled_chain_arguments` | Derived row per call step and explicit argument, retaining raw source origin and evidence IDs | Pure finite producer; query-only, not another stored fact authority |
| `summary_flows` / steps | Derived origin-specific path and ordered typed proof | Shared publication validator, Delta and native reader |

The projection selects raw return contributions with at least two calls and
keeps each candidate's argument multiplicity. The producer requires a dense
step sequence, the exact source-argument span of each next call, one source
argument per call, every other argument's normal witness, a proven return
condition and the preceding-call completion check. The innermost direct formal
read needs an exact ty reaching definition or the bounded lexical parameter
witness of [ADR-0055](../../adr/0055-modeled-source-read.md), cited beside its raw flow fact.
It sorts rows before proof construction. The finite cap is eight call steps
and 128 argument rows; the explicit depth/work boundaries preserve unknown rather than claiming a
completed or absent path. The established DataFusion joins, window count and
typed query row fit the existing §B2/§B3 contract; the custom Rust loop is the
Python evaluation-order rule under §B5. A generic graph reachability library
would not supply argument evaluation or pinned model semantics.

| Judgment | Scoped result |
|---|---|
| A1–A3, FP-01–06 | Satisfied for this chain: extraction, derivation, pure admission and serving have distinct owners; depth three changes rows, not mechanisms. |
| G1–G3, G6–G7, CI-G1–CI-G2 | Satisfied in tested cases: all steps retain source/model IDs and order, and missing or raising evidence withholds the result. The shared validator reconstructs the path from the source tables. |
| G4–G5 | Satisfied within this slice: the pure producer has no acquisition or publication effect; the existing snapshot append and native generation remain the publication boundary. |
| G8 | Satisfied: DataFusion builds the relational candidates and the bounded bespoke rule addresses Python evaluation semantics not supplied by the adopted libraries. |
| CI-G3 | Unaffected: the gold reference is not an analysis input. |

Applicable DP-01/02/03/08/11/13/16/18/19/21/23 and CI-01/02/04/06/08/11 are
satisfied for this bounded path. No new §B decision is needed: this implements
the ordered call-path choice in [ADR-0028](../../adr/0028-call-result-provenance.md).
General nested expressions, other transfer kinds, non-import callee resolution
and path-specific earlier-call evaluation remain outside this slice and in the
plan. The eight-step/128-row limits are implemented controls, not a measured
pilot budget; ADR-0028's pilot-cost revisit remains due at integrated exit.

**Tested 2026-09-26:**
`cargo test -p lctx-analytics --lib summaries::finite --quiet` passed
15 focused pure tests, including three-step row shuffling and incomplete/unknown
argument controls.
`RUST_MIN_STACK=16777216 INSTA_UPDATE=no cargo test -p cpg-core --test compile pinned_identity_models_require_and_publish_their_real_formals --quiet`
passed the two- and three-call real-source/Delta/publication positives and the
raising/nonidentity withholding cases. A deleted-formal control has no
parameter-origin contribution and no positive path.
`RUST_MIN_STACK=16777216 INSTA_UPDATE=no cargo test -p cpg-core --test bundle finite_depth_and_unsupported_refusals_reach_the_native_response --quiet`
passed the two- and three-call native positives, raising sibling unknown, and
deleted formal's empty source path.
`just fmt`, `just test-all` and `just pilot` are **not_run** pending functional
completion.

**Accept scoped at Tested strength.** The enclosing Stage 3 evaluation and
multi-channel composition contracts remain incomplete.
