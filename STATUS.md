# Status

_Updated 2026-09-26 under the [handoff skill](.claude/skills/handoff/SKILL.md); work is on `main`._

## Increment and slice

- **Increment 4, Stage 3 is functionally incomplete.** The [forward plan](docs/plans/behavioral-model-forward-plan_2026-09-24.md) owns the execution order (§3) and W1–W16 finding disposition (§6). The library-fit reviews are dated evidence, not authority.
- Remediation landed with focused evidence: W1 typed native IPC, W2 facet values, W3 served support, W4 builtin access, W6 bounded condition decisions/catalog admission, W7 value-reach fixed point, W8 hermetic corpus identity, W9 canonical cache fill, W11 finite primitive reasoning, W13 iterative SCC and type-term closure, W14 analyzer/run-context/suppression corrections, W16 controlled vLLM launch. W15 [ADR-0048](docs/adr/0048-schema-rebuild-policy.md) selects fresh current-store rebuild; no historical binary reader is planned.
- The order-1 finite producer checks source-ordered earlier calls for direct, modeled, assignment and local returns; L2 admits ordered pass-only finalizer suites. [ADR-0054](docs/adr/0054-origin-specific-summary-identity.md) gives each unaggregated value contribution a checked stable origin id through summary, Delta and native boundary. A real same-fact two-origin source now retains a proved value and a `call_transfer` unknown through one Delta/native generation; W5's work/node-limit traces remain open.
- [ADR-0053](docs/adr/0053-bounded-scc-summary-worklist.md)'s SCC-local value worklist admits finite-base paths through self/mutual recursion, with origin-specific depth and append-only `summary_pair_work_limit` boundaries (code 25). An exact second boolean literal specializes one conditional callee guard through a cited entry-value link and bounded BDD restriction; a real true-literal recursive positive and false-literal withholding control reach Delta/native. The [scoped review](docs/design_review/reviews/design_review_recursive-literal-control_2026-09-26.md) and plan W12 state the remaining general substitution, other channel and cap-trace limits.

## Last verified (2026-09-26)

| Command | Outcome |
|---|---|
| `cargo test -p lctx-analytics --lib summaries::finite --quiet`; `cargo clippy -p lctx-analytics --lib --quiet -- -D warnings` | `passed`: 13 pure finite controls, including true/false exact-literal recursion, base-free, parallel/open origin, shuffle and low cap; targeted library Clippy. |
| `INSTA_UPDATE=no cargo test -p cpg-schema --test codebooks --quiet`; `INSTA_UPDATE=no cargo test -p cpg-schema --test contracts --quiet`; `cargo clippy -p cpg-schema --lib --quiet -- -D warnings` | `passed`: reviewed append-only step code 14 and boundary code 25, generated-rule snapshots, and targeted schema Clippy. |
| `RUST_MIN_STACK=16777216 INSTA_UPDATE=no cargo test -p cpg-core --test compile nested_returns_need_an_uncontrolled_exit_before_becoming_value_summaries --quiet` | `passed`: a real true-literal recursive path cites the guard link; a false-literal recursive path and same-value self-call stay unknown; publication validation runs. |
| `RUST_MIN_STACK=16777216 INSTA_UPDATE=no cargo test -p cpg-core --test compile pinned_identity_models_require_and_publish_their_real_formals --quiet` | `passed`: existing modeled and local positive paths still publish after the worklist change. |
| `cargo test -p cpg-schema --test ids --quiet`; `INSTA_UPDATE=no cargo test -p cpg-schema --test contracts --quiet` | `passed` earlier on 2026-09-26: ADR-0054 origin recipe and ten schema contracts. |
| `RUST_MIN_STACK=16777216 INSTA_UPDATE=no cargo test -p cpg-core --test bundle finite_depth_and_unsupported_refusals_reach_the_native_response --quiet`; `uv run --no-sync pytest python/lctx_mcp/tests/test_native_semantics.py::test_native_index_refuses_missing_proof_steps -q` | `passed`: the real true-literal recursive proof reaches native; a false-literal recursive path stays unknown; shared validation rejects link removal and native rejects a missing conditional link. The same bundle test traces distinct proved/open origins on one raw return fact through Delta/native. |
| `RUST_MIN_STACK=16777216 INSTA_UPDATE=no cargo test -p cpg-core --test bundle finalizer_proof_round_trips_through_the_native_generation_reader --quiet` | `passed`: companion focused native proof script uses the current paged API. |
| `cargo clippy -p cpg-core --test bundle --quiet -- -D warnings` | `passed` after replacing a pre-existing unnecessary lazy `bool::then` use in `flow_model.rs`. |
| `just docs-test`; `just docs-check`; `git diff --check` | `passed`: 38 docs tests, 96 canonical pages, no link errors or whitespace defects. |
| `just fmt`; `just test-all`; fresh `just pilot`; all-techniques digest; Q01/Q03/Q05/Q09; structured evaluation; clean-wheel query | `not_run`: defer until the entirety of planned functional scope is implemented. |

## Open boundary

- The 2026-09-25 `just test-all` attempt stopped after two stale Python tool-list tests, later corrected by a focused check; no complete rerun exists. A schema/query digest repin may be needed at integrated acceptance. Focused green results are not integrated qualification.
- Plan orders 1–9 remain partial: general argument and predecessor evaluation, exit/fate semantics, fully composed proofs, model families, conditional recursive BDD/argument substitution and other channels, claim discharge, independent oracle challenge, and complete FORMAT 8 serving. W7's production-cap native trace, W9 live embedding replay, W11 operation-wide Q09 and W12 pilot cost remain open. W10 and W14/F15 have plan-owned deferred triggers.
- A BDD cap, open call, CrossHair/Pysa silence or fake embedder never establishes a negative/live result. The Stage 3 pilot, end-to-end native query and integrated tests must follow functional completion.

## Next

Extend W12 from the exact literal-controlled recursive case to bounded general argument and condition composition, then value/transform/effect/exception/role paths and discharge in plan order. Close W5's remaining work/node-cap Delta/native cause traces alongside that scope. Run only targeted checks until all functional work is implemented.
