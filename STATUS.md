# Status

_Updated 2026-09-26 under the [handoff skill](.claude/skills/handoff/SKILL.md); work is on `main`._

## Increment and slice

- **Increment 4, Stage 3 is functionally incomplete.** The [forward plan](docs/plans/behavioral-model-forward-plan_2026-09-24.md) owns the execution order (§3) and W1–W16 finding disposition (§6). The library-fit reviews are dated evidence, not authority.
- Remediation landed with focused evidence: W1 typed native IPC, W2 facet values, W3 served support, W4 builtin access, W6 bounded condition decisions/catalog admission, W7 value-reach fixed point, W8 hermetic corpus identity, W9 canonical cache fill, W11 finite primitive reasoning, W13 iterative SCC and type-term closure, W14 analyzer/run-context/suppression corrections, W16 controlled vLLM launch. W15 [ADR-0048](docs/adr/0048-schema-rebuild-policy.md) selects fresh current-store rebuild; no historical binary reader is planned.
- The order-1 finite producer checks source-ordered earlier calls for direct, modeled, assignment and local returns; L2 admits ordered pass-only finalizer suites. [ADR-0054](docs/adr/0054-origin-specific-summary-identity.md) gives each unaggregated value contribution a checked stable origin id through summary, Delta and native boundary. W5's single real two-origin Delta/native trace and work/node-limit traces remain open.
- [ADR-0053](docs/adr/0053-bounded-scc-summary-worklist.md)'s SCC-local value worklist admits unconditional finite-base paths through self/mutual recursion, with origin-specific depth and append-only `summary_pair_work_limit` boundaries (code 25). The [scoped review](docs/design_review/reviews/design_review_recursive-value-worklist_2026-09-26.md) and plan W12 state the limit: a real conditional recursive base reaches the local seed but remains unknown until cross-invocation argument/atom substitution is proved. Other summary channels and discharge remain open.

## Last verified (2026-09-26)

| Command | Outcome |
|---|---|
| `cargo test -p lctx-analytics --lib summaries::finite --quiet`; `cargo clippy -p lctx-analytics --lib --quiet -- -D warnings` | `passed`: 12 pure finite controls, including recursive finite-base/no-base, parallel/open origin, shuffle and low cap; targeted library Clippy. |
| `INSTA_UPDATE=no cargo test -p cpg-schema --test codebooks --quiet`; `INSTA_UPDATE=no cargo test -p cpg-schema --test contracts --quiet`; `uv run --no-sync pytest python/lctx_mcp/tests/test_native_semantics.py::test_native_open_boundaries_keep_sibling_origin_ids -q` | `passed`: reviewed append-only code 25 and generated-rule snapshots; synthetic native origin/reason admission after rebuilding the editable extension. |
| `RUST_MIN_STACK=16777216 INSTA_UPDATE=no cargo test -p cpg-core --test compile nested_returns_need_an_uncontrolled_exit_before_becoming_value_summaries --quiet` | `passed`: a real conditional self-recursive source produces a local candidate but no unjustified recursive positive; publication validation runs. |
| `RUST_MIN_STACK=16777216 INSTA_UPDATE=no cargo test -p cpg-core --test compile pinned_identity_models_require_and_publish_their_real_formals --quiet` | `passed`: existing modeled and local positive paths still publish after the worklist change. |
| `cargo test -p cpg-schema --test ids --quiet`; `INSTA_UPDATE=no cargo test -p cpg-schema --test contracts --quiet` | `passed` earlier on 2026-09-26: ADR-0054 origin recipe and ten schema contracts. |
| `RUST_MIN_STACK=16777216 INSTA_UPDATE=no cargo test -p cpg-core --test bundle finite_depth_and_unsupported_refusals_reach_the_native_response --quiet`; `uv run --no-sync pytest python/lctx_mcp/tests/test_native_semantics.py python/lctx_mcp/tests/test_value_paths.py -q`; `just py-fixture` | `passed` earlier on 2026-09-26: focused current FORMAT 8 native boundary and real fixture, not recursive positive. |
| `just docs-test`; `just docs-check`; `git diff --check` | `passed`: 38 docs tests, 95 canonical pages, no link errors or whitespace defects. |
| `just fmt`; `just test-all`; fresh `just pilot`; all-techniques digest; Q01/Q03/Q05/Q09; structured evaluation; clean-wheel query | `not_run`: defer until the entirety of planned functional scope is implemented. |

## Open boundary

- The 2026-09-25 `just test-all` attempt stopped after two stale Python tool-list tests, later corrected by a focused check; no complete rerun exists. A schema/query digest repin may be needed at integrated acceptance. Focused green results are not integrated qualification.
- Plan orders 1–9 remain partial: general argument and predecessor evaluation, exit/fate semantics, fully composed proofs, model families, conditional recursive BDD/argument substitution and other channels, claim discharge, independent oracle challenge, and complete FORMAT 8 serving. W7's production-cap native trace, W9 live embedding replay, W11 operation-wide Q09 and W12 pilot cost remain open. W10 and W14/F15 have plan-owned deferred triggers.
- A BDD cap, open call, CrossHair/Pysa silence or fake embedder never establishes a negative/live result. The Stage 3 pilot, end-to-end native query and integrated tests must follow functional completion.

## Next

Finish W12's real terminating recursive path with exact argument substitution and condition publication, then extend value/transform/effect/exception/role composition and discharge in plan order. In parallel with that scope, close W5's remaining Delta/native cause traces. Run only targeted checks until all functional work is implemented.
