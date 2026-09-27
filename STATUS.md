# Status

_Updated 2026-09-27 under the [handoff skill](.claude/skills/handoff/SKILL.md); work is on `main`._

## Increment and slice

- **Increment 4 / Stage 3 remains functionally incomplete.** The operator-approved [forward plan §3.0](docs/plans/behavioral-model-forward-plan_2026-09-24.md#30-consolidated-execution) owns S0–S8; §6 owns finding disposition. ADR-0057/0058/0059 accept targets, not integrated completion.
- **Current S2b/S3a slice:** a separate source/model certificate proves a bare return of a unique `as` name inside an active pinned context when its explicit entry argument is an immutable current parameter. Both lexical reads, source origin and actual site/argument are retained. Raw nonidentity/call-crossing flags stay unchanged; coverage stays open.
- Shared source/native admission requires successful assignment before the value witness, cleanup after it and matching return-condition scope. A local base return requires exactly one value basis independently of completion; omitting both the certificate and witness is rejected. Full raw-fact semantics remain part of S6.
- Every production value summary now requires an ordered return-completion certificate, including empty obligations and explicit condition scope. Publication reconstructs it; shared source/native admission rejects missing, foreign, swapped or omitted evidence and retains typed condition refusals. The 64-step proof cap is unchanged.
- **Migration:** compiler output **90**, extractor output **33**, synthesis template 20, catalog format **4**, FORMAT **9**. Fresh stores/generations follow ADR-0048. New context and return-certificate schemas and codebooks were read and accepted. Remaining S6 semantic queries/evidence are open.
- **Earlier subsets:** typed value/site coverage and complete empty escaping-exception domains under site entry; fresh immediate nested/default certificates; shared callee admission; transfer alternatives and lexical parameter-identity certificates; typed current RCA/Pass C support. Raw provider approximation stays recorded; witnesses do not close unrelated paths/channels.
- The [bounded review](docs/design_review/reviews/design_review_stage3-channel-contracts_2026-09-26.md) distinguishes tested subsets from assembled Stage 3 acceptance. No remote push requested.

## Last verified (2026-09-27)

Every Cargo/native-build command uses `CARGO_TARGET_DIR=/home/paul/library-context/target`: the inherited shell points at `/home/paul/pse-arrow/target`.

| Command | Outcome and scope |
|---|---|
| `INSTA_UPDATE=no RUST_MIN_STACK=16777216 LCTX_WRITE_KNOWN_ANSWERS=1 LCTX_PY_FIXTURE=/home/paul/library-context/build/py-fixture cargo nextest run --release -p cpg-schema -p cpg-core -p lctx-analytics --lib --test contracts --test codebooks --test compile --test bundle -E 'binary(contracts) \| binary(codebooks) \| test(context_value::) \| test(summary_contract::) \| test(summaries::finite::) \| test(context_protocols_bind_class_and_constructor_roles_independently) \| test(serving_schema_digests_are_the_shared_known_answers) \| test(writes_the_python_fixture_generation) \| test(a_generation_rebuilds_to_the_same_bytes)' --status-level fail --final-status-level fail` | `passed`: 45 cases, 160 skipped; `/tmp/lctx-stage3-context-value-final.log`. Positional/keyword and two-formal source controls, mutation/absent/sibling/override withholding, active-lifecycle unit, source/native tamper controls and deterministic generation. |
| Initial focused context-value selection with `INSTA_UPDATE=new` | `failed`: three expected schema/codebook migrations, subsequently read/accepted; 38 cases passed. `/tmp/lctx-stage3-context-value-tests.log`. |
| `uv run --no-sync pytest python/lctx_mcp/tests/test_server.py python/lctx_mcp/tests/test_digest.py`; `uv run --no-sync pyrefly check python/lctx_mcp/src/lctx_mcp/generation.py` | `passed`: 15 tests; 0 type errors. `/tmp/lctx-stage3-context-value-python.log`, `/tmp/lctx-stage3-context-value-pyrefly.log`. |
| `uv sync --frozen --reinstall-package lctx-semantics` | `passed`: current native editable build, `/tmp/lctx-stage3-context-value-sync2.log`; `INSTA_UPDATE=no RUST_MIN_STACK=16777216 cargo nextest run --release -p cpg-core --test compile -E 'test(context_protocols_bind_class_and_constructor_roles_independently)' --status-level fail --final-status-level fail` then `passed`: 1 case, 30 skipped, `/tmp/lctx-stage3-context-value-native-current.log`. |
| `uv run --no-sync python docs/design_review/evidence/2026-09-27_sync-contexts/runtime_oracle.py` | `passed`: 14 isolated generated programs, each with integer and fresh opaque-object invocations; [evidence](docs/design_review/evidence/2026-09-27_sync-contexts/README.md) records source hash and lifecycle traces. Compiler fixtures never executed. |
| `just fmt`; `just test-all`; fresh `just pilot`; structured Q01/Q03/Q05/Q09; clean wheel; live W9; all-techniques/cost; `just docs-check` | `not_run`: integrated qualification awaits assembled S1–S7 functionality. |

## Next work and boundaries

- Continue named-handler cleanup boundaries and resource/callback fates after this bounded entry-value checkpoint. Named-handler cleanup, resources/callbacks and remaining pinned model families/independent challenges stay open. Unsupported argument/entry domains, custom managers, exception groups and deferred execution remain named boundaries.
- S4/S5 retain stable multi-control/residual conjunction, all-channel SCC semantics with separate witness progress, actual callee/model coverage and claim-specific discharge. New behavioral FCA/registry stays Stage 4.
- S6/S7 retain FORMAT 9 semantic queries/full evidence, actual default-cap producer→Delta→native traces and independent compiler/oracle challenges. Stage 5 execution remains outside scope.
- Finish S1–S7 before integrated S8 gates. Focused receipts certify only their named cases; no current full-suite or assembled Stage 3 receipt exists.
