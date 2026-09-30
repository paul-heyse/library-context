# Status

_Updated 2026-09-30 under the [handoff skill](.claude/skills/handoff/SKILL.md); shared main._

## Planning skills (2026-09-30)

- **Implemented:** [planning skills](.claude/skills/README.md) cover review planning, plan
  creation and planning the authoring approach. Plan creation includes focused dependency
  review and a flexible document outline.
- Checks **not_run**: `just lint-agents` and `just docs-check` belong to the automatic hook;
  `just test-all` is outside this process-only change. No new hook was added.

## Phase 4 detailed design (2026-09-30)

- The [detailed design and execution plan](docs/plans/semantic-model-phase4-detailed-plan_2026-09-30.md)
  documents contracts, library choices, ordered packages, migration and independent controls.
  ADR-0105/0106 and the new architecture text are **Proposed**; P4 production work has not begun.
- The design preserves once-built stored graphs and resolves shared-vocabulary publication,
  bounded recursive witnesses/residuals, catalog ownership and the Phase 5 serving boundary.
- The [independent assembled review](docs/design_review/reviews/design_review_phase4-plan_2026-09-30.md)
  is **Accept scoped** at Proposed design level; plan §13.2 owns the receipt. Parent §8 routes
  graph-token F01: design addressed, G0 implementation controls pending.
- Product tests/pilots are **not_run** for this documentation task. ADR/docs hygiene belongs to
  the automatic end-of-turn hook; no result is claimed before it runs.
- Concurrent hook/tooling/instruction changes remain outside this design's edits and qualification.

## Main consolidation (2026-09-30)

- `main` is the sole local development branch and tracks `origin/main`. Repository-local
  `pull.ff=only` and `push.default=simple` keep ordinary development and publication on main.
- Merge `809fe5b` joins local `57a75d8` with remote `716148a`. The remote's five differences
  from the alternate P0–P2 commit were already incorporated or superseded locally.
  Tree equality with `57a75d8` and both-parent ancestry checks passed; no source files changed.
- Published and verified annotated archives: `archive/pyrefly-inproc-2026-09-30` at `5fb2eed`
  and `archive/stage3-bdd-2026-09-30` at `69bc1b1`. The BDD patch also survives in main at `2da15a6`.
- The former spike worktrees remain at their original commits in detached state, with their
  directories and files preserved. The two local spike branch names were retired after remote
  archive verification. No history was rewritten and no force-push was used.
- Product checks for this Git-only reconciliation are **not_run**: source-tree equality is the
  acceptance check; the existing Phase 3 receipts below retain their original scope.

## Phase 3 implementation and qualification

- **Implemented / focused-Tested:** R1–R3 and N1–N7/X in the
  [detailed execution plan](docs/plans/semantic-model-phase3-detailed-plan_2026-09-30.md).
  Its §11 owns package receipts and finding disposition; the
  [parent cutover plan](docs/plans/semantic-model-cutover-plan_2026-09-29.md) owns cross-phase obligations.
- Accepted decisions: [ADR-0101](docs/adr/0101-cumulative-normalized-generations.md) and
  [ADR-0103](docs/adr/0103-normalized-graph-materialization.md), which supersedes ADR-0102.
  Every new normalized collection builds native petgraph graphs once after facts collection and
  stores versioned Postcard snapshots in generation-owned BYTEA chunks. Canonical rows remain
  semantic authority. Hydration restores the graph; runtime indices remain private.
- Runtime: atomic completed stages, private facts checkpoint, source-bound reads, shared invariant
  eligibility, physical-scan admission, retained resource ownership and terminal drain/close.
  The provider fork is pinned to `a41da22`; no old-ID bridge or dual store was introduced.
- Normalization: entities/correspondence/ownership, total relationship outcomes, effective callable
  components and raw signature slots, complete call events and generated policy views, stored
  binding attempts and private full-premise admission, four typed program projections, and
  total capability/scope coverage with exact lower-evidence and frozen-output receipts.
- `lctx compile <library> --through facts|normalized --profile catalog|behavioral` publishes one
  self-contained generation without selecting it. Analysis/catalog/serving remain unavailable.
  Catalog flow stays NotRequested; uncertainty and missing evidence remain explicit.
