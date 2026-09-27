# Status

_Updated 2026-09-27 under the [handoff skill](.claude/skills/handoff/SKILL.md); work is on `main`._

## Increment and slice

- **Increment 4 / Stage 3 remains functionally incomplete.** The operator-approved [forward plan §3.0](docs/plans/behavioral-model-forward-plan_2026-09-24.md#30-consolidated-execution) owns S0–S8; §6 owns finding disposition. ADR-0057/0058/0059 accept targets, not integrated completion.
- **Prior S2b/S3a slice (5778c9a):** a separate source/model certificate proves a bare return of a unique `as` name inside an active pinned context when its explicit entry argument is an immutable current parameter. Both lexical reads, source origin and actual site/argument are retained. Raw nonidentity/call-crossing flags stay unchanged; coverage stays open.
- **Current S5 slice:** direct single-model returns in approximate regions now compose an immutable lexical-parameter identity with the existing exact model-call proof. Raw flags and independent completion obligations remain intact. The original modeled-finalizer positive regression is corrected; broader chains/assignments and coverage stay open.
- Schema/native value-basis admission follows declared path depth, so deleting a whole modeled group fails while a context-value return with a modeled predecessor still passes. FORMAT 9 carries occurrence-specific source/model commitments. Full raw-fact semantic closure remains S6.
- **Prior S2b/S3a slice (bf322ce):** reached eager call inputs have independent statement-prefix/invocation proofs; the 130-argument refusal retains its count. Native action export/consumers, contexts and nonconstant guards remain open.
- Selected named handlers retain `handler_name_cleanup`; skipped handlers do not. General binding/deletion completion remains unsupported.
- Shared source/native admission requires successful assignment before the value witness, cleanup after it and matching return-condition scope. A local base return requires exactly one value basis independently of completion; omitting both the certificate and witness is rejected. Full raw-fact semantics remain part of S6.
- Every production value summary now requires an ordered return-completion certificate, including empty obligations and explicit condition scope. Publication reconstructs it; shared source/native admission rejects missing, foreign, swapped or omitted evidence and retains typed condition refusals. The 64-step proof cap is unchanged.
- **Migration:** compiler output **93**, extractor output **33**, synthesis template 20, catalog format **4**, FORMAT **9**. Fresh stores/generations follow ADR-0048. New context and return-certificate schemas and codebooks were read and accepted. Remaining S6 semantic queries/evidence are open.
- **Earlier subsets:** typed value/site coverage and complete empty escaping-exception domains under site entry; fresh immediate nested/default certificates; shared callee admission; transfer alternatives and lexical parameter-identity certificates; typed current RCA/Pass C support. Raw provider approximation stays recorded; witnesses do not close unrelated paths/channels.
- The [bounded review](docs/design_review/reviews/design_review_stage3-channel-contracts_2026-09-26.md) distinguishes tested subsets from assembled Stage 3 acceptance. No remote push requested.

## Last verified (2026-09-27)

Every Cargo/native-build command uses `CARGO_TARGET_DIR=/home/paul/library-context/target`: the inherited shell points at `/home/paul/pse-arrow/target`.

| Command | Outcome and scope |
|---|---|
| `INSTA_UPDATE=no RUST_MIN_STACK=16777216 cargo nextest run --release -p cpg-schema -p cpg-core -p lctx-analytics --lib --test contracts --test codebooks --test compile --test bundle -E 'binary(contracts) \| binary(codebooks) \| test(summaries::finite::) \| test(pinned_identity_models_require_and_publish_their_real_formals) \| test(context_protocols_bind_class_and_constructor_roles_independently) \| test(finite_depth_and_unsupported_refusals_reach_the_native_response)' --status-level fail --final-status-level fail` | `failed`: 39 passed, 1 fixture-import edit failure. Restored conditional imports; fixture `ast.parse` passed. Affected `cargo nextest run --release -p cpg-core --test bundle -E 'test(finite_depth_and_unsupported_refusals_reach_the_native_response)' --status-level fail --final-status-level fail` with same env then `passed`: 1, 6 skipped. `/tmp/lctx-stage3-modeled-identity-tests2.log`, `tests4.log`. Original modeled-finalizer test passed. |
| `uv sync --frozen --reinstall-package lctx-semantics` | `passed`: compiler93 native rebuild before the focused tests; `/tmp/lctx-stage3-modeled-identity-sync.log`. |
| `uv run --no-sync python docs/design_review/evidence/2026-09-27_modeled-identity/runtime_oracle.py` | `passed`: seven generated object-identity controls, also compared with published/native results; [evidence](docs/design_review/evidence/2026-09-27_modeled-identity/README.md). |
| `LCTX_PY_FIXTURE=$PWD/build/py-fixture` plus release bundle `writes_the_python_fixture_generation`; `LCTX_WRITE_KNOWN_ANSWERS=1` plus release bundle `serving_schema_digests_are_the_shared_known_answers` | `passed`: fresh Python generation and the sole added serving schema digest, read/accepted. Earlier Python run failed on stale generation; refreshed run supersedes it. |
| `uv run --no-sync pytest python/lctx_mcp/tests/test_digest.py python/lctx_mcp/tests/test_native_semantics.py -q`; `uv run --no-sync pyrefly check --summary=none` | `passed`: 16 Python cases; initial Pyrefly failure in existing literal-list test annotation corrected, affected `-k private_callable_formals` replay passed and type check now 0 errors. `/tmp/lctx-stage3-modeled-identity-python3.log`, `pyrefly2.log`. |
| Call-input slice (bf322ce) | `passed`: 34 focused cases and 13 independent CALL-event programs; scope/commands in [review §23](docs/design_review/reviews/design_review_stage3-channel-contracts_2026-09-26.md#23-bounded-implementation-follow-up-reached-call-inputs). |
| Context entry-value slice (5778c9a) | `passed`: 45 focused cases; current-native 1 case, Python 15, pyrefly 0 errors, 14 generated runtime programs. Exact commands/limits in [review §22](docs/design_review/reviews/design_review_stage3-channel-contracts_2026-09-26.md#22-bounded-implementation-follow-up-context-entry-value-identity). |
| `uv run --no-sync pytest tests/scripts/test_flow_soundness.py -q` | `passed`: 18 raw-flow/region oracle cases using this checkout's release `lctx`; `/tmp/lctx-stage3-release-oracle.log`. Composed semantic coverage is separate. |
| `just fmt`; `just test-all`; fresh `just pilot`; structured Q01/Q03/Q05/Q09; clean wheel; live W9; all-techniques/cost; `just docs-check` | `not_run`: integrated qualification awaits assembled S1–S7 functionality. |

## Next work and boundaries

- Compose reached-call proofs into authored action timing and resource/callback fates. The encountered modeled-finalizer positive regression is corrected; broader approximated modeled chains/assignments require complete source proofs. Named-handler proof-or-specific-boundary requirement is met; general cleanup semantics, resources/callbacks and remaining pinned model families/independent challenges stay open. Unsupported argument/entry domains, custom managers, exception groups and deferred execution remain named boundaries.
- S4/S5 retain stable multi-control/residual conjunction, all-channel SCC semantics with separate witness progress, actual callee/model coverage and claim-specific discharge. New behavioral FCA/registry stays Stage 4.
- S7 raw-flow oracle now uses the release binary; composed semantic and default-cap challenges remain open.
- S6/S7 retain FORMAT 9 semantic queries/full evidence, actual default-cap producer→Delta→native traces and independent compiler/oracle challenges. Stage 5 execution remains outside scope.
- Finish S1–S7 before integrated S8 gates. Focused receipts certify only their named cases; no current full-suite or assembled Stage 3 receipt exists.
