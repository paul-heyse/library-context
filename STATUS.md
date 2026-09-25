# Status

_Updated 2026-09-24 under the handoff skill. Work is on `main`._

## Where we are

- The active scope is [the behavioral-model forward plan](docs/plans/behavioral-model-forward-plan_2026-09-24.md). Stage 2.9 is complete. Stage 3.0 now has exact BDD conditions and leaf identities persisted through Delta and FORMAT 6, validated native graph loading, and attributed `flow_test_types` observations. The [persisted-graph compact review](docs/design_review/reviews/design_review_stage3_0_persisted_graph_native_load_compact_2026-09-24.md) accepts that scoped path; the [test-type compact review](docs/design_review/reviews/design_review_stage3_0_test_type_observation_compact_2026-09-24.md) accepts observations with a remaining served-proof boundary.
- ADR-0024 and ADR-0025 remain proposed. Native `probe_*` calls are developer smoke, not served compatibility verdicts. Editable native import and focused probes are the design loop; a clean wheel install and generation-pinned tool call wait for Stage 3.6 product acceptance.
- Commit `0eedcaa` makes stable Rust 1.98.1, 16 Cargo jobs, sccache, incremental compilation off, and Clang/mold the development defaults (ADR-0026). Routine work stays on `main` in the current tree; a worktree is for truly concurrent production-code edits. The [single-run build screen](docs/design_review/evidence/2026-09-24_rust-build-performance/README.md) does not establish the final configuration's speed.
- The separate Z3/LLVM system migration is documented in [its evidence](docs/design_review/evidence/2026-09-24_z3-5-migration/README.md); its product acceptance boundaries are separate from this build-configuration slice.

## Last verified (2026-09-24)

| Command | Outcome |
|---|---|
| `just test-all` at the reviewed persisted-graph checkpoint | passed: 283/283 Rust, fixture generation, 96/96 Python, Pyrefly, rules, ADR/agent lint, fixtures, dependencies and gold |
| `just pilot build/store-stage3-bdd-format6-final` at that checkpoint | passed: snapshot `226d70b7c94aabca98de16a0be968229`, generation `b31985ff58878132`, 20/20 smoke; zero `budget_reached` behaviors |
| Focused test-type checks in the compact review | passed: flow leaf attribution, extractor type-row identity, and publication-link tamper rejection |
| `uv run pytest tests/scripts/test_build_measurements.py -q` | passed: 5/5 after adapting uncached controls to the default wrapper |
| `just adr lint`; `just lint-agents`; `git diff --check` | passed: 26 ADRs, instruction parity, no whitespace errors |
| `just test-all` at `0eedcaa` | failed by operator interruption during Nextest: Clippy passed, 284/285 Rust tests passed, one long test received SIGINT; remaining gate steps did not run. No product-test assertion failed before interruption |
| `just pilot` after `0eedcaa`; `just adr revisit` after ADR-0026 | not_run for this configuration slice at operator direction; no performance timing inferred from the interrupted gate |

## Open boundaries

- Stage 3 still needs the cited operation-entry-to-test bridge, exact value/effect-stability proof, typed theory, served generation-pinned semantic queries, models and summaries, differential checks, and structured exit evaluation. The `flow_test_types` Pyrefly trace is an observation, not proof of an exact runtime class or a negative compatibility verdict. The persisted-graph review's F03 and test-type review's T02–T04 carry these boundaries.
- ADR-0024/0025 remain proposed; ADR-0020 remains proposed on its increment-5 trigger. A stable 16-job, one-frontend-thread paired timing and cache recovery are not_run; ADR-0026 records the operator's chosen development defaults without claiming a speedup.
- The interrupted `just test-all` is incomplete. The operator explicitly stopped full testing for the build-configuration changes. There is no task-owned Cargo, Nextest or rustc process left from that run.

## Next

Continue the Stage 3 proof bridge from structurally attributed test leaves to operation-entry values, preserving `unknown` where source identity or effect stability is unproved. Use focused probes during design; run the broader product gate at its planned checkpoint.
