# Status

_Updated 2026-10-01 under the [handoff skill](.claude/skills/handoff/SKILL.md); shared main._

## Agent workflows (2026-09-30)
- **Implemented (ADR-0109):** [shared roles](.agents/roles/README.md) and skills use selective delegation, task-specific context, explicit escalation and the agreed stronger review defaults.

## Phase 4 execution (2026-10-01)
- The [detailed plan](docs/plans/semantic-model-phase4-detailed-plan_2026-09-30.md) owns packages and receipts.
  ADR-0105/0106 are **Accepted target**. Full D0–Q0 execution is authorized, including conditional
  operator review of explicitly unreviewed grounded briefs.
- The [independent assembled review](docs/design_review/reviews/design_review_phase4-plan_2026-09-30.md)
  is **Accept scoped** at Proposed design level. R0/R1 are integrated and the independent
  [foundation review](docs/design_review/reviews/design_review_phase4-foundation_2026-09-30.md)
  is **Accept scoped** at frozen `0a60954`; plan §13.2 owns dated focused receipts and scope.
- **Implemented / focused-Tested:** immutable vocabulary prefixes, sixteen nominal analysis owners,
  source-bound coverage/publication and discharge; N0 configuration and N1 receiver/open dispatch. Nonempty unimplemented analysis selectors still refuse activation.
- **passed, 2026-10-01:** wrapped Cargo `test --release -p lctx-model -p cpg-core --test analysis_preparation`
  (two model controls and one real PG test across both profiles), composite after fixture/boundary fixes.
  Integrated N1/G0 PG controls also passed both profiles after correcting the test's frontier scope;
  graph reuse, permit/budget refusal and lifetime controls passed. E1 spec/codec/cache/realization
  foundation controls passed, including real PG configuration; no live embedding claim.
  Actual Local publication and Use/Value/Guard entry domains passed 4 PG, 13 native and 22 model
  controls; behavioral stays Partial, catalog NotRequested. B1 canonical/base/body proofs passed 6 native
  and 12 model controls; Local finite theory/fields and actual BaseEvaluation now passed 70 combined
  model/native/PG controls; source-body publication passed 17 more; source-call headers passed 7 native/PG controls. Settings passed
  4 pure + 1 PG control across both profiles; production activation remains open.
- **passed, 2026-10-01:** R0 declared validation-view controls in memory and real PostgreSQL:
  earlier/current vocabulary routing, no widening beyond acknowledged grants, malformed-frame
  refusal and reservation release. B0 exposed and motivated the fix; its Summary publication
  still needs B3's producer capability contract. Plan §13.2 owns the composite receipt.
- **passed, 2026-10-01:** E1 original analytic text (five pure controls) and both native PostgreSQL
  profiles, including complete seal/model validation. E1 consumption then passed five pure and
  three actual PG controls across four availability cases; external effects used a contract fixture,
  so live-service behavior is not_run. A0 usage and A1 neighbour/rank/concept kernels passed; nominal
  publication remains open. E0 rendering/consumed-value replay passed 12 pure + 2 PG controls; final
  S0/E0 publication remains open. Delegation and projection-v3 source definitions passed 5 native + 9 model controls; A0 structural publication passed both
  PG profiles + 2 pure and 12 regression controls. Sealed source conclusions passed 12 model controls;
  Pass B/C now pass both PG profiles plus 3 model controls; documentary preparation passed actual PG seal/replay. Final S0/E0 remains open.
- **passed, 2026-10-01:** integrated C0 catalog (20 pure controls; native catalog plus both
  normalized PG profiles). C1/E1 per-grain coverage contracts passed two pure controls;
  C1 source/document/deployment evidence passed 10 pure, 2 integrated PG and 1 native control;
  C1/C2 declaration selection passed 29 pure + 1 native/PG control; exact domains preserve four
  outcomes and unresolved evidence. Runtime field/state links remain pending Local/B1.
- **passed, 2026-10-01:** nominal report reads passed actual PG profiles; `generation show` CLI compiled.
  Source invocation/frame now passed 6 actual PG + 1 native controls; enriched execution, full analytics/synthesis producers and upper-frontier assembly remain open.
- New P4 acceptance is **not_run** pending implementation. Compile checks and focused controls
  precede scheduled review; integrated acceptance waits for all functional packages and retirement.
## Phase 3 implementation and qualification
- **Implemented / focused-Tested:** R1–R3 and N1–N7/X in the
  [detailed execution plan](docs/plans/semantic-model-phase3-detailed-plan_2026-09-30.md).
  Its §11 owns package receipts and finding disposition; the
  [parent cutover plan](docs/plans/semantic-model-cutover-plan_2026-09-29.md) owns cross-phase obligations.
- Accepted decisions: cumulative-generation contracts now retained by [ADR-0105](docs/adr/0105-analysis-vocabulary-epochs.md) and
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
- Agent/settings and hook changes from `57a75d8` remain preserved and separately qualified.

## Next
Continue ordinary work directly on `main`. Existing after-turn hooks own formatting/catalog refresh; do not run them manually. Additional pilot-scale qualification runs only if
reactivated; detailed plan §11 retains its unmeasured scope.
Continue P4 with B1–B3, exact C1/C2 runtime fields and analytics/synthesis; downstream assembly,
retirement and Q0 remain open. Follow scheduled review boundaries.
P5 owns serving/product qualification; PR6 remains paused.
