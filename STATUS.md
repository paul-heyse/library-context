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
- **Migration:** compiler output **86**, extractor output 32; fresh stores/generations under ADR-0048. FORMAT **9** has the callable-support subset; remaining S6 contracts are unimplemented.
- **S5 transfer repair:** distinct identity/derived/call conditions, predecessor source origins, propagated provider approximation, exclusive receiver/singleton narrowing and typed served Fate transfer/scope. Extractor 32 separates call transfer from provider approximation. Unsupported/captured/decorated alternatives remain open.
- **New S1/S2 qualification gap:** `handler_entry_identity` has source entry/exit evidence but an approximated reaching value. The stricter producer correctly withholds it; an occurrence-specific source parameter-identity certificate is required before its positive native regression can pass.
- The [bounded review](docs/design_review/reviews/design_review_stage3-channel-contracts_2026-09-26.md) accepts the target and this implementation subset at source-inspection strength after F01–F07 corrections. This does not certify assembled Stage 3. No remote push requested.

## Last verified (2026-09-27 unless noted)

Cargo commands below use `CARGO_TARGET_DIR=/home/paul/library-context/target`: the inherited shell pointed at `/home/paul/pse-arrow/target`, so these checks explicitly reuse this checkout's established cache.

| Command | Outcome and scope |
|---|---|
| `INSTA_UPDATE=no RUST_MIN_STACK=16777216 cargo nextest run --release -p cpg-core -p cpg-schema -p lctx-analytics --lib --test contracts --test codebooks --test compile --test behavior --test bundle -E 'test(reach_fixed_point_tests) \| test(summaries::) \| test(pass_b::tests) \| binary(contracts) \| binary(codebooks) \| test(transfer_alternatives_keep_conditions_verdicts_and_receiver_boundaries) \| test(a_negative_claim_is_refuted_only_where_its_premise_holds) \| test(serving_schema_digests_are_the_shared_known_answers) \| test(finite_depth_and_unsupported_refusals_reach_the_native_response) \| binary(behavior)' --status-level fail --final-status-level fail` | `failed`: 57 cases, 56 passed; the existing positive native handler-return expectation is now withheld for provider approximation. All transfer fixture source→Delta→bundle→Python checks, condition/order controls, behavior_shapes checks, schema/codebook contracts and digests passed. `/tmp/lctx-stage3-transfer-final-acceptance.log`. |
| `uv run --no-sync pytest python/lctx_mcp/tests/test_digest.py -k expected_schemas` | `passed`: Python/Rust schema agreement; deliberately regenerated known answers change only the behaviors schema in this slice. |
| `INSTA_UPDATE=no RUST_MIN_STACK=16777216 cargo nextest run --release -p cpg-core --test compile -E 'test(transfer_alternatives_keep_conditions_verdicts_and_receiver_boundaries)' --status-level fail --final-status-level fail` | `passed`: final transfer fixture after rejecting multiple class declarations with one qualified name; conditional class selection cannot choose an arbitrary receiver class. |
| `uv sync --frozen --reinstall-package lctx-semantics` | `passed`: editable native package rebuilt for compiler-86 transfer/scope schemas before the final producer corrections. Fresh rebuild and complete native receipt still required with the next S1/S2 correction. Not clean-wheel qualification. |
| Earlier focused schema/model/default/callable source/Delta/native commands | `passed` in compiler outputs 82–85; scopes retained in the forward plan and bounded review. Those receipts do not certify the current handler value admission. |
| `just fmt`; `just test-all`; fresh `just pilot`; structured Q01/Q03/Q05/Q09; clean wheel; live W9; all-techniques/cost; `just docs-check` | `not_run`: formatting and integrated qualification await assembled S1–S7 functionality. |

## Next work and boundaries

- First complete the source parameter-identity certificate and restore the handler source/native control. Finish remaining S1 channel contracts and supported synchronous context entry/exit and suppression with initial S3 models; dynamic-schema representation is implemented, activation remains open. Reassess default stability before broadening the expression evaluator to effectful protocols.
- S4/S5 retain stable multi-control/residual conjunction, all-channel SCC semantics and witness separation, actual callee/model coverage, remaining transfer qualification and current typed RCA repairs. No finite witness closes all alternatives.
- S6/S7 retain FORMAT 9 semantic queries/evidence, actual default-cap producer→Delta→native traces and independent compiler/oracle challenges. Stage 5 deferred execution remains outside this scope.
- Full scope and integrated acceptance remain open; no current full-suite receipt exists. Prior runtime probes and earlier focused results remain evidence only for their stated scope.
