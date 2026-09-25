# Status

_Updated 2026-09-25 under the [handoff skill](.claude/skills/handoff/SKILL.md); work is on `main`._

## Increment and slice

- Active: [Stage 3 of increment 4](docs/plans/behavioral-model-forward-plan_2026-09-24.md). The current committed code boundary is `0cef120`; this STATUS/plan checkpoint update is the only uncommitted work while being written.
- Stage 3.0 has persisted bounded BDDs, attributed entry-value links and narrow primitive refutation. No served compatibility verdict exists.
- Stage 3.1 has a typed pinned model compiler and cited source applications for transfer, effect, callback, resource and exception candidates. The catalog is still narrow and CrossHair only covered an `int` specialization of `typing.cast`.
- Stage 3.2 cites local exits, handler clauses/actions, pinned class/MRO relationships and direct modeled-exception-to-handler candidates. Nested handler propagation, callback fate and resource lifecycle remain open.
- Stage 3.3 has raw value-flow contributions, separate inherited predecessor candidates, bounded BDD compatibility, exact whole-expression model steps and an assignment-to-return candidate (`3357e5f`, `4f1d8b4`). `summary_flows` now contains only proved direct synchronous identity returns (`7eed4e5`); `summary_boundaries` records other same-callable parameter-origin return paths (`0cef120`). No model call result is promoted to a completed flow.

## Last verified (2026-09-25)

| Command | Outcome |
|---|---|
| `INSTA_UPDATE=no cargo nextest run --release -p cpg-schema -p cpg-core -E 'test(contracts_snapshot) \| test(rules_snapshot) \| test(pinned_identity_models_require_and_publish_their_real_formals)' --no-tests=pass --no-fail-fast` | `passed` 3/3 after `0cef120` changes; positive, withholding and publication-tamper checks. |
| `INSTA_UPDATE=no cargo nextest run --release -p cpg-schema -p cpg-core -E 'test(contracts_snapshot) \| test(registry_snapshot) \| test(rules_snapshot) \| test(pinned_identity_models_require_and_publish_their_real_formals)' --no-tests=pass --no-fail-fast` | `passed` 4/4 for `7eed4e5`, including the append-only codebook snapshot. |
| `cargo clippy --release -p cpg-schema -p cpg-core --all-targets -- -D warnings`; `just adr lint`; `git diff --check` | `passed` after `0cef120` changes; ADR lint reports 32 records. |
| `just fmt`; `just check`; current-HEAD `just test-all`; fresh-store `just pilot`; Q01/Q03/Q05/Q09; clean wheel/native query; `just adr revisit` | `not_run`: operator requires all integrated testing and formatting only after the entire Stage 3 functional scope is implemented. The earlier full gate at `d61a497` does not certify this tree. |

## Known failures, blocks and decisions

- No current focused test failure is known. Release-scale pilot counts, full behavioral closure, served compatibility and integrated acceptance are unverified.
- ADR-0020, ADR-0024, ADR-0025 and ADR-0028 are proposed; ADR-0031 and ADR-0032 are accepted within their narrow tested scope. The current ADR index was read; `just adr index` was not run during this checkpoint.
- The latest compact review defers field/effect/handler/call-target coverage beyond same-callable parameter-origin returns. A positive direct flow cannot make separate unproved paths disappear; `summary_boundaries` retains them.
- Remaining functional scope: establish modeled normal completion/target closure; finish L2 handler, callback and resource fates; broaden pinned model families and CrossHair checks; compose bounded petgraph SCC flow/effect summaries and discharge `call_transfer`; run independent Pysa and CPython/Hypothesis oracles; implement FORMAT 7 native/PyO3 serving and typed filters. Only then run formatting, integrated gates, structured evaluation, clean-wheel query and increment-end review.

## Next

Define and prove the first modeled normal-return/complete-target contract from a pinned source or typed model, then use it with `modeled_exact_value_transfers` to compose a finite call-result flow. Keep candidate paths and absent proofs `unknown`; use focused checks only until the full Stage 3 functionality is assembled.
