# Status

_Updated 2026-09-25 under the [handoff skill](.claude/skills/handoff/SKILL.md); work is on `main`._

## Increment and slice

- Active: [Stage 3 of increment 4](docs/plans/behavioral-model-forward-plan_2026-09-24.md). Latest functional commits: `e97e6ba` (ordered pass-only finalizer proof, ADR-0037) and `87bd3a9` (recursive finite-flow boundary). The plan checkpoint and this handoff are the only edits in flight.
- Conditions: bounded BDD catalogs, exact entry-value links, narrow primitive refutation and native per-path work accounting are focused-tested. Operation-wide semantic compatibility is open.
- Pinned models: typed five-channel catalog and exact source applications are focused-tested. `typing.cast`/`assert_type` have asserted normal completion; Pydantic `TypeAdapter.validate_python` is a dormant potential transform with dynamic schema/effects open. Other required semantic families remain incomplete.
- L2 exits: a nested chain of sole `finally: pass` frames now has ordered source proof steps and a CPython monitoring control; other finalizers, `with`, handler propagation, callback execution and resource release remain open.
- Finite summaries: direct, modeled, unique-assignment and unconditional acyclic local value paths have canonical ordered proofs and shared reconstruction. Recursive SCC members now retain unknown boundaries until a bounded worklist proves completion. Effect/exception/role summaries and behavior `call_transfer` discharge remain open.
- Partial FORMAT 7/native serving exists for exact primitive path-local inspection. Full source-span evidence and typed operation-wide compatibility/effect/role filters are unfinished.

## Last verified (2026-09-25)

| Command | Outcome |
|---|---|
| `RUST_MIN_STACK=33554432 cargo test -p cpg-core --test compile nested_returns_need_an_uncontrolled_exit_before_becoming_value_summaries -- --exact` | `passed`; nested pass order, effectful/`with` withholding, recursive SCC boundary, shared-validator tamper controls. |
| `RUST_MIN_STACK=33554432 cargo test -p cpg-core --test compile pinned_identity_models_require_and_publish_their_real_formals -- --exact` | `passed`; modeled nested finalizer and prior model/source contracts. |
| `RUST_MIN_STACK=33554432 cargo test -p cpg-core --test compile explicit_exit_sites_have_regions_and_reject_a_doctored_span -- --exact` | `passed`; exit source equality. |
| `uv run pytest -q tests/scripts/test_flow_soundness.py::test_pending_value_return_through_nested_inert_finalizers_is_admitted` | `passed`; independent CPython 3.14 monitoring control. |
| `cargo clippy -p cpg-core --test compile -- -D warnings`; `uv run ruff check tests/scripts/test_flow_soundness.py`; `just adr lint`; `git diff --check` | `passed` after their respective latest code edits; ADR lint counted 37 records. |
| `just fmt`; `just check`; `just test-all`; fresh-store `just pilot`; Q01/Q03/Q05/Q09; structured evaluation; clean-wheel query; `just adr revisit` | `not_run`: operator requires integrated tests and formatting only after the entire Stage 3 functional scope is implemented. The earlier full gate at `d61a497` does not certify this tree. |

## Known failures, blocks and decisions

- No current focused test failure is known. Stage 3 is incomplete; a true region on `return x` does not prove preceding calls finish. The recursive guard is conservative and also withholds finite base branches.
- The latest compact reviews defer a shared frame-action source for future non-pass exits, effectful finalizer/context-manager completion, per-path recursive base admission and preceding-statement normal-outcome witnesses. `summary_boundaries` still has aggregate identity coarser than source contributions.
- ADR-0020, ADR-0024, ADR-0025 and ADR-0028 are proposed. ADR-0037 supersedes ADR-0036 for ordered pass-only frames. `just adr index` and `just adr lint` passed; `just adr revisit` remains `not_run` under the targeted-only phase.
- Pinned FastMCP/Pydantic binding, broader Pysa/CrossHair/CPython challenges, SCC value/effect/exception/role composition, operation verdict discharge, FORMAT 7 completion, and all integrated acceptance remain unverified.

## Next

Prove statement and call normal completion along one proposed return path, with a terminating recursive base branch versus an unconditional self-call as controls. Reuse Ruff/ty source facts, DataFusion joins and the existing bounded SCC/BDD owners; keep every unsupported predecessor as an explicit unknown. Continue focused checks only until the full Stage 3 functional scope lands.
