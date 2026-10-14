# Catalog creation: independent design and target review

**Decision: Revise.** The inspected architecture has a substantive, governing semantic model and useful library composition. Its main defects concern execution: repeated keyed searches inside scoped kernels, publication-prefix ordering that creates extra dependencies, loss of reusable checked properties during certification, incomplete propagation of analyzer cancellation, and scratch lifetimes that amplify live state or retained resource charges.

These are source-supported architectural findings. They do not establish the dominant cause of the previous timeout, a speedup, or a capacity limit. The current real-library completion and native acceptance boundary remains open.

## 1. Scope, standard and evidence

| Field | Assessment |
|---|---|
| Subject | Complete creation from pinned acquisition and code-facts intake through normalization, graphs, Local/Base/Completion/SourceCalls/Enriched/Models/Summary, Structural/Analytic, C0/C1/C2, S0, retrieval, admission, durable unselected publication and relevant finalization |
| Baseline | `b0306d62d2f0d4175877f5ee147471eb99bb542d`; source remained unchanged during review; new supporting evidence documents were concurrent additions |
| Standard | Core 3.3; Heuristics for Efficient Architecture 1.0, H1–H28; code-intelligence 1.5; repository binding for conduct, location and disposition |
| Tier and purpose | Formal design/target |
| Reviewer and date | Independent principal design reviewer; 2026-10-14 |
| Primary workload | Ordinary comprehensive Catalog creation for FastMCP 4.0.5, including its selected release distributions, documentation, examples, tests, assets and necessary dependency/import context |
| Additional workload | Explicitly requested Behavioral enrichment and optional analytical techniques |
| Deployment premise | Personal single-operator project with concurrent native tasks, readers, cancellation and crash/recovery obligations; no distributed deployment requirement inferred |
| Evidence strength | **Implemented** source mechanisms and **Interface-checked** library boundaries; remedies **Proposed** |
| Execution | Read-only source investigation. Builds, probes, acquisition, services, database operations, real-library compilation, benchmarks and qualification **not_run** |
| Exclusions | Operator adoption, MCP runtime execution, embedding-service execution, protected evaluation and broad quality campaigns; no general Stage 3–5 research backlog activated |

The governing criteria are the standard and functional target. Existing ADRs, plans, policies, pins and operator decisions describe the subject; they did not constrain selection of the recommended architecture.

