# Status

_Updated 2026-09-27 under the [handoff skill](.claude/skills/handoff/SKILL.md); work is on `main`._

## Increment and slice

- **Increment 4 / Stage 3 remains functionally incomplete.** The operator-approved [forward plan §3.0](docs/plans/behavioral-model-forward-plan_2026-09-24.md#30-consolidated-execution) owns S0–S8; §6 owns finding disposition. ADR-0057/0058 accept the target, not its completion.
- **Current S1/S2/S6 slice:** typed value-origin and execution-site coverage; complete empty escaping-exception domains under statement entry; shared callee proof admission in source composition and native loading; fresh nested definition-header completion without evaluating its body.
- Site coverage excludes async/generator scopes, proves neither reachability nor generic raise/catch/suppression activity, and uses no fabricated parameter identity. Call-origin coverage remains open; default invocation requires availability/stability evidence even for unused omitted formals.
- Shared proof admission preserves `summary_proof_limit`; source reconstruction still owns binding/evaluation. The independent native adjacency classifier is deleted.
- **Model contracts:** catalog format 3 requires authored phase applicability and typed static/runtime/unresolved validation schema attribution separate from the effect subject. Runtime candidates retain a schema source or explicit reason; resource endpoints remain input/output only. No production validation effect is active. Extraction-only compilation emits no coverage or reconstructed entry links against an absent analysis catalog.
- **Defaults:** the pure source binder admits a fresh undecorated nested synchronous definition immediately followed by its sole direct return call. Defaults are evaluated at definition; availability, Boolean value and stability remain separate proof evidence. Alias/escape, intervening action, effectful argument, missing default and unsupported forms stay open.
- **Callable support:** private/nested guard links share source admission, and FORMAT 9 `callable_parameters` closes native callee proofs without exposing private operations. Public-only parameter projection and FORMAT 8 reader are removed; general S6 semantics remain open.
- **Migration:** compiler output **87**, extractor output 32; fresh stores/generations under ADR-0048. FORMAT **9** has the callable-support subset; remaining S6 contracts are unimplemented.
- **S5 transfer repair:** distinct identity/derived/call conditions, predecessor source origins, propagated provider approximation, exclusive receiver/singleton narrowing and typed served Fate transfer/scope. Extractor 32 separates call transfer from provider approximation. Unsupported/captured/decorated alternatives remain open.
- **Source identity:** the handler return now has an occurrence-specific lexical parameter certificate. Raw provider approximation remains recorded; only the matching origin can use it, with separate entry/exit proofs. Native checks reject missing/foreign/duplicate/uncited support; publication reconstructs the source proof. S1/S2/S6 remain partial.
- The [bounded review](docs/design_review/reviews/design_review_stage3-channel-contracts_2026-09-26.md) accepts the target and this implementation subset at source-inspection strength after F01–F08 corrections. This does not certify assembled Stage 3. No remote push requested.

## Last verified (2026-09-27 unless noted)

Cargo commands below use `CARGO_TARGET_DIR=/home/paul/library-context/target`: the inherited shell pointed at `/home/paul/pse-arrow/target`, so these checks explicitly reuse this checkout's established cache.

| Command | Outcome and scope |
|---|---|
| `INSTA_UPDATE=no RUST_MIN_STACK=16777216 cargo nextest run --release -p cpg-schema -p cpg-core -p lctx-analytics --lib --test contracts --test codebooks --test bundle --test compile -E 'test(summaries::) \| binary(contracts) \| binary(codebooks) \| test(finite_depth_and_unsupported_refusals_reach_the_native_response) \| test(transfer_alternatives_keep_conditions_verdicts_and_receiver_boundaries) \| test(serving_schema_digests_are_the_shared_known_answers)' --status-level fail --final-status-level fail` | `failed`: 42 passed, 1 failed. Certificate/native malformed-evidence cases passed; the old 64-step fixture now needed one extra identity certificate. `/tmp/lctx-stage3-parameter-identity-final.log`. |
| `INSTA_UPDATE=no RUST_MIN_STACK=16777216 cargo nextest run --release -p cpg-core --test bundle -E 'test(finite_depth_and_unsupported_refusals_reach_the_native_response)' --status-level fail --final-status-level fail` | `passed`: adjusted fixture finalizers preserve exact 64/65-step controls with the unchanged production limit. Source→Delta→native handler and missing/foreign/removed/duplicate certificate cases pass; publication rejects missing source certificates. `/tmp/lctx-stage3-parameter-identity-cap-control.log`. |
| `uv run --no-sync pytest python/lctx_mcp/tests/test_digest.py -k expected_schemas` | `passed`: Python/Rust schema agreement; known answers add only the new certificate schema. Store/rule/codebook snapshots reviewed and accepted as compiler-87 migration. |
| `uv sync --frozen --reinstall-package lctx-semantics` | `passed`: editable native rebuilt with certificate reverse closure before the final focused checks. Not clean-wheel qualification. |
| Earlier focused schema/model/default/callable/transfer commands | `passed` for their recorded compiler outputs; those receipts do not establish assembled Stage 3 acceptance. |
| `just fmt`; `just test-all`; fresh `just pilot`; structured Q01/Q03/Q05/Q09; clean wheel; live W9; all-techniques/cost; `just docs-check` | `not_run`: formatting and integrated qualification await assembled S1–S7 functionality. |

## Next work and boundaries

- Continue typed current RCA modality/phase/evidence repair in S5. Finish remaining S1 channel contracts and supported synchronous context entry/exit and suppression with initial S3 models; dynamic-schema representation is implemented, activation remains open. Reassess default stability before broadening the expression evaluator to effectful protocols.
- S4/S5 retain stable multi-control/residual conjunction, all-channel SCC semantics and witness separation, actual callee/model coverage, remaining transfer qualification and current typed RCA repairs. No finite witness closes all alternatives.
- S6/S7 retain FORMAT 9 semantic queries/evidence, actual default-cap producer→Delta→native traces and independent compiler/oracle challenges. Stage 5 deferred execution remains outside this scope.
- Full scope and integrated acceptance remain open; no current full-suite receipt exists. Prior runtime probes and earlier focused results remain evidence only for their stated scope.
