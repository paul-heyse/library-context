# Status

_Updated 2026-09-25 under the [handoff skill](.claude/skills/handoff/SKILL.md); work is on `main`._

## Increment and slice

- **Product:** Stage 3 of increment 4 is in progress and **functionally incomplete**; production
  checkpoint `acbcee5` (since then only one crate doc comment changed). The
  [forward plan](docs/plans/behavioral-model-forward-plan_2026-09-24.md) is the sole execution
  plan: §1 current state and qualification boundary, §3 queue, §6 findings W1–W16 (the single
  disposition owner for ARC-01–03, RF/F01–F16 and RFU/F01–F08).
- **Documentation migration: complete** (`94f4eb3`..this commit; ADR-0042). DESIGN keeps scope,
  §B1–§B14, the dependency rule and deferrals; eight [focused owners](docs/design/README.md) hold
  current contracts with known defects linked to plan items; 19 ADRs remain, with ADR-0043–0047
  consolidating the rationale in force. Earlier plans, 105 closed reviews, the research input,
  retired standard files, unused evidence and 28 superseded ADRs were removed in `df1ebd8`; the
  migration plan and its two reviews (lifecycle target review; assembled review, Accept with
  changes, F01–F05 applied in `3825357`) are removed in this commit. Recovery:
  [historical recovery](docs/README.md#historical-recovery).

## Last verified (2026-09-25)

| Command | Outcome |
|---|---|
| `just docs-test` | `passed`: 38 ADR, resolver and publisher tests (retired predecessors, proposed replacements, history-aware numbering, shallow clones, scope options). |
| `just adr lint`; `just lint-agents`; `just docs-check` | `passed` on the final tree: 19 records, curated site with zero link errors. |
| Focused Ruff/pyrefly on `scripts/{adr,docs}.py` and their tests; `git diff --check` | `passed`. |
| Browser at `/` and `/library-context/` | `passed` in `dd796e0`: Current default, Current/Reference/Everything only, relocated DESIGN fragment, retained finding anchor, prefixed results. Remote CI `not_run`. |
| `just test-all` (2026-09-25, before the migration) | `failed` then corrected: 313/313 release Rust, 110/112 Python; stale tool-list expectations fixed (`test_server.py` 12/12 `passed`). The rerun was stopped at operator direction; a complete pass is **not observed**. |
| Fresh `just pilot`; Q01/Q03/Q05/Q09; structured evaluation; clean-wheel query; increment-end review | `not_run`; not in documentation scope, and Stage 3 exit is incomplete. |

## Known failures, blocks and decisions

- Open product defects and their closure checks are forward-plan §6 items; no cleanup closed any
  (W15 closed only its documentation mismatch). Most consequential: native proof-kind admission
  (W1), unsupported no-read claims for qualified/aliased builtin access (W4), lost facet verdicts
  and claim citations (W2, W3), coupled summary policy and discarded refusal causes (W5),
  `Model::reach` not a fixed point (W7), BDD decision/support/aggregate budgets (W6).
- ADR-0020, ADR-0024, ADR-0025 and ADR-0028 remain proposed. Accepting ADR-0043–0047 restated
  decisions already in force and certifies no implementation.
- About 290 code comments still cite retired ADR ids as history; update each at its file's next edit.
- Moved-aside stores under `build/store-pre-*` and `build/store-stage3-*` await the operator.

## Next

Product: W1 (ARC-01). Derive native proof-kind admission from the schema contract and verify a
real finalizer-bearing generation through the native executor; then W4 and W5 before any new SCC
or limit policy (forward plan §3.1).
