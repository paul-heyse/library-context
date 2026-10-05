# Testing architecture pivot — coordinated implementation plan

**Implemented / Tested; assembled qualification composite passed, 2026-10-05.** This plan schedules the full testing, validation and oracle architecture
pivot requested after the [source review](../design_review/reviews/design_review_test-validation-oracle-architecture_2026-10-04.md).
It owns the combined target, execution dependencies and sole current disposition of F01–F04.
The [validation plan](validation-execution-pivot-plan_2026-10-04.md) owns runtime validation design;
the [verification plan](verification-execution-pivot-plan_2026-10-04.md) owns test execution design.
The current implementation and receipts below distinguish focused controls from assembled acceptance.

## 1. Purpose and authority

Replace assurance organized around historical stages, duplicate replays and repeated complete
fixtures with assurance organized around current contracts and the mechanisms that can violate
them. Reduce the amount of validation required through structural guarantees, execute each remaining
obligation over its exact inputs once where possible, and reuse acknowledged conclusions over owned
immutable state. Make verification preparation proportional to the boundary exercised.

All existing test scope is eligible for replacement or deletion. A passing historical suite is
neither a prerequisite nor a retention reason. Existing test names, snapshots, campaigns, generic
gates and evidence folders do not define the required target. Surviving product guarantees do:
nominal identity, complete declared premises, precise uncertainty, bounded execution, honest partial
outcomes, lifecycle safety and served fidelity. Changing a guarantee requires an explicit decision
and its architectural owner; removing a redundant implementation of it does not.

The applicable standard is Core 3.2 with code-intelligence 1.3 and the
[repository binding](../design_review/design_principles/binding/library-context.md). This plan's
foundation assessment used current source and focused independent advice at `daca90cf`, 2026-10-04.
It is **Interface-checked** within those inspected boundaries, not an enclosing certification.
Architectural ownership remains with [semantic model §15](../design/sections/semantic-model.md),
[storage](../design/sections/storage-and-publication.md),
[validation/evaluation](../design/sections/validation-and-evaluation.md) and their decisions.
ADR-0126 accepts the pivot; these plans specify its implementation and acceptance boundary.

Q0 fixture acceptance is closed at its existing coordinator by the composite receipt below. Q1
packet-benefit evidence and Phase 5 real-library acceptance remain at their existing coordinators.
Their surviving obligations are mapped to replacement controls during P0/P5.
Cancelled compilation and older focused receipts do not become qualification. Real-library
activation, evaluation comparisons and opening heldout data remain outside this scope.

## 2. Foundation assessment and selected target

Model declarations already own schemas, input closure, invariant logic and stage requirements.
PostgreSQL already owns publication locks, importer grant revocation, immutable completed outputs,
append-only vocabulary epochs and receipt effects. Those are suitable foundations; their current
registration and execution paths need consolidation. Cargo/uv already cache build artifacts, and
the environment wrapper fingerprints native-adapter source membership/content. No generic impact
engine or persistent test-pass cache is needed.

| Question | Selected boundary and reason | Consequence |
|---|---|---|
| Who defines an assurance obligation? | The component owning the meaning. Model semantic obligations have one model-owned definition and explicit relation/stage references. | Complete input closure and distinct bindings replace repeated callback factories or a single accidental relation anchor. |
| What needs runtime checking? | Facts not already guaranteed by construction and physical enforcement at the actual ingestion boundary. | Remove repeated null/type/key checks only after the boundary closes all bypasses; retain independent lowering challenges. |
| When is semantic validation reusable? | After successful acknowledgement over exact owned immutable inputs, including vocabulary prefix and semantic configuration. | Publication establishes the conclusion; later reads check its authority and current admission without replaying the same question. |
| What detects privileged mutation? | Explicit operator audit/repair, outside routine immutable-state read admission. | Reconsider old superuser-tamper controls; migrate relevant ones to audit and delete excluded promises through an ADR/owner change. |
| Where is physical work shared? | Within a stable validation session, with bounded buffers and compatible ordering. | Digest calculation and several checks may share a stream; incompatible ordering or excessive retained state uses separate scans. |
| What is shared by tests? | Immutable inputs and expensive preparation with an explicit lifetime. | Fresh leases, guards, request state and mutation isolation remain per case. No cached assertion success. |
| What determines a check run? | Changed contract and affected consumers, selected explicitly by the implementer. | Small named family commands and one deliberate assembled qualification replace unconditional broad preparation/gates. |

