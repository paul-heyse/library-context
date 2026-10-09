# Agent effectiveness follow-up evidence

**Question:** which changes to this workstation, repository commands, agent configuration and native/library tooling would make Codex more effective here, including useful abilities and concrete execution friction?

**Consumers:** the [principal design review](../../reviews/design_review_agent-effectiveness-followup_2026-10-09.md) owns the dated judgment; the [follow-up plan §8](../../../plans/agent-effectiveness-followup-plan_2026-10-09.md#8-finding-and-recommendation-disposition-owner) now owns scheduled F01–F05, selected recommendations and transferred AE-24. The October 7 [workspace plan](../../../plans/agent-workspace-effectiveness-plan_2026-10-07.md) retains its other work and historical receipts. Collecting this evidence establishes no closure.

**Scope:** personal Linux workstation and library-context, Codex primary, limited Claude Code. The five criteria are simpler command construction, fewer environment complications, parameterized related operations, readable outcomes/recovery, and freedom to use native tools and explicit overrides. User/system-specific customization is a viable recommendation. Portability, an additional registry, mandatory discovery ritual and another gate are not goals.

**Date/baseline:** 2026-10-09. Assignment began at `d2341cc4a3fcaf02f5a3ca9cb1b5d0a4d0490ed5`; concurrent plan work advanced HEAD to `6c9755226a2c6960e4680a9db644e52f2abb3808`. Product/compiler/serving/model/fixture changes were already dirty. Current source inspection describes that mixed tree. A review is not product acceptance, and no dirty implementation was reverted or qualified here.

## Evidence routes

| Document | Question and evidence boundary |
|---|---|
| [Recent command retrospective](retrospective.md) | Completion-event window October 6–9, exact failed invocations, surrounding corrections, attribution and classifier limits. Original local transcripts are authoritative; raw extraction stays ignored. |
| [Frozen aggregate](metrics.json), [reproduction helper](mine_codex.py) | Counts and extraction contract. The helper imports the retained October 7 miner; it is a bounded review instrument, not production tooling. |
| [Codex/Rust capabilities](capabilities-codex-rust.md) | Native continuation, durable runs, configuration diagnostics, semantic navigation/refactoring, Cargo ownership/diagnostic JSON, nextest and compilation profiling. Documentation/installed/exposed/exercised distinctions are explicit. |
| [SurrealDB capabilities](capabilities-surrealdb.md) | Pinned SDK and CLI, built-in HTTP versus embedded stdio MCP, schema/query/EXPLAIN, statement errors, session and fixture scope. Proposed integrations have not been exercised. |

The retrospective cutoff is `2026-10-09T18:45:09Z`; the current review thread and descendants are excluded. Current installed tooling discovery is dated separately and is not evidence that older commands used today's binary/configuration. Selected safe command excerpts are published because the user requested concrete execution evidence. Private configurations, credentials, raw human messages and raw session streams are not published.

## Current source/owner routes

| Responsibility | Source or owner |
|---|---|
| Functional target and current qualification | [STATUS](../../../../STATUS.md), [product target](../../../design/sections/api-and-evidence-product.md), [DESIGN §15 semantic owner](../../../design/sections/semantic-model.md#section-15) |
| Workflow and preparation boundaries | [DESIGN](../../../design/DESIGN.md), [AGENTS commands](../../../../AGENTS.md#commands), `justfile`, `scripts/workspace_env.py`, `scripts/build_environment.py` |
| Verification selection, readiness and outcomes | `scripts/verify.py`, `scripts/qualify.py`, `tests/scripts/test_verify.py`, previous [workspace plan](../../../plans/agent-workspace-effectiveness-plan_2026-10-07.md) |
| Durable process supervision and cleanup | `scripts/runs.py`, `scripts/harness.py`, `tests/scripts/test_runs.py` |
| Native fixture and diagnostic scope | `scripts/surrealdb_fixture.py`, [SurrealDB operations](../../../surrealdb.md), `crates/lctx-surrealdb/src/reader.rs` |
| Build/cache/profile composition | `.cargo/config.toml`, workspace `Cargo.toml`, `scripts/compile_profile.py`, `scripts/compile_profile_tools.py`, current [compilation plan](../../../plans/rust-compilation-costs-plan_2026-10-08.md) |
| Configuration and role routing | `.codex/config.toml`, user Codex configuration (redacted inspection), [.agents roles](../../../../.agents/roles/README.md), selected shared library skills |

Independent bounded source mapping and library research informed a fresh design reviewer. The reviewer inspected decisive source independently and judged static evidence sufficient for lifecycle, observation and preparation findings. No new probe was needed to establish those source-level consequences. The example at `/home/paul/corpus-intelligence/docs/design_review/reviews/design_review_agent-effectiveness-followup_2026-10-08.md` supplied questions, not transferable findings or receipts.

## Outcomes and reproduction

**passed, 2026-10-09:** the retrospective extraction command below; installed version/help inspection, scoped Context7/official documentation retrieval and bounded Cargo metadata listed with exact commands in the capability documents. These are discovery/extraction receipts, not product tests or performance/effectiveness measurements.

```sh
uv run --no-sync python docs/design_review/evidence/2026-10-09_agent-effectiveness-followup/mine_codex.py
```

It writes `build/agent-effectiveness-followup/2026-10-09/{commands.jsonl.gz,tool-errors.jsonl.gz,index.json,metrics.json}`. Reproduction needs the original local Codex database/logs and retained miner, and can observe later database changes even with a fixed timestamp filter. The committed aggregate is the dated observation; it is not guaranteed byte-for-byte reproduction from future state.

**failed, historical/lookup boundaries only:** selected command/call failures in the retrospective; missing or incorrect research source paths corrected as reported in the capability evidence. Corrected diagnostic invocation does not make an underlying failing operation pass.

**not_run:** fresh-session configuration effects, Rust semantic MCP requests, SurrealDB MCP registration/handshake or schema query, SDK stream conformance, product compilation/tests, assembled qualification, real-library pilots and any effectiveness/performance comparison. No environment repair/sync, service installation/start, operator database/client access, configuration or shared-skill change occurred in this review.

Publication/link/format checks belong to the principal review's final receipt and STATUS. The evidence does not repeat their mutable disposition.
