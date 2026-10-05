# Status

_Updated 2026-10-05 under the [handoff skill](.claude/skills/handoff/SKILL.md); shared main._

**Qualitative execution-fit clarification (Implemented guidance, 2026-10-05).**
Core, review/planning skills and agent instructions assess tradeoffs in plain language; no implied
cost models, runtime accounting or extra proof machinery. Quantitative claims still need measurement.
`just docs-check` **passed** (315 pages, zero link errors); `just lint-agents` **passed**.
Product qualification **not_run** for this documentation change; heuristic adoption is separate.

**Execution-fit principles adopted (Implemented / Accept scoped, 2026-10-05; ADR-0127).**
Core/template 3.3 and CI 1.4 add FP-07/A4 with twelve architectural lenses; independent semantic
and evidence gates remain. The [adoption assessment](docs/design_review/reviews/design_review_architectural-criteria_2026-10-05.md#10-standard-adoption-and-independent-assessment--2026-10-05)
closes standard findings F01–F04 across five active repositories and the template.
Next: record and plan the graph replacement. Product/store behavior is unchanged.
The [capability assessment](docs/design_review/reviews/design_review_surrealdb-capabilities_2026-10-05.md)
recommends native functions, graph/search composition, sealed realizations and exports; it owns F01–F03.
The [target review](docs/design_review/reviews/design_review_graph-native-target_2026-10-05.md) recommends
hard pivot now, without compatibility or intermediate PG redesign. Both remain Proposed / Accept scoped.
The [earlier review §11](docs/design_review/reviews/design_review_surrealdb-graph-store_2026-10-05.md#11-authority-changes-and-dispositions-slot-11)
owns its F01–F14 until the replacement plan absorbs them. Production code and stores are unchanged.

**First real-library catalog compile failed (F01).** The authorized FastMCP 4.0.5 behavioral run
through catalog (all analytics, fake embedder) went as follows:
- Extraction completed in about 28 min.
- About 1 h of store-side reference anti-joins followed.
- PostgreSQL then terminated the lifecycle connection for idle-in-transaction at 10:09 EDT.
- The attempt ended "cleanup unconfirmed".

Interrupted generation `786cd6d58dc5dccea686c0a54a8f0dcd` (facts layer only, about 16.6M rows,
staging) still exists; it is disposable under the pivot direction. This review did not remove it.

**Testing, validation and verification pivot implemented / Tested; assembled qualification composite passed.**
The [assurance coordinator](docs/plans/testing-architecture-pivot-plan_2026-10-04.md#7-current-contractcontrol-map-and-execution-checkpoint)
owns the current contract/control map and closed F01–F04. ADR-0126 replaces broad gate cadence
and routine privileged-tamper checking with explicit contract families, owned immutable proof
reuse and independent read-only audit. Supporting validation/verification plans own detailed scope.

`lctx-model` owns each semantic definition, revision, references and complete ordered premises.
PostgreSQL acknowledgements bind exact installation/generation/model/layout/configuration and
immutable frames/prefixes. Proof hits still require current admission; failed, partial, interrupted
or unconfirmed execution supplies no hit. Validation sessions share compatible streams and bounded
preparation; all checks finish before acknowledgement. Writer draining/revocation and exact prefix
meaning remain required. `lctx generation audit` challenges bodies, references and proof semantics
without repair, grants or new acknowledgements.

Seven explicit `verify-*` families prepare only their declared tools/adapters/store/CLI closure;
`qualify` prepares their union once and collects independent family/leaf failures. Read-only serving
cases share an immutable seed and fresh request state; mutation cases stay isolated. Obsolete oracle
claims/evidence, duplicate replay factories and legacy broad aliases are removed. No persistent
assertion-pass cache, compatibility receipt readers or historical runtime archive is added.

## Last verified — 2026-10-05

**Policy adoption:** `just docs-check`, `just lint-agents`, `just adr-lint` **passed** (310 pages, zero link errors); independent review Accept scoped.
`just qualify`, real-library compilation and benchmarks **not_run** (assessment-only scope).

**Earlier design-review session (store, compile and docs only; no product code changed):**

| Command / boundary | Outcome |
|---|---|
| `target/release/lctx store check` (default store, before the compile) | **failed**: installed from a different model/lowering; orphan schema of an earlier generation |
| `target/release/lctx store reset --confirm lctx`, then `store check` | **passed**: model `0ea88254…` installed; 0 generations, 0 findings |
| `target/release/lctx compile fastmcp --through catalog --profile behavioral --techniques +communities,+pagerank,+fca,+rca,+knn,+type-layer,+mention-layer,+knn-layer --embedder fake` | **failed** (F01); log `build/review-compile-catalog-behavioral_2026-10-05.log` |
| Phase B probes on synthetic data and the retained facts layer | Outcomes in review §10.2 and the four `docs/design_review/evidence/2026-10-05_*` folders; catalog journeys and analytics parity **blocked** by F01 |
| `just turn-end`; `just docs-check` | **passed**; 306 pages, 0 link errors |
| `just qualify`, Clippy, product families | **not_run**: documentation and review scope only |

**Assurance pivot (earlier on 2026-10-05):**

Commands use `python3 scripts/build_environment.py -- …` or the normalized shell environment.
The deliberate `NEXTEST_TEST_THREADS=8 just qualify` initially failed; affected repair reruns
completed every required family/leaf. This is a **composite pass**, not an initially clean gate.
The coordinator records detailed commands, initial failures, repairs and their bounded receipts.

| Command / boundary | Outcome |
|---|---|
| `just qualify`: analytics, independent raw-flow/served oracles, tooling and compile-fail docs | **passed** within the assembled run; actual served oracle passed again after Corpus repair |
| `just verify-model` plus affected serving-contract target | **composite passed**, original family623; mandatory-domain refusals retained |
| Provider family and repaired empty-name control | **composite passed**, original87; later empty Flow library/shape/capture controls43 passed |
| `just verify-store --command rust` / `--command python` | **passed**, 59 actual disposable PostgreSQL and two Python controls |
| Definition/vocabulary/checkpoint premise controls | **passed**, 29; exact authority, frozen empties, rollback and missing-premise refusals |
| Producer structural/selection family | **composite passed**, 18; eight affected controls passed after typed fixture/premise repairs |
| `just verify-serving --command python` | **passed**, 25 MCP controls with rebuilt/imported native adapters |
| `just verify-serving --command serving` plus affected qualification/description reruns | **composite passed**, 22; actual admission, cohort, evidence/packets, native both-profile/startup/transport and conditional/generated/Terminal/raised journeys |
| `cargo nextest run --release -p lctx --test serving_qualification --test model_describe --no-fail-fast --no-tests=fail` | Qualification **passed**, actual Catalog/Behavioral stdio and received corruption challenges; deliberate digest update reviewed/accepted, separate `INSTA_UPDATE=no --test model_describe` **passed** |
| Corpus correspondence and normalization/analytics/definition controls | **passed**, 39; original contexts, links, ambiguity and document ownership counterexamples |
| Structural/handoff/closure/schedule/definition controls | **composite passed**, 32; complete Structural target8 passed after new fixture repairs |
| Model-domain library/expected-domain/schedule/definition controls | **passed**, 17 after correcting the new fixture's expected Frontier refusal; roles/foreign inputs/missing artifacts/resource limits retained |
| Actual PG compatible-stream/proof-reuse/audit receipt control | **passed**, one after named receipt-row harness repair |
| Scheduling/readiness controls | **passed**, 20; mocked scheduling controls do not certify actual families |
| `UV_NO_SYNC=1 just clippy` | **passed**, full release workspace/all-targets/keep-going, warnings denied, after final production repair |
| Applicable Python type/lint, agent/ADR/docs, fixture/gold/rules/dependency leaves | **passed** within the composite receipt; final `just docs-check` passed300 pages with zero link errors |
| Scope-end `just turn-end`, rebuilt adapters/imports and post-format `INSTA_UPDATE=no --test model_describe` | **passed**; reviewed digest-only change, schema/definition/premise agreement retained |
| Comparable timings / total RSS | **not_run**; setup/scan/replay counters establish no Measured speed claim |
| Real-library reconstruction/activation, live vectors, product comparisons and heldout | **not_run / stopped**, outside this pivot |

Empty document-only inputs now propagate admitted NoScope through Flow, Local, Structural Controls
and Model production/replay. Catalog targets do not create missing Python evidence in a domain with
no requested Python roots. Nonempty partial outcomes, NotRequested, budgets and refusals remain.
Independent scoped source reviews found no further material production defect; runtime commands
establish the Tested boundaries.

## Next and limits

The [target coordinator](docs/plans/target-implementation-alignment-plan_2026-10-04.md#8-current-q0-composite-qualification-receipt)
owns closed Q0 fixture acceptance and TA-F/LL-F disposition. The
[enrichment coordinator](docs/plans/code-facts-analytical-enrichment-plan_2026-10-04.md#7-current-disposition-and-checkpoint)
retains O08 and open Q1 before/after packet-benefit evidence; correctness controls do not certify
that product-benefit set. Root scope-end formatting/generation uses `just turn-end`; conservative
raw-source identity changes require affected description/native agreement, not an unchanged full
functional campaign.

**Phase 5 real-library activation remains stopped at the operator's review pivot.**
[Phase 5 §10](docs/plans/semantic-model-phase5-detailed-plan_2026-10-01.md#10-finding-routes-limits-and-current-state)
owns activation/serving limits.

The default store was reset and reinstalled with the current model on 2026-10-05 for the
authorized review compile. It now holds only the interrupted generation above; nothing is published
or selected. Client registrations, live vectors, gold and heldout are untouched. Real-library
upper-frontier compilation is blocked on F01.

The three clean integrated assurance worktrees were removed; the two older target-alignment trees
retain their current consumers. Logs/caches and unrelated concurrent work are preserved. The operator
cleared approximately560 GiB with `cargo clean`; the agent ran no clean or broad reset. Cargo jobs16,
frontend1, release profile and Nextest8 remain unchanged.