- The [foundation review](docs/design_review/reviews/design_review_phase3-foundation_2026-09-30.md),
  [normalization review](docs/design_review/reviews/design_review_phase3-normalization_2026-09-30.md)
  and [assembled exit review](docs/design_review/reviews/design_review_phase3-exit_2026-09-30.md)
  are **Accept scoped** after corrections. This does not establish completion of Q.
- **Qualification boundary:** additional full-library output comparisons and measurements were
  stopped at user direction. Pilot-scale reservations/RSS, graph hydration timing and measured
  budget refusals remain unqualified. Stage joins retain complete charged typed
  collections; no streaming-join, absolute RSS cap or speedup claim is made.

## Current checks (2026-09-30)

Cargo commands use `python3 scripts/build_environment.py --`. Functional work is committed as `820f548`. Exact commands and raw logs are in
[Phase 3 qualification evidence](docs/design_review/evidence/2026-09-30_phase3-qualification/README.md).

| Command / scope | Outcome |
|---|---|
| Wrapped focused model/native/core tests, including both PG profiles | passed; detailed plan §11 owns package receipts |
| `just fmt`; `just fmt-check lint` | passed after scoped lint corrections |
| `just test-all` / component reruns | composite gate passed: initial workspace run 702 passed/34 failed/12 skipped; all failures repaired and covered by 129 passing contract controls plus the PG repeat suite; reviewed model snapshot accepted |
| `just py-check` | passed: 219 Python tests, 56 explicit skips, Pyrefly |
| `just rules-scan rules-test lint-agents fixtures-check deps gold test-doc` | passed; cargo-shear retains five inventoried dormant P4 test warnings |
| `just test-postgres` | passed: 257 tests, two tests and one binary skipped; current release CLI built |
| Wrapped release `installation reset_reinstalls_new_model` | passed: reset retires installations missing newly added control tables |
| `just adr lint`; `just docs-check` | passed: 56 records; 235 canonical pages, zero link errors |
| `uv run python .../migrate.py` | passed: current store rebuilt, five retained-service fingerprints unchanged, importer eight provider plus two lifecycle/writer slots |
| Actual FastMCP facts/catalog command | passed: 528.641s; unselected published generation `7456ea1e2518e198597c8b67e0a18561` |
| Additional full-library comparisons / measurements / refusals | not_run at user direction; started normalized pilot stopped and aborted; store check passed with one generation, zero findings |
| Product/served evaluation, held-out confirmation and performance comparison | not_run: phases 4–5 remain unavailable |

Gate results are composite after initial lint, snapshot, fixture-contract and SQL-rule failures.
Fixtures now select their declared facts/analysis scope; production validation was not relaxed.
The reviewed snapshot adds 72 relations, six invariants and native function-origin classification.
Reset initially refused absent old-installation control tables; its regression and rerun passed.

## Prior phases and preservation

- Phases 0–2 are **Implemented / Tested**, including remaining PostgreSQL qualification and
  all native fact producers. Their historical receipt is in parent plan §4.2 and
  [facts qualification](docs/design_review/evidence/2026-09-30_facts-qualification/README.md).
- `lctx-model::domain` owns semantic contracts; PostgreSQL is the sole relational store.
  DataFusion supplies in-process compute and source-bound transport. Python remains a thin adapter.
- Old facts generations were removed during the current model rebuild. Retained services and
  protected service backup remain preserved; no rollback generation or compatibility reader exists.
- The legacy `cpg-schema` crate and dormant consumers remain only for named P4/P5 obligations
  in detailed plan §9.1. Meaningful P3 source/stub, Unicode/re-export, synthetic and conditional
  declaration expectations have moved to native normalized tests.
- The formerly concurrent agent/settings, instructions and automatic after-turn hooks were
  committed in `57a75d8` and preserved by consolidation. Unrelated generic skill-validator
  metadata failures were not changed or tested.

## Next

Continue ordinary work directly on `main`. Existing after-turn hooks own formatting and catalog
refresh; do not run or validate them manually. Additional pilot-scale qualification runs only if
reactivated; detailed plan §11 retains its unmeasured scope.
When P4 implementation is requested, begin detailed plan D0 (decision adoption), then R0/R1
(immutable vocabulary publication and derived-result foundation). Follow its dependency graph
and scheduled review boundaries. P5 owns serving/product qualification; PR6 remains paused.
