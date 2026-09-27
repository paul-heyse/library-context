# Status

_Updated 2026-09-27 under the [handoff skill](.claude/skills/handoff/SKILL.md); work is on `main`._

## Increment and slice

- **Increment 4 / Stage 3 remains functionally incomplete.** The operator-approved [forward plan §3.0](docs/plans/behavioral-model-forward-plan_2026-09-24.md#30-consolidated-execution) owns S0–S8; §6 owns finding disposition. ADR-0057/0058/0059 accept targets, not integrated completion.
- **Prior S2b/S3a slice (5778c9a):** a separate source/model certificate proves a bare return of a unique `as` name inside an active pinned context when its explicit entry argument is an immutable current parameter. Both lexical reads, source origin and actual site/argument are retained. Raw nonidentity/call-crossing flags stay unchanged; coverage stays open.
- **Current S2b/S3a slice:** reached eager call inputs have separate ordered statement-prefix and invocation proofs, persisted and source-reconstructed through shared admission. A call before a raise or a call that raises can be reached without normal return. Oversized refusals retain the actual count and `invocation_argument_limit`; contexts/nonconstant guards remain open. Native call-entry export/action consumers remain S3/S6.
- Selected named handlers retain `handler_name_cleanup`; skipped handlers do not. General binding/deletion completion remains unsupported.
- Shared source/native admission requires successful assignment before the value witness, cleanup after it and matching return-condition scope. A local base return requires exactly one value basis independently of completion; omitting both the certificate and witness is rejected. Full raw-fact semantics remain part of S6.
- Every production value summary now requires an ordered return-completion certificate, including empty obligations and explicit condition scope. Publication reconstructs it; shared source/native admission rejects missing, foreign, swapped or omitted evidence and retains typed condition refusals. The 64-step proof cap is unchanged.
- **Migration:** compiler output **92**, extractor output **33**, synthesis template 20, catalog format **4**, FORMAT **9**. Fresh stores/generations follow ADR-0048. New context and return-certificate schemas and codebooks were read and accepted. Remaining S6 semantic queries/evidence are open.
- **Earlier subsets:** typed value/site coverage and complete empty escaping-exception domains under site entry; fresh immediate nested/default certificates; shared callee admission; transfer alternatives and lexical parameter-identity certificates; typed current RCA/Pass C support. Raw provider approximation stays recorded; witnesses do not close unrelated paths/channels.
- The [bounded review](docs/design_review/reviews/design_review_stage3-channel-contracts_2026-09-26.md) distinguishes tested subsets from assembled Stage 3 acceptance. No remote push requested.

## Last verified (2026-09-27)

Every Cargo/native-build command uses `CARGO_TARGET_DIR=/home/paul/library-context/target`: the inherited shell points at `/home/paul/pse-arrow/target`.

| Command | Outcome and scope |
|---|---|
| `INSTA_UPDATE=no RUST_MIN_STACK=16777216 cargo nextest run --release -p cpg-schema -p cpg-core -p lctx-analytics --lib --test contracts --test codebooks --test compile -E 'binary(contracts) \| binary(codebooks) \| test(call_execution::) \| test(evaluation::) \| test(completion::) \| test(context_protocol::) \| test(call_execution_proves_reached_inputs_without_inventing_callee_completion)' --status-level fail --final-status-level fail` | `passed`: 34 cases; `/tmp/lctx-stage3-call-entry-tests5.log`. Schema migrations read/accepted. Includes source/Delta tamper controls and independent worker comparisons. |
| `uv run --no-sync python docs/design_review/evidence/2026-09-27_call-entry/runtime_oracle.py` | `passed`: 13 generated CPython 3.14.7 CALL-event programs; [evidence](docs/design_review/evidence/2026-09-27_call-entry/README.md). No callee completion/action claim. |
| `uv sync --frozen --reinstall-package lctx-semantics` | `passed`: compiler92 native rebuild; `/tmp/lctx-stage3-call-entry-sync.log`. |
| `INSTA_UPDATE=no RUST_MIN_STACK=16777216 cargo nextest run --release -p cpg-core --test compile --test bundle -E 'test(pinned_identity_models_require_and_publish_their_real_formals) \| test(finite_depth_and_unsupported_refusals_reach_the_native_response)' --status-level fail --final-status-level fail` | `failed`: 1 passed (current native), 1 failed (modeled try/finally identity lacks lexical source certificate); `/tmp/lctx-stage3-call-entry-native-regression.log`. The positive regression is retained in [plan §3.0](docs/plans/behavioral-model-forward-plan_2026-09-24.md#30-consolidated-execution); no baseline run attributes its introduction. |
| Context entry-value slice (5778c9a) | `passed`: 45 focused cases; current-native 1 case, Python 15, pyrefly 0 errors, 14 generated runtime programs. Exact commands/limits in [review §22](docs/design_review/reviews/design_review_stage3-channel-contracts_2026-09-26.md#22-bounded-implementation-follow-up-context-entry-value-identity). |
| `uv run --no-sync pytest tests/scripts/test_flow_soundness.py -q` | `passed`: 18 raw-flow/region oracle cases using this checkout's release `lctx`; `/tmp/lctx-stage3-release-oracle.log`. Composed semantic coverage is separate. |
| `just fmt`; `just test-all`; fresh `just pilot`; structured Q01/Q03/Q05/Q09; clean wheel; live W9; all-techniques/cost; `just docs-check` | `not_run`: integrated qualification awaits assembled S1–S7 functionality. |

## Next work and boundaries

- First repair the encountered modeled-finalizer positive regression with occurrence-specific source/model identity, retaining raw flags and completion obligations. Then compose reached-call proofs into action timing and resource/callback fates. Named-handler proof-or-specific-boundary requirement is met; general cleanup semantics, resources/callbacks and remaining pinned model families/independent challenges stay open. Unsupported argument/entry domains, custom managers, exception groups and deferred execution remain named boundaries.
- S4/S5 retain stable multi-control/residual conjunction, all-channel SCC semantics with separate witness progress, actual callee/model coverage and claim-specific discharge. New behavioral FCA/registry stays Stage 4.
- S7 raw-flow oracle now uses the release binary; composed semantic and default-cap challenges remain open.
- S6/S7 retain FORMAT 9 semantic queries/full evidence, actual default-cap producer→Delta→native traces and independent compiler/oracle challenges. Stage 5 execution remains outside scope.
- Finish S1–S7 before integrated S8 gates. Focused receipts certify only their named cases; no current full-suite or assembled Stage 3 receipt exists.
