# Status

_Updated 2026-09-29 under the [handoff skill](.claude/skills/handoff/SKILL.md); shared `main`._

## Current checkpoint: semantic model cutover accepted (docs only)

- **Decided 2026-09-29.** The operator accepted three records:

  | Record | Decision |
  |---|---|
  | [ADR-0082](docs/adr/0082-relation-centric-semantic-model.md) | One declared relation model with a single owner per semantic question |
  | [ADR-0083](docs/adr/0083-postgresql-relational-store.md) | PostgreSQL replaces Delta as the single relational store; DataFusion stays the compute engine |
  | [ADR-0084](docs/adr/0084-layered-hard-cutover.md) | A hard layered cutover with legacy adapters and exact parity |

  The target is [DESIGN §15](docs/design/sections/semantic-model.md). ADR-0082 supersedes the proposed ADR-0024 and ADR-0083 supersedes ADR-0067; both retired records were deleted, with their surviving clauses carried.
- **Execution owner:** the [cutover plan](docs/plans/semantic-model-cutover-plan_2026-09-29.md). Its phases are 0 core, 1 store, 2 facts, 3 normalized relations, 4 analysis and catalog, 5 serving. A phase exits only with no legacy code in its layer. Stage-3 research engines are triaged by ablation at phase-4 entry, with operator confirmation.
- **Product work is paused** (PR6 and new features) until phase 5. The [forward plan](docs/plans/behavioral-model-forward-plan_2026-09-24.md) keeps product context. PR0 comparison remains independently blocked.
- **Review:** [semantic data model review](docs/design_review/reviews/design_review_semantic-data-model_2026-09-29.md) (Revise), with [probes P0/P3/P8](docs/design_review/evidence/2026-09-29_semantic-data-model/README.md). Cutover plan §8 owns F01–F13. By operator decision, they are closed by construction in the new contracts, not repaired in legacy code.
- **Nothing is implemented yet.** Delta, `cpg-schema` and the bundle import remain the implemented pipeline. DESIGN §3–§14 describe that legacy pipeline until each layer cuts over.

## Implemented baseline (unchanged code at `fedd4a0`)

- **PR5 completed at bounded Tested strength** (ADR-0081): bounded invocation packets and sections, scoped browse, independent evidence search, ordered comparison, and coarse catalog/retrieval rebuilds. [PR5 review](docs/design_review/reviews/design_review_pr5-agent-journeys-rebuild_2026-09-29.md).
- **Formats:** compiler112, extractor37, template21, catalog7, bundle16/projection6/wire4, migration012.
- **Runtime:** two ready NVFP4 profiles, with behavioral selected. The runtime (ADR-0080) and the [PostgreSQL runbook](docs/postgresql.md) are current. All 58 protected benchmark files are preserved.
- **Build (ADR-0079):** dated nightly `2026-09-29`, shared Cargo intermediates. [Build evidence](docs/design_review/evidence/2026-09-28_cargo-cache/README.md).

## Verification boundary

| Command, 2026-09-29 | Outcome and scope |
|---|---|
| `just adr index`; `just adr lint` | **passed:** 43 records; every design reference resolves and governed sections name their records |
| `just docs-check` | **passed:** 177 canonical pages, zero errors. The external graph review is tracked and excluded from publication |
| `just lint-agents` | **passed** |
| P0 `p0_queries.sh`; P3 probe binaries; P8 `p8_volume.py build/store` | **passed** (review evidence); the probes changed no product code |
| `just fmt`, `just test-all`, `just pilot`, MCP journeys | **not_run:** docs-only change. The product code is unchanged since the PR5 gate |
| PR5 composite gate (485 Rust, 204 Python, 19+3 real-PG) and qualified journeys | **passed** in the other session's run, recorded in [PR5 evidence](docs/design_review/evidence/2026-09-28_pr5/README.md); it is not a fresh run here |

## Next

Cutover **phase 0** ([plan §4](docs/plans/semantic-model-cutover-plan_2026-09-29.md#phase-0--core-contracts-store-kernel-and-migration-tooling)):
- scaffold `crates/lctx-model`;
- build declarations, identity, vocabulary and policies, and port the condition kernel;
- build the store kernel in `lctx-postgres` and the stage table;
- build the adapter, legacy-ID and parity frameworks;
- build the known-answer library.

Verification during the phase is compile checks and focused tests. Phase 0 exits with a fresh design/target review of the core contracts (WP0.10) before phase 1 moves the store.
