# Status

_Updated 2026-09-25 under the [handoff skill](.claude/skills/handoff/SKILL.md); work is on `main`._

## Increment and slice

- Active: [Stage 3 of increment 4](docs/plans/behavioral-model-forward-plan_2026-09-24.md). Stage 3.0 has bounded BDD conditions, attributed entry-value proofs and focused primitive refutation, but no served compatibility verdict.
- Stage 3.1's committed model compiler (`dd5341f`–`0a7333b`) parses a tagged catalog, binds pinned external definitions/formals and publishes typed transfer, effect, callback, resource and exception rules with per-channel coverage. The CrossHair receipt covers only the `int` specialization of `typing.cast`.
- Stage 3.2's structural exit/handler sources (`6110afd`, `000542f`, `d2183e5`, `fd7e690`/ADR-0027) cite return/raise/finally, handler clauses/actions and pinned handler type sources. Unresolved enclosing frames withhold a definite raise-escape conclusion.
- Source/model bridge (`3878aeb`–`fa8d134`) applies pinned model targets at source calls, binds exact explicit arguments across every signature, and publishes candidate callback, resource, transfer and effect sites. Each has shared reconstruction, positive/withholding/tamper checks and a compact review. Candidate action rows preserve modality/open dispatch and are not whole-operation fates.
- The working tree was clean after `fa8d134` before this handoff update. `STATUS.md` is the only intended new edit at this checkpoint.

## Last verified (2026-09-25)

| Command | Outcome |
|---|---|
| Focused `INSTA_UPDATE=no cargo nextest run --release -p cpg-schema -p cpg-core -E '…' --no-tests=pass --no-fail-fast` at each of `d0eb167`, `a6222ee`, `fa8d134` | `passed`: 6/6, 7/7 and 7/7 respectively, covering source cases, tamper, schema/rules and output ledger. Earlier Stage 3 slices recorded their focused outcomes in commit messages/reviews. |
| `cargo clippy --release -p cpg-core -p cpg-schema --all-targets -- -D warnings` after each of those slices | `passed`. |
| `cargo fmt --all -- --check`; `git diff --check` before those commits | `passed`. |
| `just adr index`; `just adr revisit` | `passed`: ADR-0002's `just deps` trigger passed; other triggers are manual. |
| `just test-all` at `d61a497`; `just pilot build/store-stage3-entry-links-v2-2026-09-24` at `d61a497` | `passed` historically, before subsequent Stage 3 changes; they do not certify current HEAD. |
| `just check`; current-HEAD `just test-all`; fresh-store `just pilot`; Q01/Q03/Q05/Q09; clean wheel/native query | `not_run`: the operator requires integrated tests only after the full functional scope is implemented. |

## Known failures, decisions and blocks

- No current focused test failure is known. The integrated Stage 3 result remains **unverified**; no pilot counts, performance or served compatibility result can be claimed for current HEAD.
- ADR-0020, ADR-0024 and ADR-0025 remain proposed. The Stage 3 source reviews defer handler catch/completion, runtime resource identity/release, candidate-call normal completion, source effect subjects, and L3/serving evidence closure.
- Remaining functional scope: broaden the pinned model catalog and CrossHair oracle; resolve L2 handler, callback and resource fates; compose bounded SCC summaries that discharge `call_transfer`; run Pysa and CPython/Hypothesis oracles; implement FORMAT 7 native/PyO3 serving and filters; then evaluate Q01/Q03/Q05/Q09 and run the clean wheel and integrated gates.

## Next

Derive an attributed modeled exception source and resolved handler/raise matching boundary before L3 summary composition. Keep `just test-all`, `just pilot` and structured evaluation deferred until all Stage 3 functional scope is assembled.
