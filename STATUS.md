# Status

_Updated 2026-09-25 under the [handoff skill](.claude/skills/handoff/SKILL.md); work is on `main`._

## Increment and slice

- Active: [Stage 3 of increment 4](docs/plans/behavioral-model-forward-plan_2026-09-24.md). Stage 3.0 has bounded BDD conditions, attributed entry-value proofs and focused primitive refutation, but no served compatibility verdict.
- Stage 3.1's model compiler parses a tagged catalog, binds pinned external definitions/formals and publishes typed transfer, effect, callback, resource and exception rules. The catalog remains small (`typing.cast`, `print`, `open`, `atexit.register`); the CrossHair receipt covers only the `int` specialization of `typing.cast`.
- Stage 3.2's source relations cite return/raise/finally, handler clauses/actions and pinned handler types. Source/model bridges apply candidate transfer, effect, callback, resource and exception actions while retaining modality and open dispatch.
- ADR-0028's source bridge (`62ea65e`, `81d4d12`, `deb3560`) retains exact argument value spans, ordered nested value-call paths and a unique Ruff call/argument link or typed unknown. It does not discharge a transfer.
- The latest L2 slices (`719e9c0`, `9c23705`, `ed7662c`) bind modeled exception classes to pinned context facts, connect potential modeled raises to enclosing handler clauses with explicit ancestry-walk coverage, and cite Pyrefly's pinned MRO for positive ancestor candidates. Different-class nonmembership, handler selection/completion and exception escape remain unknown.
- The working tree is clean at this handoff after the STATUS commit.

## Last verified (2026-09-25)

| Command | Outcome |
|---|---|
| `cargo clippy --release -p cpg-extract -p cpg-core -p cpg-schema --all-targets -- -D warnings` | `passed` after `ed7662c` changes. |
| `cargo fmt --all -- --check`; `git diff --check`; `just adr lint` | `passed`; ADR lint reports 30 records. |
| Focused receipts for `719e9c0` and `9c23705` | `passed`; their exact selections and limits are in the corresponding standard/compact reviews and commit messages. |
| `just check`; current-HEAD `just test-all`; fresh-store `just pilot`; Q01/Q03/Q05/Q09; clean wheel/native query; `just adr revisit` | `not_run`: the operator requires only targeted testing until the full Stage 3 functional scope exists. The last full gates at `d61a497` do not certify current HEAD. |

Focused release selection (`passed`: 5/5, 173 skipped; reviewed schema/rule/codebook snapshots; missing MRO and forged candidate status rejected):

```bash
INSTA_UPDATE=no cargo nextest run --release -p cpg-schema -p cpg-core -E 'test(contracts_snapshot) | test(rules_snapshot) | test(registry_snapshot) | test(an_attempt_publishes_every_table_and_readers_see_only_published_rows) | test(modeled_exception_handler_candidates_follow_exact_try_body_ancestry)' --no-tests=pass --no-fail-fast
```

## Known failures, decisions and blocks

- No current focused test failure is known. The integrated Stage 3 behavior, pilot counts, release-scale cost and served compatibility remain unverified.
- ADR-0020, ADR-0024, ADR-0025 and ADR-0028 remain proposed. ADR-0029 and ADR-0030 are accepted for their narrow implemented boundaries.
- The latest standard review defers cross-module ancestor capture (F01) until a real model needs it and handler clause selection/completion (F02) before any served catch. Candidate MRO nonmembership cannot refute a catch while a modeled class may denote possible subclasses.
- `value_flows` merges source uses and drops the raw `flow_values.fact_id` needed to join `flow_value_call_links`. L3 must preserve per-source path provenance or derive summaries directly from raw facts; joining by sink span or `through_call` would be unsound.
- Remaining functional scope: broaden pinned model families and CrossHair checks; finish L2 handler, callback and resource fates; compose bounded SCC summaries that discharge `call_transfer`; run independent Pysa and CPython/Hypothesis oracles; implement FORMAT 7 native/PyO3 serving with effect/role/compatibility filters; only then run the integrated gates, structured evaluation, clean wheel/native query and increment-end review.

## Next

Derive L2 handler selection and normal/exceptional completion with an explicit modeled-raise assumption, clause order, nested frame propagation and unknown outcomes. Preserve the pinned MRO fact as a positive class witness; carry unresolved class relationships and walk limits into summary boundaries. Keep full `just test-all`, fresh `just pilot` and structured evaluation deferred until all Stage 3 functional scope is assembled.
