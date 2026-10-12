# SurrealDB lifecycle target implementation plan

**Proposed implementation track, 2026-10-12.** Plan creation is complete; production execution and acceptance are **not_run**. Source baseline: `1e26996dab8bf5dbcd6c89e135e523db7c170a90`; no production source changes separate it from the review baseline. This document owns the combined target, sequence, current disposition and receipts for this track.

## 1. Purpose, authority and reading route

Implement the complete actionable scope of the [lifecycle target review](../design_review/reviews/design_review_surrealdb-lifecycle-target_2026-10-12.md), preserving its F01–F05 and RC01–RC06 identities. The target is a Rust-owned pinned API/evidence catalog, shared immutable SurrealDB persistence and exact retained serving, with efficient independent auditing and integrated SurrealKit **1.0.0-beta.1** development tooling. The [source evidence](../design_review/evidence/2026-10-12_surrealdb-lifecycle-target/README.md) qualifies library claims; source inspection is not runtime acceptance.

| Companion | Responsibility |
|---|---|
| [Native access and completion](surrealdb-lifecycle-native-plan_2026-10-12.md) | Protected selection, bounded set-oriented auditing, scoped completeness reuse and convergent completion; F01/F02/F03/F05 |
| [Development tooling](surrealdb-lifecycle-development-plan_2026-10-12.md) | Generated schema, beta Rust integration, remote test isolation, configuration and checked disposal |
| [Lifecycle integrations](surrealdb-lifecycle-integrations-plan_2026-10-12.md) | F04, composed diagnostics, retention and conditional native/platform capability decisions |

The architectural collection remains authority: [semantic model §15](../design/sections/semantic-model.md), [storage/publication](../design/sections/storage-and-publication.md) and [serving](../design/sections/synthesis-and-serving.md), with their ADRs. The core 3.3 principles, efficiency heuristics 1.0 and code-intelligence profile 1.5 govern consequential choices. Plans describe changes; an accepted plan or ADR does not establish implementation or Tested behavior.

This is a distinct, prioritized track. The efficiency, persisted-execution, SA9 and other in-process plans remain paused and unchanged until this track completes. Their receipts and broader obligations are not imported as acceptance prerequisites. In particular, old S04 overlaps F01 and old N10 overlaps F05, but N10's wider storage migration and unrelated numerical/default-profile work are not implicitly scheduled here. New findings have their sole disposition in §6; older IDs remain with their existing owners until LC03 reconciles them.

### Design-phase hard pivot

The operator clarified on 2026-10-12 that existing data and evidence need not be retained. Regenerate useful current views, fixtures and acceptance evidence from pinned inputs. Do not require old datasets, historical benchmarks, proof captures, old failure reproduction, converters, rollback assets, dual writes or legacy runtime readers. No archive is a delivery prerequisite.

Actual live owners must still be stopped/drained before replacing their state. A fresh target is independent of unresolved old state: new work need not repair an old campaign merely to begin. When replacing a held installation, settle only the live ownership/disposal obligations needed for that replacement; explicit discard and regeneration can replace recovery of its old data. Report an interrupted outcome as interrupted/Unknown rather than relabeling it successful. Keep only the minimal current transition identity until disposal is acknowledged, then retire obsolete state and transition machinery. No destruction occurs during plan creation.

## 2. Combined target and foundation assessment

Execution and retention are independent. Acquisition inputs, attempts, contributions, exact views, admission, publications, selection, pins, reusable products, backups, native obligations and host attachments have different authorities. Classify them by authority, mutability, visibility, eligibility, retention consumer, liveness/uncertainty and recovery obligation. These distinctions belong at actual decision/inspection interfaces, not in a new universal lifecycle enum or registry.