The trust choice is material: routine consumers will trust store-owned acknowledged immutability,
rather than promise to rediscover arbitrary administrative row mutation on each read. Publication
must drain writers, freeze the acknowledged input, validate it and commit the proof as one owned
transition. A completed transaction or a receipt row alone does not establish this premise.
Administrative repair quiesces consumers and rebuilds affected state with a new installation or
generation identity. See validation V1/V4 for enforcement and audit obligations.

Rejected alternatives are retaining three replay names with callback/name deduplication; deleting
two relation attachments and relying on the third; adding a global cache beside receipts; accepting
READ COMMITTED as a stable preparation boundary; and memoizing test success over mutable effects.
These hide dependencies, retain redundant authorities or lack the needed stability premise.
READ COMMITTED is usable when locking/immutability establishes unchanged logical frames; its
transaction identity alone is not that premise. Always rehashing and replaying is defensible under a different tamper-detection contract, but is
not the selected routine-read target. If a legitimate runtime writer cannot be excluded, strengthen
its closure boundary before granting proof reuse for that input; do not silently assume immutability.

## 3. Shared contracts

An **obligation definition** supplies stable semantic identity/revision, exact required input roles
and the model-owned checking implementation. Relations and stage uses reference it. A **binding**
resolves a definition to exact generation sources, closed prefixes, model/lowering identity and
relevant semantic settings. Different prefixes or profiles are different questions. Definitions
and references participate in model identity; no callback address or incidental test name is a key.

An **acknowledged conclusion** records successful complete validation of a binding under the
selected ownership premise. Missing premises, failures, partial execution, cancellation and
unconfirmed commit never establish success. Existing receipts are the persistence owner; their
schema/meaning changes in the validation work, with a rebuild rather than compatibility readers.

A **prepared frame** is a session-owned view/stream of one exact physical input. It records stable
snapshot/locking premises, canonical ordering and charged lifetime. It is an execution facility,
not another semantic model. Resource admission remains current-request work even on a proof hit.
Reuse may avoid replay allocation/work, but cannot fabricate available memory or capability.

Verification controls identify the current contract they challenge and their expectation source.
Generated declarations test lowering agreement, not independent semantic truth. Same-provider CLI
parity tests the driver/configuration. Independent finite matrices, authored counterexamples and
bounded CPython observations challenge shared semantic mistakes. Product usefulness comparisons
remain separate from correctness and are not activated here.

## 4. Execution packages and readiness

The root integrator owns shared model interfaces, recipes, policy/ADR changes and acceptance.
Component executors own their production and harness changes. Parallel work is useful after shared
contracts are available; shared editing surfaces require explicit ownership even when logically
independent. No package requires an old-green baseline or a full gate after its slice.

| Package | Result and responsibility | Prerequisite | Local verification |
|---|---|---|---|
| P0 — Declare the current assurance contract | Root with model/store/verification owners writes the pivot ADR, updates owning design text, and maps current product obligations and open Q0/Q1/Phase 5 requirements into replacement controls. Distinguish structural, semantic, driver, independent, lifecycle and transport evidence. Retire obsolete claims/evidence immediately when their active obligations have a home. | This Proposed target; current owners and source review. | Static contradiction/consumer audit; docs checks. No legacy suite prerequisite. |
| P1 — Canonical obligation ownership | Validation V0/V2 supplies explicit definitions/references, complete closure and unique exact binding execution; migrates source-call replay and every declaration/stage/selection consumer of the changed interface. | P0 semantic contract; no immutable proof reuse needed yet. | Model compile and finite-model coverage/refusal controls. |
| P2 — Owned freeze and prepared validation | Validation V1/V3 establishes ordinary and vocabulary closure under drained writers and stable session inputs; bounded scan fanout and structural-check removal use that premise. | P0 trust decision; P1 interface for execution integration. | Real disposable PG writer/closure controls and deterministic counters; focused replay challenges. |
| P3 — Acknowledged result reuse and audit | Validation V4 replaces repeated production rehash/replay with exact proof lookup, current admission and explicit physical audit/repair. Migrates publication, read grants, selection, closure, final admission and strict consumers together. | P1 implemented binding and P2 verified freeze/preparation. | Hit/miss and dependency discrimination, interrupted commit, prefix stability, budget and audit controls. |
| P4 — Verification footprint and prerequisites | Verification T0–T3 consolidates contract families/cohorts, shares immutable seeds, scopes adapter preparation and independent oracles, deletes obsolete controls and old evidence. | P0 family/expectation map. Seed reuse requires P2/P3 only where it depends on the new immutable-store contract. | Focused family execution, isolation and readiness invalidation controls; no whole-workspace gate. |
| P5 — Policy and assembled acceptance | Verification T4/T5 integrates named commands, replaces broad cadence, reconciles Q0/Q1 and surviving Phase 5 fixture qualification, retires old gates and unused harnesses, and records target qualification. | P1–P4 functional migration complete; every surviving obligation has an implemented control. | One new assembled qualification, full keep-going Clippy and applicable non-functional checks on the same tree; repair and rerun failures. |
| P6 — Benefit characterization and closure | Integrator records structural/work reduction and bounded comparable timings where useful; moves enduring contracts to owners and retires plans/review when their last current obligations close. | P5 acceptance for completion; measurement timing can be independent when sources are stable. | See §6; measurements are optional for functional closure and required for Measured speed claims. |

