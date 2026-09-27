# Status

_Updated 2026-09-27 under the [handoff skill](.claude/skills/handoff/SKILL.md); work is on `main`._

## Increment and slice

- **Increment 4 / Stage 3 remains functionally incomplete.** The operator-approved [forward plan §3.0](docs/plans/behavioral-model-forward-plan_2026-09-24.md#30-consolidated-execution) owns S0–S8; §6 owns finding disposition. ADR-0057/0058/0059 accept targets, not integrated completion.
- **Current S2b/S3a slice:** catalog format 4 binds CPython 3.14.7 `nullcontext`/`suppress` class, allocation and initialization separately. Fresh synchronous source sites admit ordered arguments, entry registration before `as` assignment, reverse exits, suppression only on Raise and supported outcome replacement. Provider exit observations are not fabricated.
- Every production value summary now requires an ordered return-completion certificate, including empty obligations and explicit condition scope. Publication reconstructs it; shared source/native admission rejects missing, foreign, swapped or omitted evidence and retains typed condition refusals. The 64-step proof cap is unchanged.
- **Migration:** compiler output **89**, extractor output **33**, synthesis template 20, catalog format **4**, FORMAT **9**. Fresh stores/generations follow ADR-0048. New context and return-certificate schemas and codebooks were read and accepted. Remaining S6 semantic queries/evidence are open.
- **Earlier subsets:** typed value/site coverage and complete empty escaping-exception domains under site entry; fresh immediate nested/default certificates; shared callee admission; transfer alternatives and lexical parameter-identity certificates; typed current RCA/Pass C support. Raw provider approximation stays recorded; witnesses do not close unrelated paths/channels.
- The [bounded review](docs/design_review/reviews/design_review_stage3-channel-contracts_2026-09-26.md) distinguishes tested subsets from assembled Stage 3 acceptance. No remote push requested.

## Last verified (2026-09-27)

Every Cargo/native-build command uses `CARGO_TARGET_DIR=/home/paul/library-context/target`: the inherited shell points at `/home/paul/pse-arrow/target`.

| Command | Outcome and scope |
|---|---|
| `INSTA_UPDATE=no RUST_MIN_STACK=16777216 cargo nextest run --release -p cpg-schema -p lctx-analytics -p cpg-core --lib --test contracts --test codebooks --test compile --test bundle -E 'test(completion_proof::) \| test(summary_contract::) \| test(context_protocol::) \| test(summaries::finite::) \| test(completion::) \| binary(contracts) \| binary(codebooks) \| test(context_protocols_bind_class_and_constructor_roles_independently) \| test(a_generation_rebuilds_to_the_same_bytes)' --status-level fail --final-status-level fail` | `passed`: 57 cases, 147 skipped. Ordered/source/native tamper controls, actual bounded implication, unknown versus mismatch, deterministic generation. `/tmp/lctx-stage3-context-final2.log`. |
| Focused `cargo nextest run --release` schema/compile/bundle regression selection | `failed`: one expected schema-migration difference, subsequently read/accepted; the other 18 cases passed. Finalizer serving, existing call/default paths, unchanged 64/65 proof boundary, serving schema digest and regenerated Python fixture. `/tmp/lctx-stage3-context-regression.log`. |
| `uv run --no-sync pytest python/lctx_mcp/tests/test_server.py python/lctx_mcp/tests/test_digest.py`; `uv run --no-sync pyrefly check python/lctx_mcp/src/lctx_mcp/generation.py` | `passed`: 15 tests; 0 type errors. `/tmp/lctx-stage3-context-python.log`, `/tmp/lctx-stage3-context-pyrefly.log`. |
| `uv sync --frozen --reinstall-package lctx-semantics` | `passed`: current native editable build, `/tmp/lctx-stage3-context-native-sync3.log`. `INSTA_UPDATE=no RUST_MIN_STACK=16777216 cargo nextest run --release -p cpg-core --test compile -E 'test(context_protocols_bind_class_and_constructor_roles_independently)' --status-level fail --final-status-level fail` then `passed`: 1 case, 30 skipped, `/tmp/lctx-stage3-context-current-native.log`. |
| `uv run --no-sync python docs/design_review/evidence/2026-09-27_sync-contexts/runtime_oracle.py` | `passed`: 14 isolated finite runtime cases, source hash and lifecycle traces in [evidence](docs/design_review/evidence/2026-09-27_sync-contexts/README.md); compiler fixtures never executed. Entry-result runtime observation alone is not compiler identity qualification. |
| `just fmt`; `just test-all`; fresh `just pilot`; structured Q01/Q03/Q05/Q09; clean wheel; live W9; all-techniques/cost; `just docs-check` | `not_run`: integrated qualification awaits assembled S1–S7 functionality. |

## Next work and boundaries

- Prove context entry-result identity separately from manager identity after this bounded lifecycle checkpoint. Named-handler cleanup, resources/callbacks and remaining pinned model families/independent challenges stay open. Unsupported argument/entry domains, custom managers, exception groups and deferred execution remain named boundaries.
- S4/S5 retain stable multi-control/residual conjunction, all-channel SCC semantics with separate witness progress, actual callee/model coverage and claim-specific discharge. New behavioral FCA/registry stays Stage 4.
- S6/S7 retain FORMAT 9 semantic queries/full evidence, actual default-cap producer→Delta→native traces and independent compiler/oracle challenges. Stage 5 execution remains outside scope.
- Finish S1–S7 before integrated S8 gates. Focused receipts certify only their named cases; no current full-suite or assembled Stage 3 receipt exists.
