# Status

_Updated 2026-10-05 under the [handoff skill](.claude/skills/handoff/SKILL.md); shared main._

**Graph-native complete compiler stage: in_progress.** ADR-0128 adopts the hard pivot grounded in
only the two named [target](docs/design_review/reviews/design_review_graph-native-target_2026-10-05.md)
and [SurrealDB capability](docs/design_review/reviews/design_review_surrealdb-capabilities_2026-10-05.md)
reviews. The [coordinator §8](docs/plans/graph-native-pivot-plan_2026-10-05.md#8-current-checkpoint)
owns package state, commands and current limits; its §7 owns finding disposition.

`cpg-core` now compiles store-free completed IPC inputs; `lctx-model` owns typed graph mappings,
nominal references and explicit semantic policies. Graph artifact admission/export and all-frontier
integration remain under verification. Native extraction/effects have focused passing controls;
the upper suites are being rerun after logical Arrow decode and optional-profile premise repairs.
A compile check or worker assignment is not completed compiler acceptance.

`lctx compile --artifact-only --output DIR` is the integrated artifact route. Ordinary compile
returns unavailable before acquisition until the native publisher exists. PostgreSQL backend,
binding, lifecycle commands and old serving effects are retired; MCP startup/dispatch are
unavailable until native serving is implemented. Operator databases and registrations are untouched.

Native SurrealDB querying is selected. The design principles and efficient-architecture companion
guide qualitative first-principles/library-informed choices. No detailed examined-work accounting,
performance proof or query-adoption gate is required. No Measured performance claim is made.

**Current verification boundary, 2026-10-05:** focused extraction controls passed (22), then native
semantic/effect/runtime controls passed (27). Launcher/build-environment controls passed (20).
The first extraction+upper selection failed:26 passed,18 failed; its upper failures are repaired
or still being rerun, as detailed by the coordinator. Artifact/profile/frontier controls and final
scope-end leaves are pending. Previous PostgreSQL receipts establish no graph-native acceptance.

**Next:** complete finite graph closure and model-runtime retirement, pass store-free upper/artifact
and CLI controls, run applicable leaves, and finish root `just turn-end`. Native realization,
publication, native serving and assembled `just qualify` follow this stage. Real-library/live-vector
qualification and operator adoption remain separately authorized Q1 work; they are not_run.

Gold/heldout inputs, live embedding services and client registration remain unchanged. Preserve
concurrent edits. No broad Cargo cleanup or operator-store action is part of this stage.