P4's pure harness and truthful-oracle work can proceed alongside P1/P2. P3 depends on implemented
freeze behavior, not just a settled interface. Runtime migration and its revealing controls belong
in the same package; no final inventory phase may defer a consumer needed for soundness.
Until T4 changes policy, current commands remain current instructions. T4 changes the instruction,
recipe and qualification-owner text together before running replacement assembled acceptance;
there is no requirement to run the retired full suite first.

## 5. Findings and current checkpoint

**Sole disposition owner, 2026-10-05.** P0–P5 are complete within the selected fixture and assurance scope; assembled qualification composite passed below. Source findings
retain their original diagnosis and IDs; remedies are assessed against this selected target.

| Finding | Current disposition | Responsible component and closure evidence |
|---|---|---|
| F01 | Closed / Tested — P1/P5, 2026-10-05 | `lctx-model` declarations and stage consumers. One source-call definition executes once per exact binding, with coverage when any referring relation is used in a supported finite model; malformed headers/inventory/missing invocation still refuse. |
| F02 | Closed / Tested — P4/P5, 2026-10-05 | Verification/build-environment owners and affected serving/provider harnesses. Selected pure families require no adapters/PG; expensive read-only cohorts share one seed; mutation isolation and independent failure reporting pass. |
| F03 | Closed / Tested — P2/P3/P5, 2026-10-05 | `lctx-postgres::generations` with model bindings. Freeze, exact prefix/result reuse, current budget admission and audit controls pass; counters show eliminated equivalent replay/preparation. No speed claim from counters alone. |
| F04 | Closed / Tested — P0/P4/P5, 2026-10-05 | Validation/evaluation owner and oracle harness owners. Every current coverage claim names a live correctly classified control; obsolete Pysa/CrossHair evidence and dangling consumers are removed after active obligations move. |

