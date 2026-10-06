# Status

_Updated 2026-10-05 under the [handoff skill](.claude/skills/handoff/SKILL.md); shared main._

**Graph-native model/compiler stage: complete / user-accepted, 2026-10-05.** The user explicitly
stopped further tests and accepted completion of M1/C1/C2-N/C2-U, compiler-side A1/A2 and their
G0/S1 interfaces. The stage is Implemented with scoped Tested evidence, not an all-checks-passed
or Measured claim. ADR-0128 grounds the hard pivot only in the named
[target](docs/design_review/reviews/design_review_graph-native-target_2026-10-05.md) and
[SurrealDB capability](docs/design_review/reviews/design_review_surrealdb-capabilities_2026-10-05.md)
reviews. The [coordinator §8](docs/plans/graph-native-pivot-plan_2026-10-05.md#8-current-checkpoint)
owns commands and limits; its §7 owns finding disposition.

`cpg-core` compiles store-free completed IPC inputs in a charged spillable workspace. `lctx-model`
owns typed graph mappings, nominal references, semantic policies and completed artifact admission.
All-frontier/profile artifact controls passed within the compiler run. Compiler-side projections,
selected analytics, embedding reuse and independent retrieval controls are integrated. Source
repairs from the scoped implementation review are accepted; its static judgment remains bounded.

`lctx compile --artifact-only --output DIR` exports admitted artifacts. Ordinary compile returns
unavailable before acquisition until native publication exists. PostgreSQL backend/binding,
lifecycle commands and old serving effects are retired; MCP remains unavailable until native
serving. The broad CLI model snapshot control and its baseline are retired; model introspection
and focused semantic controls remain. Operator databases and registrations are untouched.

**Verification, 2026-10-05:** `cargo check --locked -p cpg-core -p lctx-model -p lctx --tests`
**passed**; `just ready` **passed**; `just verify-model` **passed**, 648 controls;
`just verify-analytics` **passed**, 14 controls. `just verify-compiler --command producer`
**failed / interrupted by user direction**: 186 passed, one obsolete Catalog/Flow test premise
failed and one Summary control received SIGTERM. The obsolete assertion is repaired (`e9bcdd4a`),
rerun **not_run**. Summary completion on the final production baseline remains unverified.
The coordinator records exact scope and logs; the stopped control is not an observed production failure.

Queued CLI/provider checks and subsequent flow/oracle/tooling controls are **not_run**, stopped
before execution. Scope-end leaves and assembled `just qualify` are **not_run**, waived by the
explicit stop/completion instruction. `UV_NO_SYNC=1 just turn-end` bookkeeping **passed**
(ADR index, unchanged Hakari output and formatting); synchronization was disabled to avoid an
unnecessary native rebuild during formatting. No functional tests were restarted.
No earlier PostgreSQL receipt establishes graph-native operational acceptance.

**Next:** P1/P2 native realization/publication, then S2/S3 native serving and published exports,
remaining I1 integration and assembled Q0. Real-library/live-vector checks and operator adoption
remain separately authorized Q1 work, **not_run**. The complete operational pivot is still pending.
No further compiler-stage test or legacy snapshot work is scheduled by this completion.

Native SurrealDB querying is selected. Efficient design follows first principles and library
capabilities; detailed examined-work accounting, performance proofs and query-adoption gates are
not required. No Measured performance claim is made. Gold/heldout inputs, live embedding services
and client registrations are unchanged; preserve concurrent work and avoid broad Cargo cleanup.
