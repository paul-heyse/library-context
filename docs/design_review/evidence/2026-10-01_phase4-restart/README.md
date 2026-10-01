# Phase 4 restart checkpoint — 2026-10-01

This folder preserves selected existing receipts because the operator requested a restart checkpoint.
It creates no new qualification gate. The [Phase 4 plan §13.3](../../../plans/semantic-model-phase4-detailed-plan_2026-09-30.md#133-restart-checkpoint-and-remaining-work)
owns the resume order; the [behavioral review](../../reviews/design_review_phase4-behavior_2026-10-01.md)
retains the open P4B3-F01 evidence. All worker changes are integrated on main through `18c3809`.
Raw logs are Git LFS artifacts. Older receipts qualify their named scope/tree, not this complete checkpoint.

Commands below use `python3 scripts/build_environment.py --` before Cargo unless stated otherwise.

| Receipt | Outcome and boundary |
|---|---|
| [Final merged compile](raw/phase4-restart-final-check1.log) | **passed**: `cargo check -p lctx --tests`, 59.72s, after `18c3809`. Includes F0 and final normalized metadata/refutation integration. The unregistered MDX draft is not compiled. |
| [F0 merged compile](raw/phase4-restart-integration-check1.log) | **passed**: `cargo check -p lctx --tests`, 36.48s, at `37cb4d9`. |
| [Worker F0 compile](raw/phase4-frontiers-check.log) | **passed** worker development-profile check; superseded for compilation by the merged check above. |
| [F0 release build](raw/phase4-frontiers-native.log), [isolated retry](raw/phase4-frontiers-native-isolated.log) | First **failed** with missing `invoked.timestamp` build path (exit 101); retry stopped for operator restart (exit 130), runtime **not_run**. Targets were `cargo test --release -p lctx --test compile_facts binary_publishes_upper_frontiers_with_seedless_catalog_and_explicit_outcomes -- --nocapture`; isolated retry used worker-local target/build directories. |
| [Summary run 8](raw/phase4-summary-publication8.log) | **failed**: `cargo test --release` targets `summary_publication`, `finite_summaries`, `read_channels`, `summary_consequences`. Both actual PG Summary cases failed (closed-reader declaration and aggregate/member qualification); three native controls passed. Production membership and test input-closure corrections are committed. |
| [Summary run 10](raw/phase4-summary-publication10.log) | Enclosing command **failed** after interruption: actual Catalog PG case passed; Behavioral depth-eight case interrupted during expensive replay, not qualified. Companion `transfer_composition` target passed 12. |
| [Summary run 11](raw/phase4-summary-publication11.log) | **failed** during compilation: missing output `.rcgu.o` path. `cargo test --release -p cpg-core --test summary_publication -- --nocapture` did not reach runtime. |
| [Summary run 12](raw/phase4-summary-publication12.log) | Same focused Summary command stopped during rebuild for operator restart; runtime **not_run**. No semantic failure inferred from interruption. |
| [Cumulative schedules](raw/phase4-final-schedules2.log) | **passed**: `cargo test --release -p lctx-model --test analysis_schedule --test upper_frontier_closures`, two static controls; no provider qualification. |
| [Final frontier model](raw/phase4-final-model-tests1.log) | **passed**: release `domain::analysis::frontier` filter, three pure controls. Other filtered integration binaries ran zero tests. |
| [Native final frontier](raw/phase4-final-native-root1.log) | **passed**: `cargo test --release -p cpg-extract --test final_coverage`, one captured-native control. |
| [Authored patterns compile](raw/phase4-authored-code-root-check1.log), [pure controls](raw/s0-authored-pattern-pure2.log) | **passed**: root `cargo check -p cpg-core --lib`; worker `cargo test --release -p lctx-model --lib domain::synthesis`, 47 controls. These precede final negative-kind checkpoint. |
| [Summary S0 controls](raw/s0-summary-consumer-pure4.log) | **passed** worker release `domain::synthesis` filter, 44 pure controls; no final S0/E0 publication qualification. |
| [B1 native](raw/phase4-field-read-qualified-native.log), [B1 PG](raw/phase4-field-read-qualified-pg.log) | **passed**: `cargo test --release -p cpg-extract --test read_channels` (3), `cargo test --release -p cpg-core --test base_execution field_read_screen` (2 actual PG). |
| [Partial F01 compile](raw/phase4-symbolic-check4.log), [native inventory](raw/phase4-symbolic-native-shape1.log) | **passed** worker `cargo check -p lctx-model --release` with worker-local target/build directories; `cargo test --release -p cpg-extract --test symbolic_fields -- --nocapture` (one diagnostic capture). Record admission still refuses; this is not association qualification. |
| [Runtime prerequisites](raw/phase4-runtime-integration5.log) | **passed**: `cargo test --release --no-fail-fast -p cpg-core --test catalog_evidence --test catalog_selection --test model_publication --test structural -- --nocapture`, seven actual PG controls. |
| [Analytic report](raw/phase4-analytic-report1.log) | **passed**: `cargo test --release -p cpg-core --test analytic selected_algorithms_publish_actual_nominal_results -- --nocapture`, one actual PG control. |
| [X0 candidate inventory](raw/x0-candidate-inventory.txt) | Read-only inventory of 49 candidate legacy paths. Deletion **not_run**. Re-inventory and preserve concurrent edits before retirement; this file is not deletion authority. |

The operator reported clearing disk space during the missing-build-path failures. This is consistent
with their diagnostics, not evidence of a semantic regression or proof that all cache issues are fixed.
The final Summary test fixture uses depth two, retaining intact, permutation and nine mutation controls;
production defaults are unchanged. That corrected fixture has not completed actual Behavioral PG runtime.

`just test-all`, full Phase 4 acceptance, live embedding, pilots and performance measurements are **not_run**.
All build jobs launched for this work were stopped or completed before handoff. Agent worktrees and caches
remain preserved; there is no need to depend on `/tmp` or merge their copied private prerequisites.
