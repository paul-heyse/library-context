# Persisted execution: architectural causes behind the correction sequence

**Design tier · target purpose · deep review · 2026-10-07**

## 1. Assessment and review boundary

**Decision: Revise.** The persisted compiler has useful foundations: immutable completed contributions, exact view and vocabulary bindings, explicit semantic admission, independent external-state reconciliation, and publication separate from selection. The recent corrections strengthen those foundations. They do not yet establish an adequate complete execution route across compiler, publication, restore and published querying.

The failures have several architectural causes. Captured facts were interpreted through executable identities; native selection assumptions were independently reproduced in different consumers; bounded payload delivery followed unbounded native candidate preparation; generated physical fields grew from the global declaration inventory rather than their physical consumers; and effectful workflows made failure reporting and transport granularity local choices. These causes explain why repairs at one site repeatedly exposed a related problem at another.

The recommended direction retains the useful semantic and lifecycle distinctions while completing their operation contracts. Rust should continue to own fact meaning, coverage, exact dependency universes and publication admission. SurrealDB should execute qualified indexed and bulk operations without intermediate whole-match arrays where a streaming route suffices. DataFusion and Arrow should retain compact projected computation, and Tokio should retain acknowledged task drainage. None of these recommendations requires another backend, a general executor, cross-run resumability, a permanent query-proof register or a performance-accounting system.

| Review field | Boundary |
|---|---|
| Functional target | A pinned API and evidence catalog whose answers preserve provider attribution, coverage, original evidence and exact realization identity; retained behavioral analysis is enrichment. |
| Standard | Core/template 3.3, heuristics companion 1.0, code-intelligence profile/review additions 1.5, and the library-context binding declared by [standard.toml](../design_principles/standard.toml). |
| Principal reviewer | Fresh delegated design reviewer; the coordinator publishes this judgment. |
| Primary correction window | `32a2b9fb` through `1be0dd96`, approximately 16:34–18:34 EDT on 2026-10-07. |
| Current source | `1be0dd96` plus the four explicitly included uncommitted files below. |
| Scope expansion | Upstream declarations and analogous consumers followed when they explain a correction or expose the same architectural assumption. This is not a complete pivot audit. |
| Workload premise | Multi-provider library captures; growing fact and semantic-kind inventories; exact selected closure with skew and shared targets; both compilation profiles; private publication, external transport and restore; version-scoped evidence requests over a published realization. |
| Deployment premise | One operator with managed local native storage, concurrent owned tasks and possible interruption. Private attempts may be discarded; uncertain effects must remain distinguishable. |
| Exclusions | Real-library/Qwen qualification, operator activation, protected evaluation populations, quantitative performance qualification and the unrelated agent-workspace work. |
| Evidence method | Read-only source, types, current owners, commit changes and attributed historical receipts. No new probes, builds, tests or operator operations. |

The included dirty files are [artifact_manifest.rs](../../../crates/cpg-core/src/artifact_manifest.rs), [facts.rs](../../../crates/cpg-core/src/facts.rs), [workspace.rs](../../../crates/cpg-core/src/workspace.rs) and [semantic-model.md](../../design/sections/semantic-model.md). Their current changes are **Implemented, uncommitted and unqualified**. In particular, the dirty `AdmittedFacts` change shares recorded-provider reporting with manifest construction; it is not a successful restored-artifact receipt.

The unrelated dirty evidence index, agent-workspace evidence folder and agent-workspace plan were preserved and excluded.