The workload is the actual pinned library definition, not a small fixture substitute. [`libraries/fastmcp/pyproject.toml:10`](../../../libraries/fastmcp/pyproject.toml#L10) selects `fastmcp[anthropic,openai,gemini,azure,apps,code-mode,tasks]==4.0.5`, three first-party release distributions, and corpus revision `004bf15a2ba99f077160993c00a404e1a9da83ea`. Dependency context is required where imports, types, exports or selected models need it. Removing that context or substituting `Any` would change the answer.

A premise in the performance inquiry needs correction: ordinary Catalog creation does **not** execute the complete Behavioral analysis chain. [`compilation/uniform.rs:11`](../../../crates/cpg-core/src/compilation/uniform.rs#L11) routes its seven behavioral owners to compact `NotRequested` production. [`producer_contract/facts.rs:324`](../../../crates/lctx-model/src/domain/producer_contract/facts.rs#L324) requests ty Flow only for Behavioral. Optional analytics are demand-gated. Those distinctions are already meaningful reductions in necessary work; comprehensive API/evidence coverage remains required.

Supporting inquiries provide navigation and bounded comparisons:

- [Intake and analyzers](../evidence/2026-10-14_catalog-creation-performance-target/intake-and-analyzers.md)
- [Algorithms and library fit](../evidence/2026-10-14_catalog-creation-performance-target/algorithms-and-library-fit.md)
- [Scheduling and execution](../evidence/2026-10-14_catalog-creation-performance-target/scheduling-and-execution.md)
- [Catalog contracts and physical completion](../evidence/2026-10-14_catalog-creation-performance-target/catalog-contracts-and-physical-path.md)

Decisive citations used below were independently inspected. These supporting documents do not create separate overall verdicts or disposition ledgers.

## 2. Integrated architectural assessment

The product needs more than extracted declarations. A useful answer distinguishes public exposure from implementation identity, source signatures from effective invocation, source options from runtime defaults, documentary use from executable proof, and incomplete coverage from absence. The implementation represents and governs those distinctions through owned normalization, binding, evidence, selection and synthesis operations. This is stronger than a common output schema.

The physical system is less well aligned. Some operations already use exact scoped inputs, compact topology, charged state and bounded native windows, but then perform repeated correspondence searches within that scope. Other operations inherit ordering from cumulative publication prefixes even when their semantic inputs are ready. Final certification sometimes reconstructs a result or property because it crossed an internal boundary, rather than because its validity premises changed. These causes compound: unnecessary ordering increases the critical path, and repeated preparation increases work on that path.

The appropriate direction is to retain the semantic authorities and specialize their physical realization. It does not require replacing the graph engine, adding a generic scheduler, introducing a second canonical store, or adopting a broad cache/incremental framework.

The recent native corrections are important preservation constraints. [`completed_state.rs:417`](../../../crates/lctx-native/src/completed_state.rs#L417) can borrow the inventory of a still-owned completed source while independently checking the exact client/resource owner, parent incarnation and frozen admission. [`installed_view/live.rs:89`](../../../crates/lctx-native/src/installed_view/live.rs#L89) reuses an independently earned protected inventory instead of recapturing every contributor for each selected view. These mechanisms exemplify preservation of established properties under unchanged premises. They are not missing features and are not re-reported as defects.

## 3. Responsibilities and governing meaning

| Owner | Coherent responsibility and consumer contract | Appropriate reason for change |
|---|---|---|
| Acquisition/captured inputs | Pinned analyzer-readable bytes, corpus roles, environment/configuration and source identity | New release, source format or acquisition semantics |
| Ruff/Pyrefly/ty adapters | Independent provider assertions and exact attachment to canonical source/role identity | Provider revision or newly exposed observation |
| Normalization | Entity correspondence, ownership, callable/signature interpretation, receivers, complete events and bindings | New domain distinction or interpretation |
| Local through Summary | Qualified finite behavioral reasoning, model application, proof, coverage and explicit residuals | New semantic operation or supported runtime model |
| Projection/graph owners | Named universes and relation meanings; mapping to compact graph topology | New projection semantics or physical representation |
| Structural/Analytic | Attributed structural witnesses and separately qualified heuristic results | New structural question or selected analytical consumer |
| C0/C1/C2 | Public API universe, original evidence associations and declaration/requirement classification | New public form, option/evidence kind or selection predicate |
| S0/retrieval | Programmatic claims, selected briefs, addressable units and exact original/support linkage | New claim/rendering/retrieval contract |
| Compiler storage/native/publisher | Exact completed ownership, persistence, admission, publication and finality | Trust transition, physical access or lifecycle change |

Dependency direction remains credible: effects and representations consume model-owned meanings. The local pure operations can be reasoned about and exercised without starting an analyzer, database or server. Native persistence and publication appropriately require actual native verification.

A representative fidelity inventory follows.

| Family/relation | Authority and fidelity | Consequential distinctions and consumers |
|---|---|---|
| Canonical syntax/lexical facts | Independent latest Ruff; extracted/native structural observations | Exact captured bytes, occurrences and roles; normalization and original evidence |
| Types, public names, signatures and calls | Pinned Pyrefly fork; typed native assertions | Source/effective/native roles, possible alternatives, unresolved remainder and provider support |
| Flow | ty runtime view; requested only by Behavioral | Original/view identity, native scope, candidate/reaching inventory, attachment uncertainty |
| Normalized events and bindings | Model-owned derivation from complete attributed premises | Site/origin/context, alternative identity, policy membership, binding/refusal and receiver roles |
| Behavioral results | Model-owned finite operations | Channel, phase, condition, model, proof, coverage and five-way verdict/refusal meaning |
| Structural reach/path | Exact over declared graph and bounds | Invocation versus definition arcs, parallel ArcIds, boundaries, unresolved events and partial reasons |
| Analytics | Declared heuristic or exact finite analytical operation | Projection/settings/seed/convergence and incomplete enumeration; never behavioral authority |
| Catalog/evidence/selection | Model-owned derived contracts | Exposure/path/candidate, source/effective default, original scenario, requirement and applicability |
| S0/retrieval/publication | Programmatic derivation over one exact realization | Claims and units retain their source/support identities; publication remains distinct from selection |

For example, [`normalized/event_normalization.rs:78`](../../../crates/lctx-model/src/domain/normalized/event_normalization.rs#L78) indexes and assembles events by site, origin and context, retaining attributed targets and resolutions before policy interpretation. [`catalog/build.rs:643`](../../../crates/lctx-model/src/domain/catalog/build.rs#L643) preserves Absent, Unknown, Unavailable, Literal, Expression and Factory defaults rather than evaluating source expressions. [`catalog/evidence/frames.rs:46`](../../../crates/lctx-model/src/domain/catalog/evidence/frames.rs#L46) requires exact lower-frame correspondence and rejects ordinary/compact predecessor overlap. These are governing operations, not merely labels.

The model is adequate for the inspected finite contract. General heap identity, arbitrary decorator equivalence, async cleanup and recursive all-channel closure remain outside that contract and must remain uncertain. Source async syntax can support an invocation form or evidence association without proving a complete async lifecycle.

## 4. Composition and complete execution

| Operation | Question, universe and method | Limits, completeness and evidence |
|---|---|---|
| Intake | Analyze selected pinned roots with required import context; separate provider/session semantics | Explicit provider coverage and attachment boundaries; native heaps are distinct from typed-transfer budgets |
| Normalization | Resolve entities, signatures, receivers, complete event alternatives and binding | Exact scoped inputs and owned refusal/ambiguity outcomes |
| Local | Establish supported entry/transfer/theory premises | Catalog compact `NotRequested`; Behavioral uses qualified native evidence |
| BaseEvaluation | Evaluate supported expressions without borrowing later enrichment | Distinct work/depth/refusal inventory |
| BaseCompletion | Establish supported body outcomes and pending-outcome handling | Normal/abrupt/finalizer distinctions; unsupported lifecycle remains open |
| SourceCalls | Admit exact source invocation/body/default/release correspondence | Exact site, signature enumeration, actual/formal/default and prior-owner evidence |
| EnrichedExecution/Models | Apply admitted source/model results at their supported phase | No circular certification of earlier inputs; pinned applicability and independent channel coverage |
| Summary | Compose finite transfers over callee-first SCCs with semantic delta work | Pareto depth/cost representatives; distinct proof evidence; explicit depth/pair/proof residuals |
| Structural | Public candidates, bounded delegation, usage, handoffs and requested controls | Complete named graph universe; per-seed bounds and explicit unresolved/partial outcomes |
| Analytic | Selected ranking, communities, concepts, neighbours or RCA | Optional; heuristic settings and incomplete enumeration stay visible |
| C0 | Public exposures, variants, constructors, fields and access paths | All competing candidates and source/context/default distinctions retained |
| C1 | Original evidence/scenarios and contextual associations | Total native frame domain; artifact-scoped rich construction and exact parent linkage |
| C2 | Declaration domains and classification evidence | Consumes C0/C1 and owned metadata; support/conflict/unknown remain separate |
| S0 | Ground assertions and select briefs after structural/optional inputs | Sole shared finding writer; no admitted outcome means no fabricated brief |
| Retrieval | Render addressable roots/briefs with exact original/support closure | Charged windows; requested vectors have explicit tokenizer/service/spec requirements |
| Admission/publication | Freeze, certify semantic/graph/original/physical closure, derive serving structures and publish | Actual state reconciliation and writer drainage precede visibility; output remains unselected |

The graph/analysis design contains several strengths worth retaining.

[`MaterializedGraph`](../../../crates/lctx-model/src/domain/projection/snapshot.rs#L21) uses a directed petgraph with `ArcId` edge weights. Its independent vertex universe preserves isolates, and parallel relationships retain their identities. [`snapshot.rs:221`](../../../crates/lctx-model/src/domain/projection/snapshot.rs#L221) traverses the required universe before applying output selection.

[`PreparedGraphs`](../../../crates/cpg-core/src/analysis_graphs.rs#L81) loads only selected projections, streams selected chunks, releases encoded representations after hydration, retains one topology per projection and releases it after its final consumer. Behavioral SCC preparation binds to exact materialization identity.

[`summary_schedule.rs:34`](../../../crates/lctx-model/src/domain/execution/summary_schedule.rs#L34) uses petgraph SCCs and canonical callee-first condensation order. The semantic fixpoint is separately owned: [`summary_worklist.rs:89`](../../../crates/lctx-model/src/domain/execution/summary_worklist.rs#L89) excludes dominated/equal-cost reopening, while [`summary_production.rs:2409`](../../../crates/lctx-model/src/domain/execution/summary_production.rs#L2409) retains admissible independent proof evidence. SCCs are therefore a schedule, not a substitute for transfer semantics or completeness.

[`conditions/kernel.rs:106`](../../../crates/lctx-model/src/domain/conditions/kernel.rs#L106) preflights operand-pair work and uses bounded BDD apply. A refused operation is distinct from false. BDDs compress conditions; they do not eliminate proof alternatives or relationship evidence.

Catalog delegation is bounded breadth-first traversal with a canonical parent and capped shortest-final-arc witnesses. [`delegation.rs:460`](../../../crates/lctx-model/src/domain/analysis/delegation.rs#L460) does not enumerate every path through reconvergent or cyclic topology. Describing it as unrestricted exponential path search would be incorrect.

## 5. Findings

The findings below are dated assessments. **Disposition transferred2026-10-14** to the [combined Catalog creation coordinator §6](../../plans/catalog-creation-plan_2026-10-14.md#6-sole-disposition-and-execution-checkpoint), which also preserves the predecessor Catalog IDs and remaining acceptance. This navigation/disposition update changes neither the review judgment nor its evidence; CE01 is not closed here.

<a id="F01"></a>

### F01 — Scoped kernels repeatedly search keyed correspondence instead of preparing it

**Priority: High for Catalog.** FP-07, DP-10, DP-13/14, CI-07; **A4 violated**.

Scoping controls input size, but does not make repeated scans efficient. C0 already builds a charged resolution→candidate index at [`catalog/build.rs:344`](../../../crates/lctx-model/src/domain/catalog/build.rs#L344). It nevertheless scans all scoped exposure candidates per exposure, scans class metadata/member observations per class with inner resolution searches, and repeatedly filters field declarations/default assessments/links. Public-path expansion repeatedly scans ancestry, declarations and bindings; [`catalog/paths.rs:64`](../../../crates/lctx-model/src/domain/catalog/paths.rs#L64) additionally searches from the start of the pending map for the next unfinished item.

The same cause appears in C1’s option→frame, coverage→artifact, source→use, binding→option and deployment→ownership correspondence, including [`catalog/evidence/build.rs:497`](../../../crates/lctx-model/src/domain/catalog/evidence/build.rs#L497) and [`:695`](../../../crates/lctx-model/src/domain/catalog/evidence/build.rs#L695). Structural frame construction scans modules/uses per callable and links per member at [`structural/build.rs:162`](../../../crates/lctx-model/src/domain/structural/build.rs#L162). These are equality, grouping and existence operations, rather than graph questions.

Alias-rich class APIs, inherited members and evidence-dense corpus artifacts increase both the number of lookup requests and the searched relation size. A 32-root window does not remove amplification inside a large dependency closure. The finding is about repeated scoped work, not repeated scans of the entire database.

**Correction, Proposed.** The owning model operations should prepare charged invocation/window-local buckets for the actual correspondence keys and use a deterministic ready queue for expansion. Retain every competing row and apply existing ambiguity/context checks. Where inputs are still columnar, bulk relational joins or semi-joins before rich decoding are a viable alternative; small local maps are simpler once the rich rows are already needed. Do not introduce a persisted index cache or generic indexing framework by default.

Preserve public path identity, MRO precedence/shadowing, rebound and open-ancestry boundaries, unknown correspondence and every default state. Two aliases to the same class still require distinct public answers.

**Closure.** Source inspection should show one preparation per stable grain and no repeated full-relation lookup for the identified keys. Independent revealing cases should retain two public aliases, competing candidates, incomplete MRO, crossed contexts and field-default distinctions. A relational remedy also needs inspection of its actual plan; the presence of a join expression does not establish an indexed access path.

<a id="F02"></a>

### F02 — Cumulative publication prefixes impose execution dependencies beyond required semantic inputs

**Priority: High for Catalog.** FP-03/07, DP-10/19/20, H2/H8/H22; **A4 violated**.

The schedule derives genuine dependencies from relation inputs, invariant premises and contribution assembly. It also extends an epoch input to every member of that group and makes each later publication group depend on the preceding group at [`stages.rs:828`](../../../crates/lctx-model/src/domain/stages.rs#L828) and [`:848`](../../../crates/lctx-model/src/domain/stages.rs#L848). [`analysis/owner.rs:313`](../../../crates/lctx-model/src/domain/analysis/owner.rs#L313) fixes a singleton upper-owner sequence. Cumulative uniform checkpoints participate through [`owner.rs:160`](../../../crates/lctx-model/src/domain/analysis/owner.rs#L160) and [`:251`](../../../crates/lctx-model/src/domain/analysis/owner.rs#L251).

The executors then serialize providers, normalization and upper stages: [`facts.rs:40`](../../../crates/cpg-core/src/facts.rs#L40), [`compilation.rs:717`](../../../crates/cpg-core/src/compilation.rs#L717), [`:740`](../../../crates/cpg-core/src/compilation.rs#L740) and [`:839`](../../../crates/cpg-core/src/compilation.rs#L839). This is stronger than deterministic publication ordering. It prevents ready independent work from overlapping and makes a new independent owner inherit unrelated ordering.

Some ordering is necessary. Source-call reasoning cannot precede its admitted evaluation/completion/binding premises. Vocabulary assembly cannot expose unfinished contributions. Final freeze must drain actual writers. The defect is treating a complete cumulative physical prefix as the universal prerequisite even where a narrower exact dependency set supplies the operation’s meaning.

**Correction, Proposed.** Separate semantic/invariant prerequisites, contribution assembly and final visibility barriers from incidental prefix ordering. Compose ready work with ordinary existing futures/tasks over exact completed views. Begin with useful independent operations—such as deployment after acquisition alongside Python analysis—and independent upper preparation whose inputs are demonstrably complete. Compact Catalog unrequested declarations should retain their individual semantics without automatically recreating the Behavioral execution chain.

This is not a recommendation to maximize every pool. Pyrefly already uses a shared library worker pool. Overlap must consider CPU occupancy, memory, native submission backpressure, sibling cancellation and the common native write point described below.

**Closure.** Trace an additional independent provider/owner: it should wait only for its actual premises and required assembly. A focused scheduling control should preserve deterministic identities under shuffled completion, prevent unfinished reads, cancel/join siblings on failure and settle accepted ingress. A later representative run may measure latency; source correction alone establishes removal of the false dependency.

<a id="F03"></a>

### F03 — Certification loses some established immutable properties and reconstructs expectations unnecessarily

**Priority: High for Catalog; consequential trust-boundary design.** FP-02/05/07, DP-03/09/10/19/23, H18–H20; **A4 violated**.

The compiler establishes normalized semantic authority at [`compilation.rs:810`](../../../crates/cpg-core/src/compilation.rs#L810), retaining exact checked inputs for later consumers. Final artifact preparation calls semantic certification again at [`artifact.rs:1736`](../../../crates/lctx-compiler-store/src/artifact.rs#L1736). [`admission.rs:823`](../../../crates/lctx-compiler-store/src/admission.rs#L823) requires fresh executed evidence; the known-premise shortcut at [`:1301`](../../../crates/lctx-compiler-store/src/admission.rs#L1301) is available only when `earn` is false. Specialized scoped checks consequently repeat their ordinary execution. Durable whole-input reuse covers only eligible branches and does not remove that general distinction.

Not every final check is redundant. Later outputs, changed selected vocabularies, cross-relation closure, frozen native ownership and imported bytes introduce real uncertainty. The defect is the absence of a same-run route that carries an actually earned scoped property when its exact ordered inputs, interpretation, ownership and immutable validity premises are unchanged.

There is a particularly clear expectation-reuse case in publication. [`derived_search.rs:2342`](../../../crates/lctx-native/src/derived_search.rs#L2342) lowers canonical content into ordered expected search rows, writes and reconciles actual rows, then drops the expected preparation. Final publication invokes [`reconcile_search_owned`](../../../crates/lctx-native/src/derived_search.rs#L2362), lowering those expectations again through [`staging.rs:335`](../../../crates/lctx-native/src/control/staging.rs#L335). This is the same lowering, not an independent semantic oracle. A fresh actual-state comparison remains necessary; regenerating unchanged expectations is a different obligation.

**Correction, Proposed.** Let the existing admission owner carry genuine scoped validation evidence within the protected operation lifetime. At final freeze, establish which premises are unchanged, certify changed/new scopes and independently validate the actual native realization. Retain exact model/invariant/factory/input-role identity. A cached Boolean, digest, caller label, serialized receipt or producer success is insufficient.

For derived search, retain budgeted provenance-bound ordered expectations through finalization and compare them against fresh actual rows after writer drainage. Cold/foreign input still derives its own expectations and earns its own admission. Retain complete EOF, missing/extra/different-row detection, transaction finality and definition-epoch checks.

The current protected native inventory handoff demonstrates the appropriate pattern; it should not become a second durable proof cache or a blanket authorization to skip checking.

**Closure.** Trace at least one unchanged normalized property through final certification without repeating its semantic kernel, and one changed premise that forces revalidation. A derived-row mutation between preparation and sealing must still fail. Changed input, encoder, projection, recipe, model or invariant identity must refuse reuse. Detached import must remain independently admitted. Cancellation must release scratch only after owned drainage. Rule impacts RC02 apply before adopting this direction.

<a id="F04"></a>

### F04 — Provider cancellation does not reach Pyrefly’s native task queue

**Priority: Medium; both profiles.** FP-02/07, DP-15/19/20, H4/H21/H24; **A4 violated on cancellation/failure work**.

[`bundle.rs:481`](../../../crates/cpg-extract/src/bundle.rs#L481) creates a provider cancellation signal; dropping the run signals cancellation and registers the provider thread for joining. This safely retains drainage ownership. The initial analyzer call at [`pyrefly_stage.rs:518`](../../../crates/cpg-extract/src/pyrefly_stage.rs#L518) and later dependency waves do not receive the transaction cancellation handle. During native work the adapter can therefore continue computing results that no consumer can publish.

The exact pinned fork exposes `Transaction::get_cancellation_handle` at `pyrefly/lib/state/state.rs:907`. Its task heap checks the signal before queued tasks and clears pending work at `crates/pyrefly_util/src/task_heap.rs:168`. This is cooperative cancellation; active callbacks can finish. The fork’s public `run` discards the internal cancellation result at `state.rs:2319`, which is a material integration constraint.

**Correction, Proposed.** Register a scoped transaction-local cancellation handle with the existing provider-run owner, synchronizing registration against cancellation already signaled. Explicitly reject interrupted execution before querying/emitting completed facts. Continue joining native workers and settling accepted ingress. Do not reset and reuse an interrupted transaction or create a separate polling service.

**Closure.** A deterministic local cancellation case should stop queued work, refuse completion, join workers and retain accepted-ingress finality. Cancellation during an active opaque/lazy query remains a separately stated cooperative limitation. No hard cancellation-latency bound is claimed.

<a id="F05"></a>

### F05 — Behavioral Flow gathers all module results before output backpressure can act

**Priority: Medium; Behavioral only.** FP-07, DP-10/20, H16/H17/H21; **A4 violated**.

[`ty_flow.rs:571`](../../../crates/cpg-extract/src/ty_flow.rs#L571) collects admitted source inputs, then calls `cpg_flow::index`, and only afterward lowers/emits the returned flows. [`cpg-flow/lib.rs:405`](../../../crates/cpg-flow/src/lib.rs#L405) prepares the complete virtual import root and Program, computes every module result and collects them into a vector.

Preparing a coherent import universe has a semantic purpose. Retaining every completed `ModuleFlow` before lowering the first does not. It creates a whole-input result barrier and combined live state outside sink backpressure. More complex corpus modules increase retained intermediates even when native output queues are bounded.

**Correction, Proposed.** Preserve the complete prepared Program/import root, but expose module-result iteration or a callback while that database remains alive. Lower and emit one result before computing the next. Preserve module-local errors, original/runtime-view identity, source correspondence and unavailable/partial coverage. A per-file database would repeat preparation and weaken import semantics; parallel Salsa queries are a separate option needing exact snapshot/thread/memory qualification.

**Closure.** Source must show that first emission no longer waits for a full result vector. A cross-module two-module control should preserve output/identity, and sink refusal should prevent computation of subsequent modules. Representative peak-memory/latency measurement remains later qualification. This correction has no direct Catalog latency benefit.

<a id="F06"></a>

### F06 — Transient scratch and its charges outlive their useful unit

**Priority: High where it causes artificial resource refusal; Catalog and Behavioral manifestations.** FP-05/07, DP-10/20, H12/H17; **A4 violated**.

C0 path expansion creates a function-scoped `StateCharge` at [`catalog/paths.rs:57`](../../../crates/lctx-model/src/domain/catalog/paths.rs#L57). Per-expansion MRO `chain` and `nearest` maps allocate against that charge at [`:80`](../../../crates/lctx-model/src/domain/catalog/paths.rs#L80) and [`:162`](../../../crates/lctx-model/src/domain/catalog/paths.rs#L162). The temporary maps drop at the end of the iteration, but their charges are not released.

Behavioral controls repeat the pattern. [`structural/controls.rs:586`](../../../crates/lctx-model/src/domain/structural/controls.rs#L586) creates one charge before the per-root loop; each root’s visited set and pending vector grow it, then drop without returning that charge. [`charged.rs:148`](../../../crates/lctx-model/src/domain/charged.rs#L148) and [`:174`](../../../crates/lctx-model/src/domain/charged.rs#L174) show that these containers do not own the external `StateCharge` or release it on drop; release occurs through explicit operations or the charge owner’s eventual destruction.

Thus admission can accumulate bytes for dead scratch across independent public paths or roots. A legitimate workload can be refused even though its current scratch plus retained output fits the intended resource envelope. This is distinct from necessary retained output and from actual native heap size.

Controls also retain all pending entries until traversal ends and copy full path prefixes for outgoing edges at [`controls.rs:635`](../../../crates/lctx-model/src/domain/structural/controls.rs#L635) and [`:691`](../../../crates/lctx-model/src/domain/structural/controls.rs#L691). Visited semantic states prevent infinite expansion, but reconvergence can accumulate duplicate pending prefixes before those entries are skipped.

**Correction, Proposed.** Match scratch reservations to the expansion/root that owns the corresponding allocations; keep retained queue/index/output charges separate. Release temporary state and reservations together. For controls, index outgoing flows by caller/formal and use a predecessor arena with queue handles, consuming processed entries. Preserve the authoritative visited key—including formal and suppression state—and canonical witness selection.

No generic resource framework is needed. Fixing the charge lifetime is independently useful before changing published path representation.

**Closure.** A small-budget case with many independent class paths must retain the same output without accumulating dead MRO scratch charge. A two-root control must show that first-root scratch is released before the second starts. A diamond/cycle control must preserve canonical witnesses, distinct formals/suppression states, work/depth limits and output charging. The correction must not undercharge a buffer or witness that remains live.

## 6. Change, growth and failure scenarios

| Scenario and kind | Expected owner and propagation | Assessment |
|---|---|---|
| Another pinned library release — instance/input revision | Acquisition and affected source/provider/normalized products; earlier publications retain identity | Coherent route. Do not reuse release/corpus sessions merely because versions or filenames match |
| New analyzer observation — domain concept/provider boundary | Typed attributed record, coverage, attachment and consuming operation | Existing separation is sound. New syntax/type/flow meaning may legitimately affect several contracts |
| New pinned callable/context model — domain extension | Model applicability, phase/channel rules, checked operation and Summary/S0 consumers | Explicit model operations provide a credible route; no generic runtime equivalence inferred |
| Async lifecycle support — domain extension | New entry/exit/outcome/resource/exception semantics and evidence | Requires deliberate semantic work. Current source invocation evidence cannot be promoted into cleanup proof |
| Local C0 index or DataFusion bulk correspondence — mechanism substitution | C0 physical preparation; attributed outputs and ambiguity/default semantics unchanged | Preferred bounded route for F01; no forwarding abstraction needed |
| Dependency-ready execution — composition/mechanism | Exact selected inputs, contribution completion and final freeze | F02 requires revision; preserve genuine predecessor and assembly dependencies |
| Alias-rich classes/deep MRO — growth/skew | C0 correspondence/path owner | F01 and F06 expose avoidable work and artificial retained charges |
| Many public wrappers sharing a suffix — growth/reconvergence | Delegation/witness owner | Current BFS is finite; multi-seed sharing remains a consequential investigation, not an established replacement |
| Recursive SCC with late better proof — behavioral composition | Summary semantic frontier/worklist | Existing delta/Pareto design is appropriate; preserve independent evidence and explicit residuals |
| Cancellation during native analysis — failure | Provider and native-library ownership | F04; safe joining already exists, useful cancellation is incomplete |
| Actual derived-row mutation before sealing — failure/trust transition | Publisher/native reconciliation | Final actual-state comparison must detect it even after F03 expectation reuse |
| Cold import or changed incarnation — mechanism/trust transition | Fresh receiving/admission owner | Never inherits same-run authority from bytes, receipts or equal hashes |
| Failure after acknowledged publication — lifecycle | Publication marker and enclosing completion | Preserve committed identity independently of cleanup failure or Unknown |

## 7. Library fit and complete alternatives

The direct selected graph and condition mechanisms fit their roles. petgraph 0.8.3 supplies topology/SCC/traversal under exact identity mappings; biodivine-lib-bdd 0.6.3 supplies bounded Boolean algebra; DataFusion 55.1.0 supplies relational selection and bulk compute. None substitutes for Python transfer, binding, coverage or proof semantics.

A graph is useful for topology, a relational plan or local index for correspondence, and a domain worklist for transfer/fixpoint operations. Moving every operation into SurrealQL or every operation into a graph traversal would lose that distinction. Conversely, returning prepared graph adjacency to relational rows for each BFS edge would add movement and setup.

| Alternative | Benefit and burden | Judgment |
|---|---|---|
| Current baseline | Explicit immutable semantic/native ownership; scoped kernels; broad serial/prefix execution and repeated preparation | Retain foundations; revise identified physical causes |
| Local charged indexes/queues | Removes repeated correspondence work with narrow ownership and no durable lifecycle | Preferred F01/F06 route where rich input is already required |
| Bulk DataFusion joins/selection | Preserves set intent and may reduce rows before decoding | Suitable where input remains columnar; qualify duplicate/null/context semantics and actual physical plan |
| Ordinary dependency-ready composition | Exposes useful overlap without a generic workflow runtime | Preferred F02 direction after exact prerequisites and shared resources are resolved |
| Same-operation checked evidence and ordered expectations | Avoids unchanged reconstruction while retaining fresh actual-state checking | Preferred F03 direction, conditional on complete protected validity premises |
| Salsa/Datalog/general incremental runtime | Could serve repeated-edit or difficult recursive consumers; adds dependency, lifecycle and semantic integration | Not justified by one-shot creation alone; Summary already has delta work |
| Whole graph/store replacement | Could change access and write behavior, but also migration, recovery, query and ownership burdens | No evidence here selects it over narrower corrections |
| All-pairs closure or all-path retention | Simplifies some repeated queries while potentially creating dense retained results | Unsuitable default for bounded public-seed/evidence demand |

The existing sparse weighted PageRank kernel is not shown to violate library-first merely because petgraph has a function named PageRank. The inspected built-in does not discharge the same weighted/residual/work contract. Leiden is already library-owned. FCA has a useful independent library oracle, but replacement must preserve support pruning, bounded work, implication output and ordering semantics.

The following investigations remain prioritized choices, rather than additional proven defects:

| Investigation | Profile and decision it affects | Settling evidence and preservation |
|---|---|---|
| Native analyzer heap/import envelope | Both; feasibility and useful session overlap | Representative peak native heap and import work, separated by release/corpus/profile; retain full required import semantics |
| Common native attempt write point | Both; useful native concurrency and fencing granularity | Exact effect/freeze/retirement protocol comparison, followed only if needed by targeted contention evidence |
| Multi-seed traversal/shared suffixes | Catalog and Behavioral Structural; independent BFS versus shared work | Alias-chain/diamond/cycle comparison retaining per-seed bounds, shortest witnesses, unresolved/boundary and partial outcomes |
| Compact published witnesses | Both; flat path-step output versus shared evidence DAG | Identify consumers requiring enumeration; preserve path/ArcId/qualification identity and bounded explanation; RC03 applies if selected |
| Optional community pair/context growth | Explicit analytics only; incidence versus clique/lattice materialization | Consumer-demand and equivalent weighting/evidence comparison; high-degree and independent-attribute shapes |
| Summary candidate filtering | Behavioral only; descriptor/port indexes versus current endpoint buckets | Recursive SCC with incomparable costs, equal independent evidence and late improvement; preserve complete qualifying combinations |
| Lexer-only runtime rewrite | Behavioral only; avoid original-source AST used solely for tokens | Exact lexer/parser recovery parity on malformed strings, f-/t-strings, Unicode/BOM/CRLF and sentinel refusal |
| Shared inherited-field correspondence | Both; retained compact source index versus a second root parse | Actual overlap and retained-memory comparison, preserving exact attachment and refusal semantics |
| Release/corpus analyzer preparation reuse | Both; library-owned session reuse versus separate contexts | Exact resolver/configuration/source dependencies; never merge distinct import semantics under a constant configuration |

The native common write point is source-established. [`authorization.rs:230`](../../../crates/lctx-native/src/control/authorization.rs#L230) maps compiler effects to open/frozen attempt owners; [`:692`](../../../crates/lctx-native/src/control/authorization.rs#L692) and [`:705`](../../../crates/lctx-native/src/control/authorization.rs#L705) touch and lock that owner in intent/payload transactions. [`control/schema.surql:157`](../../../crates/lctx-native/src/control/schema.surql#L157) makes the touch an actual predicate-revision write. Frontend concurrency cannot remove this point. The review does **not** establish that it is removable without equivalent delayed-operation, epoch, ABA, Unknown and recovery protection, or that it caused the timeout.

## 8. Foundation and gate verdicts

Architectural judgments and fidelity gates are independent.

| Foundation | Verdict | Reason |
|---|---|---|
| FP-01 Separation of concerns | Satisfied in inspected scope | Acquisition, observation, interpretation, analysis and effects have coherent owners |
| FP-02 Stable contracts | Satisfied at source strength | Nominal inputs/outcomes and explicit provider/model/trust distinctions support credible substitutions |
| FP-03 Composition | Satisfied semantically | Owned operations compose without a second interpreter; F02 separately defeats physical execution fit |
| FP-04 Explicit model and authority | Satisfied in inspected finite scope | Operations govern construction, binding, uncertainty, evidence and outcomes; schema names alone were not the basis |
| FP-05 Explicit structure | Violated for F06 resource lifetime; otherwise supported | Scratch allocations and their external charges have mismatched lifetimes |
| FP-06 Local reasoning | Satisfied in inspected scope | Pure finite operations have explicit inputs; native effects retain appropriate actual-native boundaries |
| FP-07 Execution fit | Violated | F01–F06 demonstrate avoidable work, ordering or lifetime amplification |

Supporting-rule assessment: DP-01/02/04/05/07/08/11/12/15/18/19/21/24 and CI-01–06/09–13 are supported by the inspected authority, typed semantics and lifecycle routes. DP-03/09/23 require F03’s distinction between earned validity and new uncertainty. DP-10 and DP-20 are violated by the identified execution/lifetime causes. DP-13/14/16/17 support the bounded library/local-preparation remedies; no new generic framework is justified. CI-07/08 support scope-appropriate compute and compact bounded evidence; honest refusal alone does not settle A4.

| Gate | Verdict in reviewed source-architecture scope | Evidence and limit |
|---|---|---|
| G1 Authority | Pass | Model-owned meaning; derived graph/store/serving forms retain explicit identity and authority |
| G2 Semantic fidelity | Pass | Provider, relation, role, phase, defaults, unknowns and partial outcomes remain distinguishable |
| G3 Validity | Pass | Typed ingress, shared invariant/admission operations and private admitted constructors provide explicit rejection boundaries |
| G4 Hidden behavior | Pass | Acquisition/extraction/store/embedding effects are explicit; analyzed inputs/configuration are captured |
| G5 Consistency and recovery | Pass at source-contract strength | Completion, freeze, admission, marker and cleanup are separate; Unknown does not become permission to discard |
| G6 Transformation and reuse | Pass at inspected contract strength | Exact selected identities and declared models constrain reuse; proposed remedies must retain them |
| G7 Truthful capability claims | Pass | Current owners distinguish source integration, focused evidence, incomplete journeys and qualification |
| G8 Library leverage | Pass | Established topology/BDD/relational/analytic mechanisms are used; custom domain kernels have non-equivalent requirements |
| CI-G1 Fidelity | Pass | No inspected route relabels inference as runtime flow or promotes unknown/heuristic evidence into proof |
| CI-G2 Evidence closure | Pass at construction-contract strength | Exact invocation/source/reference closure and publication identities govern constructed claims and units |
| CI-G3 Evaluation integrity | Pass for inspected creation path | Committed model/input configuration supplies production; no private oracle input was found in the inspected route |

These passes are architectural source judgments, not claims that every implementation arm or current native journey passed execution. Full evaluator behavior and all served multi-call journeys were not requalified. No passing gate offsets A4 or F06.

## 9. Verification and unresolved boundaries

Static source reads and targeted searches **passed**. Early guessed navigation paths that did not exist were corrected by repository discovery; they supplied no negative capability evidence.

No new probe was required to establish the repeated loops, ordering, vector barrier or charge-lifetime findings. The proposed remedies have consequential semantic questions—particularly cancellation-result handling, exact checked-evidence transport, lexer recovery and witness sharing—which need the revealing closure controls specified above when implemented.

Current [STATUS](../../../STATUS.md) and [Catalog coordinator §6](../../plans/catalog-creation-plan_2026-10-14.md#6-sole-disposition-and-execution-checkpoint) retain the historical journey boundary: run `343d1b` reached retrieval and timed out during independent admission inventory. That is an incomplete journey, not a measured diagnosis. Focused native/cold controls and source acceptance do not close CE01, real FastMCP creation, publication/query/original composition or enclosing qualification.

The review did not count or reacquire corpus files, establish whole-process memory ceilings, audit every expression/model arm, execute current native guards or demonstrate end-to-end product advantage. These are evidence limits, not invented defects.

## 10. Rule impacts and disposition

No rule changes are applied by this review.

| ID | Current rule and location | Proposed change and dependent finding | If retained |
|---|---|---|---|
| <a id="RC01"></a>RC01 | Immutable vocabulary-prefix publication: [facts-and-identity §3.5](../../design/sections/facts-and-identity.md#section-3-5), source line97. Actual-predecessor ownership and protected operation-owned selections: [semantic-model §15.1](../../design/sections/semantic-model.md#section-15-1), lines97/109. The stronger fixed checkpoint/order policy is implemented in `analysis/owner.rs:160,251,313` and `stages.rs:828,848`: epoch reads include whole groups and later groups acquire preceding-group dependencies | Replace that universal execution prerequisite with exact semantic/invariant dependencies and required assembly barriers; revise affected prefix/checkpoint selection contracts where necessary. Preserve immutable published prefixes, exact selected views and genuine predecessor dependencies; F02 | Useful overlap remains confined to work already permitted by the fixed chain; excess upper ordering remains unresolved |
| <a id="RC02"></a>RC02 | [Storage-and-publication §6.1](../../design/sections/storage-and-publication.md#section-6-1), lines141/148, requires executed production checks/private admission and permits qualified whole-input reuse, conservatively retaining ordinary scoped/companion/uniform-owner/unrequested validation. Fresh earning: `admission.rs:823,1301`. Publication/freeze guarantees: [semantic-model §15.11](../../design/sections/semantic-model.md#section-15-11), line881; [storage §6.2](../../design/sections/storage-and-publication.md#section-6-2), line208; [ADR-0145](../../adr/0145-holistic-state-ownership.md), lines39/47. Search reconstructs expectations in `derived_search.rs:2362`/`control/staging.rs:335` after dropping earlier preparation at `derived_search.rs:2342` | Extend the conservative reuse boundary to genuinely earned same-operation scoped properties and provenance-bound ordered expectations under complete unchanged protected premises. Preserve fresh frozen/actual-state reconciliation, private admission and independent cold/foreign admission. Clarify reconstruction wording in [storage-and-publication](../../design/sections/storage-and-publication.md), line445; F03. ADR-0145's admission/publication guarantees need no supersession | Optimize within fresh reconstruction; unchanged scoped semantic work and ordinary expected-search derivation continue to repeat |
| <a id="RC03"></a>RC03 | Flat Structural `Path/PathStep` at `domain/structural/records.rs:101,143`, and `ControlPath/ControlStep` at `domain/structural/controls.rs:109,122`, plus their consumer contracts | Conditional: if compact published witness sharing is selected, version the representation/identity contract and migrate actual consumers; this remains an investigation, not prerequisite to F06’s internal lifetime correction | Use internal predecessor arenas and materialize current bounded outputs; published prefix duplication remains part of that retained contract |

F01, F04, F05 and the internal F06 corrections require no selected dependency change or new architectural rule merely to use existing indexes, cancellation, streaming and scoped reservation owners. Any broader analyzer-session unification or native fencing redesign requires its own concrete rule-impact decision once selected.

`artifact_admission.rs:1–2` requires real execution of applicable production gates; it does not independently require repeating an already executed gate whenever admission is assembled. Its `validate_current` documentation at line48 explicitly recognizes protected provenance of original visits. RC02 targets the current conservative scope exclusions and fresh-earning implementation policy, while preserving executed-gate provenance, exact current premises and independent native verification. F03's diagnosis and proposed boundary remain unchanged.

The [combined coordinator §6](../../plans/catalog-creation-plan_2026-10-14.md#6-sole-disposition-and-execution-checkpoint) now owns scheduled disposition for these findings, retaining their source-review IDs. Existing Catalog findings retain their original IDs and evidence boundaries; this document does not rewrite their closure evidence or CE01.

## 11. Priority and relationships

| Consequence-based priority | Responsible change | Findings |
|---|---|---|
| High | Correct stale scratch charging and lifetime boundaries that can refuse valid Catalog work | F06 |
| High | Remove repeated keyed correspondence and pending-prefix rescans on mandatory Catalog paths | F01 |
| High | Resolve universal prefix ordering and preserve same-run validity/expectations across certification | F02, F03 |
| Medium | Propagate cancellation into queued analyzer work while preserving refusal/drain | F04 |
| Medium, Behavioral | Stream Flow results and reduce control pending-prefix/live-state amplification | F05, F06 |
| Further decision | Native fencing granularity, multi-seed sharing, analytical incidence/lattice demand and native heap envelope | Investigations in §7 |

This priority is not an implementation schedule. Exact ownership/validity contracts precede safe scheduling or certification reuse. F01/F06 can make useful progress without waiting for broad protocol changes. Increasing concurrency before removing unnecessary work may enlarge memory overlap or conflict at native ownership points. None of these remedies permits narrowed evidence coverage, arbitrary worker caps, weaker finality or relaxed admission.

## 12. Architectural judgment

| Judgment | Verdict | Basis |
|---|---|---|
| A1 Localize change | Satisfied in inspected scope | Credible provider, domain-extension and physical-substitution routes; genuine semantic changes affect their responsible owners |
| A2 Encode domain meaning explicitly | Satisfied in inspected finite scope | Normalization, binding, model application, coverage, selection, proof and publication behavior consume substantive owned definitions |
| A3 Extend through composition | Satisfied semantically | New supported workflows can compose existing operations; no second policy interpreter or universal framework is required |
| A4 Fit execution to supported workload | Violated | Mandatory Catalog work contains repeated correspondence, excess ordering/reconstruction and stale scratch charges; Behavioral adds a whole-result barrier and avoidable queue work |

**Bounded decision: Revise. Enclosing Catalog-creation architecture: needs revision at Implemented/Interface-checked source strength.** Preserve the explicit semantic model, exact source/provider identities, unknown outcomes, shared prepared graphs, SCC/delta reasoning, independent cold admission and drained unselected publication.

The next consequential decisions belong to the coordinator/operator’s design and plan route: resolve RC01/RC02 and assign the source-supported physical corrections. Functional acceptance must subsequently establish the complete current-source FastMCP journey. This review establishes neither release qualification nor a measured performance improvement.
