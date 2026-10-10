# SurrealDB architecture and capability leverage

**Design-tier · target-purpose review · 2026-10-10**

**Decision: Revise.** Retain the explicit semantic model, immutable completed views, direct publication, exact reader pins and specialized Rust computation. Revise the migration operation and several physical database access paths. The evidence supports substantial changes within these boundaries; it does not establish that replacing SurrealDB, moving all computation into the database, or introducing a universal graph, cache or workflow framework would improve the complete system.

The principal architectural mismatch is between carefully bounded semantic operations and their physical realization. The system often has the right identities, ownership and refusal behavior, but still repeats catalog preparation, expands complete selected membership for a limited relation request, or decomposes an available set operation into sequential database crossings. Migration adds a separate problem: its target schema, row transformation and recovery journal do not form a complete contract for intermediate states.

These are source-established architectural defects. No latency, throughput, capacity or attribution of the reported migration duration follows from this review.

## 1. Scope, baseline and evidence strength

The functional target is a version-pinned API and evidence catalog: discover a feature, inspect its invocation and configuration, and follow its support and uncertainty within one pinned realization. Behavioral enrichment remains qualified enrichment rather than a prerequisite for the first catalog product.

The reviewed workload is one operator with concurrent commands and worktrees, repeated compilation and validation, retained releases, sparse evidence requests, complete analytical projections, high-degree ownership and accumulating control history. Small output limits coexist with potentially large selected publications. Maintenance, interruption and recovery are supported operations rather than exceptional assumptions.

| Field | Reviewed scope |
|---|---|
| Baseline | Dirty `main`, HEAD `125e74f5775e1354ce900cc8d71b190e311dac27`; current uncommitted implementation included |
| Standard | Core/template 3.3, efficient-architecture heuristics 1.0, code-intelligence profile 1.5, library-context binding |
| Principal reviewer | Independent delegated design reviewer; root reconciled research, verified candidate evidence and published the document |
| Enduring owners | [Storage and publication §6](../../design/sections/storage-and-publication.md), [semantic model §15](../../design/sections/semantic-model.md), [synthesis and serving §11](../../design/sections/synthesis-and-serving.md), and adjacent compiler, host-service and recovery consumers |
| Proposed work considered | [Holistic state-management plan](../../plans/holistic-state-management-plan_2026-10-10.md) |
| Method | Read-only source, types, current owners, exact pinned upstream interfaces and selected historical receipts; bounded code mapping and library research informed the independent assessment |
| Review investigation effects | No database queries, probes, builds, tests, restarts or remediation. Root publication changes only this document, navigation and existing handoff/receipt owners. |
| Publication path | `docs/design_review/reviews/design_review_surrealdb-architecture-and-capability-leverage_2026-10-10.md` |

The assembled holistic-state review is a historical lead, not the source of this verdict. Current ADRs, dependency holds and mechanism choices were treated as subjects eligible for revision, not acceptance criteria. The `neo4j-surrealdb` skill, Context7 documentation, official documentation and exact GitHub source supplied capability leads; the selected contracts were checked at their stated revisions.

**Evidence vocabulary.** Current executing source is **Implemented**. Inspected upstream contracts are **Interface-checked**. Corrections and adoption directions are **Proposed**. Historical executions retain their date, candidate and scope. This review establishes no fresh **Tested** or **Measured** implementation claim.

### The current migration failure

The historical run `build/runs/20261010T174350.233Z-05d48c/output.log:47–50` records the failed same-target validation upgrade and admission remaining closed. Selected diagnostic fields identify installer SHA256:

`53f28eeb61887c168fdd2ecb35a662d8aea35b1bfd7b53779b5b290992f14369`

Its primary error was a mandatory integer `native_retirement.root_count` receiving `NONE`; subsequent statements reported the cancelled transaction and aborted commit. The coordinator independently matched that binary hash and found the retirement update literal that omits both new count fields, at binary byte offset `62069966`.

This corroborates F01 against the actual candidate. It does not supply a live statement trace or phase timings. The failed candidate also predates the latest page-local origin memoization, so that later source optimization must not be credited to the failed execution.

