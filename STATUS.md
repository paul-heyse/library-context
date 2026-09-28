# Status

_Updated 2026-09-28 under the [handoff skill](.claude/skills/handoff/SKILL.md); work is on `main`._

## Increment and checkpoint

- **Increment 4 / Stage 3 remains functionally incomplete.** [Forward plan §3.0](docs/plans/behavioral-model-forward-plan_2026-09-24.md#30-consolidated-execution) owns P0–P7 and the frozen exit rule; §6 owns finding disposition.
- **P0 done; P1 implemented, change review to settle; P2 partly done.** ADR-0064,
  decorated-summary refusal, claim-keyed discharge and native citation admission remain the current
  semantic slice. The concurrent discharge review, W5/W7/W12 traces and P3 Logger models are unchanged.
- **PostgreSQL PG12–PG15 implemented (`4288940`):** resumable COPY publication, immutable generation selection,
  Rust query/hydration and async MCP cutover, exact pgvector ranks, qualification-gated HNSW,
  admitted DataFusion federation and coherent operational reports. Portable files remain canonical
  export/reference/recovery inputs. Online MCP requires a ready PostgreSQL generation.
  [Plan](docs/plans/postgresql-integration-plan_2026-09-27.md), [runbook](docs/postgresql.md),
  [dated evidence](docs/design_review/evidence/2026-09-28_postgresql-query/README.md).
- **Architecture:** ADR-0069 adds query/publication contracts to ADR-0068; ADR-0067 retains Delta
  authority. [Fresh bounded review](docs/design_review/reviews/design_review_postgresql-query-integration_2026-09-28.md): **Accept scoped**; SQLx lease correction tested closed. Outer joins remain local.
- **Versions:** compiler **107**, extractor **33**, template **20**, catalog **7**, FORMAT **12**;
  projection format **2**, standard embedding width **1024**. Rebuild older portable generations.
  Owned provider pin: `paul-heyse/datafusion-table-providers@e6fc4c40`.
- **Operator database:** PG18.6/pgvector 0.8.6 and migrations 001–004 remain deployed. New 005–007
  migrations ran only in disposable databases. Preserve `build/postgresql-pg8-baseline-6403b60/`.
- **Development workflow:** root editable uv environment and cached Cargo target; rebuild native
  bindings in place when needed. Standalone wheels/clean installs are deferred until distribution
  requires them. No further wheel work is part of this phase.

## Last verified (2026-09-28)

All Cargo commands used `/home/paul/library-context/target`. Broad checks followed functional scope.
The evidence page records exact commands and distinguishes retained passes from repair reruns.

| Command | Outcome and scope |
|---|---|
| `just test-all` and localized repair selectors | **passed across retained/repair receipts:** 437 ordinary Rust tests; the initial invocation failed on nine old consumers, all nine passed targeted repair |
| `just py-check` and ranking guard repair | **passed across retained/repair receipts:** 169 Python tests; two explicit PG tests skipped here and run separately; Pyrefly passed |
| `just test-postgres` | **passed:** 17 real PostgreSQL tests and two async lifecycle tests; acquired-corpus report correction passed its focused rerun |
| `just fmt-check lint`; final report Clippy; remaining policy/SQLx recipes | **passed:** strict Rust/Python checks, 84 fixtures, rules/agents, dependency/gold policy and fresh-schema SQLx metadata |
| Full frozen reference selector | **passed:** 6,302 operation answers, 20 briefs and four facet pages; same canonical inputs |
| `just pilot build/postgresql-pg15-pilot-store build/pg15-pilot.log …` | **passed:** fresh fake-vector FastMCP compile, PostgreSQL import and all 20 MCP brief smoke checks |
| `just docs-check`; scoped publisher | **failed:** unchanged supplied external review lacks H1. **passed:** ADR/agent checks and 153 canonical pages/offline links excluding only that input |

## Known boundaries and next

- **Next PostgreSQL step: PG16**, coordinated operator migration/import/rollover and recovery;
  PG17 owns assembled live ANN/latency/RSS/WAL acceptance. Exact retrieval stays the default;
  no production ANN profile is qualified or selected. The tiny-fixture ANN refusal is tested.
- [Forward-plan §6.1](docs/plans/behavioral-model-forward-plan_2026-09-24.md#postgresql-findings) owns PGS/PGK/PGE/PGF/PGQ disposition; broader deployment findings remain open. W9 stays closed; arbitrary endpoint attestation remains W16 work.
- PostgreSQL work does not complete Stage 3. Resume P1 review disposition, P2.3 default-cap traces,
  then P3/P4 channels and the frozen semantic exit packet.
- The undocumented analysis-free documentation seed still fails `semantic:documentation-only-has-outcome`
  fail-closed; generated packages only, unscheduled. Live embedding was not rerun in this slice.
- Unrelated Stage 3 review and external-review input remain preserved and uncommitted.
