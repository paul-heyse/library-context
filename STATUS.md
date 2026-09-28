# Status

_Updated 2026-09-28 under the [handoff skill](.claude/skills/handoff/SKILL.md); work is on `main`._

## Increment and checkpoint

- **Increment 4 / Stage 3 remains functionally incomplete.** [Forward plan §3.0](docs/plans/behavioral-model-forward-plan_2026-09-24.md#30-consolidated-execution) owns P0–P7 and the frozen exit rule; §6 owns finding disposition.
- **P0 done; P1 implemented, change review to settle; P2 partly done.** ADR-0064, decorated-summary refusal, claim-keyed discharge and native citation admission remain the semantic slice. Discharge review, W5/W7/W12 traces and P3 Logger models are unchanged.
- **PostgreSQL PG0–PG17 completed for local exact serving (`a0ea858`):** generation-pinned Rust/async MCP queries, conditional mixed ANN with physical-index admission, COPY publication, admitted DataFusion federation, operational diagnostics, complete artifact-bearing recovery and operator cutover. [Detailed plan](docs/plans/postgresql-integration-plan_2026-09-27.md), [runbook](docs/postgresql.md), [current evidence](docs/design_review/evidence/2026-09-28_postgresql-operations/README.md).
- **Architecture:** ADR-0070 carries publication/query contracts and binds physical ANN admission and recovery; ADR-0068 owns the expanded stack, ADR-0067 canonical Delta authority. [Assembled review](docs/design_review/reviews/design_review_postgresql-operations_2026-09-28.md): **Accept scoped** for local exact serving.
- **Versions:** compiler **107**, extractor **33**, template **20**, catalog **7**, FORMAT **12**; projection **2**, standard embedding width **1024**. Owned provider pin remains `paul-heyse/datafusion-table-providers@e6fc4c40`.
- **Operator deployment:** PG18.6/pgvector 0.8.6, migrations **001–008**. Live generation `32b4cb5d…` is selected with exact profile `ff645e4a…`; prior ready generation retained. Serving budget is four SQLx plus two provider connections per process.
- **Preservation:** protected populated operator backup is `build/postgresql-pg17-operator.dump` plus receipt/artifacts. Matching schema004/schema007 rollback assets remain in `build/postgresql-pg16-baseline-9471b93/`; PG7 assets remain in `build/postgresql-pg8-baseline-6403b60/`. Retain all ready generations/artifacts.
- **Development workflow:** root editable uv environment, native extensions rebuilt in place and existing cached Cargo target. No wheels built; distribution packaging remains conditional.

## Last verified (2026-09-28)

Full gates followed functional completion. Later numerical-oracle and membership changes have focused receipts; unrelated gates were not repeated.

| Command | Outcome and scope |
|---|---|
| `just fmt`; `just test-all` | **passed** on repair: 439 ordinary Rust, 171 Python, 18 real PostgreSQL and two async controls; strict checks, rules, agents/ADRs, 84 fixtures, dependency/gold policy and fresh SQLx metadata |
| Targeted comparator, foreign-ID and real-pilot physical-admission selectors; affected Clippy/Pyrefly | **passed:** numeric crossings/order/ties, malformed membership, stale pin/selection and already-pinned ANN refusal after reindex |
| Fresh `just pilot …`; live `lctx compile … --embedder vllm`; `just embed-conformance` | **passed:** fake/live compile and 20-brief MCP smoke; Rust/Python live 1024 contract |
| Cold/warm vector comparison; PG/embedder-offline `lctx bundle` | **passed:** exact vector bytes and all 52 rebuilt files |
| Live `lctx serving qualify-hnsw …` | **failed ANN admission:** recall preserved, natural plans/latency benefit failed. Exact remains selected; confirmation **not_run: no admissible candidate** |
| Operator migrate/check/import/reconcile/select/report; `postgres_workload.py` | **passed:** deployed exact MCP and two-generation rollover; search p95 87/99 ms at concurrency1/4 |
| `postgres_backup.py backup/restore-drill` | **passed:** 69 logical tables, both ready generations, captured exact selection and least-privilege native/MCP serving in **15.31 seconds** |
| `just docs-check`; final affected format/lint checks | **passed:** 155 canonical pages, offline links, ADR/agent checks and affected format/Ruff checks; supplied untitled external input preserved and excluded by the existing publisher mechanism |

## Known boundaries and next

- No ANN profile is qualified or selected. Measured exact rank p95 21.77/15.30 ms beats ANN 53.26/44.96 ms. A larger eligible corpus or improved natural plans can trigger new disjoint calibration/confirmation; no automatic activation.
- [Forward-plan §6.1](docs/plans/behavioral-model-forward-plan_2026-09-24.md#postgresql-findings) owns PGS/PGK/PGE/PGF/PGQ/PGO dispositions. Conditional ORM/ADBC/protocol/HA/pruning work remains under PostgreSQL plan §7; W9 stays closed and arbitrary endpoint attestation remains W16.
- Resume Stage 3 P1 review disposition, P2.3 default-cap traces, then P3/P4 channels and the independent frozen semantic exit packet. PostgreSQL qualification does not complete Stage 3.
- The generated analysis-free documentation seed still fails `semantic:documentation-only-has-outcome` fail-closed; unscheduled. No larger-corpus scaling or full float64 ordinal-parity claim is made.
