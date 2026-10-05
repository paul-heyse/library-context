# Status

_Updated 2026-10-05 under the [handoff skill](.claude/skills/handoff/SKILL.md); shared main._

**Efficient-architecture companion adopted (Implemented guidance, 2026-10-05).**
The [Heuristics for Efficient Architecture](docs/design_review/design_principles/core/efficient-architecture-heuristics.md)
(version 1.0, H1–H28) now accompany the principles through agent, planning, review and execution
routes. Existing principles and overlapping lenses remain; material choices use qualitative
assessment before physical mechanisms become entrenched. No new gate or proof machinery.
Documentation builds, lint, formatting and product tests: **not_run**, per user instruction.

**Graph-native plan set authored; replacement remains Proposed / not_started.**
The [replacement coordinator](docs/plans/graph-native-pivot-plan_2026-10-05.md) owns the combined
hard-pivot target, dependencies, current GN01–GN03/capability F01–F03 disposition and completion.
Its four supporting plans cover model/compiler, SurrealDB realization, native serving and projections.
The source reviews are [graph-native target](docs/design_review/reviews/design_review_graph-native-target_2026-10-05.md)
and [SurrealDB capabilities](docs/design_review/reviews/design_review_surrealdb-capabilities_2026-10-05.md).
Other reviews were not additional design grounds. Focused independent authoring advice's analysis
readiness and retained ranking corrections are incorporated; this is plan assessment, not runtime acceptance.

Native SurrealDB querying is selected in the proposed replacement. FP-07/A4 guide efficient design
using first principles and library understanding. Query plans/metrics are optional diagnostics;
no detailed work-accounting, performance proof or query-adoption gate is introduced. The plans
schedule direct replacement/deletion, fresh artifacts and targeted functional controls, with no
compatibility readers, dual stores, intermediate PostgreSQL redesign or historical runtime archive.

**Production/store behavior remains the PostgreSQL baseline.** This session changes plans,
disposition navigation and handoff only. No compiler, database, embedder or client registration changed.
G0 schedules architectural decision recording; accepted ADR bodies and executable contracts are unchanged.

## Current limits and existing evidence

**First real-library upper compile remains failed.** The earlier authorized FastMCP 4.0.5
behavioral/all-analytics/fake-embedder run extracted in about 28 minutes, then spent about an hour
in store reference checks before an idle-in-transaction disconnect; cleanup was unconfirmed.
Interrupted generation `786cd6d58dc5dccea686c0a54a8f0dcd` (facts only, about 16.6M rows, staging)
was last observed on 2026-10-05. It was not inspected or removed in this authoring session;
it is disposable under the planned pivot. No generation is published or selected in that receipt.
The two source reviews describe the failure and its replacement route; older evidence is not a
new measurement or acceptance result.

**Earlier assurance pivot: Implemented / Tested within its composite fixture scope, 2026-10-05.**
The [assurance coordinator](docs/plans/testing-architecture-pivot-plan_2026-10-04.md#7-current-contractcontrol-map-and-execution-checkpoint)
owns the contract/control map, closed findings and exact commands. The initial
`NEXTEST_TEST_THREADS=8 just qualify` failed; affected repairs/reruns produced a composite pass,
not an initially clean gate. Seven focused families, actual disposable PG/native/MCP journeys,
independent raw-flow/served oracles and applicable leaves passed within that receipt. They do not
qualify the graph replacement or a real-library catalog. NoScope and nonempty partial/refusal
semantics remain in the baseline; duplicate replay factories and obsolete oracle claims were retired.

The [target-alignment coordinator](docs/plans/target-implementation-alignment-plan_2026-10-04.md#8-current-q0-composite-qualification-receipt)
owns its closed fixture Q0. The [enrichment coordinator](docs/plans/code-facts-analytical-enrichment-plan_2026-10-04.md#7-current-disposition-and-checkpoint)
retains O08 and open Q1 packet-benefit evidence. The
[Phase 5 plan](docs/plans/semantic-model-phase5-detailed-plan_2026-10-01.md#10-finding-routes-limits-and-current-state)
owns stopped real-library activation/serving limits until G0 carries the applicable obligations forward.
Correctness controls establish neither packet benefit nor comparison/superiority.

Execution-fit core 3.3 / CI 1.4 adoption is Implemented under ADR-0127. Its prior docs/agent/ADR
checks passed, 2026-10-05. This session applies that standard without reclassifying historical reviews.
Gold, heldout, live vectors, registrations and operator store state remain untouched. The prior
worktree/cache/build-policy checkpoint remains: older target-alignment worktrees have current
consumers; Cargo jobs 16, frontend 1, release profile and Nextest 8 are unchanged. No cleanup ran.

## Verification for this authoring scope — 2026-10-05

- `just docs-check`: **passed**, 315 canonical pages and zero link errors. ADR/agent checks
  passed within the same command. This verifies publication on the current mixed documentation tree.
- Root runs `just turn-end` as the final generation/formatting step after this handoff;
  its outcome is reported in the session/commit rather than predicted here.
- `just qualify`, product families, real-library compilation/activation and performance probes:
  **not_run**; plan-authoring scope. Static library/source research is not a runtime test.

## Next

Start at [coordinator §4](docs/plans/graph-native-pivot-plan_2026-10-05.md#4-coherent-packages-and-dependency-order):
G0 records the direct replacement, then M1 and S1 settle graph/native-operation contracts together.
C1 store-free facts and P1 native realization consume those slices; C2/A2 integration follows
actual Local/Model/vector readiness. No intermediate PostgreSQL repair or old-green gate is required.
Q0 owns assembled functional acceptance; Q1 schedules separately authorized fresh library/live-vector
reconstruction and operator adoption. Historical artifact retention is not a prerequisite.
