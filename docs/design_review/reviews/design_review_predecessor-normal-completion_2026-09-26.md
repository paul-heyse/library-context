# Design review: narrow predecessor normal completion

**2026-09-26 · change/conformance · scoped review.** Core standard 3.0, code-intelligence
profile 1.1, library-context binding. Subject: Stage 3 order 1's first normal-completion
witness across `cpg-schema::behavior`, `lctx-analytics::summaries::finite`, publication validation
and FORMAT 8 native serving. The [forward plan](../../plans/behavioral-model-forward-plan_2026-09-24.md#3-stage-3-execution-queue)
owns its current disposition. This review covers one positive and three withholding fixtures, not the
general call-site evaluator or assembled Stage 3 architecture.

## 1–5. Ownership, contract and change scenario

`cpg-schema` owns the pinned model assertion, closed target and argument/callee query contract.
Its one `simple_argument_evidence_sql` classifier supplies both whole-return model paths and
the new `preceding_normal_call_arguments` relation. Ruff's direct literal syntax and lexical
reference resolution are cited, not treated as runtime observations. The ty region contributes
the preceding call's path condition and approximation status; an earlier module-level import
must have a `true`, non-approximate ty region. A source-order byte position
orders it before the return. `lctx-analytics` consumes these typed rows and owns the pure finite
decision and ordered proof. `cpg-core` only acquires inputs, publishes the outcome, and validates
stored steps by recomputing that producer. The native reader accepts the append-only
`preceding_call_normal` proof kind from the schema contract and serves the cited path or
boundary from one pinned generation.

| Relation | Fidelity and identity | Unknown / consumer meaning |
|---|---|---|
| `model_applications` | Derived application of a pinned `model_targets` assertion to one Pysa call target, with explicit modality and closure | `normal_return` concerns the callee after arguments; it alone cannot clear a predecessor |
| `preceding_normal_call_arguments` | Query-only, one exact call fact, earlier module import binding/region and per-argument fact/evaluation witness in source order | Emits nothing for a local/non-import callee, non-simple or unpacked argument, or open target; the producer checks import condition and withholds otherwise |
| `preceding_call_regions` | Ty statement region plus Ruff source site | Approximate/missing region or condition withholds; compatible does not mean definitely executed |
| `summary_flow_steps` | Canonical ordered proof identity with reference and codebook checks | `preceding_call_normal` records safety if the call runs, not its execution; missing step fails shared equality validation |

The realistic next change is a parameter-name or nested-call argument. It must add its own
evaluation/exit witness to the schema-owned classifier or a typed sibling relation; the finite
producer may compose only that evidence and must retain `unsupported_control_flow` for an
unexplored operand. It must not mark every name or Pysa edge as normally evaluated. The current
classification is deliberately narrow: one closed, definite pinned call target, an earlier
module-level `from` import with an unconditional ty region, ordinary direct literal or
unshadowed builtin-name arguments, dense ordinals and a non-approximate call region. The
proof cites import binding/region, callee resolution, each argument, call site, Pysa target and model id before the
raw identity step. A BDD-proved disjoint call needs no completion witness.

A conditional local import was the independent challenge: flow-insensitive lexical resolution
selected its modeled target, and the first candidate query wrongly admitted it. ty's use-def
map is scope-local and reports a module import as unbound inside a function, so a generic
callee-use reach requirement also rejected the valid top-level import. The corrected query
requires the exact earlier module-level import, and the producer checks its ty condition as
`true`. A guarded module import supplies the second withholding control. This correction
stays within the existing schema/query and pure-producer owners; it does not claim to solve
general callable binding or import completion.

DataFusion CTEs/window counts implement the all-arguments test and sole-application check in
the existing derived-relation owner; Ruff and ty already supply the syntax and path inputs.
There is no generic library primitive that establishes Python expression completion from those
facts. A second interpreter or new framework would duplicate provider semantics. The pure
producer's small ordered join is appropriately bespoke; a richer evaluator belongs to later
order 1 work, not a new process or crate.

## 6–8. Gates, judgments and findings

For this supported case, A1–A3 are **satisfied**: one classifier is shared by two consumers;
schema, producer, validator and native decoder change only at their declared contracts; pure
inputs test the decision without extraction or serving. FP-01–FP-06 and applicable
DP-01/02/03/08/11/15/16/18/21/23/24 are **satisfied for the scoped extension**. CI-01/02/04/06/08/11/13
are satisfied at the same scope: a candidate target is never relabelled as an executed call, and
the unknown sibling remains visible. CI-03/05/07/09/10/12 do not receive a changed mechanism
in this slice.

G1–G8 and CI-G1/CI-G2 **pass for the scoped claim**: the append-only code and reference snapshot,
shared reconstruction validator, source-order proof and native positive/unknown outputs were
checked. G5/G6 are bounded to one published fixture; shuffled real extraction and integrated
qualification are not claimed. CI-G3 is not exercised by this code path; gold inputs remain
outside the compiler. No new finding changes the accepted architecture. The enclosing order 1
scope is **unresolved**, especially evaluated parameter reads, nested/raising calls, branch
termination and path-specific predecessor identity; [plan order 1 and order 3](../../plans/behavioral-model-forward-plan_2026-09-24.md#3-stage-3-execution-queue)
own those actions. An empty `preceding_normal_call_arguments` result is a refusal to prove
completion, not evidence of a raising call.

**Tested 2026-09-26:** `cargo test -p lctx-analytics --lib summaries::finite::tests --quiet`
passed pure positive/incomplete and BDD-cap controls; `RUST_MIN_STACK=16777216 cargo test -p
cpg-core --test bundle finite_depth_and_unsupported_refusals_reach_the_native_response --quiet`
passed a real CPython 3.14.7 pinned `typing.cast(object, 1)` predecessor, a `1 / 0` sibling,
conditional local-import and guarded module-import withholding cases, Delta→FORMAT 8→native
response, and a forged omitted-step validator check.
`RUST_MIN_STACK=16777216 cargo test -p cpg-core --test compile
pinned_identity_models_require_and_publish_their_real_formals --quiet` passed after the shared
classifier refactor. `INSTA_UPDATE=no cargo test -p cpg-schema --test codebooks --test contracts
--quiet` passed after inspecting and accepting the append-only snapshots. `just fmt`,
`just test-all`, fresh `just pilot`, other order 1 controls and clean-wheel serving are
`not_run`. Decision: **accept this bounded change/conformance slice**; Stage 3 remains open.