| Foundation | Assessment and target consequence |
|---|---|
| Model/private admission | Reuse typed descriptors, qualification and privately earned admission. Physical completion and retention do not grant semantic eligibility. |
| Native realization selection | Reuse held realization/incarnation inventory and contributor-owned aliases. Migrate serving's obsolete SQL rather than add an old membership field. |
| Physical audit | Preserve independent exact output/EOF checks; replace scalar crossings and repeated unchanged byte observation. Existing in-process identical-view caching is useful but does not solve overlapping cold audits. |
| Blob completion | Preserve immutable bytes, layouts and active incarnations. Revision changes need owned recapture/revalidation, not blanket acceptance or a global compile lock. |
| Host/native composition | Reuse admission, maintenance, run/storage owners and separate completion axes. Host exit, SDK invalidation and database DROP do not individually prove remote finality or reclamation. |
| Schema/development | Rust/native declarations remain source. SurrealKit consumes generated assets on explicitly disposable targets; its test result and disposal outcome remain independent. |

A single database platform is useful for shared content, transactional ownership, connected selective access and search. It does not move host process identity, executable ownership or raw captures into the semantic graph. Prefer native set operations and complete library capabilities where they preserve the contract; retain application code for the remaining admission, selected ownership and uncertainty obligations.

Credible growth is several libraries/releases, larger originals, overlapping dependency views, high-degree public hubs, concurrent compiles/readers and interrupted effects. No cloud/distributed deployment or retained behavioral-research backlog is assumed. Benefits are structural hypotheses about removed work; no speed/capacity claim or historical benchmark requirement is introduced.

## 3. Confirmed rules and decision routes

The operator explicitly accepted every disposition below during planning on **2026-10-12**. The later hard-pivot instruction above governs data/evidence retention. No rule was applied to production merely by publishing these plans.

