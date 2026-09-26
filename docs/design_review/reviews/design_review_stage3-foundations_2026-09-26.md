# Design review: Stage 3 composition foundations

**2026-09-26 · Change / conformance · Scoped acceptance.** Core 3.0, code-intelligence 1.1,
repository binding. This is implementation evidence within ADR-0057, not Stage 3 acceptance.

## 1. Scope, ownership and scenarios

Inspected changes relative to `34c8784`: `condition_kernel/substitution.rs`, schema-owned
`summary_contract`, `behavior.rs`, analytics `evaluation` and `summaries/finite`, core
acquisition/publication/reconstruction, native admission, and the changed source/native tests.
The existing main worktree was preserved. This review was performed by the implementing agent,
not an independent reviewer.

| Relation / owner | Meaning, fidelity and identity | Consumer and boundary |
|---|---|---|
| Placed Ruff syntax / extraction | Attributed source structure; fact ids | Pure expression evaluator; no Python execution |
| `closed_expression_evaluations` / schema + analytics | Derived normal completion, separately optional exact Boolean, refusal/work; snapshot + syntax fact | Shared argument adapter; source reconstruction rejects changed results |
| `LocalCallArgument` / schema | Ordered explicit argument, independent formal mapping, normal witness and optional exact value | Pure finite producer; missing required arguments and witnesses refuse |
| Control group / schema | Checked direct entry links covering every effective callee atom | Producer and native admission; source producer additionally proves binding/substitution |
| SQL-only analysis tables / DataFusion + core | Same declared schema, key and semantic query | Arrow batches to Delta; row decoding retained only for semantic consumers |

Adding a supported closed operator changes the pure evaluator and its focused semantic controls;
predecessor/model/local callers consume the same row contract. Adding another argument changes
source rows, not a new nullable control column in every consumer. A source-level second predicate
still requires a new stability proof: an earlier user-defined truthiness method could change the
later tested object. The fixture preserves that unknown instead of promoting hypothetical links.
These traces support FP-01/02/03/04/05/06 for the bounded changes. Other channels remain unresolved.

## 6. Correctness and fidelity gates

| Gates | Scoped verdict | Evidence and remaining limit |
|---|---|---|
| G1, G2 | pass | Schema-owned evaluation/binding/proof contracts; normal completion, exact value, source flow and normal-read witness stay distinct |
| G3 | pass | Reviewed schema/codebook snapshots; shared source reconstruction and forged-value rejection |
| G4 | pass | Pure BDD/expression/summary functions; core owns storage; probes are isolated and never compiler inputs |
| G5, G6 | pass for bounded slice | Simultaneous original-DAG substitution with cumulative bounds; per-expression depth/work; ordered arguments, required-formal checks, deterministic controls; native default-expression-cap traces |
| G7 | pass | Implemented/Tested claims name focused commands; no speed, memory, full-suite or pilot claim |
| G8 | pass | Bounded biodivine apply/transfer, existing Arrow strict conversion/concat/sort and DataFusion joins; no new framework or dependency |
| CI-G1 | pass for stated model | Unsupported operands and unproved later-predicate identity remain unknown; numeric normal results do not become Boolean values |
| CI-G2 | pass for existing path contract | Source/native path checks retain cited raw source, normal-read, expressions and control links; expanded FORMAT 9 closure is not certified |
| CI-G3 | pass for changed path | Nine independent CPython observations; no gold/oracle output enters analysis. Generated-program campaign remains open |

## 7. Findings and verification boundary

No known wrong positive remains in the changed admitted forms. The source-level multi-control
stability gap is an explicit implementation limit under [S2/S4](../../plans/behavioral-model-forward-plan_2026-09-24.md#30-consolidated-execution),
not a reason to weaken entry-link barriers. The target review's F01 has its flattened control and
native adjacency problem removed; F02 semantic/witness scheduling and F04 operation-wide
coverage remain open. F03 preparation order is corrected; frame/exception composition remains.

Expression caps now survive the local-call producer through Delta/native at default limits.
That does **not** close the original W5 BDD/work, W7 reach-work or W12 pair-work obligations,
nor establish cap propagation through every modeled/predecessor path. S7 owns those obligations.

Focused receipts (2026-09-26):

- `INSTA_UPDATE=no cargo nextest run --release -p lctx-analytics -p cpg-schema --lib --test codebooks --test contracts -E 'test(evaluation::tests) | test(summaries::finite) | test(summary_contract) | test(condition_kernel::substitution) | binary(codebooks) | binary(contracts)'`: passed 38 cases, including malformed/duplicate mappings, capture/swaps, missing atom coverage, expression order, limits and reviewed contracts.
- `RUST_MIN_STACK=16777216 INSTA_UPDATE=no cargo nextest run --release -p cpg-core --test compile --test bundle -E 'test(nested_returns_need_an_uncontrolled_exit_before_becoming_value_summaries) | test(pinned_identity_models_require_and_publish_their_real_formals) | test(finite_depth_and_unsupported_refusals_reach_the_native_response)'`: passed all three cases after the twenty-table transport refinement, including source reconstruction/tampering, nested expression positives, missing/raising counter-controls and default expression-cap native traces.
- `uv sync --locked --reinstall-package lctx-semantics`; `uv run --no-sync pytest python/lctx_mcp/tests/test_native_semantics.py -q`: passed; twelve native semantic cases.
- [Independent CPython probe](../evidence/2026-09-26_closed-expression-composition/README.md): passed nine cases. This is not exhaustive equivalence.

The source/native fixture setup uses a fake embedder only for unrelated ranking setup; it does
not qualify an embedding service. Formatting, integrated tests, fresh pilot, live replay, clean
wheel and the assembled review are `not_run` pending the full functional scope.

## 8. Library fit and alternatives

The pinned DataFusion/Arrow skill and current DataFusion documentation agree that `collect`
retains Arrow batches. Reuse the existing strict declared-schema adapter, concat and canonical
sort for twenty SQL-only table writes, preserving semantic reconstruction; no RSS improvement is
claimed. Projected nested correlated `EXISTS` failed in the pinned planner, so required-formal
coverage uses grouped counts and joins. The successfully executed source checks qualify that route.

biodivine 0.6.3 owns bounded Boolean operations. The small original-DAG composer adds the missing
simultaneous capture-safe substitution and cumulative admission, without implementing Boolean
algebra again. Existing Ascent/datafrog comparison remains required when multi-channel state
exists. Python closed-expression semantics stay a bounded domain transformation over provider
facts; using a general runtime interpreter would introduce execution and environment dependence.

## 12. Architectural judgment

**A1 satisfied for this slice:** semantic transformations can be exercised without extraction,
Delta or Python. **A2 satisfied for this slice:** schema contracts and shared reconstruction own
meaning; ordered evaluations and formal bindings remain separate. **A3 satisfied for this slice:**
another closed expression or argument reuses the same producer/consumer boundary. This is
**scoped acceptance**, with full Stage 3 architecture unresolved until S1–S8 are complete.
[The forward plan](../../plans/behavioral-model-forward-plan_2026-09-24.md) owns execution status;
this review is dated evidence, not a second progress register.
