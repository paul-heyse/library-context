# Status

_Updated 2026-09-25 under the [handoff skill](.claude/skills/handoff/SKILL.md); work is on `main`._

## Increment and slice

- Active: [Stage 3 of increment 4](docs/plans/behavioral-model-forward-plan_2026-09-24.md). Commit `4af94e8` contains ADR-0039's BDD predecessor refinement, provisional-gate repairs, accepted snapshots and deferred Rust formatting. The Python tool-inventory correction and deferred Python formatting accompany this handoff. Stage 3 is not functionally complete.
- L1/L1.5: bounded BDD catalogs, exact entry-value links, narrow primitive refutation and native path-local work accounting exist. Operation-wide compatibility is open.
- L4: typed five-channel pinned models and exact source applications exist. `typing.cast`/`assert_type` assert total normal return; Pydantic `TypeAdapter.validate_python` remains a dormant potential transform with dynamic schema/effects open. Broader pure/helper, async/context and HTTP/server families remain.
- L2/L3: pass-only nested finalizers have ordered proof steps; other frame fates remain unknown. Direct, exact modeled, unique-assignment and unconditional acyclic wrapper value paths have canonical ordered proofs and shared reconstruction. ADR-0039 ignores a prior call only when its ty region is definitely BDD-incompatible with the return; compatible/missing/approximate/capped predecessors withhold a direct positive. Recursive modeled/assignment members, effect/exception/role summaries and `call_transfer` discharge remain open.
- FORMAT 7/native `inspect_value_paths` provides path-local inspection, not operation-wide compatibility/effect/role verdicts.

## Last verified (2026-09-25)

| Command | Outcome |
|---|---|
| `just fmt` | `passed`; deferred Rust/Python formatting applied. |
| `just test-all` (post-repair attempt) | `failed`: 313/313 release Rust tests passed; Python had 110 passed and 2 stale tool-inventory failures. The gate stopped before pyrefly, rules, fixtures, deps and gold. |
| `uv run pytest -q python/lctx_mcp/tests/test_server.py` | `passed`: 12/12 after adding `inspect_value_paths` to the protocol expectation. |
| `just test-all` (final rerun) | `not_run` to completion: started, then stopped at operator direction during the Rust phase. The operator assumes the remaining checks will pass; that is an expectation, not an observed complete-gate result. |
| `just adr lint`; `just adr revisit`; `git diff --check` | `passed` before a separate draft ADR-0040 appeared in the shared tree; ADR lint counted 39 records at that point. |
| Fresh `just pilot`; Q01/Q03/Q05/Q09; structured evaluation; clean-wheel query; increment-end review | `not_run`; Stage 3 functionality and exit are incomplete. |

## Known failures, blocks and decisions

- No unresolved focused semantic failure is known. The corrected raw value-flow contribution key is a schema migration: local and upstream transfer flags now distinguish paths; compiler output version is 74. The multi-release validator now selects one distinct snapshot, and MRO shape validation compares distinct ancestor facts while preserving library/corpus provenance. Reviewed schema, rule, codebook and fixture snapshots were accepted.
- The preliminary full gate did not finish. In particular, the post-fix whole Python suite, pyrefly, rules, fixture parsing, dependency policy and gold checks have no observed current-tree pass; record the operator's pass assumption separately. The pilot and all Stage 3 exit measurements are still absent.
- Deferred review findings: normal completion of compatible preceding calls and non-call operations; effectful finalizers/`with`; callback/resource fates; bounded SCC composition; path-specific boundary identity; dynamic-schema Pydantic effects; full native evidence and filters. An empty summary remains unknown, never refuted.
- ADR-0020, ADR-0024, ADR-0025 and ADR-0028 remain proposed. ADR-0039 supersedes ADR-0038; ADR-0037 supersedes ADR-0036. Separate uncommitted draft ADR-0040, ADR-0023 edits and core design-standard edits appeared in this shared tree during the checkpoint; preserve and reconcile them independently from this Stage 3 work. The last ADR index/lint predate that draft.

## Next

Build a cited normal-outcome witness for a compatible preceding source call, with unconditional self-recursion, finite base-before-recursion and disjoint-branch controls. Then extend it to non-call predecessors and SCC composition per the plan queue. At the eventual integrated exit, run the complete gate, fresh pilot and listed evaluation/serving checks; do not promote the present assumption to a measured result.