[STATUS](../../../STATUS.md) and [persisted execution plan §9.1](../../plans/persisted-graph-execution-plan_2026-10-07.md#91-current-execution-checkpoint-2026-10-07) own current execution evidence. Existing scheduled findings retain their disposition there. The new findings in this review remain unscheduled; publishing this review does not authorize remediation.

## 2. Responsibilities and governing distinctions

The [product owner](../../design/sections/api-and-evidence-product.md), [semantic owner](../../design/sections/semantic-model.md) and [storage/publication owner](../../design/sections/storage-and-publication.md) establish the functional context. Their selected mechanisms and earlier ADRs describe the subject under review; the standard and functional target govern this target judgment.

| Responsibility | Owner and consumer contract | Relevant reason for change |
|---|---|---|
| Attributed facts, coverage, nominal identity, model invariants | `lctx-model`; consumers preserve provider/run/context, relationship meaning and explicit unknowns. | A new fact family, semantic distinction or coverage policy. |
| Provider implementation and extraction | `cpg-extract`; explicit captured inputs produce bounded typed contributions. | Analyzer APIs, extraction behavior and implementation builds. |
| Completed contributions, exact views, frozen bindings | Model contracts realized by `lctx-surrealdb`; pending values do not establish presence or absence. | Physical access, storage layout and acknowledged completion. |
| Compilation, semantic admission and prepared input reuse | `cpg-core`; operations consume exact immutable premises. | A new compilation composition or an admission boundary. |
| Native access and generated physical projections | `lctx-surrealdb`; preserve exact demand, residual predicates, terminal success and mappings to nominal keys. | Planner capabilities, query shapes and physical layout. |
| Publication and external-state reconstruction | `lctx-publisher`; seal admitted content or independently admit transported content before exposing a handle. | Transport, reconciliation and visibility obligations. |
| Published selection and packets | Model operations plus `lctx-serving`; evidence remains relative to one complete realization. | A new answer or rendering, without reinterpreting source authority. |
| Owned asynchronous and blocking work | Attempt owners, native calls and bridge; cancel admission, retain submitted work, drain before releasing its resources. | Scheduling and runtime placement. |

The most consequential distinctions are:

- A provider capability declaration is different from a captured provider implementation and invocation.
- A nominal reference is different from physical owner presence in an exact view.
- A sum variant’s semantic identity is different from its flattened physical columns.
- An output batch limit is different from the working set needed to prepare its candidates.
- Primary failure, drainage failure and uncertain cleanup are different outcomes.
- A semantic declaration, SQL statement, transaction and transport request need not be the same physical execution unit.
- Ordinary semantic admission, producer replay and independent external-state reconciliation establish different properties.

The correction sequence repeatedly crossed these distinctions without a complete shared contract. The resulting repairs should be assessed as operation-boundary corrections, not isolated message, query or fixture changes.

### Fact and fidelity boundary

| Family or relation | Authority and fidelity | Coverage/identity obligation | Reviewed consumers |
|---|---|---|---|
| Provider coverage and invocations | Captured provider/run/context records; provider-attributed observations. | Independent family/profile/scope obligations; empty, partial and NotRequested remain distinct. | Facts availability, manifest outcomes, cold admission. |
| Signature type subjects and observations | Typed `Parameter`/`Return` sum and attributed typing observations. | Variant identity, qualification and scope remain intact; alternatives survive. | Operation-core packets. |
| Call/event ownership and membership | Explicit semantic selection over exact table/view epochs. | References cannot fabricate physical owners; contextual and ordinary ownership policies remain separate. | CallScopes, selected native closure, SourceCall analogues. |
| Parameter naming links | Derived normalized relation with explicit direction. | Incoming entity links needed by S0 are part of its selected evidence universe. | S0 selection and parameter rendering. |
| Native scope projections | Mechanical derived fields and indexes. | Never become a second fact authority; absent and null handling match the operation. | Compiler reads, published readers, projections and serving functions. |

Evaluation algorithms and protected populations were not reviewed. The fixture discussion concerns independent admission versus producer replay, not acceptance of evaluator quality.

## 3. What the corrections establish—and what they do not

The following episodes are causal evidence. Their current implementation status must not be confused with new execution by this review.

| Episode | Assumption exposed | Current assessment |
|---|---|---|
| Exact-key demand and compound membership selection, `32a2b9fb` | A nominal `IN` predicate and a compound index were assumed to preserve a selective physical route. | Writer-derived membership RecordIds and exact contributors improve point access. Historical native controls are attributed in §9.1. |
| Grouped forward fields, `af9a0339` | Each semantic reference field became another physical projected read. | One exact table/frontier projection now preserves distinct target epochs while sharing source work. Preserve this correction. |
| Prepared scope constants/index drivers, `9d7f4d27` | Inline mapping/casts inside an indexed predicate were assumed to be planner-visible constants. | Compiler correction is implemented and historically tested; analogous published sites remain. |
| Signature ports, `acc1d0ef` | Semantic sum ports could be selected through physical field names. | Canonical `SignatureTypeSubject` identities now distinguish parameter and return subjects. Preserve the model’s sum semantics. |
| Original-error preservation, `b3e1cc84`/`03380c43` | Cleanup sequencing could replace the explanation of the failed operation. | Particular paths improve; sibling workflows still have inconsistent completion outcomes. |
| Bounded restore parsing, `6963c2f0` | A complete dump was an appropriate single HTTP import unit. | The pinned parser preserves statement/transaction boundaries. One-request-per-unit remains avoidable amplification. |
| Exact existing owners, CallScope and S0, `c327adba` | Reaching an owner key was equivalent to finding that owner; incoming naming relationships could be omitted. | Exact owner presence and incoming parameter links are implemented. Ordinary/contextual policies must remain deliberate. |
| Recorded providers, `3d857bf8`, plus dirty `AdmittedFacts` | Cold admission could regenerate provider identities and reporting from the running executable. | Recorded identity binding is implemented; dirty reporting sharing closes another local duplication. The complete binding contract and focused restored acceptance remain open. |
| Generated declaration windows, `1be0dd96` | All generated setup declarations could share one query deadline. | Batching is implemented and compile-checked. The cited setup timeout does not identify its exact failing statement, and the latest runtime repair is not established. |

The restore failure after parser pinning is especially revealing. [Provider build identity](../../../crates/cpg-extract/src/bundle.rs), lines 29–52, includes the lockfile and source closure. Adding the pinned parser changed current executable provider IDs. Those new IDs properly identify new extraction, but do not redefine already captured facts. A cold reader must bind the captured supplier under an independently supported semantic contract.

The restore journey reached import, cold state validation and native copy before failing expected coverage admission. That is materially different from the earlier HTTP import timeout. The later failure demonstrates an attribution/admission boundary problem; it does not invalidate the parser correction or prove the whole restored journey now succeeds.

The availability regression initially attempted replacement of a completed owner. Correcting that fixture preserves immutable ownership and provides a legitimate rejection case. Moving generic fixtures from producer replay to semantic admission also preserves the right distinction: a production admission failure is not interchangeable with a diagnostic replay mismatch.

## 4. Findings

| ID | Architectural cause | Primary owner | Consequence |
|---|---|---|---|
| [F01](#f01) | Failure completion is locally reconstructed. | Attempt/workflow completion owners. | Primary cause and uncertain cleanup can be lost or confused. |
| [F02](#f02) | Captured-provider binding still needs a second interpretation of executable declarations. | Provider declaration/admission contract. | Ordinary provider changes require hidden central edits and can refuse valid cold captures. |
| [F03](#f03) | Indexed scope lowering is independently authored across consumers. | Shared native access. | A compiler planner correction does not propagate to published evidence operations. |
| [F04](#f04) | Bounded delivery follows whole-match native candidate materialization. | Native selected access. | High-degree selections retain growing candidate arrays before delivering bounded rows. |
| [F05](#f05) | Global semantic inventory automatically expands physical fields on every backing table. | Generated native schema. | Semantic-kind growth adds setup/write machinery unrelated to individual physical consumers. |
| [F06](#f06) | Restore equates an execution unit with a transport request. | Restore import. | Small independent statements cause repeated HTTP crossings and tempfile writes. |

### <a id="f01"></a>F01 — Attempt completion does not preserve a complete failure outcome

**Diagnosis: Implemented source defect, 2026-10-07.** Recent fixes preserve original errors in particular paths, but siblings independently decide which failures survive.

In [CLI compile](../../../crates/lctx/src/compile.rs), lines 126–137, successful abandonment is followed by `drained?`, which can replace the primary compile failure. The `Workspace::new` failure branch at line 56 can similarly replace its construction error with abandonment failure.

[Publisher](../../../crates/lctx-publisher/src/lib.rs), lines 23 and 50, sequences `abandon(...).await?` and `drained?` after a failed publication operation. Either may replace the primary cause. [Facts inspection](../../../crates/cpg-core/src/facts.rs), lines 109 and 129, discards drainage errors. [Restore](../../../crates/lctx-publisher/src/backup.rs), lines 193–200, has its own `cleanup_result` rule; import lines 220–224 preserve the primary import error but discard simultaneous invalidation failure. Invalidation is session cleanup, not proof of native writer drainage, so these exposures must not be conflated.

**Cause and consequence.** Completion is an owned domain operation, but its observable result is still determined by local `?` order or string concatenation. A failure with unconfirmed cleanup can be reported as an ordinary primary error, while a cleanup failure can obscure the defect that initiated cleanup. This weakens local reasoning and recovery decisions even when incomplete content remains fail-closed.

This review does **not** establish that these paths publish partial content. Native failure/admission guards, terminal drainage and private database ownership are preservation constraints.

**Proposed correction.** One ordinary completion operation should consume the primary result plus owned drain/cleanup outcomes and produce an outcome that retains both the original cause and cleanup certainty. Workflow-specific effects remain with their owners. Human messages are projections of that result; a general retry framework is unnecessary.

**Counterexample.** A semantic rejection followed by successful cleanup must remain a semantic rejection. Conversely, a successful computation followed by unconfirmed cleanup cannot become success. Combining both cases into a generic infrastructure string loses required distinctions.

**Closure.** Inspect all identified siblings through the shared operation; use focused failure combinations to distinguish primary-only, cleanup-only and simultaneous failures, including an interrupted drain retried to completion.

**Principles/judgments:** FP-01/05/06, DP-19/21; A1/A2/A3 and recovery aspects of G5/G7.

**Disposition:** Unscheduled required completion-contract decision; this review owns the finding until transferred.

### <a id="f02"></a>F02 — Recorded facts admission still reconstructs the supplier contract

**Diagnosis: Implemented architectural gap, including dirty source, 2026-10-07.**

[Recorded coverage](../../../crates/cpg-core/src/facts.rs), lines 147–199, constructs current executable declarations with a sentinel configuration, separately enumerates six provider prototypes, identifies contributions by producer name, applies named configuration exceptions and conditionally compares build identities. `Stage.coverage` in [stages.rs](../../../crates/lctx-model/src/domain/stages.rs), lines 164–176, carries concrete provider IDs. It does not itself supply the stable role/binding contract needed to interpret recorded suppliers independently of executable builds.

The dirty `AdmittedFacts` change usefully shares availability and reporting with [manifest outcomes](../../../crates/cpg-core/src/artifact_manifest.rs), lines 163–183. The [workspace cache](../../../crates/cpg-core/src/workspace.rs), lines 385–448, binds profile and all nine exact completed views. Preserve those corrections. Neither removes the central knowledge about prototype suppliers and configuration categories.

**Cause and consequence.** Executable extraction declarations and captured implementation bindings were initially treated as one authority. The correction now remaps between them using another hand-maintained interpretation. Adding a covering provider or changing its configuration binding requires both its declaration and knowledge in the central prototype/exception logic. A declaration can be valid for extraction while cold admission cannot identify its supplier.

This is not an allegation that arbitrary altered configurations currently pass all admission. It is an extension and authority-boundary defect demonstrated by the separate binding knowledge.

**Proposed correction.** Give the provider declaration/admission owner a complete ordinary contract separating:

1. family/profile/scope obligations and supported semantic contract;
2. the provider role satisfying those obligations;
3. captured implementation, configuration, invocation and exact source bindings.

Extraction binds an executable implementation to that contract. Cold admission binds recorded implementations without constructing default executable instances or independently classifying configuration exceptions. Availability and manifest reporting consume the same admitted binding result.

Retain independent obligations: coverage rows cannot choose their own expected supplier, and exact views cannot be widened merely because a recorded provider is inconvenient.

**Counterexample.** A parser-only executable rebuild should not invalidate a compatible captured supplier. An analyzer revision that changes fact meaning must not automatically gain compatibility merely because its tool name matches. Unknown/incompatible contracts require explicit refusal or a deliberate migration.

**Closure.** Trace a new provider/family binding from its authoritative declaration through extraction, cold admission and reporting without a second supplier list or configuration classifier. Focused cold controls must preserve compatible captured builds and reject absent, extra, ambiguous and incorrectly attributed suppliers/outcomes.

**Principles/judgments:** FP-02/03/04/06, DP-01/04/05/08/09/24, CI-01/04/10; A1/A2/A3 and G1/G6.

**Disposition:** Unscheduled required binding-contract decision. Current restore acceptance remains with plan §9.1.

### <a id="f03"></a>F03 — Shared native selection leaves planner-sensitive lowering in consumers

**Diagnosis: Implemented propagation defect; particular published plan shapes remain unresolved.**

The compiler correction prepares scope constants before indexed selection in [compiler.rs](../../../crates/lctx-surrealdb/src/compiler.rs), lines 768–778. Analogous expressions remain in:

- [NativeReader records](../../../crates/lctx-surrealdb/src/reader.rs), lines 118–127;
- [native projection roots](../../../crates/lctx-surrealdb/src/projections.rs), lines 229–231;
- [member-selection function](../../../crates/lctx-serving/src/selection.rs), line 104;
- [capability-search selection](../../../crates/lctx-serving/src/operations.rs), line 481.

These sites build map/cast expressions inside scope predicates. The correction window already established that the same expression shape could hide the selective constant from the locked planner.

**Cause and consequence.** “Indexed scope selection” is treated as a shared capability, while the query-shape knowledge that makes it work remains distributed among compiler, reader, projection and serving owners. Correcting the compiler therefore leaves supported published journeys with the same unresolved physical premise.

This is source evidence of duplicated lowering and a material execution-fit gap. It is not a measurement that every cited query scans its entire table or exceeds a deadline.

**Proposed correction.** Shared native access should consume typed scope demand, table/family scope and residual policy, prepare planner-visible constants and choose a qualified physical route. Serving functions may execute natively while using the same lowering authority; moving every query to Rust is unnecessary. The prepared query must also carry its selected-result statement index and terminal/failure contract: `NativeReader::query` reads statement 0 and `records` reads statements 0/1, so merely prepending `LET` preparation changes the values consumed.

Keep exact membership/view constraints, null and missing behavior, context predicates, ordering and residual filters. Locked containment expansion limits justify finite windows; they do not justify truncating demanded scopes.

**Counterexample.** A request spanning more than 32 scopes must still return the complete selected union. A null-valued field requiring a different predicate cannot be forced through the non-null atomic index route. A published reader must retain its pinned realization rather than acquire a compiler-private authority.

**Closure.** Inspect the actual locked plans for the identified concrete uncertainties and route these consumers through the common lowering. Independent finite/native controls should challenge large scope demand, residual filters and absent/null distinctions.

**Principles/judgments:** FP-01/02/03/07, DP-08/10/14/15/16; A1/A3/A4, G6/G8.

**Disposition:** Unscheduled shared-access correction. Earlier compiler controls retain their existing plan ownership.

### <a id="f04"></a>F04 — Bounded selected streams prepare whole native match arrays

**Diagnosis: Implemented execution amplification, 2026-10-07.**

[Compiler scope selection](../../../crates/lctx-surrealdb/src/compiler.rs), lines 768–781, creates a `LET` array of matching IDs for each scope window, flattens/distincts the combined arrays, then selects and orders semantic keys. Only after this preparation do `SelectedRows` and membership/payload windows bound late hydration.

General selected/unfiltered paths in lines 791–809 likewise materialize candidate/member node arrays. Their complete universe may be semantically necessary; a whole resident ID array is not thereby necessary.

**Cause and consequence.** The design bounds request values and payload batches but lets match cardinality determine native preparation state before the first useful batch. A high-degree scope can fit a streamed answer while requiring large arrays first. A gRPC row stream cannot undo eager preparation inside its query.

This interacts with F03: a selective index driver can still feed an unsuitable whole-result intermediate. It also interacts with cancellation: until preparation yields, client backpressure does not govern that native working set.

**Proposed correction.** Compare a direct ordered native candidate stream with bounded exact-membership intersection and late projected hydration. SurrealDB 3.3 has ordered union-index execution, including deduplication for overlapping array-element branches; this is an **Interface-checked alternative**, not proof that the desired composed query already selects that plan.

Where exact membership/order cannot be expressed efficiently in one native operation, stream compact candidates into an existing bounded/spillable merge route. Keep the exact owner universe and do not hydrate unrelated rich bodies. No new general execution planner is required.

**Counterexample.** A physical row may match several scope values and belong to overlapping completed contributions. Naively concatenating windows duplicates it; a `LIMIT` before exact owner intersection can omit legitimate members. The substitute must preserve union semantics, exact owner filtering, nominal order and terminal failure.

**Closure.** Establish a concrete composed route whose large-match state follows a stream or bounded/spillable mechanism. Retain overlapping/null/shared-target and exact-epoch cases. Quantitative throughput or capacity improvement remains a separate measurement.

**Principles/judgments:** FP-07, DP-08/10/14/20, CI-08; A4 and G8.

**Disposition:** Unscheduled native-access decision; no supported result may be capped merely to close it.

### <a id="f05"></a>F05 — Global reference inventory becomes automatic physical machinery

**Diagnosis: Implemented generated-layout amplification, 2026-10-07.**

[Schema generation](../../../crates/lctx-surrealdb/src/schema.rs), lines 127–173, gathers the global atomic scope-field inventory from graph and compiler declarations. `scope_schema` installs a scalar `scope_<field>` definition for every inventory entry on each backing table and builds `scope_keys` from that same global set.

[Reconciliation](../../../crates/lctx-surrealdb/src/reconciliation.rs), lines 315–336, derives the corresponding physical values in Rust; loader construction also calls that derivation. Locked SurrealDB document processing visits table field definitions in dependency order during writes.

**Cause and consequence.** A new nominal reference anywhere enlarges physical declarations and value evaluation for tables that may never contain it. The shared array-element scope index is useful. Automatically retaining every scalar copy on every table is a separate decision, and the reviewed current sources show particular scalar consumers rather than a need for that universal projection.

Batching setup declarations treats the request deadline but leaves this expanded schema and write work. The earlier timeout does not identify an exact declaration; this finding therefore makes no claim that the global inventory caused that recorded timeout.

**Proposed correction.** Generate scope-key inventories from the relations actually lowered to each table. Retain scalar projections for named consumers and indexes, such as the reviewed context predicates, rather than automatically copying every atomic field. Prefer existing stored `VALUE` fields for indexed values; replacing them with indexed `COMPUTED` fields is not established as a supported alternative.

Preserve independent reconciliation. Expected-value reconstruction at an external trust boundary has a purpose distinct from pre-write duplication; deleting the audit merely because the server derives values would weaken assurance.

**Counterexample.** A field absent in one row kind may be valid in another row kind sharing the table. Inventory narrowing must use the complete table’s declared relation set, not a convenient sample. A current query using `scope_context` cannot silently lose that projection.

**Closure.** Trace a reference added to a compiler-only family: unrelated backing tables should not acquire irrelevant scalar definitions. Inspect actual scalar consumers and schema snapshots, preserve scope-key equality and independent reconciliation, and qualify the changed physical schema explicitly.

**Principles/judgments:** FP-01/04/07, DP-10/14/16, heuristics H2/H15/H27; A1/A4.

**Disposition:** Unscheduled physical-projection decision, including the rule impact in §8.

### <a id="f06"></a>F06 — Restore uses correct semantic units at scalar transport granularity

**Diagnosis: Implemented crossing amplification, 2026-10-07.**

[ImportUnits](../../../crates/lctx-publisher/src/backup.rs), lines 250–373, delegates lexical/AST boundaries to pinned `surrealdb-syn` and `surrealdb-sql` 3.3.0 and keeps explicit transactions whole. Preserve this correction.

`import_units`, lines 376–398, rewrites a tempfile and performs an HTTP import for every emitted execution unit. A nontransaction statement therefore becomes a separate file rewrite and awaited request even when many adjacent units are small.

**Cause and consequence.** Semantic execution-unit boundaries are correctly modeled, but also dictate transport boundaries. Restore work grows with both dump bytes and statement count through repeated crossings. The alternative is already compatible with the parser’s complete units and the native importer’s handling of multiple units.

**Proposed correction.** Aggregate contiguous whole execution units into byte- and statement-bounded import requests. Preserve initial/request-local `OPTION IMPORT`, order, whole transactions, limits and checked request completion. Oversized indivisible units still refuse honestly; increasing a deadline is not the correction.

**Counterexample and intentional change.** With several independent units in one request, later units may execute before the client observes an earlier unit’s error. The current scalar route stops before submitting the next unit. A batched route is acceptable only if the owner explicitly selects that private-staging failure behavior, abandons failed staging and never exposes it. A requirement for immediate stop between every unit would retain scalar requests.

**Closure.** Inspect selected importer semantics and verify ordered multi-unit requests, unchanged transaction boundaries, embedded semicolons, request limits and early/late failures. Preserve primary and cleanup information through F01. The parser tests alone do not qualify the new batching or whole restore.

**Principles/judgments:** FP-03/07, DP-10/14/19/20; A3/A4 and G8.

**Disposition:** Unscheduled transport-granularity decision, dependent on the explicit rule impact in §8.

## 5. Complete alternatives and preservation constraints

The six findings are related without being one defect. F02 establishes captured semantic premises. F03 gives consumers a common physical interpretation of demand. F04 completes that interpretation’s working-set behavior. F05 reduces unnecessary preparation/write machinery. F01 governs failed completion; F06 changes the unit whose failure that contract must report.

A conforming complete alternative is:

1. Bind captured suppliers to independently supported coverage contracts.
2. Produce one admitted facts result reused under its exact immutable premises.
3. Lower typed native demand through shared prepared scope/index selection.
4. Stream compact candidates through exact membership and late hydration.
5. Install only physical projections needed by the table and its consumers.
6. Transport whole import units in bounded batches under explicit private failure semantics.
7. Report primary and cleanup outcomes through one owned completion rule.

This is a **Proposed composition**, not an implemented replacement or accepted execution plan.

| Existing/library capability | Judgment and limit |
|---|---|
| SurrealDB 3.3 RecordId point reads and indexed scope access | Retain writer-derived membership mapping and verified owners. An index’s presence does not establish its chosen plan. |
| SurrealDB 3.3 ordered/deduplicating union-index execution | Interface-checked candidate for F04. Exact owner intersection and requested order still need composed-plan qualification. |
| Pinned SurrealDB AST parser/execution units | Retain for restore. Also preferable to extending declaration splitting by raw semicolons if generated declarations gain literals/blocks. No present broken generated literal was established. |
| Generated declaration windows | Retain coarse checked installation. The current `split(';')` route is explicitly restricted to finite generated declarations; do not generalize it into arbitrary SQL parsing. |
| DataFusion 55.1 streaming/provider capabilities | Retain projected native input, explicit pushdown and honest statistics. `StreamingTableExec` supports fetch; the wrapper currently hides delegation. |
| DataFusion fetch opportunity | A bounded improvement, not a full-scan diagnosis: outer `LIMIT` already stops polling. Native early termination must follow semantic ordering/filter completeness. |
| Arrow 59.3 coalescing | Retain the custom compact edge buffer’s stated ownership rationale. FixedSizeBinary takes the generic coalescing route, retaining slices before concatenation. A blanket replacement can increase retained backing memory. |
| Tokio 1.53.2 task ownership | Retain acknowledged task drainage. `JoinSet::shutdown` aborts tasks; it is not equivalent to completing already submitted native work. |
| Blocking work and futures-util 0.3.34 `Shared` | Retain durable join ownership through interrupted/retried drains. Running blocking work is not forcibly abortable; shared join results address a real lifetime need. |

The following are additional preservation constraints:

- Pending contributions never establish complete results or absence.
- Exact relation/view/vocabulary epochs are retained, including same-type different-epoch targets.
- Canonical sum identities remain independent of flattened Arrow/native columns.
- Ordinary and contextual ownership policies remain separately defined.
- Incoming parameter-link selection remains part of S0’s evidence universe.
- External import/reconciliation retains independent checks; ordinary compilation avoids self-import.
- No transaction spans extraction, CPU kernels or unrelated external files.
- Selection remains explicit and serving pins the complete realization.
- No compatibility store, historical runtime archive or resumability protocol is introduced.

The scoped library investigation found no basis for replacing all custom traversal, buffering or drain code simply because a generic library operation exists.

## 6. Change and failure scenarios

| Scenario and change kind | Expected ownership | Assessment |
|---|---|---|
| Parser-only executable build change; implementation binding | Captured supplier binding and current extraction remain separate. | Recent recorded-provider correction moves correctly; F02 leaves the binding contract incomplete. |
| Add a covering fact family/provider; domain extension | One coverage/provider contract plus genuinely new extraction behavior. | Central supplier/configuration knowledge causes hidden propagation in F02. |
| Grow scope degree/content; workload growth | Shared native demand preserves exactness while streaming necessary candidates. | F03/F04 leave planner visibility and preparation state incomplete. |
| Add a nominal reference to a compiler-only family; domain extension | Model declaration and relevant mechanical lowerings. | F05 propagates physical work to unrelated backing tables. |
| Replace dump-import request granularity; mechanism substitution | Preserve parser units, transactions, limits and private visibility. | F06 has a credible bounded-batch route, requiring explicit failure-semantics selection. |
| Cancel while native work or a join is outstanding; lifecycle failure | Attempt owner retains tasks and charges until drain completion. | Existing shared-join/drain mechanisms are useful; F01 leaves outcome reporting incomplete. |
| Select an orphan or a missing owner at an older epoch; semantic boundary | Exact selection must preserve legitimate unknowns without manufacturing owners. | Recent strict owner-presence correction is a strength; do not mechanically convert every contextual `own` to `own_existing`. |
| Render parameter/return typing and parameter names; composition | Canonical subject variants and incoming naming evidence. | Recent signature/S0 fixes preserve required distinctions; no additional typed-port defect established. |

## 7. Independent judgments, gates and evidence limits

### Architectural judgments

| Judgment | Verdict | Reason |
|---|---|---|
| A1 Localize change | **Violated** | F01/F02/F03 independently reproduce completion, supplier-binding and native lowering decisions; F05 expands unrelated physical machinery. |
| A2 Encode domain meaning explicitly | **Violated in the reviewed operation boundaries** | Captured-versus-executable supplier binding and complete failure outcomes are not yet governed through adequate shared contracts. Typed fact/coverage/sum records themselves are important strengths. |
| A3 Extend through composition | **Violated** | New provider bindings and analogous access consumers require hidden central or consumer-specific interpretations; complete failure/transport composition remains open. |
| A4 Fit execution to supported workload | **Violated** | Whole-match preparation, automatic physical-field expansion and scalar restore crossings introduce avoidable amplification. Particular replacement plans remain Interface-checked/Proposed; no speed claim is made. |

### Foundations and supporting rules

FP-01/02/03/04/06 are violated at the identified operation boundaries, with DP-01/04/05/08/09/19/21/24 explaining the ownership and distinction failures. FP-05 is partly satisfied by private construction, completion guards and exact view types, but incomplete for the full failure outcome.

FP-07 is violated by F04–F06 and materially unresolved at F03’s published plan sites. DP-10/14/16 apply to the identified repeated work and generated machinery. DP-15/23 require qualification of the selected replacement boundaries, rather than arbitrary blanket native/library adoption.

CI-01/04/10/13 remain preservation obligations. CI-02/03/05 are supported by the inspected typed variants, relationship identity and exact selection boundaries. CI-07/08 constrain physical alternatives: reachability must not replace semantic selection, and a bounded payload must not be described as a bounded complete operation. CI-09/12 were not independently assessed.

### Correctness and fidelity gates

| Gate | Verdict | Scoped evidence and limit |
|---|---|---|
| G1 Authority | **Unresolved** | Exact captured authority improves; F02’s complete supplier contract remains open. No new competing canonical store was found. |
| G2 Semantic fidelity | **Pass for inspected corrected distinctions** | Canonical signature sum identities, exact epochs and S0 incoming parameter links are preserved in source. This is not whole-product fidelity acceptance. |
| G3 Validity | **Unresolved for complete cold journey** | Shared semantic/reference/state checks remain. Latest restored outcomes and dirty admission/reporting changes are unqualified. |
| G4 Hidden behavior | **Pass for inspected boundary** | Acquisition, native effects and replay are explicit; cold admission should not acquire current extraction authority. No operator/protected-data action occurred. |
| G5 Consistency and recovery | **Unresolved** | Private incomplete-state refusal and terminal drainage are strengths; F01 loses required observable cleanup distinctions. No partial-publication failure was demonstrated here. |
| G6 Transformation and reuse | **Unresolved** | Nine exact views/profile bind facts reuse; current cold reporting and alternative selection composition remain open. |
| G7 Truthful capability claims | **Pass for current scoped reporting** | STATUS/plan distinguish implementation, partial receipts and pending acceptance. Uncommitted changes and unmeasured benefits remain explicitly limited. |
| G8 Library leverage | **Unresolved overall; revision required** | Established bulk/stream capabilities offer credible remedies for F04/F06, but their complete selected routes remain unqualified. Arrow/Tokio bespoke boundaries have concrete reasons to remain. |
| CI-G1 Attribution/fidelity/model | **Unresolved** | F02 and cold restored admission remain consequential. Recent sum/ownership corrections preserve fidelity. |
| CI-G2 Served evidence closure | **Unresolved for changed assembled realization** | Inspected corrections improve packet inputs; this review did not execute the complete final serving/restore journey. |
| CI-G3 Non-circular evaluation | **Not assessed** | Evaluation algorithms/population isolation were outside this causal review. Fixture admission/replay separation is not evaluator qualification. |

### Verification

**Product execution: not_run.** No product build, `just verify-*`, parser probe, native plan probe, real-library compilation or operator command was executed for this review. Coordinator documentation maintenance is separate from product verification.

Historical **Tested** claims are attributed exclusively to plan §9.1 and their named commands/logs. They include exact-view/closure controls, owner/skew controls, finite parameter/S0 controls, parser controls and bounded native/MCP journeys. They do not qualify this dirty source or a proposed alternative.

The retained nine-premise availability log failed at `NativeCompilerStore::begin` before coverage assertions. It cannot establish a coverage failure or identify the exact setup statement. The latest declaration-window check is compile-only. The retained bounded restore log establishes progress through import/cold validation/copy and failure at coverage admission; it does not establish completed restore.

The decisive remaining uncertainties are specific:

- the complete captured-provider/reporting contract and final cold admission;
- chosen plans at the identified published scope sites;
- a streaming exact-membership route preserving order/overlap;
- importer behavior when several independent units share a failed request;
- the actual consumers of retained scalar scope projections.

These affect the remedies and acceptance. Unexamined whole-pivot breadth is not itself a finding.

## 8. Rule impacts and disposition

Most recommendations refine the existing target: exact captured attribution, shared access, bounded bulk execution and explicit orphan reporting are already intended. They do not require reversing native persistence, direct sealing or frozen dependency foundations.

Two existing concrete behaviors would change:

| Rule impact | Current rule/location | Proposed change and dependency | If retained |
|---|---|---|---|
| <a id="rc01"></a>RC01 | Global atomic scope inventory automatically supplies scalar projections on each backing table: `schema.rs` 127–173; storage §6.1’s derived scope representation. | F05 narrows table inventories and retains scalar copies for named consumers. This is an explicit physical-schema change; update the owning section and schema contract through the existing decision route. | Keep the schema but retain F05’s generated/write amplification; batching setup does not close it. |
| <a id="rc02"></a>RC02 | Restore currently awaits each execution unit before submitting the next: `backup.rs` 376–398. | F06 batches whole units and permits later independent units in a private request to execute before the client observes an earlier failure. Select this internal failure behavior explicitly. | Keep immediate inter-unit fail-stop; scalar transport remains and F06 requires another conforming route or scoped consequence. |

**No other rule reversal is required.** F01–F04 refine intended contracts. Whole-lockfile provider build provenance may remain for new extraction; F02 does not require deleting it or accepting unsupported analyzer semantics. Parser ownership, exact membership, independent external reconciliation and terminal drainage remain.

| Finding | Current disposition | Required decision/revisit trigger |
|---|---|---|
| F01 | Unscheduled; source review owns it. | Select complete attempt failure outcomes before further sibling cleanup changes are treated as closure. |
| F02 | Unscheduled; source review owns it. | Select the captured-provider contract before closing restored coverage or adding another covering provider. |
| F03 | Unscheduled; source review owns it. | Resolve shared lowering and the identified published plan uncertainties before claiming selective access across consumers. |
| F04 | Unscheduled; source review owns it. | Resolve whole-match preparation before accepting high-degree selected streaming as a complete bounded route. |
| F05 | Unscheduled; source review owns it. | Select RC01 when revising setup/schema growth or adding nominal fields. |
| F06 | Unscheduled; source review owns it. | Select RC02 before changing restore request batching or accepting statement-heavy restore execution fit. |

Existing scheduled findings and prior correction statuses remain in the persisted execution plan. A subsequent plan should transfer these stable source IDs to one execution owner rather than create another status register.

Consequence priority and dependency order differ. Captured admission and complete failure reporting are the most immediate semantic/recovery concerns. Shared native lowering is a prerequisite for a consistent selected-access remedy. Candidate streaming and schema reduction address different growth dimensions. Restore batching depends on an explicit failure behavior and complete outcome reporting.

## 9. Decision and next architectural step

**Revise the reviewed compiler/publication/restore/access boundaries.** The current architecture has a credible core and several sound corrections, but A1–A4 are not satisfied for the reviewed change and growth scenarios. The remaining admission and recovery uncertainties also prevent correctness acceptance of the complete changed journey.

This decision is a target-design judgment, not remediation authorization, release qualification or rejection of SurrealDB. The enclosing persisted pivot remains in progress; areas outside the causal scope are not certified.

The next consequential step is for the coordinator/operator to create the bounded correction plan from these findings, selecting RC01/RC02 where desired and preserving the stated semantic, lifetime and assurance guarantees. Implementation should resume against those complete operation boundaries, with focused controls that challenge their counterexamples. Real-library work, live models, operator activation and quantitative performance remain separately authorized.

## 10. Source anchors and baseline reproducibility

The review and library researchers inspected the resolved source at the exact versions in
[Cargo.lock](../../../Cargo.lock), [workspace dependencies](../../../Cargo.toml) and
[native SDK dependencies](../../../crates/lctx-surrealdb/Cargo.toml). The source locators below
are relative to the local Cargo registry directory
`/home/paul/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/`.
They establish **Interface-checked** capabilities, not execution of the proposed compositions.

| Claim used in this review | Exact pinned source inspected |
|---|---|
| Containment expansion has a 32-value planning boundary and overlapping branches require deduplication (F03/F04). | `surrealdb-core-3.3.0/src/exec/index/analysis.rs`, lines 1055, 1188–1240. |
| Native union-index scans can merge ordered streams; planner selection still matters (F04). | `surrealdb-core-3.3.0/src/exec/operators/scan/union_index.rs`, lines 46–63, 149–177; `src/exec/planner/select/mod.rs`, lines 2510–2522. |
| Stored field definitions are processed in dependency order on writes (F05). | `surrealdb-core-3.3.0/src/doc/field.rs`, lines 245–252. |
| Indexed COMPUTED fields are rejected; ordinary index creation is synchronous unless selected otherwise. | `surrealdb-core-3.3.0/src/legacy/expr/statements/define/index.rs`, lines 88–104, 271–303. |
| Grammar-aware statement parsing and whole transaction execution units exist (F06 and declaration batching). | `surrealdb-syn-3.3.0/src/lib.rs`, lines 139–145; `src/parser/mod.rs`, lines 601–615, 694–725; `surrealdb-sql-3.3.0/src/ast.rs`, lines 85–125. |
| Native import streams input; the HTTP SDK consumes the returned statement results and rejects errors (F06). | `surrealdb-core-3.3.0/src/kvs/ds.rs`, lines 5384–5460, 5538–5546; `src/dbs/executor.rs`, lines 2413–2434; `surrealdb-3.3.0/src/engine/remote/http/mod.rs`, lines 521–580. |
| Stream rows remain provisional until terminal success; dropping a stream does not establish immediate remote quiescence. | `surrealdb-3.3.0/src/method/query.rs`, lines 281–355; `surrealdb-core-3.3.0/src/dbs/executor.rs`, lines 280–288. |
| DataFusion's streaming leaf supports fetch, but wrapper defaults can hide it. | `datafusion-physical-plan-55.1.0/src/streaming.rs`, lines 325–329, 388–398; `src/execution_plan.rs`, lines 784–800; filter/limit contract in `datafusion-session-55.1.0/src/table.rs`, lines 149–184. |
| FixedSizeBinary coalescing retains slices before concatenation, unlike the reviewed compact edge builders. | `arrow-select-59.3.0/src/coalesce.rs`, lines 685–703; `src/coalesce/generic.rs`, lines 69–79, 94–98. |
| Task shutdown/abort differs from completing submitted work; running blocking tasks cannot be forcibly aborted. | `tokio-1.53.2/src/task/join_set.rs`, lines 371–383; `src/task/blocking.rs`, lines 21, 70, 106–119. |
| Cloneable shared future results already supply the interrupted-drain join mechanism. | `futures-util-0.3.34/src/future/future/mod.rs`, lines 480–485. |

Context7 discovery covered the relevant library documentation before transferring claims from
these pinned sources. Current-main examples were not treated as exact-version proof. Useful
documentation leads were the [SurrealDB query documentation](https://github.com/surrealdb/docs.surrealdb.com/blob/main/src/content/reference/rust/methods/query.mdx),
[index documentation](https://github.com/surrealdb/docs.surrealdb.com/blob/main/src/content/reference/query-language/statements/define/indexes.mdx),
[HTTP import documentation](https://github.com/surrealdb/docs.surrealdb.com/blob/main/src/content/reference/rest-api/http-protocol.mdx)
and [DataFusion custom-provider documentation](https://github.com/apache/datafusion/blob/main/docs/source/library-user-guide/custom-table-providers.md).

The current source baseline is `1be0dd96714aba85aeb0a01af48758a8e314f9b9` plus these exact
reviewed dirty contents (SHA-256, 2026-10-07). These hashes identify the inspected inputs;
they are not semantic-validity evidence.

| Dirty input | SHA-256 |
|---|---|
| `crates/cpg-core/src/artifact_manifest.rs` | `05ced681ec0dcdbd3b52b2449018e91f72377d475c33c31f0b7620cc3230f533` |
| `crates/cpg-core/src/facts.rs` | `6e9379940b7873d3ca7808026786cfafcb4c57fb2ecdfb2dea067474377e1316` |
| `crates/cpg-core/src/workspace.rs` | `0656c42db0ecee8a65f615b53b6b051ce1ff8255a2551ba90dd72c22a8d0413d` |
| `docs/design/sections/semantic-model.md` | `88634393b97700b72a072905c35ed3757231db8076c0aa443d7085eee0204cc6` |

Historical run identifiers and full commands remain in execution-plan §9.1. The coordinator
also directly inspected `/tmp/persisted-retained-nine-premise-availability.log` (setup timeout)
and `/tmp/persisted-schema-window-check.log` (compile-only check). Temporary log paths may expire;
the scoped conclusions and their attribution are recorded in §3 and §7 above. No optional
probe was needed to establish the structural findings. Replacement behavior remains Proposed
or Interface-checked until its named closure evidence is obtained.