This coordinator does not take O08 or TA-F/LL-F disposition from other coordinators. The
[analytics assurance owner](../design/sections/analytics.md#section-9-6) now owns K1/K2 meaning, bounded independent controls and production fit rationale; the analytical
enrichment coordinator still owns O08. The supporting document is retired. Existing historical
qualification links alone do not require retention.

P0/T0's accepted contract is ADR-0126. Current model/store/validation owners carry canonical
obligations, acknowledged immutability, explicit audit and evidence classification. The retired
Pysa/CrossHair folders and analytic supporting plan have been removed after surviving requirements
moved to validation/analytics. P1–P4 implementation is integrated: model-owned definitions, exact owned acknowledgements, bounded
stream groups, explicit audit, scoped preparation and read-only cohorts. The composite P5 receipt below establishes the selected model/store/native/MCP and assurance
boundaries. Optional timing and real-library acceptance are excluded.

## 6. Acceptance, measurements and retirement

Functional completion requires the whole selected architecture, not a smaller improvement to the
old gates: canonical coverage; enforceable immutable acknowledgement; bounded physical preparation;
exact result reuse; truthful independent evidence; isolated cohorts; scoped prerequisites; replacement
commands and policy; and deletion of superseded paths. Required new controls must actually run.
`not_run` or selected package receipts cannot close assembled acceptance by aggregation alone.
Expected refusals and known partial outcomes are successful controls only when their asserted
contract is satisfied. Missing Docker/native tooling is a named block, never a mocked pass.

One deliberate assembled run exercises all new families and representative production journeys
on the same source/dependency tree, including actual PostgreSQL, native/wire agreement and MCP
listing/calls. Full `cargo clippy --release --workspace --all-targets --keep-going -- -D warnings`
runs without fail fast after functional migration. Other non-functional checks follow T4's affected
surfaces; rerun failed checks and controls affected by repairs. No formatting or generators mid-work;
the root agent runs `just turn-end` at the end of a turn that changed files.

Expected efficiency improvements are **Proposed**: fewer independent compilation units sharing
large helpers, one expensive seed per compatible cohort, no unrelated native readiness, one replay
per exact obligation and fewer equivalent row scans. Record process/setup/scan/replay counts on
bounded fixture workloads. Optional timings compare identical source inputs and semantic outputs,
same release/toolchain, stable Cargo paths, documented cold/warm state and concurrency; separate
compilation, readiness, database setup, validation and assertions. Preserve resource/profile policy;
never use `cargo clean` on shared intermediates. Do not rerun an old full suite to create a baseline.
Missing optional timing is a limit, not a functional blocker or a Measured benefit.

Deletion is part of each migration: old factories, duplicate predicates, unsupported oracle claims,
obsolete evidence, legacy test targets/helpers and superseded commands go when the replacement
owns their surviving obligation. Rebuild regenerable receipts/state under the new model, quiescing
readers before replacing any project runtime state. No old-format readers, dual gates, rollback
assets or historical evidence archive are added. Unrelated concurrent edits and build caches remain
preserved. Any proposed new hook/register/document must have a concrete current consumer.

## 7. Current contract/control map and execution checkpoint

**Tested mapping, 2026-10-05; assembled qualification composite passed.**
This table maps surviving guarantees, including Q0 fixture, open Q1 and Phase 5 fixture obligations, to the
launcher selections in `scripts/verify.py`. Existing coordinators retain their finding IDs and
real-library stop; this is no second disposition register.

| Guarantee / expectation source | Required replacement family and concrete controls |
|---|---|
| Typed identity, complete premise/ref closure, nullable Named fields, append-only codebooks and wire/request resource meaning | Model family (`lctx-model`); reviewable CLI schema snapshot `lctx::model_describe` in the serving assembly. Snapshot changes require reviewed migration, never auto-update |
| Bounded finite analytics semantics, support and sound partial outcomes | Analytics family; `implication_oracle` retains direct incidence, independent concepts and odis consequences. O08/Q1 remain at enrichment coordinator |
| Current linked Ruff/ty/Pyrefly facts, source correspondence and retained Flow timing | Provider family: native `typed_flow`, `typed_calls`, `native_overload_origins`, `typed_ruff_context`, `harness` and Flow shapes/capture timing. Harness CLI comparison is parity |
| Actual role/schema lowering, acknowledgement, delayed/queued writers, immutable prefix, cancellation, exact proof hits/misses and explicit corruption audit | Store family: generation/stage/vocabulary/read/publication/lifecycle/catalog/install/analysis controls; isolated serving-shape drift uses the actual Catalog model in `lctx::serving_admission`, plus validation-session unit controls. Disposable PG installation checks the owned schema; default operator store is excluded |
| Producer-to-store structural/behavioral/facts/selection and original-packet fidelity, missing/empty/partial distinctions | Serving assembly includes `cpg-core` structural/frontier/selection/validation-view/facts-generation/admission targets plus current `lctx` packet/evidence/native/admission journeys |
| Remaining Q0/Q1 conditional/generated/Terminal/raised positive and incomplete packets | Serving assembly: `serving_packets`, `conditional_output_packets`, `terminal_question_packets`, `raised_type_selection`; isolated source/profile/seeds remain distinct |
| Actual native conversion, MCP listing/calls, metadata and resource bytes | Serving qualification and current actual-stdio callers; Python wire/envelope/unit controls run once, without separately re-running scripts those Cargo callers already invoke |
| Shared immutable preparation without shared request state | `serving_cohort::immutable_catalog_cohort`: default Catalog find/retrieval/mandatory-packet cases use one fixture, fresh executions and local ranking/cursors. Mutation/lifecycle/corruption cases stay isolated |
| Independent raw-flow and original served claims | Oracle family: actual CPython raw-flow comparison and Rust `serving_soundness` producer/store/served comparison. Its observation-bundle driver is not separately counted as served correctness |
| Readiness membership/content/lock/model/config changes, scope selection and honest failure collection | Tooling family `test_build_environment`/`test_verify`; native import/model agreement also requires actual provider/serving controls after scoped readiness |
| Rust authority boundaries and documentation/process meaning | Compile-fail docs and affected ADR/agent/docs/Python/dependency leaves, full keep-going Clippy at assembled scope completion |

**Composite qualification receipt, 2026-10-05:** all required families and applicable leaves have passing receipts from the deliberate assembled run and affected repair reruns. The initial `just qualify` failed; this is not an initially clean run. Repairs preserved refusal/partial assertions. Optional timings, live vectors, product comparisons, heldout and real-library activation are not_run.

**Commands and affected receipts:** normalized commands use
`python3 scripts/build_environment.py -- …` (or its `--shell` environment).

- **passed:** `uv run --no-sync pytest tests/scripts/test_verify.py tests/scripts/test_build_environment.py -q`, 20 launcher/readiness controls. Mocked launcher controls establish scheduling behavior, not actual family qualification.
- **passed:** `cargo test --release -p lctx-model --test validation_definitions --test validation_views --test dependency_closure --test analysis_sources --test analysis_expected --no-fail-fast`, 24 controls after explicit source-check selection repair.
- **passed:** `cargo check --release -p lctx-postgres -p lctx -p cpg-core --tests`, integrated model/store/CLI compile; subsequent focused controls compiled the newer session/audit changes.
- **composite passed:** actual disposable PG new proof reuse, delayed/queued writer and immutable prefix controls. The first run exposed two harness mistakes (prefix views hide introduction metadata; duplicate IDs collapse); corrected row/body assertions passed on rerun.
- **composite passed:** generation/publication/audit controls, four tests; lifecycle/catalog controls, 14 tests. Complete typed Catalog premises use the model-owned inventory and exact dependency closure in one five-producer Facts group. State-only conformance uses a separate minimal model. Earlier failures exposed omitted premises, unrequested Flow writers and a missing authored assumption set; checks/preflight were retained, fixtures corrected. Final held-empty relation acknowledgements now resolve through final receipts without producer-stage authority.
- **passed:** two final-empty-vocabulary/audit-frame controls; grouped vocabulary retains exact prefix identity, while held empty vocabulary without an epoch receipt uses its acknowledged final frame.
- **failed initially:** `NEXTEST_TEST_THREADS=8 just qualify`. Analytics, both oracles and tooling passed; provider/store/producer/serving and applicable leaves exposed the repairs below. T4 policy and legacy-alias deletion are integrated.
- **composite passed, 2026-10-05:** model family623 (621 initially passed; complete serving-contract target21 passed after fixing mandatory `domains` in its response fixture), targeted provider empty-name control1, store Rust family59 and scheduling/readiness controls20. Checkpoint acknowledgements retain frozen empty ordinary frames, independently of producer/source capabilities; rollback, writer exclusion, exact misses/hits and reservation release pass.
- **composite passed, 2026-10-05:** producer18. The repair rerun selected eight structural/selection controls and passed all eight, including the seven earlier failures. Catalog vocabulary closure now resolves real checkpoint-qualified empty premises; adversarial fixtures select the typed nominal subject rather than a physical sum column. Coordinated model/store planning also passed29 definition/vocabulary/checkpoint controls. Missing premises remain refusals, extra acknowledged inputs select no unrelated checks, and checkpoint-only frames resolve before candidate execution.
- **passed:** `NEXTEST_TEST_THREADS=8 just verify-serving --command python`, 25 MCP controls after actual native-adapter rebuild and MCP/semantics/storage imports. Ruff/types/agent/ADR leaves passed on the resumed tree.
- **passed:** the CLI description migration assertion within `NEXTEST_TEST_THREADS=8 just verify-serving --command serving`. The reviewed migration leaves all878 relation schemas unchanged; canonical definition metadata, explicit references, publication checks and the single replay replace the old description. The later Structural replay migration adds its mandatory Local coverage premise; all39 publication checks are unchanged. Latest `INSTA_UPDATE=no` description assertion passed after reviewing the digest-only update.
- **passed:** `NEXTEST_TEST_THREADS=8 just verify-store --command python`, two controls. A combined release qualification/served-oracle rerun passed the independent served oracle again; qualification remained failed.
- **composite passed:** serving assembly22. Its21 previously passing controls cover actual admission, immutable cohort, packets, native both-profile agreement, evidence and conditional/Terminal/raised journeys. The remaining `serving_qualification` control passed on the current source, including actual Catalog and Behavioral stdio and received generation/evidence/channel/default corruption challenges. `NEXTEST_TEST_THREADS=8 INSTA_UPDATE=new cargo nextest run --release -p lctx --test serving_qualification --test model_describe --no-fail-fast --no-tests=fail` passed qualification217.793s and deliberately emitted the description digest update; the reviewed snapshot was accepted and `INSTA_UPDATE=no` on `--test model_describe` passed1. No failed assertion is counted as passing.
- **passed:** document-only Corpus and Installed frames remain distinct. Normalization/analytics preserve exact links, original contexts, ambiguity and document/target ownership; stored provenance/link/context/artifact-use counterexamples pass. Empty Flow inventories return no modules. Local uses the admitted domain; Structural Controls uses its exact Local NoScope receipt. Model production/replay checks captured requested Python roots after predecessor/binding replay, so catalog target absence on a document-only frame no longer invents IncompleteCoverage. Nonempty Partial, NotRequested and budget/refusal contracts remain unchanged.

**Affected repair controls, 2026-10-05:** integrated `cargo check --release -p lctx-model -p lctx --tests` **passed**; release Nextest `lctx-model` targets `corpus_mentions`, `domain_normalized`, `domain_analytics`, `validation_definitions`, `dependency_closure`, `analysis_sources`, `analysis_expected` and `analysis_schedule` **passed**, 39 controls. Release `cpg-flow` library/`flow_shapes`/`capture_timing` **passed**, 43. Structural/handoff/closure/schedule/definition controls **composite passed**, 32 after correcting new fixture setup; complete Structural target8 passed on rerun. The actual PG compatible-stream/proof-reuse/audit control **passed** after naming the saved receipt row. `UV_NO_SYNC=1 just clippy` **passed**, full release workspace/all-targets/keep-going with warnings denied, after four harness repairs. The latest Model-domain library/expected-domain/schedule/definition rerun passed17 controls after correcting the new fixture's expected Frontier refusal; missing artifacts, foreign inputs, role selection and resource failure remain tested. Both native adapters rebuilt/imported successfully. Actual both-profile qualification and the reviewed description assertion passed. Full `UV_NO_SYNC=1 just clippy` passed again after the final production repair, workspace/all-targets/keep-going with warnings denied. Scoped static reviews found no material production defect; functional commands provide the Tested boundaries.


**Final artifact agreement, 2026-10-05 — passed.** `UV_NO_SYNC=1 just turn-end` passed;
Hakari reported no changes, Python formatting/fixes changed nothing, and inspected Rust changes
were reflow/import ordering. Formatting re-keyed the conservative source fingerprint only.
`uv sync --locked --inexact --package lctx-mcp` rebuilt both native adapters; actual MCP/semantics/
storage imports passed. Release `--test model_describe` rebuilt the current CLI, emitted only the
expected digest change, and passed with `INSTA_UPDATE=no` after exact snapshot review/acceptance.
All model schemas, definitions/references and premise lists match the qualified pre-format tree.
Full keep-going Clippy and functional receipts precede this formatting; no unchanged functional
campaign was repeated. `UV_NO_SYNC=1 just docs-check` passed300 canonical pages with zero link errors.

Publication checks share compatible ordered streams within a publication group; pure checks share
within their pure group. Both use the same prepared receipt session. Cross-kind row fanout is not
claimed: distinct finish/source contexts keep the two groups separate. Candidate ID digests join
compatible checker streams. Focused two-check controls assert one scan and exact reused questions
assert zero scans/executions. These work counts do not establish a timing benefit.

No Measured speed claim or real-library activation is made.