| Source rule impact | Accepted disposition | Implementation route and prerequisite |
|---|---|---|
| [RC01](../design_review/reviews/design_review_surrealdb-lifecycle-target_2026-10-12.md#RC01) | Change: permit explicitly owned schema/permission/destructive-test databases; ordinary data attempts may share validation | LC00 records the bounded exception to storage §6/runbook/AGENTS through a new refining decision or supersession required by the ADR owner; LD02 implements ownership before beta test launch. |
| [RC02](../design_review/reviews/design_review_surrealdb-lifecycle-target_2026-10-12.md#RC02) | Change: retain managed default, permit qualified test-target binding; secure connector only for an actual need | LC00 and LD00 define explicit target authorization; LD02 implements binding. No general remote/cloud support or automatic protocol fallback. |
| [RC03](../design_review/reviews/design_review_surrealdb-lifecycle-target_2026-10-12.md#RC03) | Preserve Rust/model authority; tool consumes generated schema/query assets | LD01 updates the owning generated-output and schema contracts. No handwritten equivalent semantic schema or introspection authority. |
| [RC04](../design_review/reviews/design_review_surrealdb-lifecycle-target_2026-10-12.md#RC04) | Preserve hard cutover; regenerate data/evidence without legacy runtime paths | LC00/LC02 apply ADR-0161's current-state direction; no historical replay acceptance prerequisite. |
| [RC05](../design_review/reviews/design_review_surrealdb-lifecycle-target_2026-10-12.md#RC05) | Preserve patched progressive/finality requirements; reconcile complete provenance and qualification descriptions | LD00/LD04 reconcile pins, SDK/server source and actual owned generations. A recipe is not installed acceptance; no upgrade is assumed. |
| [RC06](../design_review/reviews/design_review_surrealdb-lifecycle-target_2026-10-12.md#RC06) | Preserve explicit ownership, Unknown outcomes and operator non-adoption | LC00/LD02/LO01 adapt disposal to regeneration without requiring historical-data retention. No automatic sweeps, timer activation, shared-service reset or operator selection. |

LC00 applies only decisions whose meaning changes, with the ADR skill and matching architectural-owner edits. Accepted ADRs remain immutable; preserve unaffected publication/admission contracts when refining test policy. Dependency pin additions remain ordinary dependency work, not an ADR in themselves. A material new rule change discovered during implementation returns for explicit disposition before dependent work.

## 4. Packages and prerequisite order

The table is a useful execution route, not a barrier after every row. Component owners may implement ready independent branches concurrently; shared native/model/manifests and integration have one coordinator. Companion package sections supply detailed behavior and acceptance.

| Package | Required input | Delivered capability / completion boundary |
|---|---|---|
| LC00 — target decisions | Approved scope and RC dispositions | Durable changed-rule decisions, architectural owner updates, precise cutover/target ownership; pre-stage design review of newly specified boundaries before their implementation |
| LN01 — protected serving | Existing exact native selection plus scoped interface design | All operation/member/evidence consumers use protected physical namespaces; revealing actual native serving controls pass |
| LN00 — completion validity | Current mutation/retirement paths, without old captures | Explicit same-incarnation audit premises and changed-premise retry classification; foundation for LN03/LN04/LN05 |
| LN02 — bulk independent audit | Existing streaming/budget owners; LN00 where blob validity is consumed | Byte-charged indexed batches with exact correspondence and terminal checks; F02 controls pass |
| LN03 — scoped completeness reuse | LN00, working independent audit route | Whole-blob and immutable closure work deduplicated within protected scope without losing per-layout/range checks |
| LN04 — convergent completion | LN00, same-incarnation recapture/revalidation contract | Valid equal peers converge; mutation/ABA/cancellation controls pass; no global serialization |
| LO00 — typed receipt causes | Existing model completion and native typed errors | Persisted machine-readable diagnostics and actual consumers, retaining independent outcome axes |
| LD00/LD01 — beta/schema integration | LC00 for changed policy; pinned library source | Exact dependency/features and generated assets; independently configured Rust Sync session; parser/schema equivalence controls |
| LD02/LD03 — isolated test capability | LC00, working LD00/LD01; LO00 diagnostic contract | Owned remote target, beta Rust test worker, complete recovery inventory and checked disposal; focused failure controls |
| LN05/LO02 — capability decisions | Settled semantic contracts; LD02 only for required probes | Supported decisions for broader completeness reuse and every conditional avenue; justified adoption delivered before track completion |
| LO01/LD04 — inspection and provenance | LO00; actual ownership/configuration routes | Composed inspection, honest support/version matrix and operational documentation; no competing authority |
| LC01 — assembled review/acceptance | All required integrations and fit decisions implemented | Independent integrated design review, affected journeys and one assembled qualification at the complete functional boundary |
| LC02 — current-state cutover | Working replacement, live-owner inventory and LC01 candidate evidence | Quiesce only selected project targets; discard/regenerate pinned-input state; retire obsolete paths/state; verify current target and cleanup |
| LC03 — handback | LC01/LC02 complete, no unresolved required scope | Reconcile affected older plans and architectural owners using exact receipts; then identify their next executable work |

LN01 has highest product priority and need not wait for tooling or all capability studies. LN00 is a design prerequisite, not itself implementation of reuse. LN02/LN03 may develop together, but F02 and F03 retain separate closure obligations. LO00 can proceed independently; its diagnostic projection must exist before test ownership consumers finalize their persisted reporting. LD02 supplies a safe probe environment, not a prerequisite to pure source decisions. LC02 may include earlier explicitly owned disposable-target cutovers; its final current-state acceptance is after replacement integration. If cutover affects an assembled journey, rerun only that affected journey before declaring completion.

### LC00, LC01, LC02 and LC03 in practice

LC00 writes a combined decision explaining the protected selection, completion validity and development-target boundaries. Use the source review as supported input; a fresh design reviewer examines consequential new contracts at the binding's pre-stage cadence. This is not a new codebase-wide review or a demand for historical qualification. Changes within already accepted boundaries can begin with their applicable bounded assessment.

LC01 combines native serving/evidence, concurrent completion, independent audit, typed diagnostics and isolated-tool cleanup. Run focused compile and contract controls during implementation. At whole functional completion, run the affected functional controls and applicable leaves, then `just qualify` once under the current shared-contract/receipt/transport policy. Resolve scope-owned failures and rerun affected boundaries; wider defects retain explicit out-of-scope ownership. A fresh integrated design/target review must distinguish source acceptance from execution outcomes and Measured claims. A cleanup failure cannot be hidden by passing test bodies.

LC02 chooses current views by actual consumer, regenerates them and fresh revealing evidence from pinned inputs, and removes obsolete code/data. Do not preserve superseded formats merely for rollback. Do not automatically open or reset shared/operator installations: target changes use explicit maintenance ownership. If an old dedicated campaign is inaccessible, use an independent owned target for this track; if replacing that campaign is actually needed, drain/discard it without recovering its old semantic data. A local process exit alone is insufficient to release its remote effects. Activation/selection of an operator snapshot remains separately authorized.

LC03 is the first package that edits existing implementation plans. Compare each affected package/finding with the delivered contract and actual evidence, close only its satisfied obligations, transfer remaining scope without duplication, and retain genuinely unrelated backlog. Neither this track's acceptance nor current regeneration closes historical SA-F02 or other separately scoped claims automatically; if a historical-only requirement no longer serves a current consumer, explicitly retire it with the owner's rationale rather than pretending it passed. Update STATUS and the architectural collection, retire documents only after their surviving obligations have owners, and then resume the user's older queue.

## 5. Complete review coverage

| Review obligation | Route and required disposition |
|---|---|
| F01 membership and evidence bypass | LN01; all reachable consumers, exact copies, selected aliases and no legacy field |
| F02 scalar audit crossings | LN02; true set-oriented access, byte limits, complete output proof |
| F03 repeated originals/overlapping closure | LN03 mandatory scoped reuse; LN05 explicit broader-witness decision and implementation if adopted |
| F04 durable failure causality | LO00 plus LO01; typed persisted round trip, safe opaque cause and actual inspector |
| F05 equal concurrent completion | LN00/LN04; owned recapture, same-incarnation validity and real interleavings |
| Taxonomy and lifecycle independence | §2, LC00, LO00/LO01; actual owners/projections rather than universal status |
| Promotion, admission, publication/selection and reader lifetime | Preserved shared contracts; LN01, LD01, LC01/LC02 prove relevant seams |
| Cross-owner effects, retirement and coherent backup | LO01/LO02, LC01; retain finality/ownership while regenerating current state |
| Beta Rust integration, modules/targets, sync/watch, seeds/tests/types/reports | LD00–LD03; explicit per-command support, generated input and owned disposal |
| Configuration, connection/session and server/SDK provenance | LD00/LD02/LD04; explicit needed options, no clone/invalidate finality shortcut |
| Pure/schema/remote test boundaries | LD03 and LC01; each result states the boundary it actually establishes |
| References, events, changefeeds/live queries/history and bounded sweeps | LO02 capability rows C01–C03; decide equivalence/fit; no automatic timer or cascade |
| Graph/GQL, adjacency/count/materialized views and lexical/vector/hybrid lowering | LO02 C04–C05; selected universe, attributed multiplicity and declared ranking remain authoritative |
| Database APIs, modules and file bytes | LO02 C06–C08; authorization, executable/object lifecycle, selected backup and disposal considered |
| Studio/MCP/EXPLAIN/parser tools, telemetry and agent access | LO01/LO02 C09–C10, LD01; actual consumers, least privilege and on-demand diagnostics |
| Complete storage alternatives and future changes | LO02 C11 plus §2/LC01; simple complete-route comparison, no speculative second service |
| Regeneration, prerequisite order and old-plan reconciliation | §1/§4, LC02/LC03; hard pivot, truthful separate acceptance and delayed handback |

Every conditional avenue receives a supported fit decision. An adopted mechanism requires implementation and its own acceptance before LC01; a justified rejection or deferral with an actual trigger completes that avenue's decision scope. Availability is not an adoption requirement. A required finding cannot be closed by deferral. Decision rows live in the integrations companion; their implementation status/receipts live only in §6.

## 6. Sole disposition and execution checkpoint

**2026-10-12: plans authored; no production implementation, test/probe, dependency move, database query, service action, migration, discard or activation performed.** Operator-confirmed design directions are distinct from working prerequisites. The five source findings are scheduled, not closed. Capability decisions proposed from source remain distinct from tested substitutions.

| Source finding | Disposition | Responsible components | Closure owner / required evidence |
|---|---|---|---|
| [F01](../design_review/reviews/design_review_surrealdb-lifecycle-target_2026-10-12.md#F01) | Open — scheduled, not implemented | native selected access; serving selection/evidence | LN01 actual populated operation/evidence journey, foreign copy and aliases |
| [F02](../design_review/reviews/design_review_surrealdb-lifecycle-target_2026-10-12.md#F02) | Open — scheduled, not implemented | native installed-view audit/access | LN02 source removed scalar crossings plus multibatch/corruption/late-error controls |
| [F03](../design_review/reviews/design_review_surrealdb-lifecycle-target_2026-10-12.md#F03) | Open — scheduled, not implemented | originals; installed-view/compiler preparation | LN03 scoped readback/closure reuse and independent range/corruption checks; LN05 broader decision |
| [F04](../design_review/reviews/design_review_surrealdb-lifecycle-target_2026-10-12.md#F04) | Open — scheduled, not implemented | model/native diagnostics; host receipt consumers | LO00 actual persisted typed cause round trip plus unknown/committed/storage preservation |
| [F05](../design_review/reviews/design_review_surrealdb-lifecycle-target_2026-10-12.md#F05) | Open — scheduled, not implemented | native originals and effect retry composition | LN04 concurrent equal peers and mutation/ABA/cancellation controls |

All LC/LN/LD/LO packages in §4 are **not started**. Mandatory investigation outcomes remain open: LN00 mutation premises, LN05 retained-witness fit, LD00 dependency/features and Any-session compatibility, LD01 beta schema transformation equivalence, LD02 cleanup failure behavior, and LO02 unresolved capability substitutions. Each companion names the evidence, dependent work and outcome route; these are executable bounded investigations, not permission to silently choose a weaker contract.

**Next executable work:** LC00's changed-rule/target decision and bounded LN01 selection contract; run LN00 and LD00 source/compile investigations independently. No old paused campaign must be borrowed first. If source changes before execution, reconcile only material drift against these contracts. Record subsequent command/run IDs, dates, outcomes and cleanup separately here; do not copy an old plan's pass into this track.

### Plan-publication evidence, 2026-10-12

- **Passed:** focused `python3` document check: four new plans plus added links in four updated files; 48 local links/13 fragments, all 18 package IDs, F01–F05/RC01–RC06 and C01–C11 present; STATUS within 120 lines and existing implementation plans unchanged. This checks navigation/coverage, not semantic correctness.
- Independent read-only focused design advice identified two beta integration clarifications: internally created tester clients need whole-service containment, and default-module tests need a generated projection. Both are incorporated; no other material contradiction or missing review obligation was reported. This is design advice, not formal architectural acceptance; LC00/LC01 retain their review cadence.
- **Failed:** `just docs-check` stops at 14 existing ADR/architectural-reference lint mismatches (ADR-0152/0155/0156/0157/0158/0160/0161 and ADR-0151's DESIGN decision link). ADR/design sources and checker implementation are unchanged by this track. Full publication checking did not reach its later stages; these repository repairs are outside plan authoring.
- **Not_run:** product builds/tests, database/service actions, beta installation, probes, qualification and activation; this turn authors plans only.

Scoped `just turn-end --paths` provides applicable end-of-turn housekeeping without changing other owners. Publication/navigation do not certify architecture or runtime behavior. Execute no production package merely because plan creation is complete.
