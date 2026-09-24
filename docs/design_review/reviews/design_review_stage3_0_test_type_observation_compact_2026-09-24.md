# Stage 3.0 test-use/type observation — compact change review

## Scope and decision

**Subject:** the `flow_test_types` fact family, structural operand attribution, exact-range Pyrefly trace, and publication proof-link validation. **Date:** 2026-09-24. **Standard:** core 2.0, code-intelligence profile 1.0, library-context binding. **Tier:** compact change review. **Decision:** accept as an attributed type observation, conditional on the final gates below; it cannot authorize typed contradiction or an operation-level compatibility verdict.

The producer starts with a `flow_test_leaves` row. It follows only AST forms lowered by `Translator::test` and selects the modeled place operand by operator role. It then requires exactly one same-scope, same-span, same-place, non-annotation `flow_uses` row. The exact operand range is submitted to Pyrefly's type trace. A missing operand, use or trace emits no `flow_test_types` row. Synthetic pattern leaves have no operand mapping. The `PyreflyTrace` origin explicitly denotes an observation, not exact runtime class or value stability.

## Gates

| Gate | Verdict | Evidence and limit |
|---|---|---|
| G1 Authority | Satisfied in scope | `flow_test_leaves` retains provider predicate and atom identity; `flow_test_types` cites the leaf, use and type-term facts. No observation rewrites the BDD. |
| G2 Fidelity | Satisfied in scope | Structural selection rejects a sibling class operand and multiple candidate uses. It does not infer runtime exactness from a Pyrefly term. |
| G3 Validity | Satisfied in scope | The publication validator checks the single cited leaf and use, Site atom/place/span/module, non-annotation use, term fact and origin. A Delta-backed tamper test changes the place and requires rejection. |
| G4 Hidden behavior | Satisfied | The trace reads the pinned in-process Pyrefly answers for analyzed source; no analyzed package is imported or executed. |
| G5 Recovery | Satisfied in scope | Extractor output version 25 and schema snapshots make the new table and fact ids an explicit migration. |
| G6 Reuse | Satisfied | The producer uses ty's use rows, Pyrefly's exact-range trace and the existing type-term builder. |
| G7 Claims | Unresolved for Stage 3.6 | The generation has no served entry-to-test bridge or typed theory proof. Compatibility remains unknown without these. |
| G8 Library leverage | Satisfied | Existing AST, Pyrefly and DataFusion/Arrow paths handle parsing, tracing and validation; custom code is limited to source identity and proof admission. |
| CI-G1 Fidelity | Satisfied in scope | A type trace is named as an observation and missing rows remain unknown. |
| CI-G2 Evidence closure | Unresolved for serving | The new fact is not yet a served citation or proof witness. |
| CI-G3 Evaluation integrity | Satisfied | No gold capability family enters extraction. |

**Principle result:** DP-01/02/03/08/11/13/19/20/21/22/23/24 and CI-01/02/04/06/08/10/13 are satisfied for this scoped observation. DP-14 and CI-11 remain unresolved for a typed or served behavioral verdict. The source-identity claim is **Tested** in focused fixtures; the pilot counts and timing are **Measured** for FastMCP 4.0.5 on 2026-09-24.

## Findings and disposition

| ID | Finding | Disposition |
|---|---|---|
| T01 | A contained use is often not the tested operand (the prior pilot had 3,870 leaves with multiple contained non-annotation uses). | Operator-specific AST attribution, followed by an exact-one use match; the sibling `C` in `isinstance(x, C)` is a counterexample. |
| T02 | Pyrefly trace types, including narrowed or annotated builtins, do not prove exact runtime class or standard equality. | The only admitted origin is `PyreflyTrace`; no typed exclusion consumes it. Add separately cited literal or runtime-guard exact-origin and effect-stability witnesses before theory comparison. |
| T03 | Publication checks source identity and cited rows, but does not independently reparse the operand AST. | Keep the producer's focused compound/sibling/annotation/synthetic tests; do not promote the row to a sound negative verdict until the later source/value proof has its own checks. |
| T04 | An absent trace or ambiguous use loses a positive observation. | Count `test_type_unmapped` and `test_type_trace_missing` per module in flow coverage; absence means unknown. |

## Verification

The new contract and codebook snapshots were inspected before `cargo insta accept`. `cargo test -p cpg-flow --test flow_shapes test_leaves_keep_compound_operands_and_distinct_match_arms --quiet` **passed** 1/1; `cargo test -p cpg-extract --test flow_runtime_resolution test_type_rows_name_the_selected_operand_and_trace --quiet` **passed** 1/1; `cargo test -p cpg-core --test compile published_test_type_link_tamper_is_rejected --quiet` **passed** 1/1. The first `just test-all` **failed** only its two expected migration guards (extractor id snapshot and all-techniques digest); both were reviewed and updated, with focused reruns **passed**.

`just pilot build/store-stage3-test-types` **passed** on snapshot `51842cde463da4406f5ccf19072ec7e1`, generation `938aa19e92ba9d52`, and 20/20 serving smoke briefs. It published 6,882 `flow_test_leaves` and 4,712 `flow_test_types`; coverage details sum to 2,167 unmapped operands and 3 missing traces. The pilot's `behaviors.boundary_reason = 6` count remained zero. The new type trace stage measured 0.03 s; the whole compile measured 69.5 s and 4,203 MiB peak RSS. That whole-run timing is one host run, not an isolated before/after speed comparison. The final `just test-all` result is pending in this draft.
