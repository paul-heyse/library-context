# Design review: parameter-name argument evaluation

**2026-09-26 · change/conformance · scoped review.** Core standard 3.0, code-intelligence
profile 1.1 and library-context binding. Subject: the append-only
`parameter_name_normal` status in `modeled_argument_evaluations`, the shared query-only
argument classifier and Stage 3 order 1's finite predecessor consumer. The
[forward plan](../../plans/behavioral-model-forward-plan_2026-09-24.md#3-stage-3-execution-queue)
owns the current work. This is a bounded direct-name case, not general expression completion.

## 1–5. Ownership, contracts and scenario

Ruff supplies an exact whole-argument `ExprName`; lexical resolution supplies candidate binding
identity; ty supplies the reaching definition and condition at the same byte range. The schema
query owns their join. It emits `parameter_name_normal` only for exactly one reaching row, with
no null definition, approximation or loop carry, matching the unique lexical parameter binding
and an admitted condition root. The evidence id is ty's `flow_reaching.fact_id`; an arbitrary
same-spelling name cannot stand in. `cpg-schema` owns the append-only status, its known/unknown
check and evidence reference. The pure finite producer consumes the typed evaluation just as it
consumes literal and builtin witnesses. `cpg-core` publishes and reconstructs the same result;
the native reader sees the already validated ordered proof, not a second name evaluator.

| Relation | Authority and fidelity | Missing/partial interpretation |
|---|---|---|
| `references` / `reference_resolutions` | Ruff lexical name and candidate binding, flow-insensitive | A spelling or single lexical candidate alone does not prove a runtime value |
| `flow_uses` / `flow_reaching` / `flow_definitions` | ty use-def at that name's byte range, with condition and approximation | Null, multiple, loop-carried or bounded reaching rows withhold normal evaluation |
| `modeled_argument_evaluations` | Derived per-argument status and cited evidence, shared with the predecessor query | Unknown remains an explicit `outside_provider_model` row; a missing positive is not a raise assertion |

The selected change scenario is a parameter sibling in `typing.cast(typ, value)`. Previously
the dynamic `typ` or a parameter named `object` was withheld merely because it was not a
literal/builtin. Both are valid direct parameter reads when ty and lexical binding agree; the
model's identity transfer can then be admitted. `cast(1 / 0, value)` remains unknown, as does a
parameter whose reaching set includes possible deletion. This is one status addition in the
schema-owned classifier and a condition/evidence check, rather than special cases in each
summary path. A future local-variable read needs its own source definition and completion
contract; widening `ParameterNameNormal` to it would misstate this proof.

The query uses DataFusion joins and grouped candidate counting over the existing Ruff/ty
tables. ty's use-def semantics are the library capability; a custom name reachability walker
would duplicate it and lose the provider's null/approximation states. The pure producer still
checks dense argument order and composes the cited proof. No new crate, adapter framework or
independent Python evaluator has a consumer here.

## 6–8. Judgments, gates and decision

A1–A3 and FP-01–FP-06 are **satisfied for this slice**: the status has one semantic owner,
both finite paths reuse the classifier, and the producer is testable with explicit inputs.
Applicable DP-01/02/03/08/11/13/15/16/21/23/24 and CI-01/02/04/06/08/11 are satisfied
at the tested boundary. G1–G8 and CI-G1/CI-G2 **pass for the scoped cases**: the new status is
append-only, the schema check and source reference validate its fact id, and the real positive
and withholding fixture reaches Delta and native serving. CI-G3 is unchanged; the gold is not
an input. Multiple-reaching definitions, local names, nested expression calls and operation-wide
normal completion are **unresolved** in the enclosing Stage 3 design and remain with plan order
1. No new architectural finding requires an ADR; this is a new instance of the accepted typed
argument-evaluation contract.

**Tested 2026-09-26:** `RUST_MIN_STACK=16777216 cargo test -p cpg-core --test bundle
finite_depth_and_unsupported_refusals_reach_the_native_response --quiet` passed a normally
reaching parameter predecessor and a possibly deleted parameter withholding case through
publication and native query. `RUST_MIN_STACK=16777216 cargo test -p cpg-core --test compile
pinned_identity_models_require_and_publish_their_real_formals --quiet` passed after updating
the expected dynamic/shadowed parameter outcomes and retaining the raising/non-exact controls.
`INSTA_UPDATE=no cargo test -p cpg-schema --test codebooks --test contracts --quiet` passed
after reviewing/accepting the append-only status, schema check and evidence-reference snapshots.
`just fmt`, `just test-all`, fresh `just pilot` and the complete Stage 3 evaluation are
`not_run`. Decision: **accept this scoped change/conformance slice**; the integrated stage
remains open.
