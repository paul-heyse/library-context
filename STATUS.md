# Status

_Updated 2026-09-26 under the [handoff skill](.claude/skills/handoff/SKILL.md); work is on `main`._

## Increment and slice

- **Increment 4, Stage 3 remains functionally incomplete.** The
  [forward plan](docs/plans/behavioral-model-forward-plan_2026-09-24.md) is the execution owner;
  §3 gives the dependency order and §6 owns W1–W16 disposition. The 2026-09-25 library-fit
  reviews are evidence, not authority.
- **Landed remediation:** W1 typed native IPC admission; W2 typed facet values; W3 served finding
  support; W4 resolved builtin access; W8 hermetic corpus identity; W9 canonical cache winner;
  W14 Pyrefly function status, ty run context and suppression visited state; W16 controlled
  vLLM launch. W15 [ADR-0048](docs/adr/0048-schema-rebuild-policy.md) chooses a fresh current
  store rebuild instead of historical binary compatibility. These have focused evidence only.
- **Stage 3 progress:** [ADR-0050](docs/adr/0050-finite-summary-outcomes.md) owns explicit finite
  summary inputs/refusals. The order-1 predecessor proof admits literal, signed numeric literal, builtin, parameter and unique-assignment arguments for a sole pinned normal-return call, and separates
  a terminating branch from an unconditional recursive predecessor. W5 atom-limit publication
  reaches Delta/FORMAT 8/native; actual work/node limits reach the pure producer. W6 decision,
  effective-support and aggregate native admission controls pass. W7's bounded source fixed point
  ([ADR-0051](docs/adr/0051-value-reach-worklist.md)) passes cyclic, shuffle and production-cap
  pure controls. W11 passes a CPython literal matrix and native path-local string/int controls.
  W13 uses iterative petgraph SCCs ([ADR-0052](docs/adr/0052-iterative-scc-schedule.md)) and
  DataFusion distinct recursive type-term set closures. L2 admits ordered pass-only finalizer suites; order 1 signed literals landed at `451a623`.

## Last verified (2026-09-26)

| Command | Outcome |
|---|---|
| `cargo test -p cpg-schema --test recursive_type_terms --quiet`; focused `cpg-core --test analysis` type-layer/FCA cases | `passed`: cyclic type-term closures and real consumers; FCA used `RUST_MIN_STACK=16777216`. |
| `cargo test -p cpg-schema --lib decision_checks_large_result_and_over_preflight_contradiction --quiet`; `... production_retained_limit_refuses_valid_shared_tail_before_hydration --quiet` | `passed`: task/retained caps remain unknown/refused. |
| `uv run --no-sync pytest python/lctx_mcp/tests/test_native_semantics.py::test_native_catalog_refuses_aggregate_retained_limit_before_hydration python/lctx_mcp/tests/test_native_semantics.py::test_native_catalog_distinguishes_stored_from_retained_shared_tail -q` | `passed`: native aggregate refusal and diagnostics. |
| `cargo test -p lctx-analytics --lib actual_predecessor_work_and_node_caps_keep_specific_unknown_causes --quiet` | `passed`: specific producer refusals, no positive flow. |
| `RUST_MIN_STACK=16777216 INSTA_UPDATE=no cargo test -p cpg-core --test compile real_flow_input_row_order_keeps_published_contribution_bytes --quiet` | `passed`: canonical published Arrow IPC bytes equal after extracted-row reversal. |
| `cargo test -p cpg-core --lib production_work_cap_keeps_cycle_dependents_open --quiet`; `cargo test -p cpg-extract --lib edited_or_added_unselected_helper_facts_equal_a_clean_recomputation --quiet` | `passed`: production work cap and warm/clean corpus edit/addition. |
| `uv run --no-sync python docs/design_review/evidence/2026-09-26_primitive-python-lowering/probe.py`; `cargo test -p cpg-schema --lib finite_primitive_lowering_matches_recorded_cpython_314_predicates --quiet`; `uv run --no-sync pytest python/lctx_mcp/tests/test_native_semantics.py::test_native_string_membership_and_integer_equality_stay_path_local -q` | `passed`: CPython 3.14.7 finite lowering and native two-path control. |
| Focused `cpg-core --test compile nested_returns_need_an_uncontrolled_exit_before_becoming_value_summaries` and `--test bundle finalizer_proof_round_trips_through_the_native_generation_reader` (`RUST_MIN_STACK=16777216 INSTA_UPDATE=no`, `--quiet`) | `passed`: two ordered pass steps through Delta, shared validation and native reader. |
| `just adr lint`; `git diff --check` | `passed` on the 2026-09-26 focused work. |
| `just fmt`; `just test-all`; fresh `just pilot`; all-techniques digest; Q01/Q03/Q05/Q09; structured evaluation; clean-wheel query | `not_run`: defer until all planned functional scope is implemented. |

## Known boundaries and decisions

- The code is **not integrated-qualified**. The 2026-09-25 `just test-all` attempt stopped after
  two stale Python tool-list tests, later corrected by a focused check; no complete rerun exists.
  A schema/query digest repin is likely at integrated acceptance after W13's SQL change.
- Order 1 still needs general argument/predecessor evaluation and path-specific origin identity.
  Orders 2–9 (exit/fate, composition proof identity, modeled results/families, recursive summaries,
  discharge, independent challenge and complete serving) are not finished. W5 work/node caps lack
  Delta/native publication traces; W7's production cap lacks one; W9 live replay lacks a running
  controlled embed service; W11 lacks real Delta/MCP Q09; W12 engine comparison and pilot cost
  remain. W10 and W14/F15 retain the plan's deferred triggers.
- Do not count a fake embedder as live W9 evidence. W6's large native repeated-root input checks
  the shared loader preflight; the distinct-valid-root production limit is proved in Rust. No
  negative claim may be drawn from a BDD cap, CrossHair/Pysa silence or an open source path.

## Next

Implement order 1's next source-cited predecessor/evaluation case in `cpg-schema::behavior` and
`lctx-analytics::summaries`, then extend order 2's exit witnesses with positive, withholding and
shared-validator controls. Compare SCC worklist, Ascent and datafrog before order 6 (W12); defer formatting, integrated tests, pilot and evaluation.
