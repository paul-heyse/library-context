# Status

_Updated 2026-09-27 under the [handoff skill](.claude/skills/handoff/SKILL.md); work is on `main`._

## Increment and slice

- **Increment 4 / Stage 3 remains functionally incomplete.** The operator-approved [forward plan §3.0](docs/plans/behavioral-model-forward-plan_2026-09-24.md#30-consolidated-execution) owns S0–S8; §6 owns finding disposition. Accepted ADRs are targets, not integrated completion.
- **Current S1/S3 slice (ADR-0060):** every existing effect/callback/resource candidate has an assessment. Potential partial I/O can activate from proved invocation even when normal return is unproved. Completed serialization/compression and registration retain outcome refusals. A reached call never establishes an actual write or callback invocation.
- Shared schema admission checks candidate/application/call/phase/condition, subject binding and argument support, ordered invocation and any callee-normal proof. Authored subjectless effects need no invented subject. Normal/Finally preserve the original 64-step cap; exact exceptional outcomes and returned-resource identities remain unsupported. Publication reconstructs actions and upstream source semantics; native actions/full semantic closure remain S6.
- **Prior S5 slice (613ad45):** immutable lexical-input certificates compose with exact model-call proof for direct modeled returns in approximate regions. The original modeled-finalizer positive regression passes unchanged. Raw flags, independent completion obligations and open coverage remain; broader chains/assignments stay open.
- **Prior S2b/S3a slices:** source-return commitments, pinned nullcontext/suppress lifecycle and narrow immutable-parameter entry-value identity have focused source/Delta/native and independent runtime controls. Named handlers retain `handler_name_cleanup`; skipped handlers do not. General cleanup remains unsupported.
- Call executions (bf322ce) prove reached eager inputs separately from return. The 130-argument refusal retains its count. Active contexts/nonconstant guards, broader callee forms and optional defaults on non-total pinned models remain open.
- **Migration:** compiler output **94**, extractor output **33**, synthesis template 20, catalog format **5**, FORMAT **9**. Action/effect schemas and append-only codebooks were read and accepted. Fresh stores/generations follow ADR-0048. Installed native remains compiler93; no native action claim is made.
- Earlier typed value/site coverage, complete empty escaping-exception domains under entry, fresh nested defaults, shared callee admission, transfer alternatives, source parameter identity and current RCA/Pass C fidelity remain partial implemented subsets. No positive witness closes unrelated paths/channels.
- The [bounded review](docs/design_review/reviews/design_review_stage3-channel-contracts_2026-09-26.md) separates slice evidence from assembled Stage 3 acceptance. No remote push requested.

## Last verified (2026-09-27)

Every Cargo/native-build command uses `CARGO_TARGET_DIR=/home/paul/library-context/target`: the inherited shell points at `/home/paul/pse-arrow/target`.

| Command | Outcome and scope |
|---|---|
| `INSTA_UPDATE=no RUST_MIN_STACK=16777216 cargo nextest run --release -p cpg-schema -p cpg-core -p lctx-analytics --lib --test contracts --test codebooks --test compile -E 'binary(contracts) \| binary(codebooks) \| test(models::) \| test(actions::) \| test(action_triggers_preserve_partial_io_and_withhold_unproved_outcomes) \| test(call_execution_proves_reached_inputs_without_inventing_callee_completion) \| test(validation_schema_candidates_keep_attribution_separate_from_subjects)' --status-level fail --final-status-level fail` | **passed:** 30, 177 skipped, 5.694 s; `/tmp/lctx-stage3-actions-tests6.log`. Includes Normal/Finally contract controls, ordered-evidence/scope mutations, 64→65 cap, malformed descriptors, real source/Delta assessments and publication rejection. Earlier selections had expected migrations and corrected test-control errors; no unresolved failure from this slice. |
| `uv run --no-sync python docs/design_review/evidence/2026-09-27_action-triggers/runtime_oracle.py`; `uv run --no-sync ruff check docs/design_review/evidence/2026-09-27_action-triggers/runtime_oracle.py` | **passed:** nine independent generated CPython 3.14.7 partial-I/O cases; harness line lengths corrected. [Evidence and limits](docs/design_review/evidence/2026-09-27_action-triggers/README.md). Analyzer fixtures were never executed. |
| `just adr index`; `just adr lint`; `git diff --check` | **passed:** ADR-0060 target accepted after bounded review; whitespace check passed. No product completion claim. |
| Prior compiler93 source/native qualification | **passed within its scope:** original modeled-finalizer test; affected bundle replay after fixture repair; 16 Python schema/native cases; Pyrefly 0 errors; seven generated identity controls. Exact commands/failures/replays in [review §24](docs/design_review/reviews/design_review_stage3-channel-contracts_2026-09-26.md#24-bounded-implementation-follow-up-modeled-return-identity). |
| `uv run --no-sync pytest tests/scripts/test_flow_soundness.py -q` (99b5105) | **passed:** 18 raw-flow/region cases using this checkout's release binary; composed semantic/original-cap challenges remain open. |
| `just fmt`; `just test-all`; fresh `just pilot`; structured Q01/Q03/Q05/Q09; clean wheel; live W9; all-techniques/cost; `just docs-check` | **not_run:** integrated qualification awaits assembled S1–S7 functionality. |

## Next work and boundaries

- Separate exact pinned-model default availability from normal return, with its own authored premise/evidence and positive/withholding controls; no inferred default value or omitted-argument subject. Continue broader bindings/models and resource/callback fates. Native action export/selection remains S6.
- S4/S5 retain stable multi-control/residual conjunction, all-channel SCC semantics with separate witness progress, actual callee/model coverage and claim-specific discharge. New behavioral FCA/registry stays Stage 4.
- S6/S7 retain FORMAT 9 semantic queries/full evidence, original W5/W7/W12 default-cap producer→Delta→native traces and independent compiler/oracle challenges. Stage 5 execution remains outside scope.
- Finish S1–S7 before integrated S8 gates. Focused receipts certify only their named cases; no current full-suite or assembled Stage 3 receipt exists.
