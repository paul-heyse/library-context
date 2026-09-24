# Status

_Updated 2026-09-24 under the handoff skill. Work is on `main`; no push was requested._

## Where we are

- The active scope is [the behavioral-model forward plan](docs/plans/behavioral-model-forward-plan_2026-09-24.md). Stage 2.9 is complete: ADR-0022 is accepted, the compact re-review and Stage 2 structured evaluation passed, and the live pilot smoke passed.
- Stage 3 is in **design and focused-probe mode** by operator direction. The BDD spike (`2da15a6`) and the bounded kernel/native developer-probe foundation (`f2549cb`) are committed. ADR-0024 and ADR-0025 remain **proposed**. The [standard design review](docs/design_review/reviews/design_review_stage3_kernel_serving_standard_2026-09-24.md) accepts their corrected proposal, including the F05 leaf-identity correction at design level; the [compact review](docs/design_review/reviews/design_review_stage3_0_kernel_native_compact_2026-09-24.md) accepts only the foundation slice.
- The user chose ADRs and focused probes for this phase. The proposed serving design permits bounded Rust semantic queries over one pinned generation, replacing ADR-0010's materialized-only executor constraint. Editable native import and focused tests are the design loop; one clean wheel install and generation-pinned tool call wait for Stage 3.6 product acceptance, then repeat at release. The wheel/archive and isolated wheel-test environment created during this session were removed.

## Last verified (2026-09-24)

| Command | Outcome |
|---|---|
| `cargo test -p cpg-schema condition_kernel --lib --quiet` | passed: 8/8 bounded-kernel tests |
| `uv run pytest python/lctx_mcp/tests/test_native_semantics.py -q` | passed: 1/1 editable native developer smoke |
| `just test-all` at `f2549cb` | passed: 277/277 Rust, fixture generation 1/1, 94/94 Python, Pyrefly, 7/7 rules, 25 ADRs, 65 fixture parses, dependency/fork/shear/gold checks |
| `just pilot` at `f2549cb` | passed: snapshot `ecf8b9cdc11ebaf4c1dae61fa9b35dee`, generation `88660525d69030df`, 20/20 FastMCP smoke; 44.9 s total, 4,227 MiB peak RSS. Compiler still uses Stage 2 DNF |
| `CARGO_TARGET_DIR="$PWD/target" cargo run --locked --manifest-path docs/design_review/evidence/2026-09-24_bdd-pilot-survey/Cargo.toml -- build/store ecf8b9cdc11ebaf4c1dae61fa9b35dee` | passed: 13,775/13,775 stated conditions converted and validated in 0.97 s; p95 8, max 53 nodes per root; 23,760 unique node ids; one `SourceOverBudget` sentinel |
| `cargo test -p cpg-flow --test flow_shapes --quiet` | passed: 28/28 existing Stage 2 flow shape checks; the read-only test-leaf join probe inspected pilot compound and fixture `match` cases |
| `lctx query --store build/store --snapshot ecf8b9cdc11ebaf4c1dae61fa9b35dee` duplicate-`flow_uses.use_id` count | passed: 0 duplicate IDs in this pilot snapshot; source-use role remains unproved |
| `target/release/lctx flow docs/design_review/evidence/2026-09-24_entry-value-bridge/probe.py` | passed: direct, rebind, post-call and nonlocal-write cases inspected; reaching alone does not establish cross-call value stability |
| `just adr lint`; `just adr revisit`; `git diff --check` | passed: 25 ADR records, dependency revisit check, no whitespace errors |
| `just check` for this design-only slice | not_run: operator chose ADRs and focused probes; the last broader gate is the `just test-all` result above |

## Open boundaries

- The single Stage 2 `over_budget` condition row is referenced by 237 `flow_regions` and 846 `flow_reaching` facts across modules. Materialized-row conversion cannot recover those distinct source expressions. Construct BDDs in `cpg-flow` **before** DNF truncation, migrate `flow_model` with the root/node schema, then measure whether the 66 recorded `budget_reached` claims fall.
- BDD nodes are not yet a published Delta or serving-bundle relation; the proposed `flow_test_leaves` row/key contract, typed test-use observations, exact-value/effect-stability proofs, generation-pinned semantic queries, models catalog, handlers/callbacks/resources, SCC summaries, Pysa differential, and Stage 3 structured exit evaluation are **not_run/not implemented**. The native `probe_*` calls are developer smoke only, not FastMCP verdicts.
- ADR-0024/0025 remain proposed; ADR-0020 remains proposed on its separate increment-5 trigger. No new manual revisit trigger was established by this session. The standalone survey's own lock may differ in unrelated transitive packages from the product lock, so it is design evidence, not product acceptance.
- Uncommitted edits in `AGENTS.md`, `docs/pins.md` and `.claude/skills/README.md` are outside this slice and were preserved.

## Next

Probe exact source-use attribution for compound and synthetic pattern atoms against the proposed `flow_test_leaves` row contract; keep unproven mappings unknown. Then implement only the reviewed direct/no-effect proof in the later product slice. Keep broad packaging gates for the later product checkpoint.