Main has completed the control-format-4 upgrade; validation remains on format 3 with admission closed. Those are scope-specific outcomes, not whole-service qualification. The later failure supersedes earlier “translation running” prose as a current-state claim. The continuation exited1 with confirmed cleanup at19:02:54UTC on2026-10-10. Queued runs `20261010T181816.092Z-d35925` and `20261010T182220.387Z-e52348` failed prerequisite waits; their native test bodies are **not_run**. Existing actual receipt ownership stays in [coordinator §9.1](../../plans/persisted-graph-execution-plan_2026-10-07.md#91-current-execution-checkpoint-2026-10-07).

## 2. The governing model and responsibility boundaries

The architecture has several distinct kinds of authority that should remain distinct:

| Responsibility | Governing authority and contract | Reason for change |
|---|---|---|
| Semantic facts and graph meaning | `lctx-model::domain`, including `Relation`, fields, invariants, attribution, proof requirements and projection roles | A new phenomenon, fidelity distinction or domain operation |
| Completed contribution and view identity | Exact immutable input/output descriptions, dependency views, vocabulary and implementation identities | A changed compilation contract or acknowledged source |
| Native physical realization | `lctx-surrealdb` envelopes, membership indexes, aliases, admission and native queries | Access patterns, storage layout or backend capability |
| Compilation orchestration | Declared model/provider schedule and stage-owned preparations | A new composition or scheduled consumer |
| Specialized computation | Model-owned kernels, DataFusion and graph-library adapters | A genuinely different mathematical or program-analysis operation |
| Publication and recovery | Publisher manifest sealing, actual-state inspection and independent cold admission | Publication, export or recovery guarantees |
| Serving | Exact pinned realization, request preparation, ranking policy and continuations | A served operation or answer policy |
| Runtime authorization and retirement | Issuance era, original owner epoch, object incarnation, durable obligation and executor outcome | A lifecycle or recovery operation |
| Host service and fixtures | Stable installed generation, borrowers, maintenance and logical validation attachments | Deployment, executable replacement or test isolation |

`crates/lctx-model/src/domain/model.rs:8–40` is more than a common output schema: its relations retain invariant, proof, attribution and projection metadata and expose governing validation operations. Scope programs declare roots, directions and predicates separately from native SQL. Product reuse has its own explicit request contract.

The physical schema does not simply create one table or execution stage per semantic kind. `crates/lctx-surrealdb/src/schema.rs:50–120` groups records into fixed physical envelopes and derives their relevant layouts from model declarations. Atomic nominal identifiers deliberately preserve whole-key meaning rather than inheriting array-element behavior. This is a useful separation between domain concepts and physical organization.

The model is adequate for the inspected catalog, completed-view and selected-evidence distinctions. It explicitly distinguishes physical, virtual and absent roots, selected membership from output filtering, nominal identity from physical payload, and semantic content from runtime authorization.

The important inadequacy is in the **upgrade operation**. Its intermediate-state contract does not describe a complete compatible row state or sufficiently precise durable progress. A well-modeled published graph does not compensate for an incomplete model of a supported state transition.

### Fact and fidelity

| Family or answer | Authority and fidelity | Coverage and identity | Reviewed consumers |
|---|---|---|---|
| Syntax, typing and runtime-flow facts | Independently attributed provider facts; their meanings remain distinct | Pinned provider/source/configuration context and explicit coverage | Completed views, normalization and publication |
| Normalized relations and resolutions | Model-owned derivation over declared inputs | Exact consumed views; unresolved is distinct from absent | Analytical projections and catalog synthesis |
| Behavioral results | Exact or conservative only under their stated finite model | Model, policy and proof identity | Retained enrichment and qualified answers |
| Communities, similarity and embeddings | Governed heuristic or statistical results | Projection, encoder and policy identity | Ranking and enrichment, not behavioral proof |
| Catalog assertions and evidence | Programmatic synthesis over admitted facts and findings | Evidence must belong to the same realization | Tools, resources, briefs and continuations |
| Search occurrence eligibility | Native occurrence records plus exact selected dependencies | Publication, family, input/window and unit rules | Lexical and vector candidate ranking |

This is a review of their persistence and consumption boundaries, not a fresh audit of every extractor’s semantic coverage.

## 3. Composition worth preserving

Several recent corrections materially improve the architecture and constrain the remaining remedies.

**One semantic closure, independent current and cold checks.** The recovery-family declarations and adapters replace separate interpretations of recoverable state. `recovery_closure.rs` supplies explicit family treatment, while native selection and dump preparation retain their physical responsibilities. Actual contributors, independent role/search claims and empty domains remain part of the closure. Sharing declarations must not turn producer expectations into the oracle for actual persisted state.

**One audit capture with owned completion.** `compiler.rs:928–1010` and publisher `inspection.rs:177–231` reuse captured actual state for verification and completed-state encoding. The publisher’s owned-read machinery retains cleanup responsibility after early failure or cancellation. The previously repeated audit-closure concern is not reissued here.

**Exact cache validity.** `domain/compilation_product.rs:14–68` includes model, implementation, policy, result contract, configuration, profile, parameters, role-scoped exact inputs and output set. A hit reattaches admitted contribution membership; it does not automatically replay its producer. Root preparation batches physical presence and retains per-root membership after shared edge discovery. Explicit absent, present and virtual outcomes, together with missing content tokens, address negative-result invalidation.

**Preparation follows real consumers.** `analysis_graphs.rs` retains graph hydration and SCC schedules for their selected projections. `compilation/preparation.rs` and stage release account for remaining consumers, including hit paths. DataFusion templates and contexts have exact immutable input identities. This is preferable to repeated hydration, but also preferable to a permanent process-wide graph inventory.

**Serving owns active state separately from optional retention.** The service retains its reader and preparation cache. Definition checking is once per service instance, not a reconnect or full definition check per request. Prepared and ranked values are charged and have explicit request, flight and retention ownership. The latest ranked-state and budget-composition corrections remain source claims until their composed controls pass.

**Selected backup has a credible coherent route.** `backup.rs:24–87` and `selected_backup.rs` retain one transaction-bound snapshot across manifest, definitions and selected content, and keep the exact session available through cancellation and invalidation. Staged bytes become a published artifact only after finalization. Imported metadata is comparison data; restore does not execute imported SQL or restore runtime grants as authority.

**Lifecycle authority is explicit.** Era, original owner epoch, executor identity, retirement incarnation, first cutoff and successor lineage are separate. Retirement nominates children and deletes outgoing holds together, and finalization requires an empty proof. History collection preserves references, sufficient provenance and permanent fencing. These distinctions should survive batching.

### Analysis and execution placement

| Question | Selected universe and mechanism | Fidelity and evidence | Work and limit distinction |
|---|---|---|---|
| Which rows belong to an exact completed view? | Native membership, aliases and exact dependency views | Exact under the view contract | Sparse nomination and complete closure are different operations |
| Which topology supports an analytic? | Declared projection hydrated into a library graph | Exact projected topology; source lineage retained | A complete global analytic can legitimately consume its complete graph |
| What behavior follows under a finite model? | Rust transfer functions, BDDs and SCC schedules | Exact/conservative under the stated abstraction | Plain reachability cannot replace transfer semantics |
| Which lexical occurrences match? | Currently complete eligible frontier followed by frozen term scoring | Frozen publication-scoped BM25 and exact eligibility | Candidate output cap does not bound examined occurrence/document work; F05 |
| Which vectors rank highest? | Exact selected eligible vector scoring | Exact ranking under its encoder/distance policy | Approximate HNSW is a different contract, not an interchangeable optimization |
| What content is recoverable? | Model closure realized through one snapshot and typed dump | Exact selected content plus independent restore admission | Streaming transport does not alone bound upstream preparation |

## 4. Material findings

The findings below have stable source-review IDs. Their diagnoses are **Implemented-source findings**; remedies are **Proposed** unless explicitly described as an inspected library capability.

No new finding is asserted to be scheduled. Each is **Pending decision** in this source review. If scheduled, the [persisted coordinator §8](../../plans/persisted-graph-execution-plan_2026-10-07.md#8-sole-finding-disposition) becomes its sole current disposition owner.

### <a id="F01"></a>F01 — Target schema enforcement precedes a complete legacy transition

**Owner:** Native upgrade and control-schema boundary.  
**Judgments:** FP-04/05/07; DP-03/15/19/24; A2 and A4.

`upgrade.rs:77–82` installs target declarations before legacy translation. The target retirement schema requires integer `root_count` and `roots_registered` with ordinary defaults (`control/schema.surql:74–75`). The existing-row update in `control/migration.rs:33` assigns neither field. Verification checks legacy protection and epoch, but does not establish those mandatory counts.

At pinned SurrealDB source revision `238bfeb11f5725bebed370167656748df8067595`, `core/src/doc/field.rs:635–648` leaves `NONE` unchanged for an existing document unless the default is `Always`. An ordinary default is not an existing-row backfill. See the [pinned field-default implementation](https://github.com/surrealdb/surrealdb/blob/238bfeb11f5725bebed370167656748df8067595/surrealdb/core/src/doc/field.rs#L635), inspected in retained local source.

**Falsifiable case.** A legitimate format-3 retirement row lacks both new fields. Installing the mandatory target fields and then executing the present update rejects the row. The historical failure and exact-candidate literal corroborate this route. Database rejection protects publication, but a safe rejection of valid migration input does not make the upgrade architecture adequate.

**Correction.** The upgrade owner must define the complete intermediate and final row states, and check their compatibility before long effectful work. Viable realizations include a compatible transitional schema followed by complete transformation and final enforcement, or an explicit complete backfill under a schema that permits the transition. Verification must establish every required target invariant.

Do not solve this by broadly changing fields to `DEFAULT ALWAYS`: that changes future write semantics. Do not invent executable root authority for ambiguous legacy jobs. Such jobs must remain explicitly protected.

**Preserved guarantees and challenge.** Generation, immutable content, original outcomes, watermarks and legacy protection remain unchanged. A malformed late row must be rejected without first committing an avoidable long incompatible transition. A protected legacy job need not become a normal executable retirement job merely to satisfy the physical schema.

**Closure evidence.** Inspect a complete source-to-target field contract and exercise populated legacy rows missing every new field, an interrupted retry, and a malformed late row. Establish final target validity and unchanged legacy protection. A fresh empty database is insufficient.

### <a id="F02"></a>F02 — The native upgrade journal identifies the operation but does not bound recovery replay

**Owner:** Native upgrade journal and migration passes.  
**Judgments:** FP-04/05/07; DP-08/19/21; A2 and A4.

The native journal records coarse phases—intent, declarations, translated and published. Apart from the already-published route, a retry repeats declaration installation and migration. Translation, cleanup construction and verification initialize local cursors again.

The current migration uses 128-row keyset pages and primary `RecordId` ranges. This review does **not** find a demonstrated quadratic repeated-prefix scan in those pages. The defect is recovery scope: after interruption near the end, completed passes and pages are repeated because their durable progress is absent.

Host checkpoints correctly preserve completed database scopes and immutable candidate identity. They do not replace progress inside the still-incomplete validation scope.

**Falsifiable case.** Interrupt after most legacy pages and cleanup work commit but before the translated checkpoint. Resume the same approved operation. The current source starts those passes from their beginnings, including already-completed setup and checks.

**Correction.** Persist the relevant pass, table and cursor under the original upgrade identity. Commit page effects and their progress together, and reconcile uncertain acknowledgements against that exact progress. Separate declaration-plan completion, translation and verification completion. Bound page effects by appropriate row and byte constraints.

This does not require a workflow platform. A small operation-owned checkpoint is sufficient. Rebuilding a demonstrably reconstructible validation scope is an alternative only after its actual protected consumers and recovery obligations are established; it is not a general deletion permission.

**Preserved guarantees and challenge.** Recovery retains source, target, generation, original operation and approved executable identity. A different installer must not silently resume old authority. Independent final validation remains necessary; “page committed” is not “publication valid.” Reusing completed verification additionally requires the exact verifier and input premises under the same closed cutover; a translated row is not its own validation oracle.

**Closure evidence.** Interrupt before and after page commit, lose an acknowledgement, and resume late in each pass. Show that committed prefixes are neither skipped incorrectly nor replayed disproportionately, and that publication still requires complete validation.

### <a id="F03"></a>F03 — An installation/publication operation repeatedly inventories unchanged catalog state

**Owner:** Loader and publisher definition preparation.  
**Judgments:** FP-01/06/07; DP-08/10/14/16/23; A4.

`Loader::install_declarations` inventories before and after each declaration group (`loader.rs:202–263`). Each inventory performs `INFO FOR DB`, then `INFO FOR TABLE` for every table, parsing their fields and indexes (`265–312`). Upgrade invokes several groups and the canonical/physical loader groups.

Publisher definition verification likewise inventories the catalog (`definitions.rs:95–111`). Publication verifies its epoch and verifies again during sealing (`native_publication.rs:94–103,191–197`).

The current declaration batching and index-readiness polling are useful corrections. Ordinary publication verifies definitions; it does **not** install DDL on every publication. A separate backup inventory inside its snapshot is also an independent trust/coherence boundary, not automatically redundant.

**Falsifiable case.** Add unrelated tables or run an upgrade with several mostly unchanged declaration groups. The operation repeatedly reads and parses the same unchanged table metadata. The amplification follows catalog size and group count, independently of how little DDL changed.

**Correction.** Let one installation operation own its exact generated declaration plan and relevant actual catalog capture. Apply the delta in bounded groups; read back and qualify changed definitions and index readiness; perform the necessary final relevant identity check once. Reuse evidence only while its validity premise—exclusive mutation authority or immutable epoch identity—holds.

Alternatives are a unified installation group or targeted metadata reads. Neither a global stale inventory cache nor removal of actual-definition verification is acceptable.

**Preserved guarantees and challenge.** Unauthorized changes, mismatched definitions and unready indexes must still be detected. Independent restore admission remains independent. An external mutation during an operation invalidates reuse unless the operation excludes that mutation.

**Closure evidence.** Trace the resulting metadata calls for changed and unchanged groups. Show bounded preparation reuse and retained mismatch/readiness detection. No elapsed-time benchmark is required to establish removal of repeated unchanged work.

### <a id="F04"></a>F04 — A limited typed relation read prepares the complete selected membership universe

**Owner:** Selected-reader relation access.  
**Judgments:** FP-03/07; DP-08/10/14; A4.

`reader.rs:212–220` routes `relation_bodies(relation, limit)` through `selected_payload_rows`. For a selected reader, that first obtains complete selection preparation (`163–187`).

`selection.rs:106–120` enumerates all selected membership pointers and expands aliases. `rows_with_budget:198–230` traverses the prepared pointers, fetches matching physical-family windows, sorts matching rows, and applies the output limit afterward. The production CLI relation query uses this route (`lctx/src/newnative.rs:159–183`).

Other candidate APIs are genuinely sparse. Full reconciliation can legitimately need complete selected content. This finding concerns the typed limited relation route, not all reads.

**Falsifiable case.** Retain many unrelated relations in a publication, then request one rare relation with limit one, or a relation with no members. The route still prepares unrelated selected membership and traverses its physical family.

**Correction.** Offer relation-aware selected membership or typed native nomination using the existing relation/view indexes and exact alias semantics. Merge ordered streams or another bounded candidate route so unrelated membership need not be prepared. Terminate only after the semantic ordering, deduplication and limit guarantees are established.

Do not push a limit before selected eligibility or apply separate per-view limits that lose the globally earliest result. A complete broad route remains appropriate for consumers that actually need it.

**Preserved guarantees and challenge.** Equal nominal identities with different physical payloads, aliases, multiple exact views, empty relations and deterministic ordering must retain their current meanings.

**Closure evidence.** Inspect the limited route’s dependence on relation membership rather than unrelated selected content. Use multiple views with a late qualifying member, aliases and an empty relation to challenge early termination and ordering.

### <a id="F05"></a>F05 — Lexical search abandons native selective nomination because of a record-source limitation

**Owner:** Serving lexical candidate generation and native search materialization.  
**Judgments:** FP-07; DP-13/14/16; CI-07; A4 and G8.

The existing search-family tables already have native FULLTEXT indexes (`materialization.rs:64–72`). Nevertheless, lexical search first builds the complete eligible occurrence/document frontier, then scores every selected document against frozen terms (`serving/search.rs:258–285`).

The comment correctly identifies that `MATCHES` on a `RecordId` source lacks a table query executor. Pinned `core/src/fnc/operate.rs:25–74` corroborates that limitation. It does not establish that table-source nomination is unavailable.

Pinned language tests demonstrate table-source FULLTEXT matching combined with scalar predicates, including matching rows whose score is zero. The mixed-index test also declares an index-iteration plan. These are inspected upstream test definitions, not tests run by this review: [zero-score matching and scalar predicates](https://github.com/surrealdb/surrealdb/blob/238bfeb11f5725bebed370167656748df8067595/language-tests/tests/language/indexes/search/select_where_matches_without_complex_query.surql), [mixed native indexes](https://github.com/surrealdb/surrealdb/blob/238bfeb11f5725bebed370167656748df8067595/language-tests/tests/language/indexes/search/select_where_matches_mixing_indexes.surql). The latter explicitly selects the compute-only planner strategy; it establishes capability, not the planner behavior of a future composed product query.

**Falsifiable case.** A publication has many eligible documents but a query term occurs in very few. The current lexical route still enumerates the full eligible frontier and performs term-scoring preparation for all selected documents.

**Correction.** Nominate matching document IDs from the actual indexed family table. Preserve the intended OR semantics over analyzed terms, including zero-IDF matches and the existing exact-match branches. Intersect bounded nominees with indexed occurrence records and the existing exact dependency/unit eligibility rules. Then apply the publication’s frozen BM25 statistics and ranking policy.

Native global BM25 scores cannot replace frozen publication-scoped statistics: unrelated retained content must not change an old publication’s answer. Neither `score > 0` nor an early global candidate limit preserves the contract.

**Alternatives and challenge.** A native table-source query or a scoped derived nomination representation may fit. Retaining the present complete frontier is justified only for a declared complete/nonselective operation, not as the default selective lexical route. Analyzer edge cases, punctuation, repeated terms, compound tokens and empty analyzed queries require qualification before selecting the exact query form.

**Closure evidence.** Demonstrate nomination completeness and exact ranking equivalence for zero-IDF matches, rare terms, analyzer edge cases and unrelated-publication growth. Inspect actual planner composition if that affects the chosen route. The diagnosis does not require a speed claim.

### <a id="F06"></a>F06 — Sparse occurrence eligibility remains sequential per occurrence and dependency

**Owner:** Native sparse membership and derived-search occurrence hydration.  
**Judgments:** FP-03/07; DP-08/10/14/16; A4.

The indexed occurrence route nominates compact rows, but then handles each occurrence separately. It deduplicates that row’s dependencies, calls `selected_candidate_ids`, and fetches its payload with a singleton `SELECT` (`derived_search.rs:57–82`).

The membership implementation loops requested nodes and views (`selection.rs:236–260`). Direct membership uses singleton node requests. An entity miss constructs a separate reverse-alias ancestry preparation for that requested entity.

This is correctly sparse; it avoids complete-view startup. Its remaining problem is set-to-scalar expansion and repeated handling of shared dependencies.

**Falsifiable case.** A selected occurrence window has many rows sharing the same unit and dependencies, or many nominees from several selected views. The client repeats membership crossings and payload queries rather than resolving that bounded window as a set.

**Correction.** Accumulate a bounded occurrence window, union its dependency identities, resolve exact membership with per-node results, retain requested-node provenance during batched alias ancestry, evaluate each occurrence’s “all dependencies selected” condition, and hydrate accepted payloads in batches.

The current internal boolean “any direct member found” helper cannot simply be reused as a per-node batch answer. A charged operation-local memo can help actual repeated dependencies, but a new global deduplication framework is unnecessary.

**Preserved guarantees and challenge.** One selected dependency must not make the whole occurrence eligible. Missing late dependencies, equal nominal/different physical identities, reverse aliases, cancellation and terminal drainage remain explicit.

**Closure evidence.** A bounded window with shared dependencies, one missing dependency and aliases should retain the same accepted occurrence set while using window-level membership and hydration. Inspect ownership through early delivery cancellation.

### <a id="F07"></a>F07 — Maintenance set operations still require singleton database crossings

**Owner:** Native history collection, retirement preparation and migration cleanup construction.  
**Judgments:** FP-07; DP-08/10/14/19; A4.

History collection reads its checkpoint, selects `LIMIT 1`, runs a guarded transaction, reads back that candidate, and repeats (`control/history.rs:65–108`). Its limit bounds candidates but does not batch their database crossings. Positively, deletion and cursor movement commit together, and the checkpoint avoids allocating a new effect record for every collected record.

Retirement registration reads incarnation separately per root (`retirement.rs:20–28`). An outgoing-hold page separately reads each child incarnation before its effect (`76–89`). Migration cleanup creation likewise submits a transaction for each extant owner.

**Falsifiable case.** Collect a large eligible terminal-history range, register many roots, or retire a high-degree object. Even within bounded pages, the number of sequential crossings follows individual candidates or children.

**Correction.** Read bounded sets of exact guards and candidates. Perform eligibility rechecks, deletion and cursor advance within bounded atomic collection pages, returning the required result rather than performing avoidable per-record readback. Construct retirement and cleanup work from bounded exact sets.

Per-item checks remain necessary. Separate network calls per item are not automatically necessary. Retain singleton operations where a real conflict or failure contract requires them, with that reason attached to the route.

**Preserved guarantees and challenge.** Batch size must respect the destructive effect budget, not merely parent count. Holds, outcome references, original provenance, incarnation changes and uncertain acknowledgement must still prevent unsafe disposal. No per-item rich effect history should be introduced merely to batch collection.

**Closure evidence.** Inspect bounded page effects and use protected candidates interleaved with eligible ones, high-degree retirement, an incarnation race and interruption around commit. Show exact budget accounting and resumable progress without singleton crossings being the default.

## 5. Capability fit and total complexity

The appropriate direction is to make the existing store and libraries do the generic work they already support, while retaining project-owned semantic and authorization contracts.

| Capability | Fit and recommendation | Integration limits |
|---|---|---|
| Indexed native selection and set queries | Use for F04–F07. Existing equality, membership, occurrence and primary-range mechanisms already provide credible foundations. | Preserve exact view, alias and dependency semantics; planner capability does not itself prove a particular composed query is selective. |
| FULLTEXT | Use native table-source nomination; retain frozen publication ranking. | No global-score substitution, positive-score filter or eligibility-after-quota shortcut. |
| HNSW/vector indexes | Credible for an explicitly approximate retrieval policy. | Not a replacement for current exact selected-vector ranking without a policy change and independent recall/eligibility qualification. |
| Native references | `REFERENCE ... ON DELETE REJECT` may enforce some target-existence obligations. | Shared ownership, retirement cutoffs and original authorization exceed simple referential integrity. CASCADE is not a blanket retirement replacement. |
| Native views and events | Consider only for a named maintained projection or invariant with a concrete consumer. | They add write work and another lifecycle. Asynchronous completion cannot stand in for admission. |
| LIVE queries/changefeeds | Useful only for a concrete notification or synchronization consumer. | Reconnect gaps and delivery acknowledgement do not replace durable effects or recovery journals. No such new consumer was established here. |
| Native transactions and RocksDB snapshots | Retain for coherent selected backup and atomic guarded effects. | Snapshot lifetime and iterator ownership are real resource obligations; physical finality and cancellation remain necessary. |
| SurrealKit | Evaluate its frozen rollout planning, preflight and completed-step journal beneath project maintenance authority. | Not a drop-in effect-fencing replacement; see below. |
| Progressive WebSocket RPC | A genuine alternative transport candidate at the pinned server revision. | Stock Rust WebSocket integration does not expose this route directly; adapter, same-session cancellation and complete terminal semantics add work. |
| DataFusion/Arrow | Retain for bounded relational computation, typed transfer and reusable immutable-input preparation. | They do not become completed-state authority. Avoid re-materialization when native selection suffices. |
| petgraph and specialized kernels | Retain for projected topology, SCCs and algorithms whose semantics fit. | A library graph is a derived representation. Graph traversal does not implement transfer functions, proof semantics or all program analysis. |

Native graph traversal/GQL remains an eligible physical execution mechanism where its selected universe, multiplicity and traversal semantics match a named operation. Its availability does not justify moving model-owned BDD transfer functions or SCC analyses into ordinary traversal. Reference/view/event and approximate-vector candidates above were screened for fit; their full composed product integrations are not qualified by this review.

### Migration-library leverage

Independent inspection of official SurrealKit revision `fd7b075c7619138cf5b8704d9f1938a63e3c977f` confirms frozen-file preflight and skipping completed rollout steps. It also confirms that `execute_step` and `record_step_complete` are separate operations (`rollout.rs:2918–2972`), so steps must tolerate effect completion without the completion mark.

Its lock expires after 900 seconds. The keepalive task logs and returns when it loses ownership (`3907–3927`); that path does not establish cancellation of a running migration step.

SurrealKit can therefore remove generic rollout-planning and frozen-input machinery, but project drainage, original-operation authority, page atomicity and loss-of-authority effect fencing remain necessary. The useful comparison is **library planning plus a thin project execution contract** against the current installer—not complete adoption against complete rejection. Sources: [rollout implementation](https://github.com/surrealdb/surrealkit/blob/fd7b075c7619138cf5b8704d9f1938a63e3c977f/crates/surrealkit/src/rollout.rs), [frozen preflight](https://github.com/surrealdb/surrealkit/blob/fd7b075c7619138cf5b8704d9f1938a63e3c977f/crates/surrealkit/src/rollout/frozen.rs).

The inspected workspace is `1.0.0-beta.6` targeting SDK3.3.0; current documentation examples using another beta are discovery leads rather than the resolved contract. File-granularity unchanged-schema skipping does not replace declaration-level diffing, and examined rollout execution does not establish terminal readiness of every desired concurrent index. Adoption must preserve the current loader's checked readback/readiness obligation. [Official library documentation](https://surrealdb.com/docs/manage/schema-migration/library) and [rollout documentation](https://surrealdb.com/docs/manage/schema-migration/rollouts) supplied navigation; exact source settles the limitations above.

### Transport and engine mechanics

The pinned server contains progressive WebSocket `query_stream`/`query_cancel`; this is not merely a feature inferred from current documentation. The stock Rust WebSocket engine does not wire that capability into the current application route. Replacing the patched gRPC path therefore requires an adapter and equivalent same-connection cancellation, transaction export, terminal error and cleanup contracts. No net complexity advantage has yet been established. The [official RPC contract](https://surrealdb.com/docs/reference/rest-api/rpc-protocol#query_stream) describes provisional rows, statement completion and outer termination; the [pinned WebSocket implementation/tests](https://github.com/surrealdb/surrealdb/blob/238bfeb11f5725bebed370167656748df8067595/surrealdb/server/src/rpc/websocket.rs) and [streaming helper](https://github.com/surrealdb/surrealdb/blob/238bfeb11f5725bebed370167656748df8067595/surrealdb/server/src/rpc/streaming.rs) provide source-level evidence.

The current SDK provenance correctly distinguishes application `End` from physical EOF and retains exact-session finalization handles. The server timeout-stack repair is a separate maintenance obligation. Changing transport does not remove that server execution defect by itself.

Pinned RocksDB source captures a transaction snapshot and explicitly requires iterator/read-option lifetimes to retain the transaction. This supports the selected-backup design, while also making long snapshot lifetimes consequential for retained engine state and compaction. Narrowing the publication pin after snapshot capture is only eligible after proving the capture/retirement race and finalization ownership; it is not an automatic simplification. See [transaction snapshots and native metrics](https://github.com/surrealdb/surrealdb/blob/238bfeb11f5725bebed370167656748df8067595/surrealdb/kvs-rocksdb/src/lib.rs).

Read-only unit inspection on2026-10-10 observed `MemoryHigh=infinity`, `MemoryMax=infinity`, `CPUQuotaPerSecUSec=infinity` and no explicit `EffectiveCPUs` restriction. That does not establish an optimal RocksDB configuration. Cache widths, compaction behavior, pinned snapshot state, inline/blocking work and current concurrency need examination against the actual service configuration. Settings from another embedded or low-cache experiment cannot be transferred without that premise. Pinned `server/src/dbs/mod.rs:906–924` passes environment configuration and runtime worker width into the managed datastore. Native RocksDB metrics already expose cache/memtables, SSTs/snapshots and pending/running compactions/flushes (`kvs-rocksdb/src/lib.rs:550–637`); use those for a concrete diagnostic question before introducing equivalent bespoke instrumentation. See [official observability configuration](https://surrealdb.com/docs/manage/observability/configuration).

Upstream reports [#7583](https://github.com/surrealdb/surrealdb/issues/7583), [#7584](https://github.com/surrealdb/surrealdb/issues/7584) and [#7586](https://github.com/surrealdb/surrealdb/issues/7586) concern low-cache stalls, embedded configuration wiring and index execution behavior. They are bounded investigation leads, not diagnoses of this migration. Managed-server configuration differs from embedded setup; report versions, effective settings and matching symptoms must be established before transferring their conclusions.

## 6. Change, growth and failure scenarios

| Scenario and change kind | Expected owner and propagation | Assessment or settling evidence |
|---|---|---|
| Add a fact family — domain concept | Model declaration, codec/admission, explicit recovery adapter and actual consumers | Coherent route. Exhaustive recovery treatment must include evidence, empty domains and physical realization; no new closure interpreter. |
| Add another library release — instance/binding | Acquisition identity and exact dependency views | Existing immutable keys support coexistence. Old answers and caches remain valid for old exact inputs. |
| Add an analytic — composition/domain operation | Projection and settings declaration, kernel adapter and named consumer | Keep graph universe separate from output selector. Reuse graph/SCC preparation only across compatible consumers. |
| Change a missing root to present — instance | Exact root outcome and member/content token identity | Current explicit negative outcomes support invalidation. Reveal with previously absent root, changed physical payload and unchanged unrelated roots. |
| Substitute WS for gRPC — mechanism | Native transport and runtime endpoint boundary | Credible but unresolved integration burden; preserve progressive finality, backup and exact-session cancellation. |
| Substitute rollout planning — mechanism | Installer planning with retained host/native authority | SurrealKit is a qualified candidate for generic planning, not a fencing replacement. |
| Sparse query over larger unrelated retained state — growth | Reader and search access paths | F04 and F05 currently violate fit; F06 concerns crossings within the sparse window. |
| High-degree ownership — skew | Retirement’s bounded edge pages | Current semantic protocol is credible; scalar preparation remains F07. Bound actual edge effects and bytes. |
| Concurrent compile, serve and worktrees — deployment | Stable service, logical attempts, exact pins and shared resource policy | Source ownership is coherent. Actual contention, cancellation and maintenance composition remain qualification obligations. |
| Late migration interruption — failure/recovery | Exact upgrade checkpoint | F02 requires durable page progress; F01 requires every intermediate state to remain compatible. |
| Unknown transaction acknowledgement — failure | Original request/outcome authority | Reconcile original identity; never acquire a fresh grant and infer the old work failed. |
| Backup concurrent with publication/retirement — composition | Snapshot, pin and finalizer | One coherent source snapshot is credible. Require race and terminal-tail controls before pin-lifetime narrowing. |
| History collection with active references — lifecycle policy | Explicit consumers, closed horizon and collector | References protect records; no age or resolved-flag shortcut. Batch execution must preserve this. |

### Global coordination and legacy retention

The global installation-row guard creates conflict between otherwise disjoint ordinary effects. `authorization.rs` uses that write to order era cuts and admission against execution. Ordinary effects also carry intent, execution and release transactions.

This is a material concurrency tradeoff, not evidence that the guard can be removed. Under snapshot isolation, replacing its conflicting write with an optimistic read can reintroduce delayed-operation/write-skew errors. First remove unnecessary crossings and batch actual work. A narrower protocol remains eligible only with equivalent ordering and recovery exclusion. Existing optional narrowing does not excuse assessment, but neither does it establish a defect without an equivalent alternative.

Migration outcome references are intentionally conservative. The source creates exact migration-owned references and history respects them; a generic owner/kind-checked release mechanism exists. The coordinator’s HS6 inventory explicitly protects legacy outcomes and has not enabled automatic release. Absence of an automatic migration-specific release caller is therefore **not reported as a leak**. Named consumers, sufficient retained provenance and qualified explicit release remain obligations before collection.

## 7. Host service, testing and operational boundaries

The stable service outside a checkout, main/validation scope separation and logical fixture attempts are appropriate for this workload. They avoid service creation and schema installation per test while permitting concurrent worktrees. Ordinary attachments do not substitute a scratch service when validation is unavailable.

Host maintenance closes borrower admission and waits for actual release. Process disappearance alone is not remote database drainage. Immutable installer and server generations make recovery reviewable; a worktree’s HEAD is not proof of the installed executable.

These choices also share an engine failure and cache/compaction domain between logical scopes. That is a reasonable simplicity tradeoff for one operator, but it requires truthful blocking and composed maintenance qualification. Test success, fixture cleanup, child exit, remote completion and service readiness remain separate facts.

The root/database credential distinction and same-installed-database cache validation are coherent. The current local-gRPC endpoint restriction is a mechanism choice; it should change if a different qualified transport is selected.

Observability already has the right authority direction: full logs and structured receipts own actual outcomes, while displays and status prose summarize them. The latest migration failure must be reflected in that summary. There is no need to introduce another progress ledger, blanket healthy-command timeout, worker cap or general audit framework.

Tests should continue to reuse immutable prerequisites while independently challenging the decision under test. Fresh malformed records, exact predicates and maintenance ownership for global DDL/crash controls are appropriate. Neither repeated setup nor a second test-only validator should become the oracle. Revealing tests for these findings belong in the existing native and host boundaries; no assembled qualification was run by this review.

## 8. Alternatives and the recommended direction

| Alternative | Benefit | Cost and decision |
|---|---|---|
| Keep the current implementation | Preserves existing ownership and avoids transition work | Leaves invalid legacy transition, disproportionate replay and known read/crossing amplification. Not accepted. |
| Correct operation boundaries within the present architecture | Reuses exact identities, native indexes, library kernels, snapshot export and host authority | Requires focused migration, catalog-preparation and set-query changes. Recommended. |
| Adopt SurrealKit planning under project fencing | Removes generic frozen-plan and rollout bookkeeping | Adds integration and version ownership; does not solve semantic backfills or page recovery automatically. Evaluate with F01/F02. |
| Move all graph/relational execution into SurrealQL | Can reduce movement for suitable indexed operations | Would relocate transfer, proof and specialized analytic semantics and may create large server intermediates. Use selectively, not universally. |
| Move all selected work into Rust/DataFusion | Uniform local compute machinery | Transfers extra rows and reconstructs work the database can nominate selectively. F04–F07 show why this is not the default. |
| Replace the store or introduce a universal state framework | Could change engine/tooling constraints | No demonstrated complete alternative removes immutable content, membership, recovery, pins, evidence and authority obligations with less total machinery. Revisit only for a concrete capability or operational limit. |
| Rebuild disposable validation state | Potentially simplest recovery for an independently proven reconstructible scope | Cannot infer disposability from the database name. Protected outcomes and consumers must be accounted for; not the current authorized remedy. |

The corrections fit together without requiring a new platform. A complete migration transition precedes durable progress design; durable progress makes bounded effects meaningful during recovery. Operation-owned catalog preparation removes repeated setup around those effects. Typed nomination and set membership let ordinary reads and search avoid unrelated work. Lifecycle batching uses the same physical principle while retaining a stricter effect contract.

Shared terminology does not make these one defect. F01 is a validity-of-transition problem; F02 is recovery amplification; F03 is repeated assurance preparation; F04 is overbroad selected work; F05 is missed native nomination; F06 and F07 are scalar crossings in different ownership regimes.

## 9. Independent judgments

### Architectural judgments

| Judgment | Verdict | Basis |
|---|---|---|
| **A1 — Localize change** | **Satisfied for inspected semantic and consumer boundaries** | Model declarations, physical realization, publication, kernels and host lifecycle have coherent owners. Proposed corrections should stay there rather than create another coordinator. This is not certification of every extractor or analytic. |
| **A2 — Encode domain meaning explicitly** | **Violated** | Catalog/content distinctions are adequate and govern behavior, but the supported upgrade lacks a complete compatible intermediate-state and durable-progress contract: F01/F02. |
| **A3 — Extend through composition** | **Satisfied for inspected domain compositions** | Exact views, declared projections, specialized kernels, publication and serving compose through explicit identities. Set-query deficiencies require local interfaces, not a universal framework. |
| **A4 — Fit execution to the supported workload** | **Violated** | F02–F07 establish avoidable replay, repeated unchanged preparation, unrelated selected work and singleton crossings under realistic growth/skew. No measured speed claim is needed for these diagnoses. |

### Correctness and fidelity gates

A source-scoped pass below is an architectural assessment of the inspected mechanism, not fresh runtime qualification.

| Gate | Verdict | Independent evidence and limit |
|---|---|---|
| **G1 — Authority** | **Pass, source-scoped** | Model meaning, exact content identity, runtime authority and derived representations have separate owners; shared recovery declarations remove competing closure interpretations. |
| **G2 — Semantic fidelity** | **Pass, source-scoped** | Physical/nominal identity, absent/virtual/present, eligibility/ranking and exact/heuristic distinctions are explicit in inspected paths. No silent collapse is established by the performance findings. |
| **G3 — Validity** | **Pass for inspected rejection boundaries** | Admission, typed reconstruction and schema checks reject invalid states. F01 rejects valid upgrade input; it is not evidence that invalid content was admitted. |
| **G4 — Hidden behavior** | **Pass, source-scoped** | Inspection, selection, setup and effectful maintenance have explicit ownership. No silent fallback service or undeclared operator adoption was found in the examined routes. |
| **G5 — Consistency and recovery** | **Unresolved for composed current-candidate operation** | Publication-last, exact requests, pins, snapshots and finalizers are credible; the failed migration stayed closed. Current patched transport/engine, interruption, cancellation and recovery composition still require their scheduled actual controls. F02 separately violates A4. |
| **G6 — Transformation and reuse** | **Unresolved for assembled qualification** | Exact keys, negative/member tokens, graph lifetimes and charged preparations are strong source evidence. Final composed cache/projection/resource controls for the dirty implementation are not established by this review. |
| **G7 — Truthful capability claims** | **Pass subject to current receipt reconciliation** | Source, proposed and runtime boundaries are explicit. Earlier “translation running” status must yield to the later failure; candidate-specific evidence must not transfer to newer source. |
| **G8 — Library leverage** | **Fail** | F05 establishes an available native table-source nomination capability that the current route bypasses because of a narrower record-source limitation. Frozen scoring remains legitimately project-owned. SurrealKit and WS are alternatives requiring further selection, not additional established failures. |
| **CI-G1 — Fidelity** | **Pass for inspected persistence/serving distinctions** | Attribution, model qualification, unknown states and heuristic ranking remain explicit. Complete provider semantic coverage was not re-audited. |
| **CI-G2 — Evidence closure** | **Unresolved for current assembled realization** | Shared closure, actual-row checks and same-realization pinning provide a credible route; composed publication/cold-restore/serving evidence controls remain required. |
| **CI-G3 — Evaluation integrity** | **Not applicable to the changed mechanisms reviewed** | Protected evaluation construction and tuning were not reviewed or changed. This is not certification of the evaluator. |

Applicable foundation verdicts follow these independent judgments: FP-01/02/03/06 are satisfied at the inspected semantic ownership boundaries; FP-04/05 are violated by the migration operation’s incomplete transition/progress contract; FP-07 is violated by F02–F07. DP-13/14/16 are violated for F05’s generic nomination route, while specialized kernels and frozen scoring have stated semantic reasons. DP-19/20 and relevant CI fidelity/lifetime rules have credible implementations but retain the composed qualification limits above.

## 10. Verification, remaining investigation and closure

| Evidence or check | Outcome |
|---|---|
| Read-only source, owner, standard and pinned-interface inspection on 2026-10-10 | **passed** for the cited review investigation; `rg`/`sed` inspection of the named executing paths and exact upstream source |
| Exact failed installer identity, `sha256sum /home/paul/.local/state/library-context/surrealdb.tools/lctx/53f28eeb61887c168fdd2ecb35a662d8aea35b1bfd7b53779b5b290992f14369/lctx` | **passed**,2026-10-10; exact digest above. Python byte inspection corroborated the omitted-field UPDATE literal. |
| Root publication check, `just docs-check` | **passed**,2026-10-10;383 canonical pages, zero link errors. Initial run failed on the pre-existing stale ADR index; `just adr-index` regenerated it from existing records before the successful rerun. No decision prose changed. |
| Historical validation migration, run `20261010T174350.233Z-05d48c` | **failed**; mandatory `root_count` received `NONE`, transaction aborted, admission closed |
| Current-source builds and focused native/host controls | **not_run** |
| Database probes, planner queries and performance measurements | **not_run** |
| `just qualify` and real-library/operator adoption | **not_run** |

Static evidence is sufficient for the seven diagnoses. The following bounded investigations affect remedy selection or enclosing acceptance:

- **Migration planning:** compare a small project checkpoint with SurrealKit planning under retained effect fencing. Settle complete field transformation, atomic progress and changed-candidate handling together.
- **Native modeling:** examine specific membership/hold existence obligations against `REFERENCE ... REJECT`, and specific aggregate/view consumers against native maintained forms. Adopt only where they delete real duplicated enforcement without changing shared retirement meaning.
- **Search:** qualify table-source nomination’s analyzer completeness and actual planner composition while retaining frozen ranking and exact eligibility.
- **Transport:** compare the complete progressive WS adapter—including backup and terminal cancellation—with the maintained gRPC patch. Feature availability alone does not select it.
- **Cache and lifetime:** finish absence/member mutation, in-flight cancellation, retention expiry and shared-budget controls. Investigate snapshot-pin narrowing only against the actual race.
- **Engine/service:** relate actual RocksDB cache/compaction/snapshot behavior and server execution widths to the installed service configuration and concurrent compile/serve workload.
- **Coordination:** consider narrower installation guards only with an equivalent delayed-operation/era-cut protocol.
- **Legacy release:** qualify named migration consumers, sufficient provenance and explicit release before destructive history collection.
- **Notifications:** investigate LIVE/changefeeds only when a concrete consumer requires them; they are not a substitute for durable outcomes.

Unexamined breadth includes a full analyzer semantic audit, every behavioral kernel, complete security configuration, whole-service cold-disaster recovery and protected evaluation. These exclusions do not themselves create findings. Missing current composed recovery and evidence-closure premises do affect enclosing acceptance.

## 11. Rule impacts and disposition

These are proposals, not applied decisions. Finding status must not be copied into a second ledger.

| ID | Rule or contract affected | Proposed treatment and dependency | If retained unchanged |
|---|---|---|---|
| <a id="RC01"></a>**RC01** | ADR-0145’s version-3 cutover contract, storage §6 and the native-upgrade runbook | Make complete intermediate-state compatibility and durable atomic pass/page progress explicit, preserving its existing immutable-content, original-authority and protection guarantees. F01/F02 depend on this clarification. Use the normal owner/ADR route where the current mechanism is bound; do not rewrite an accepted ADR in place. | A field-complete correction can repair F01, but coarse replay remains F02 and prevents architectural acceptance of supported recovery. |
| <a id="RC02"></a>**RC02 — conditional** | Local-gRPC endpoint restriction, transport ownership and SDK hold in `docs/pins.md` | If the complete comparison selects progressive WS, revise the endpoint/transport rule and retire the SDK patch only after equivalent finality, cancellation and backup qualification. | Retain gRPC and its narrow patch; F01–F07 remain actionable. No current finding requires a transport switch. |
| <a id="RC03"></a>**RC03 — conditional** | Initial global installation coordination in ADR-0145 and the already accepted holistic RC03 | A narrower protocol remains eligible only with equivalent era-cut, delayed-execution and recovery exclusion. This is the existing optional decision, not a second approval or a requirement to remove the guard. | Retain coordination and reduce avoidable work around it. F07’s batching obligation remains. |

F03–F07 otherwise refine existing declared bounded-work, sparse-read and native-execution targets. They do not require changing semantic ownership, making approximate search exact, weakening independent restore, replacing the store, or widening operator activation. SurrealKit selection would require ordinary exact dependency and integration ownership; its availability does not mandate adoption.

## 12. Decision and priority dependencies

**Bounded decision: Revise. Enclosing architecture: needs revision, with composed runtime qualification unresolved.**

The semantic graph and immutable-publication foundations are suitable for the functional target. They do not excuse incomplete migration semantics or avoidable physical amplification. The recommended changes retain their useful guarantees while making more of each complete operation owned, prepared once, selectively nominated and executed in bounded sets.

Priority follows consequence:

1. **Make the valid legacy transition complete and recoverable — F01/F02.** The native upgrade owner must settle intermediate states and durable progress before another long attempt can establish meaningful acceptance.
2. **Remove disproportionate sparse-query work — F04/F05/F06.** Reader and search owners must preserve exact eligibility, ordering and ranking while avoiding unrelated membership and scalar crossings.
3. **Remove repeated setup and maintenance crossings — F03/F07.** Loader, publisher and control owners should reuse operation-valid preparation and batch exact effects without weakening independent checks.

Prerequisite order differs from urgency. Complete transition semantics enable safe durable page recovery. A per-node membership result enables batched occurrence eligibility. Frozen lexical ranking is a preservation constraint on native nomination. Collection batching depends on qualified references and horizons; it does not authorize earlier disposal.

The next architectural decision belongs to the native upgrade owner with the coordinator: choose the complete compatible transition and atomic recovery checkpoint contract, then reconcile the seven findings into the existing disposition owner. No restart, migration repair, destructive collection or operator activation is performed or authorized by this review.
