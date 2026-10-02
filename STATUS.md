# Status

_Updated 2026-10-01 under the [handoff skill](.claude/skills/handoff/SKILL.md); shared main._

**Plan documents created, 2026-10-01:** The [Phase 5 detailed plan](docs/plans/semantic-model-phase5-detailed-plan_2026-10-01.md)
and [foundation enhancements plan](docs/plans/semantic-model-foundation-enhancements_2026-10-01.md)
are **Proposed**. The [focused target review](docs/design_review/reviews/design_review_phase5-target_2026-10-01.md)
is **Accept scoped** at its documented baseline, with no new blocking finding; this accepts the
target direction, not implemented serving. [ADR-0114](docs/adr/0114-generation-serving-contracts.md)
remains proposed. Shared planning/review guidance previously landed in `2b581ab5`.

## Document checks — 2026-10-01

| Command / scope | Outcome |
|---|---|
| `uv run --no-project --offline --no-python-downloads python scripts/docs.py build` | **passed**, canonical publication and internal links |
| `just lint-agents` | **passed** |
| `just docs-check` | **failed**, generated ADR index is stale; end-of-turn hook owns its refresh |
| Product checks, pilots and measurements for this document scope | **not_run**, no production or publisher/resolver changes |

## Phase 4 — completed within the accepted scope

The operator-authorized remaining [Phase 4 scope](docs/plans/semantic-model-phase4-detailed-plan_2026-09-30.md)
is **Implemented / Tested, 2026-10-01** on shared main. All functional packages, mapped X0
retirement and Q0 acceptance are complete within the recorded finite envelope. The
[qualification receipt](docs/design_review/evidence/2026-10-01_phase4-qualification/README.md)
retains initial failures, corrections, focused reruns and the passing complete gate.

Typed Analysis/Catalog publication works in both profiles. Exact supported standard-record source
association is shared by Local, uncertain Summary, C1 and S0; it establishes no allocation,
alias, mutation or temporal heap identity. Exact MDX/control templates and proof-backed finite
negative synthesis retain original evidence. ADR-0111 preserves actual checked-false candidates
and complete false-only native binding inventories; this is no whole-callable absence theorem.
Upper CLI controls cover seedless/nonzero briefs and fake embedding cold/warm/cleared-cache replay.

The [assembled Design/Target review](docs/design_review/reviews/design_review_phase4-assembled_2026-10-01.md)
and [F01 corrective reinspection](docs/design_review/reviews/design_review_phase4-symbolic-reinspection_2026-10-01.md)
remain **Accept scoped** at their recorded baselines. Their historical gate boundaries are unchanged.
The [parent finding register](docs/plans/semantic-model-cutover-plan_2026-09-29.md#8-findings-disposition)
owns current closure and deferrals.

## Historical Phase 4 verification — 2026-10-01

| Command / scope | Outcome |
|---|---|
| `NEXTEST_TEST_THREADS=8 just test-all`, full5 | **passed**: complete release Rust suite, Python/oracles, separate real PostgreSQL gate and compile-fail/positive doctests |
| `just hygiene` and named corrected/affected reruns | **composite passed**; initial ADR failure and all corrected receipts retained |
| `just store-check`, current empty main installation | **passed**, zero generations and zero findings |
| Actual native/Summary/C1/documentary/CLI/admission controls | **composite passed** for named scopes; mocks qualify only explicitly fake service seams |
| Live embedding, upper real-library pilots, comparisons and measurements | **not_run**, excluded scope |

Cargo commands retain the pinned toolchain, release profile, stable shared intermediates and
16 build jobs. Nextest8 limits simultaneous test processes; it does not request eight threads
per test process. Earlier compiler SIGTERM remains an unconfirmed build interruption, not a test failure.

## Next scope and standing limits

Next: D0 settles ADR-0114 and integrates the shared serving contracts, followed by ready M0/F1/N0
work under the detailed plan. F1 enables prepared classification; F2 coverage dispatch and F3
consumed-input streaming can proceed independently. Every production package remains **not_run**.
Phase 5 serving/MCP and PR6 remain **Proposed / unavailable**. This document work changes no runtime.
R4's positive simultaneous two-variant Summary qualification remains **not_run / open**;
current admission requires complete uniquely Bound variants. Revisit R4 before broadening admission.
Live embedding, product quality/comparisons, performance/hydration and total-RSS are unmeasured.

Phases 0–2 remain qualified within the [facts receipt](docs/design_review/evidence/2026-09-30_facts-qualification/README.md).
Phase 3's historical scoped boundary remains in [its plan §11](docs/plans/semantic-model-phase3-detailed-plan_2026-09-30.md#11-current-package-status-and-finding-routes)
and [receipt](docs/design_review/evidence/2026-09-30_phase3-qualification/README.md).
`lctx-model::domain` owns semantics; PostgreSQL owns canonical storage; DataFusion supplies compute.
Compilation never selects. Catalog Flow is NotRequested; behavioral uncertainty stays explicit.
X0 removed92 mapped obsolete source/snapshot paths and preserved named P5 bundle/wire/native/
Python/cursor/hydration and service obligations. Retained P5 services compile legacy contracts
transitively; no active P4 authority, old-ID bridge, compatibility reader or dual store uses them.

The current dirty source (including apparent hook formatting) was not requalified by this document
work; the Phase 4 receipt above retains its original tree boundary. Preserve unrelated dirty
agent/instruction/catalog/test work, dirty or unproven-integration worktrees,
caches and protected assets. Eleven clean integrated task worktrees were removed under the
current cleanup instruction. The automatic end-of-turn hook owns formatting and generators;
its future results are not claimed by this receipt.
