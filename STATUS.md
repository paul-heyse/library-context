# Status

_Updated 2026-09-25 under the [handoff skill](.claude/skills/handoff/SKILL.md); work is on `main`._

## Increment and slice

- Active: [Stage 3 of increment 4](docs/plans/behavioral-model-forward-plan_2026-09-24.md). `4af94e8` contains ADR-0039's BDD predecessor refinement, gate repairs, accepted snapshots and Rust formatting; `acbcee5` contains the Python tool-inventory correction and formatting. Stage 3 is not functionally complete.
- Process: [ADR-0040](docs/adr/0040-architecture-change-scenarios.md) implements repository-owned core 3.0/profile 1.1, six architectural foundations, change scenarios, A1–A3 judgments and proportional review cadence. Skills, DESIGN, binding and finding ownership are aligned. Improved architectural outcomes remain Proposed.
- L1/L1.5: bounded BDD catalogs, exact entry-value links, narrow primitive refutation and native path-local work accounting exist. Operation-wide compatibility is open.
- L4: typed five-channel pinned models and exact source applications exist. `typing.cast`/`assert_type` assert total normal return; Pydantic `TypeAdapter.validate_python` remains a dormant potential transform with dynamic schema/effects open. Broader pure/helper, async/context and HTTP/server families remain.
- L2/L3: pass-only nested finalizers have ordered proof steps; other frame fates remain unknown. Direct, exact modeled, unique-assignment and unconditional acyclic wrapper value paths have canonical ordered proofs and shared reconstruction. ADR-0039 ignores a prior call only when its ty region is definitely BDD-incompatible with the return; compatible/missing/approximate/capped predecessors withhold a direct positive. Recursive modeled/assignment members, effect/exception/role summaries and `call_transfer` discharge remain open.
- FORMAT 7/native `inspect_value_paths` provides path-local inspection, not operation-wide compatibility/effect/role verdicts.

- Documentation: ADR-0041 and the [execution/qualification plan](docs/plans/architectural-documentation-and-search_2026-09-25.md) implement focused section owners, mdBook/Pagefind search and isolated local/CI artifact commands (`b92857e`, `821173f`). Target and assembled reviews Accept; future reading/change-cost improvements remain Proposed.

## Last verified (2026-09-25)

Functional receipts below are preserved from the preceding Stage 3 session; this process revision did not rerun them.

| Command | Outcome |
|---|---|
| Documentation: `just bootstrap-docs`; `just docs-test`; `just docs-check`; focused Ruff/pyrefly; `actionlint .github/workflows/docs.yml` | `passed`, 2026-09-25: 31 tests, full local and clean-checkout publication, root/prefix browser scope and heading checks. [Plan](docs/plans/architectural-documentation-and-search_2026-09-25.md) records scope; remote CI `not_run` (workflow not pushed). Product gates were not rerun. |
| `just fmt` | `passed`; deferred Rust/Python formatting applied. |
| `just test-all` (post-repair attempt) | `failed`: 313/313 release Rust tests passed; Python had 110 passed and 2 stale tool-inventory failures. The gate stopped before pyrefly, rules, fixtures, deps and gold. |
| `uv run pytest -q python/lctx_mcp/tests/test_server.py` | `passed`: 12/12 after adding `inspect_value_paths` to the protocol expectation. |
| `just test-all` (final rerun) | `not_run` to completion: started, then stopped at operator direction during the Rust phase. The operator assumes the remaining checks will pass; that is an expectation, not an observed complete-gate result. |
| `just adr lint`; `just lint-agents`; `git diff --check` | `passed` for this process revision; ADR lint counts 40 records. Prior-session `just adr revisit` passed against 39 records and was not rerun here. |
| `uv run python /home/paul/.codex/skills/.system/skill-creator/scripts/quick_validate.py .claude/skills/design-review` | `failed`: the generic validator rejects existing Claude metadata `user-invocable` and `model-baseline`; those invocation fields were preserved. The same validator passed for `adr`, `handoff` and `design-review-code-intelligence`. |
| Markdown/manifest link inspection | `passed`: read-only `uv run python` inspection of changed document links and `standard.toml` paths. |
| Fresh `just pilot`; Q01/Q03/Q05/Q09; structured evaluation; clean-wheel query; increment-end review | `not_run`; Stage 3 functionality and exit are incomplete. |

## Known failures, blocks and decisions

- The two independent core-3.0 calibration reviews conclude **Revise**. Source inspection found native rejection of the compiler's finalizer proof kind, summary policy coupled to acquisition, and discarded refusal causes. [Plan §9.2](docs/plans/behavioral-model-forward-plan_2026-09-24.md#92-current-architectural-dispositions) owns their current dispositions and closure checks. No production fixes or runtime reproduction were performed in this process revision.
- Historical schema repairs remain: raw value-flow contribution keys distinguish local/upstream transfer flags (compiler output version 74); multi-release validation selects one distinct snapshot; MRO shape validation compares distinct ancestor facts with provenance. Reviewed schema, rule, codebook and fixture snapshots were accepted in the functional session.
- The preliminary full gate did not finish. In particular, the post-fix whole Python suite, pyrefly, rules, fixture parsing, dependency policy and gold checks have no observed current-tree pass; record the operator's pass assumption separately. The pilot and all Stage 3 exit measurements are still absent.
- The plan's functional queue retains predecessor completion, other frame/callback/resource fates, SCC composition, path-specific identity, dynamic-schema Pydantic effects and full native evidence/filters. An empty summary remains unknown, never refuted.
- ADR-0020, ADR-0024, ADR-0025 and ADR-0028 remain proposed. ADR-0039 supersedes ADR-0038; ADR-0037 supersedes ADR-0036. Accepted ADR-0040 supersedes ADR-0023 and replaces the cadence clauses carried by ADR-0021/0026; their unrelated decisions remain in force.

## Next

Start with [ARC-01](docs/plans/behavioral-model-forward-plan_2026-09-24.md#ARC-01): derive native proof-kind admission from the schema codebook and verify a real finalizer-bearing generation. Then establish explicit summary inputs/outcomes under ARC-02/03 while building the compatible-predecessor normal-outcome witness. Preserve self-recursion, finite-base and disjoint-branch controls. Follow the plan queue through non-call predecessors and SCC composition; run the complete gate, fresh pilot and evaluation/serving checks at integrated exit.
