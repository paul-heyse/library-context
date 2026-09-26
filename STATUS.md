# Status

_Updated 2026-09-26 under the [handoff skill](.claude/skills/handoff/SKILL.md); work is on `main`._

## Increment and slice

- **Increment 4 / Stage 3 remains functionally incomplete.** The [forward plan §3.0](docs/plans/behavioral-model-forward-plan_2026-09-24.md#30-consolidated-execution) owns S1–S8 execution and §6 owns W1–W16. [ADR-0057](docs/adr/0057-compositional-stage3-semantics.md) accepts shared-contract consolidation; it does not assert that the target is implemented.
- This checkpoint implements part of S1/S2/S4/S5/S7: schema-owned ordered local arguments and shared control-group proof admission; bounded simultaneous BDD substitution; a pure nested closed-expression evaluator; independent normal/exact-Boolean outcomes; required-formal and source-read checks; handler preparation before summaries; all twenty SQL-only behavior table derivations retain Arrow batches instead of decoding/re-encoding Rust rows.
- **Current-store schema migration:** compiler output 76 adds `closed_expression_evaluations`, reconstructed from source before publication, and append-only boundary codes 26/27 (`expression_depth_limit`, `expression_work_limit`). Snapshots reviewed and accepted. ADR-0048 requires a fresh store/generation; no history reader was added. FORMAT 8 remains implemented; FORMAT 9 is still a target.
- Real three-argument/reversed-keyword calls, nested closed expressions and default expression-cap boundaries reach Delta/native. The tracked raw flow and its normal-read witness are separate proof steps. Missing required formals, evaluated raising arguments and absent stable links stay unknown. The [scoped foundation review](docs/design_review/reviews/design_review_stage3-foundations_2026-09-26.md) bounds these claims; it is not assembled Stage 3 acceptance.

## Last verified (2026-09-26)

| Command | Outcome and scope |
|---|---|
| `INSTA_UPDATE=no cargo nextest run --release -p lctx-analytics -p cpg-schema --lib --test codebooks --test contracts -E 'test(evaluation::tests) \| test(summaries::finite) \| test(summary_contract) \| test(condition_kernel::substitution) \| binary(codebooks) \| binary(contracts)' --status-level fail --final-status-level fail` | `passed`: 38 focused pure/schema cases. |
| `INSTA_UPDATE=no cargo nextest run --release -p lctx-analytics --lib -E 'test(multiple_controls_preserve_order_and_require_all_normal_evaluations)' --status-level fail --final-status-level fail` | `passed` after the final argument/refusal changes; ordered multi-control, malformed/missing mappings and typed expression limits. |
| `RUST_MIN_STACK=16777216 INSTA_UPDATE=no cargo nextest run --release -p cpg-core --test compile --test bundle -E 'test(nested_returns_need_an_uncontrolled_exit_before_becoming_value_summaries) \| test(pinned_identity_models_require_and_publish_their_real_formals) \| test(finite_depth_and_unsupported_refusals_reach_the_native_response)' --status-level fail --final-status-level fail` | `passed`: all three after the final twenty-table transport change. Real extraction/Delta/shared validation, forged-expression rejection, positive/withholding native cases and default expression depth/work traces. Fake embedding only supplies unrelated fixture ranking; no live embedding claim. |
| `cargo check --release -p cpg-core -p lctx-semantics --quiet`; `uv sync --locked --reinstall-package lctx-semantics`; `uv run --no-sync pytest python/lctx_mcp/tests/test_native_semantics.py -q` | `passed`: compile, editable native build and twelve native semantic cases. |
| `uv run --no-project --offline --no-python-downloads python docs/design_review/evidence/2026-09-26_closed-expression-composition/probe.py` | `passed`: nine independent CPython 3.14.7 controls; [scope/results](docs/design_review/evidence/2026-09-26_closed-expression-composition/README.md). |
| `just adr index`; `just adr lint`; `just docs-check` | `passed`: 29 ADR records; focused documentation validation. |
| `just fmt`; `just test-all`; fresh `just pilot`; Q01/Q03/Q05/Q09; structured evaluation; clean wheel; W9 live replay; all-techniques/cost comparison | `not_run`: explicitly deferred until the entirety of S1–S7 functional scope is implemented. |

## Remaining boundaries and next work

- **Next: complete S1/S2 evaluation/completion contracts.** Extend supported reads, nested calls/defaults and path-specific predecessors, then ordered finalizer/handler/context-exit outcomes. Keep source observations, argument binding, normal evaluation, exact value and call/frame completion distinct.
- **Do not weaken `entry_links.rs` to admit multiple controls.** The pure producer accepts several supplied stable links; extraction currently withholds a later predicate after arbitrary prior truthiness because it can run user code. The real `multiple_control_true` fixture intentionally remains unknown. Add call-specific stability/completion evidence before changing that assertion.
- S3 model families/dynamic schema, S4 residual symbolic conjunction and multi-channel SCCs/semantic-versus-witness state, S5 channel/origin coverage/discharge, S6 FORMAT 9/effect-role/operation-wide compatibility, and S7 original W5/W7/W12 default-cap traces plus broader oracles remain open. New expression-cap traces do not close those older obligations. W14/F15's table-construction trigger is implemented; performance remains unmeasured.
- The earlier nested `EXISTS` form of required-formal coverage failed DataFusion physical planning; grouped required/bound counts now pass. Stale tests expecting filtered-away unknown seeds or raw flow ids as normal reads were replaced by explicit retained-unknown and separate-read assertions. No currently failing focused check is being carried forward.
- Full integrated acceptance has never been re-established after the 2026-09-25 interrupted run. The operator's earlier expectation of a pass remains an assumption, not a current test outcome. Keep focused checks only until the full functional scope lands.
