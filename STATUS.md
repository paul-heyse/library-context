# Status

_Updated 2026-09-26 under the [handoff skill](.claude/skills/handoff/SKILL.md); work is on `main`._

## Increment and slice

- **Increment 4 / Stage 3 remains functionally incomplete.** The [forward plan §3.0](docs/plans/behavioral-model-forward-plan_2026-09-24.md#30-consolidated-execution) owns S1–S8 and §6 owns W1–W16. [ADR-0057](docs/adr/0057-compositional-stage3-semantics.md) accepts the target; this checkpoint implements only part of it.
- **Implemented:** normal reads and nested pinned total calls compose through ordered expression proofs and all agreeing signatures. A pure completion owner distinguishes normal/abrupt/unknown statements and preserves or replaces pending returns through ordered finalizers. Unique local initialization outside loops is supported; implicit exception construction, rebinding/finalization, typed handlers, re-raise and context exits remain conservative.
- **Schema migration:** compiler output 77 replaces `closed_expression_evaluations` with `expression_evaluations` plus operand proofs, and adds statement/frame completion tables. Reviewed snapshots accepted; boundary codes 28–30 are append-only completion-depth, completion-work and summary-proof limits. Producer/native share 64 local proof steps. ADR-0048 requires fresh stores/generations; FORMAT 8 remains current, FORMAT 9 a target.
- The [bounded change review](docs/design_review/reviews/design_review_stage3-expression-completion_2026-09-26.md) accepts the restricted corrected slice by inspection. Its F01/F02 corrections have focused receipts below; it does not certify assembled Stage 3. Full operand evidence reaches finalizer proofs; missing/tampered steps fail shared reconstruction.

## Last verified (2026-09-26)

| Command | Outcome and scope |
|---|---|
| `INSTA_UPDATE=no cargo nextest run --release -p cpg-schema --lib --test codebooks --test contracts -E 'test(summary_contract) \| binary(codebooks) \| binary(contracts)' --status-level fail --final-status-level fail` | `passed`: 15 schema/binding cases after reviewed snapshot acceptance. |
| `RUST_MIN_STACK=16777216 INSTA_UPDATE=no cargo nextest run --release -p lctx-analytics --lib -E 'test(completion::tests) \| test(producer_and_native_share_the_proof_limit)' --status-level fail --final-status-level fail` | `passed`: six focused completion/default-cap cases. Earlier expression/finite subset: 22 passed before final cap changes. |
| `RUST_MIN_STACK=16777216 INSTA_UPDATE=no cargo nextest run --release -p cpg-core --test compile --test bundle -E 'test(composed_argument_reads_keep_ordered_source_evidence) \| test(nested_returns_need_an_uncontrolled_exit_before_becoming_value_summaries) \| test(pinned_identity_models_require_and_publish_their_real_formals) \| test(finite_depth_and_unsupported_refusals_reach_the_native_response)' --status-level fail --final-status-level fail` | `passed`: four after final corrections. Real extraction/Delta/shared validation/native serving; exact 64-step acceptance and 65-step refusal; nested calls/signatures/finalizers; unknown implicit execution and loop initialization; tamper rejection. Fake embedding only ranks fixtures. |
| `cargo check --release -p cpg-core -p lctx-semantics --quiet`; `uv sync --locked --reinstall-package lctx-semantics` | `passed`: focused compilation and editable native build. |
| `just docs-check` | `passed`: documentation/ADR/agent checks and 115 canonical pages; separate from product acceptance. |
| `HYPOTHESIS_STORAGE_DIRECTORY=/tmp/lctx-stage3-completion-hypothesis timeout 30s uv run --no-sync python docs/design_review/evidence/2026-09-26_expression-completion/probe.py` | `passed`: 120 generated CPython 3.14.7/Hypothesis 6.168.1 return/unwind controls; two implicit actions started without completing before timeout. [Evidence and limits](docs/design_review/evidence/2026-09-26_expression-completion/README.md). |
| `just fmt`; `just test-all`; fresh `just pilot`; structured/Q01/Q03/Q05/Q09 evaluation; clean wheel; live W9 replay; all-techniques/cost comparison | `not_run`: deferred until all S1–S7 functional scope is implemented, as requested. |

## Remaining boundaries and next work

- **Next: S2 path-specific predecessor sequencing.** Current predecessor admission still inspects calls; normal statement/frame certificates do not yet establish complete function-entry-to-return execution. Compose defaults, typed handlers/re-raise and supported synchronous context exits before callback/resource fate admission.
- Preserve `entry_links.rs`'s arbitrary-truthiness barrier. Real `multiple_control_true` stays unknown until call-specific primitive stability/completion evidence exists; pure supplied stable links are not extraction proof.
- S3 model families/dynamic schema; S4 residual conditions, multi-channel SCCs and semantic/witness separation; S5 channel/origin coverage; S6 FORMAT 9/typed effect-role/operation-wide compatibility; S7 original W5/W7/W12 default-cap traces and broader oracle comparisons remain open. Added expression/completion/proof limits do not close older obligations. W14/F15 transport is implemented, performance unmeasured.
- Missing completion enum registration and new signature-proof reference coverage were corrected after focused failures; the final focused commands above passed. No full integrated receipt has been established after the 2026-09-25 interrupted run. Earlier operator confidence remains an assumption, not a pass.
