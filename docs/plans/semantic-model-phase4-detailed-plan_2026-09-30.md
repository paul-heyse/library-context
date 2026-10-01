# Phase 4: analysis and catalog — detailed design and execution plan

**Accepted execution target, 2026-09-30.** This document specifies cutover Phase 4, including the upstream
prerequisites its consumers expose. Package receipts below distinguish implementation and qualification.
The [parent cutover plan](semantic-model-cutover-plan_2026-09-29.md) owns cross-phase sequence and
finding disposition; this document owns Phase 4 contracts, packages and acceptance controls.
[DESIGN §15](../design/sections/semantic-model.md) remains the accepted semantic authority.
[ADR-0105](../adr/0105-analysis-vocabulary-epochs.md) and
[ADR-0106](../adr/0106-typed-analysis-and-catalog.md) record the accepted decisions and alternatives.
[ADR-0108](../adr/0108-immutable-analysis-owners.md) refines their ordinary result ownership at
actual publication boundaries; implementation remains subject to the foundation review.

**Inspected baseline:** main at 73187700fff553a0ca74abecdcfb7e8429c06617, 2026-09-30.
Concurrent after-turn tooling/instruction changes are outside this design.
The [Phase 3 plan §11](semantic-model-phase3-detailed-plan_2026-09-30.md#11-current-package-status-and-finding-routes)
owns its composite automated qualification and explicitly unmeasured pilot-scale costs.
Source/interface observations here are **Interface-checked** on the date above; all new contracts,
algorithms and benefits are **Proposed** unless an individual receipt says otherwise.

## 1. Outcome, scope and decisions

Phase 4 publishes a self-contained generation containing facts, normalized relations, typed
analysis results, a mandatory public API/evidence catalog and deterministic retrieval inputs.
Every conclusion has one semantic owner, explicit applicability and coverage, and same-generation
evidence. A library can have a usable catalog while behavioral analysis is NotRequested or Partial.

The implementation preserves these decisions:

1. **One model.** Ordinary types and operations in `lctx-model::domain` own interpretation,
   applicability, identities, invariants, verdicts, selection and synthesis policy.
   PostgreSQL owns durable relations; DataFusion owns relational compute; analytics kernels own
   algorithm execution. SQL, Python, adapters and renderers do not redefine these meanings.
2. **One full collection.** Capture once and build cumulative frontiers in one fresh attempt.
   No partial fact updates, external-generation import, legacy-ID translation or compatibility path.
3. **Stored graphs.** Complete graph inputs before constructing native petgraph projections.
   Build and store them once per full collection; hydrate and borrow them for repeated algorithms.
   Canonical relations remain authoritative. No graph reuse hash or request-time edge reconstruction.
4. **Immutable vocabulary epochs.** A bounded set of shared vocabulary relations can acquire new
   rows through immutable, explicitly closed prefixes. Lower-stage readers retain their original
   prefix and validation receipts. Ordinary relations still have one frozen output writer.
5. **Finite proofs.** Exact finite witnesses, unresolved alternatives and recursion residuals
   remain distinct. Witness DAGs never depend recursively on final aggregate summaries.
6. **Mandatory catalog, optional enrichment.** Brief seeds, behavioral proofs, communities,
   embeddings and successful optional analyses do not define the public API universe.
7. **Current library mechanisms first.** Retain the pinned BDD kernel, petgraph, Leiden,
   DataFusion/Arrow and existing storage/client libraries. Preserve the small weighted PageRank,
   finite worklist and bounded FCA kernels where the available replacements lose required semantics.
8. **No scope inflation.** Restore retained capabilities and repair their exposed contracts.
   General effect/exception/role recursive research, unrestricted dynamic Python, a capability
   ontology, new solvers, ANN tuning and PR6 comparative evaluation retain their existing triggers.

### 1.1 Phase 4 and Phase 5 responsibility map

| Boundary | Phase 4 owns | Phase 5 owns |
|---|---|---|
| Publication | Cumulative analysis/catalog relation closure, outcomes, invariants and canonical artifacts | Generated serving views/grants/indexes and server admission |
| Behavior | Transfers, execution/completion evidence, finite summaries, obligations, discharge and verdicts | Pure native request execution over those results and bounded explanations |
| Catalog | Public slots, configuration, original scenarios/deployment, associations and checks | Complete API packets, section expansion, browse and comparison |
| Requirements | Finite predicate/domain types, closure evidence, shared pure classification and conjunction | Wire encoding, request validation, candidate loading and result pagination |
| Retrieval | Addressable units/fragments, lexical text, embedding specifications and exact consumed vectors | Query embedding, BM25/exact-vector scoring, fusion and winning-unit expansion |
| Evidence | Canonical references, derivation sources and original artifact closure | Exact-byte hydration, response budgets and stable cursors |
| Lifecycle | One attempt and atomic generation publication; compile never selects | One published generation lease per server; startup, cancellation and retirement interactions |
| Qualification | Independent semantic controls and real-store Phase 4 acceptance | Real native/MCP journeys; later product comparison remains PR0/PR6 |

Do not confuse cutover Phase 4 with the older research “Stage 4 capability registry.”
That registry is not automatically activated. No Phase 4 conclusion claims Phase 5 is available.

## 2. Baseline and implementation seams

### 2.1 What is reused and what changes

| Inspected owner | Current state and consequence | Phase 4 decision |
|---|---|---|
| `domain/normalized/{entities,links,callables,events,bindings,coverage}` | P3 owns correspondence, public exposure, callable components, complete events, binding sets and scoped availability | Consume these contracts; do not repeat signature, descriptor, owner or uniqueness inference in catalog/behavior |
| `domain/projection/snapshot.rs` | MaterializedGraph stores a native Graph; public access is typed iteration, adjacency and reachability | Add borrowed algorithm views, preserving private indices and stored object ownership |
| `domain/stages.rs` and completed-stage store | Contributors precede one writer; completion freezes whole relations | Add vocabulary epoch publication before scheduling P4 consumers |
| `domain/transfer.rs` | TransferKey still owns an `Id<ProviderSymbol>` | Migrate derived transfer ownership to normalized callable EntityRef; raw provider symbols remain premises |
| `domain/composition.rs` | CallFrame accepts admission booleans and returns flattened obligation reasons | Consume private validated normalized admission; return subject-specific outcomes for the complete alternative domain |
| `conditions/stability.rs` | Guard-only witness rule is validator-private; substitution references CallArgument | Share entry-value derivation with producers; bind substitutions through stored CallBinding including receivers |
| `domain/derivation.rs` | Global conclusion-to-step-to-premise DAG check already exists | Preserve it; introduce finite witness occurrences before recursive aggregate publication |
| `domain/record.rs` | Scalar lacks a floating-point representation | Add a checked finite metric scalar; vector bytes remain an explicitly versioned codec |
| `lctx-analytics` and dormant core modules | Valuable kernels/controls still depend on cpg-schema and old IDs | Recover their semantics over the new model, then remove old authorities |
| `cpg-core::{catalog,surface,catalog_domains}` | Reconstruct public contracts and descriptor classifications from legacy rows | Build catalog-specific conclusions over P3 owners |
| `cpg-schema::selection` | Pure classifier depends on wire types | Move semantic types and classifier into model; P5 wire depends on them |
| `scripts/producer_fingerprint.rs` | Already includes crates/*/models, scripts, specs and producer sources | Preserve that closure and explicitly record authored catalog identity; do not claim model bytes are currently unhashed |

### 2.2 Responsibilities and allowed dependencies

- **lctx-model:** analysis/catalog concepts, operation contracts, pure domain transformations,
  declarations, shared validators, profile/capability policy and borrowed topology contract.
- **lctx-analytics:** bounded computational kernels over explicit validated inputs. No SQLx,
  acquisition, environment discovery, service client or hidden policy lookup.
- **cpg-core:** stage declarations and execution, DataFusion joins, admitted loading, kernel
  invocation, external embedding effects and typed result publication.
- **lctx-postgres:** lowering, private delta ingestion, epoch/ordinary-output completion,
  generation lifecycle, artifact storage and read capabilities. It does not classify semantics.
- **cpg-extract/cpg-flow:** native facts only, with explicit behavioral dependency context
  requirements prepared before extraction.
- **lctx-embed:** the existing effectful embedding client contract.
- **lctx CLI:** explicit profile/frontier/options, preflight and outcomes. Python remains dormant
  for P5 and never gains an independent semantic implementation.

No new crate or provider framework is needed. Pure domain transformations can stay ordinary
functions. Shared validators call the same authoritative operations, while independent tests use
hand-expected answers or separately implemented oracles.

## 3. Prerequisite: immutable shared vocabulary

### 3.1 Why ordinary stage contributions cannot work

Composition creates new predicates, evaluation atoms, BDD nodes, places and qualifications.
Those relations already belong to facts. Schedule::build orders their writer after all
contributors, while normalization requires the facts writer's completed checkpoint.
Adding an analysis contributor therefore creates a real dependency cycle. Reopening a completed
table would invalidate existing receipts and let a lower reader observe later vocabulary.

Choose a finite, model-declared vocabulary assembly mechanism. It is not a general multiwriter
table facility and does not permit modifying an existing semantic row.

### 3.2 Closed epochs and physical lowering

The initial whitelist is **Literal, LiteralSet, LiteralSetMember, PlaceRoot, PathSegment,
AccessPath, Place, Predicate, EvaluationAtom, ConditionNode, Condition and AssertionQualification**.
Binding-source/projection rows remain with their normalized writer. Facts, attribution,
coverage, analysis conclusions and support relations are not generally appendable.

- One VocabularyAssembly semantic owner admits these records. Its declared publication epochs
  are nodes in the schedule; each epoch depends on its previous closed prefix and named contributors.
- Domain identity and schema stay unchanged. The store adds an internal introduced_epoch column
  to whitelisted canonical tables. It is storage metadata, never a domain key or user-supplied field.
- Producers stream bounded batches into private, insert-only delta tables. They cannot write
  introduced_epoch or update/delete canonical rows.
- An epoch publication group contains the vocabulary delta and ordinary result deltas that
  reference it. A lifecycle-owned transaction merges vocabulary, inserts results, validates the
  candidate prefix and group invariants, freezes ordinary outputs, and records their receipts
  plus the closed-epoch receipt atomically. This avoids immediate-FK failures and a contributor/
  completion cycle; it also avoids retaining the entire result set in memory.
- Producers return a ComputedStage with sealed delta receipts, not CompletedRelation. These
  receipts permit group closure but never stored reads. Every planned output, including empty
  outputs, must be sealed; all COPY operations finish before close. Hold the existing attempt
  lifecycle lock and acquire deterministic relation locks, revoke delta INSERT, and verify frozen
  content before merging. Only the group's committed acknowledgement completes its stages.
- Replace blanket staging-table INSERT grants with explicit delta-only grants for this group.
  Canonical bases receive no importer writes; growing vocabulary bases receive no importer reads.
- Equal IDs with identical payload deduplicate; conflicting payloads fail the attempt. Earlier
  rows never change. New memberships cannot mutate an old literal set's declared meaning.
- Retain the current load-then-generated-FK-validation mechanism: load all group rows, validate
  generated references inside close, and install permanent generated FKs at final seal. Current
  FKs are not DEFERRABLE. Additional semantic reference checks require the same generation and
  visible prefix; a reference to a later epoch is invalid even if its base row exists.
- Full candidate-prefix validation includes canonical condition closure, guard legality,
  literal-set membership, shared qualifiers and the ordinary outputs' invariants. A failed close
  exposes neither vocabulary nor result receipts; unconfirmed commit uses the existing terminal
  recovery protocol.
- A failed/unconfirmed close poisons the attempt; no partial acknowledgement or in-place retry.
  Successful closure drops merged deltas while retaining receipts. Final publication rejects
  unclosed groups, missing outputs and remaining ingestion privileges.

For reads, lower immutable security-barrier views with a **literal closed epoch bound** and
explicit semantic columns. Importer read connections receive SELECT on these views, never on
growing canonical vocabulary tables. No mutable session setting chooses visibility. Existing
source-bound providers register the view selected by their private permit.

CompletedRelation/ReadPermit carry the generation, relation, closed prefix, content receipt,
required invariants and availability. Verification of an old prefix scans that prefix, not the
current whole table. Final seal verifies the last closed prefix and all ordinary outputs;
published readers see the final canonical content only after normal publication.

The plan declares a finite sequence of closes: facts vocabulary; dispatch/receiver vocabulary;
base semantic vocabulary; execution/model vocabulary; summary vocabulary; catalog/synthesis
vocabulary. Split a close only where an actual stage needs another stage's new vocabulary.
No stage reads its own unfinished group. Within one group, pure kernels can share charged immutable
inputs/outputs without pretending those results are already published.

ADR-0108 keeps this whitelist unchanged. Ordinary invocation/support/coverage families are finite
nominal instances at producing owners, not shared writable tables. The six closes above are an
initial outline: execution and catalog sub-stages add model-declared closes only when a stored
consumer needs newly produced vocabulary. An ordinary output needing no new vocabulary completes
against the existing prefix. Neither a global future-owner reference sum nor one final retained
handoff substitutes for these boundaries.

Implemented boundary identity and visibility are separate: `PublicationBoundary` has append-only
named codes, while `PublicationOrder` assigns a private schedule-bound `PrefixOrdinal` to each
scheduled close. Stored view bounds and ordering use the ordinal. Ordinary completion inherits the
maximum prefix of declared acknowledged inputs, including completed handoffs; unfinished private
group handoffs cannot mint completed receipts. A vocabulary foreign key must fit this inherited
bound and any explicitly narrower input. No-input ordinary outputs remain at the Facts prefix once
it is closed. Completion checks this before publishing any output receipt or grant.

### 3.3 Metadata and profiles

AnalysisInvocation is a new derived-analysis relation; it is not a fake ProviderRun reporting
native fact coverage. It refers to existing input/context, a typed analysis definition,
configuration/model catalog, input invocations and completed source receipts.
A genuinely new AnalysisContext required by configured compilation is declared before the facts
checkpoint. Context identity does not assert that any future analysis has completed.

Catalog profile keeps Flow NotRequested. Behavioral profile augments the native dependency context
with model-required symbols/classes and exact runtime exception classes before provider execution.
The whitelist does not allow late acquisition or changes to captured bytes.

## 4. Normalized admission and graph prerequisites

### 4.1 Dispatch and class-of receiver bindings

Perform these extensions before N6 builds stored graphs in every fresh normalized collection.
They are implementation packages of Phase 4 that amend the normalized layer; they do not append
edges to a previously published graph.

Introduce DispatchAssessment and DispatchMember, keyed to the original normalized event and raw
Overrides premise. Members cite the named method, qualifying override evidence, normalized
receiver class, complete MRO membership and provider/context coverage. Retain the named member
and admitted overriders; incomplete ancestry leaves an explicit open set.

Extend the model's normalized alternative source with a tagged **Native / DerivedDispatch**
reference. A derived source names its original CallTarget and dispatch member. No producer invents
a new raw CallTarget to impersonate a native assertion. Policy evaluation, complete-event
assessments, binding sets, projection roles and validators consume this same declared source.

The projection keeps every known conservative member and its uncertainty. Summary admission still
requires complete applicability; enumeration inside the captured class universe does not prove
that an arbitrary runtime subclass cannot exist. Dynamic/open-world cases retain OverrideDispatch.

For a class method accessed on an object, a derived receiver assessment supplies **ClassOf(actual)**.
Add a typed binding-source/value representation anchored to that actual expression. It is not
identity from the instance and does not prove a concrete runtime class. Raw Receiver::Unknown
evidence is preserved. A concrete class-dependent conclusion requires its own class evidence.
The existing qualified SyntaxPlacement facts retain Attribute.value; derive the exact source
receiver from those stored premises rather than creating a duplicate raw receiver fact.

P3's normalized-only route receives these corrected semantics after the preparatory packages pass.
Update the projection/version contracts and rebuild on the next full collection. Facts-only
compilation remains independent.

### 4.2 Composition admission

Replace public summary_admitted/unique_variant booleans with private CompositionAdmission and
ValidatedBoundCall access from normalized verification. Migrate transfer owner to EntityRef and
require the occurrence-owner premise for every call.

The operation enumerates the complete domain:
**source contribution × event alternative × binding variant × applicable callee contribution**.
Each member records an exact result, proven disjointness, semantic dominance, incompatibility,
or a typed obligation. Missing targets, undetermined variants and open remainders are members,
not rows that disappear in a join.

A successful shape binding alone cannot establish effective invocation authority. For known
candidates without positive admission, retain their evidence and conservative outcome; do not
construct an authority token from a count. Complete-set strengthening is owned by the shared
discharge operation, not by a graph traversal.

### 4.3 Borrowed computational topology

Expose NativeGraphView through MaterializedGraph::with_native_graph. Use an invariant generative
brand per callback, separate from the graph borrow lifetime, for its opaque GraphNode/GraphEdge
tokens. An ordinary shared lifetime alone cannot distinguish two hydrated graphs. The higher-ranked
callback cannot return branded values; map algorithm outputs to EntityRef/ArcId inside it.
Delegate required petgraph visit traits to the stored Graph. Token constructors are private;
tokens have no serialization, and mutation/the underlying Graph are not exposed. Compile-fail
controls reject a token from a different view and token escape. A source-level API check precedes
implementing the adapter; no new dependency is required for this narrow generative boundary.

Implement only the visit traits required by SCC/traversal and named analytics. Algorithms remain
in lctx-analytics. Filtered/reversed views borrow the graph. Output selection happens after the
declared algorithm universe is established. A dense kernel-specific array for Leiden or a numeric
iteration is an admitted computational conversion, not a second canonical graph.

Hydrate each named projection once per analysis preparation lifetime; retain its reservation until
all borrowing kernels finish. Do not deserialize a graph separately for every seed, metric or query.

## 5. Shared analysis contracts

### 5.1 Invocation, result and coverage vocabulary

The following proposed relation groups enter domain::analysis_relations(); names are concrete
design names for semantic families, with all structural fields derived from their Rust declarations
during implementation. ADR-0108 distinguishes that shared meaning from concrete producing owners.

| Group | Identity, contents and governing operation |
|---|---|
| AnalysisDefinition / MethodParameters | Typed method and semantic version; explicit parameters, limits, seed and model; no JSON-interpreted domain policy |
| AnalysisInvocation / AnalysisInput | Input/context, definition, source receipts, projection references and parent invocations; parents form a DAG |
| AnalysisOutcome / AnalysisDiagnostic | Completed/Partial/Unavailable/NotRequested with typed reasons; elapsed time and measured diagnostics separate from semantic identity |
| AnalysisCapability / AnalysisCoverage / AnalysisCoveragePremise | Expected capability × scope × context from admitted inputs, exact lower membership and producing receipts; empty observations never define the expected domain |
| TransferWitness / CompositionWitness / SummaryPublication | Finite evidence occurrences, typed premise edges and aggregate publication membership |
| AnalysisObligation / ObligationSubject / DischargeEvidence | Nominal subject, channel/phase, reason, responsible domain, status and exact discharging derivation |
| Finding / FindingMember / FindingSupport | One emitter; typed finding kind, subject/members, invocation and nominal evidence; derived fidelity rather than a producer-chosen status |
| Metric / CommunityMembership / Concept / ConceptIncidence | Algorithm-owned outputs with projection, parameters, coverage and heuristic/exact-under-context interpretation |
| AnalysisPolicy / ModelCatalog | Authored, versioned policies and model definitions, separate from observations/results |

Keep native ProviderCoverage and normalized coverage intact. Analysis coverage references them and
other analysis coverage through typed membership relations. It never overwrites native coverage.
Families remain facts families; retired FactFamily codes for generic findings/graphs stay reserved.

The current provider-attributed Assertion/Support path cannot mint derived evidence by inventing a
ProviderRun. Introduce a typed support-source sum separating NativeAssertion premises from
AnalysisDerivation premises. The latter cites AnalysisInvocation, qualified proposition and exact
nominal evidence. One shared qualification/emission operation derives modality, approximation,
condition and coverage for both paths; native provider attribution remains intact. Migrate the
existing analysis foundation records through this operation rather than duplicating their policy.
A domain operation that changes the semantic frame must first supply its own validated finite
proof conclusion. For Summary composition, exact binding/substitution replay validates a
SummaryWitness in its nominal Summary invocation and caller frame. A Summary-only support-source
arm admits that conclusion to the ordinary AnalysisDerivation operation; shared source policy
recomputes its status and heuristic lineage from the actual lower supports. Generic conjunction
never substitutes atoms, and no rebased qualification is disguised as a native observation.
The witness cites Local/Model alternatives or earlier witnesses, never final Summary aggregates;
its source adapter adds no Summary dependency to earlier producing owners. This clarification is
**Accepted target, 2026-10-01**; B0 implementation and the scheduled B3 review qualify it.

**Immutable producing owners (Accepted target, ADR-0108; implementation pending).** Definitions,
parameters, authored catalogs/policies and embedding specifications have early one-shot writers.
Late invocation/input/outcome/coverage/proposition/derivation/support/obligation records are concrete
nominal owner instances generated from common declarations, with common semantic operations.
The initial owner set is Dispatch, Local, BaseEvaluation, BaseCompletion, SourceCall,
EnrichedExecution, Model, Summary, Structural, AnalyticEmbedding, Analytic, CatalogCore, CatalogEvidence, Selection,
Synthesis and Retrieval. Each has a narrow predecessor set and invariant inputs; no earlier owner
references a later owner. Native assertion/support pairs may have one early immutable inventory
derived entirely from facts. Result-dependent subjects and source receipts cannot be predeclared.

The actual stored-read order is facts → normalized preparation → Dispatch → normalized checkpoint;
Local → BaseEvaluation → BaseCompletion → SourceCall → EnrichedExecution → Model → Summary;
Local → Structural → optional Analytic; and CatalogCore → CatalogEvidence → Selection → Synthesis
→ Retrieval. CatalogCore depends on the normalized checkpoint, independently of behavior. Analytic
depends on the separate E1 AnalyticEmbedding owner only when vectors are selected. Its dynamic
invocation/coverage cannot share A1's later writer; the shared static embedding specification stays
with its early one-shot owner. Synthesis also consumes Structural, Summary outcomes
and selected Analytic conclusions; Retrieval consumes C1/Synthesis plus E1's shared foundation.
Split Model or Analytic further only for a concrete internal stored-read dependency; bounded pure
handoffs remain permitted. Later discharge records reference earlier immutable obligations.
Final coverage has a separate writer checking the expected frontier; it never appends to prior sets.

Finding/Member/Support remain S0's single-writer family. Earlier stages produce qualified conclusions
for its common emitter. The global generated derivation index checks the combined nominal DAG;
a final read-only union may simplify consumers but is not another canonical write authority.

A completed computation can have Partial evidence. Refused required execution aborts publication;
a supported analysis boundary produces an explicit partial/unavailable outcome only where that
capability's contract permits it. Missing/corrupt required results are never represented as empty.

### 5.2 Typed metrics and resource contracts

Add FiniteF64 to the field/Arrow/PostgreSQL lowering: reject NaN and infinity, normalize metric
negative zero to positive zero, compare/hash canonical IEEE bits, lower to Arrow Float64 and
PostgreSQL double precision. Metric values are payloads, not finding/member identities.
This policy does not change source Literal float meaning or vector signed-zero bytes.

All retained input indexes, graphs, conversions, BDDs, worklists, proof members, output buffers,
DataFusion operators and validation closures reserve from the same AttemptRuntime.
Use bounded batch streaming for storage and DataFusion relational projection/filter/join/aggregate
where it removes whole-input retention. Pure semantic neighborhoods can use charged maps.

Preserve one admitted heavy stage/query at a time initially; no nested Rayon pools. Keep current
memory defaults and transfer limits, with explicit algorithm limits in MethodParameters.
Spilling remains disabled until a measured need justifies a separately qualified policy.
Reservation bounds are not a whole-process RSS guarantee.

A deterministic semantic work limit records its named obligation and uncovered subject. Failure to
reserve infrastructure memory aborts the owning stage unless a predeclared partial-result contract
can publish a fully validated prefix. Do not catch arbitrary allocation errors and publish success.

## 6. Behavioral evidence and finite composition

### 6.1 Authored models and dependency context

Move parsed model definitions and their semantic operations from cpg-schema into lctx-model.
Keep the pinned external model data under that owner. Catalog revision, bytes, applicability,
phase, source/target paths and per-channel completeness are explicit, digest-bound inputs.

Before behavioral extraction, derive ModelContextRequirements from the selected model catalog:
target modules/symbols, allocation/initialization/protocol members, relevant exception classes and
required MRO closure. Resolve them through the pinned captured environment. Missing definitions
produce model applicability uncertainty, not late imports from the operator environment.

The existing producer source fingerprint already covers model directories. Preserve it across the
move and expose the authored catalog digest in analysis/context configuration. A model catalog
change must change producer/analysis identity even if the analyzed library release is unchanged.

### 6.2 Entry-value witnesses and guard/type derivation

EntryValueWitness::derive is the one operation proving that a specific access sees a parameter's
entry value. Its premises are the normalized owner/formal, exact read/place, parameter definition,
complete provider/context-qualified reaching set and complete flow coverage.
Assignments, nested/nonlocal reaches, unbound alternatives and loop-carried reaches prevent proof.
The stored validator replays the same operation.

Use that witness both for Entry-root port construction and guard stability. Parameter-only reaching
proves binding identity, not arbitrary mutable object state. StabilityBasis owns predicate
eligibility; initially only the existing IsNone/IsValue substitution domain is admitted.
GuardSubstitution references the stored CallBinding, including a bound receiver, and retains
actual value, source atom, call event and witness. Substitution also emits the actual-place
ControlInfluence and its qualification; Selection remains distinct from value transfer.

Derive class sets, exhaustiveness and scalar predicate assessments from structural TypeTerms,
attributed type observations, operand links and Complete MRO evidence. Opaque, truncated,
display-only, open class hierarchies and incomplete coverage remain explicit. Use the retained
bounded finite theory over supported scalars/classes; no string-parsed type claims or new SMT engine.
Only a complete relevant domain can support refutation.

An unsupported captured-cell output emits CapturedStateUnavailable with its declaration and
attempted transfer; append this reason without renumbering codebooks. Keep the local observation.
Foreign formal inputs are disjoint from caller ports; closure outputs are neither globals nor
malformed generations. Full closure-environment modeling is outside this restored envelope.

### 6.3 Execution and completion channels

Preserve the acyclic base/enrichment organization already present in execution.rs:

1. Derive base closed-expression evaluation and source-read evidence.
2. Derive base statement completion, pending outcomes and frame-release evidence.
3. Prepare source calls using independently available binding/default/body/release premises.
4. Derive enriched expression/completion outcomes from those admitted source calls.
5. Apply context protocols/values, modeled identities and action triggers.
6. Produce finite summaries, discharge and behavioral findings.

These are declared dependencies, not “run twice and hope for convergence.” A source call must not
prove its own body completion or default availability. Separate relations represent base evidence
and enrichment; generated final views combine them under one semantic rule.

Retain:
- ordered argument evaluation and short-circuit behavior;
- explicit Return/Raise/Break/Continue and pending-outcome replacement by finalizers;
- handler matching, bare re-raise and handler-name cleanup boundaries;
- definition-time default availability and stability, independently of callee outcomes;
- invocation, normal, exceptional and finally action phases;
- synchronous context acquisition/entry/exit and separate resource identity;
- modeled identity, transfer, effect, callback, resource and exception applicability;
- per-origin reachability, field/global reads, dynamic access and complete-negative premises.

The existing supported channels are restored. This does not promise general recursive effect,
exception or role summaries, arbitrary async/generator lifecycle or arbitrary decorator execution.
Supported positives and unsupported channels coexist explicitly.

### 6.4 SCC semantics and finite proof identity

Use petgraph's iterative SCC algorithm over the stored conservative invocation projection,
canonically ordered callee-first. Include override-derived edges before SCC discovery.
Incomplete topology prevents a completeness claim; it does not erase known finite witnesses.

Retain a deterministic SCC-local worklist. Its qualified worklist-state key contains normalized owner, source
contribution/origin, channel and phase, input/output places, transfer kind, modality,
approximation and canonical condition. It excludes proof ID, arrival order and proof cost.
Distinct invoked guard evaluations remain distinct atoms through their exact lineage.
This is not a redefinition of TransferKey: the domain's transfer aggregation key still excludes
conditions, and final alternatives merge conditions by the existing owned OR operation while
retaining all witnesses. The worklist distinguishes conditions to decide which exact evidence
states have actually been composed and which representative remains admissible.

Keep nondominated depth/cost representatives per key and retain equal-cost evidence separately.
Reprocess callers only for a new semantic state or an improved admissible representative.
Preserve the existing **call-path depth 8** and **proof-step limit 64**, and the existing named
pair/expression/completion/BDD limits as versioned policy values rather than scattered constants.
Queue admission and processing both charge deterministic work.

Unconditional semantic repeats can converge early. Guarded recursion may generate distinct exact
conditions through depth eight; at the boundary emit a stable, subject-specific residual carrying
SummaryDepthLimit. A residual key names SCC, origin, semantic ports/channel and boundary event,
not an ever-growing exhausted suffix. Any abstract transfer representing it is Over-approximate
and cannot establish, refute or discharge. Never identify different invocations' opaque guards
or existentially erase them into an Exact result.

This replaces the unresolved aspiration that all guarded recursion converge without budget
obligations: **exact finite witnesses plus explicit residual uncertainty** is the selected domain.
Retain the existing multiple-lap depth 0/1/2 control; do not stop at the first repeated event.

Witnesses cite raw evidence or earlier witness occurrences. A composition witness references its
caller/callee witnesses, exact binding admission and substitution evidence. Its construction
rank increases and its depth/cost equation is checked. Final aggregate alternatives are published
only after their component closes, citing witness members:

```text
final alternative -> publication step -> finite witness -> earlier witnesses / raw evidence
```

Witnesses do not cite final aggregates. The global derivation DAG validator remains active,
including cross-step cycles. Bounded witness admission may leave an evidence-enumeration
boundary, but response-time witness truncation cannot downgrade an already established conclusion.

### 6.5 Obligations, discharge and verdicts

Persist every applicable origin/target/variant member, including incompatible, open and refused
members. An exact finite-path positive is a different proposition from an all-alternatives
claim. A successful witness never deletes an open sibling.

Use domain::obligation's one priority/discharge/verdict owner. Replace legacy numeric priority
tables. Discharge names the exact obligation subject and an admissible proof; set completion
requires every applicable alternative under the relevant coverage contract.
A later limited proof of an already established identical semantic alternative does not undo it.

Composition returns typed per-member outcomes. Disjointness, dominance, unsupported semantics,
work exhaustion and evidence absence remain distinguishable. The five behavioral verdicts stay
Established, Conditional, RefutedUnderModel, Unknown and NotAnalysed; the four selection states
in §8 are a separate contract. No empty exception/flow table proves absence without closure.

## 7. Topology and concept analytics

| Capability | Selected implementation and semantic contract | Consumer and default |
|---|---|---|
| Delegation/reachability | Borrowed petgraph traversal with canonical tie-breaking; preserve arc IDs, parallel evidence, isolates, boundaries and requested hop bounds | Structural findings/implementation context; restored default |
| SCC scheduling | petgraph 0.8.3 kosaraju_scc over stored invocation topology; canonical component/member order | Finite summaries; behavioral profile |
| Direct usage | Typed normalized Usage policy and deterministic relational aggregation | Default seed/related ordering |
| Weighted PageRank | Retain bounded weighted power iteration, declared damping/dangling treatment, tolerance, iterations and residual | Optional ranking; disabled by default |
| Communities | leiden-rs 0.8.1 without features; GraphDataBuilder conversion with explicit weights, direction/symmetrization, parallel collapse and lineage | Optional grouping/seeds; disabled by default |
| FCA | Bounded existing closure/NextClosure kernel with equal-length FixedBitSet contexts; fcars dev oracle compares concept sets | Optional shared-signature/context suggestions |
| RCA | One declared relational scaling step over calls/handoffs and the same FCA context, then bounded concept enumeration | Optional extension of FCA; requires FCA; no recursive ontology expansion |
| kNN and layers | Existing bounded exact vector similarity over admitted embeddings, no dense all-pairs persistence | Optional doc links/community layer with named consumer |

Direct usage preserves ranking.rs's existing counting contract: each distinct official-usage call
site contributes one, divided evenly among its distinct admitted targets **before** subsystem/public
output filtering. It is not an arc count. Retain incomplete/open target coverage alongside the
score, including after override expansion; a fractional heuristic share is not target certainty.

Petgraph page_rank is not a drop-in replacement for the weighted residual-reporting kernel.
Leiden's petgraph converter cannot reinterpret ArcId weights or choose the product's mixed-layer
normalization. Conversions are explicit and charged; no second semantic graph registry is introduced.
Preserve the legacy community universe of vertices touched by admitted layer pairs; report excluded
isolates explicitly instead of silently changing the partition universe. Analytic kNN admits equal
vector dimensions and finite values before dot products: the legacy zip-based truncation of
mismatched widths must become typed refusal. Threshold, top-k and ID tie-breaking remain explicit.

For each analysis, persist universe, output selector, projection version, direction, multiplicity,
weight policy, parameters, seed, library versions, convergence/diagnostics or explicit unavailable
diagnostics. Compare community partitions by membership, not numeric labels. Heuristic rank and
community membership cannot establish controls, limits, execution facts or behavioral absence.

Retain current technique defaults: communities, PageRank, FCA/RCA, analytic kNN and optional
type/mention/kNN community layers are off. Explicit technique selection records the entire resolved
set in configuration. RCA requires FCA; extra community layers require communities. Recovery of a
technique does not assert product value or reactivate ablation/performance campaigns.

## 8. Catalog, selection, synthesis and retrieval

### 8.1 Mandatory public catalog

Define CatalogMember by the normalized public access slot and owning release/input, independent
of improved resolution or signature knowledge. Member-to-exposure/candidate links retain all
evidence. Do not key the public slot by whichever declaration currently wins.

P3 public exposures identify module-level roots. C0 owns the pure expansion into nested public
paths, joining declaration parents, occurrence ownership, entity correspondence and qualified
ancestry by typed identity. Member names determine path segments and shadowing only after those
identity joins. Preserve rebinding, nearest-MRO and unresolved candidate evidence; do not invent
raw public-name observations. The slot key is the access module and full relative public path in
its input/release. Catalog coverage requires PublicExposure, Symbols, Ancestry, Types and Callables;
it does not require the broad Entities capability, whose current inventory includes Flow.

CatalogInvocation links the member to source/effective signature variants, descriptor/binding
context and applicability. Parameter/type/default rendering follows canonical signature/type rows.
Constructor associations retain own/inherited/synthetic origin. Keep ordered overload variants;
never manufacture a union signature.

Do not restore old catalog copies of signatures, parameters, types, class identity or decorator
classification. Extend the existing callable/aspect authority only for retained property
setter/deleter, context-manager, wrapping and pinned registration metadata. Metadata recognition
does not establish wrapper body, cleanup behavior or callable equivalence.

An unresolved effective surface still exposes known source contracts and original expressions.
Catalog admission requires its own references and scoped outcomes, not flow, brief selection,
analytics seeds or successful embedding.

### 8.2 Options, fields and associations

Use tagged option subjects: parameter slot, configuration field or documented deployment control.
Defaults distinguish absent, unavailable, unknown, literal None, other literal, expression and
factory. Lack of evaluation is never “no default.”

Separate declared associations, exact initialization, exact reads and unresolved link assessments.
An exact field link cites receiver identity, applicable constructor/variant binding, assignment or
read, and relevant alias/mutation qualifications. No all-fields propagation or name-suffix inference.

Every association preserves basis, source origin, event phase, modality and normalized evidence.
A singleton candidate remains a candidate. Catalog-only compilation publishes declarations and
uncertainty; behavioral evidence may establish a more precise field relationship.

### 8.3 Original scenarios, deployment and checks

Reference SourceArtifact/ArtifactChunk and canonical source spans. Catalog evidence does not
introduce a second original-body store. Preserve ordered spans and enclosing context, explicit
option expressions, unresolved setup dependencies and separate extraction, intent, parse, binding,
environment and execution statuses.

Do not remove setup, with/exception structure or preceding mutation to invent a runnable example.
Original Markdown/MDX coordinates and extracted Python coordinates remain separate.
Compilation does not execute source snippets. Supplied TaskReportObservation and deployment
receipts can support typed check conclusions; absent execution remains NotRun.

Release requirements/extras/entry points belong to release subjects once, rather than being
copied onto every API. An all-extras analysis environment cannot prove minimum installation.
Evidence roots cover public contracts, fields, scenarios, deployment and docs outside brief seeds.

### 8.4 Finite requirement semantics

Move the existing finite requirement/domain/witness vocabulary and pure classify/joint operations
to domain::selection. Wire types depend on these operations in P5; the model does not import wire.

Persist contextual selection domains, members and closure evidence. User-specific query results
are evaluated, not exhaustively materialized. Preserve Supported, Contradicted, Unresolved and
Conflicting; discovery is the default, with separate result groups.

- Existential support needs an applicable positive; member-level absence requires complete
  relevant exposure/variant coverage.
- Universal counterexamples stay explicit; an empty domain cannot yield vacuous support.
- Conflict compares compatible contexts. Opposite observations from different overloads are
  not automatically conflicting.
- Whole-conjunction context intersection is required; pairwise compatibility alone is insufficient.
- Runtime conditions join by exact evaluation/context identity, not printed predicate text.
- Missing type evidence, unloaded/unrequested domains and foreign nominal operands cannot
  establish a negative conclusion.
- Each new predicate declares domains, dependencies, witness construction and closure once.

### 8.5 Programmatic synthesis

One model-owned emitter constructs findings and programmatic assertions from typed support.
Finding identity includes kind, semantic subject/member roles and method context; repeated supports
do not strengthen the conclusion. Input invocation links explain how stages composed.

Retain extractive/doc-supported Outcome eligibility; omit an optional brief when no admissible
Outcome exists. Keep per-value facet outcomes, assertion status floor and ceiling, every warning,
limit and unresolved condition. Statistical inputs may support navigation/grouping/doc links,
never a control or behavioral claim. unicode-segmentation remains the sentence boundary mechanism.

Validate every cited finding, witness, member, parameter and original span in the same generation.
Brief text is a reproducible rendering, not an alternative semantic store.

The old mandatory manual-review target conflicts with the absence of a review consumer/workflow.
ADR-0106 adopts conditional operator review under the existing forward-plan F1 trigger:
automated grounding admits explicitly unreviewed briefs; it does not certify extracted prose as
manually checked. No review queue is introduced. D0 accepted that policy and superseded ADR-0005,
carrying its surviving synthesis and evidence contracts into ADR-0106.

### 8.6 Retrieval inputs and embedding realization

Move the finite retrieval-view definitions and deterministic renderer to domain::retrieval.
Keep four families: API/options, source, scenario and documentation/deployment. Briefs are an
additional consumer. A unit may have multiple subjects or no exact API subject.

Separate unit identity, fragment identity, original anchor, rendering version, input digest and
embedding specification. Deduplicate identical family/rendered text for lexical corpus preparation
while retaining every occurrence, subject and anchor. A ranking unit is not a requirement witness.

Persist prepared lexical text, typed fragments, embedding specs and exact consumed vectors through
canonical model relations. No Arrow bundle/import schema returns. Keep the existing f32
little-endian codec, dimension/spec/value-digest checks, finite/unit-norm admission and signed-zero
bytes. P5 pgvector is a derived physical realization, not the authority for these values.

Avoid a scheduling cycle between analytic kNN and later synthesis/retrieval. One model-owned
EmbeddingSpec declaration is completed before either consumer, and one realization service owns
token admission, cache winner selection and canonical vector validation. **AnalysisEmbeddingUse**
is the early producer's record of a vector consumed for normalized declaration/document text;
**RetrievalEmbeddingUse** is the later producer's record for completed retrieval text. Each has one
writer and its own typed subject, exact consumed bytes/spec/input hash and availability. Their
distinct identities describe distinct consumption events, not rival definitions of a vector.
Both use the same value codec and immutable cache policy. No shared multiwriter UsedEmbeddings
table or late append to ArtifactChunk is introduced. Analytic text uses normalized/source inputs
only; it never requires a synthesized brief or retrieval unit. A lexical-only retrieval outcome
does not retroactively change an already completed analytic invocation.
Stored validation requires equal exact winning bytes for every use of the same spec/input hash
within one generation, across both consumption relations. A later use cannot replace an earlier
winner; conflicting bytes abort publication rather than silently changing an analytic input.

The effectful orchestration uses lctx-embed and the retained cache:
- declare Effect::Embedding and selected service/spec/configuration before execution;
- tokenize/admit before service/cache insertion; byte truncation is not token admission;
- key cache lookups by complete spec plus exact input hash;
- freeze the immutable winning value and consumer-use references before passing it to a kernel;
- support cold replay from canonical consumed values without the service/cache;
- disclose lexical-only availability for optional vector failure; reject corrupt required artifacts.

No fake embedding receipt establishes live service behavior or ranking quality. Query-time scoring,
RRF60, exact-name promotion, eligibility-before-ranking, BM25 and exact pgvector execution remain P5.

## 9. Library qualification and alternatives

**Interface-checked, 2026-09-30.** Resolve documentation with Context7, then check the exact pin,
feature set and local source. Current-branch documentation is a navigation aid, not version proof.
The library-utilization catalog is optional context and is not a validation gate.

| Capability | Pin/mechanism selected | Alternatives and decision basis |
|---|---|---|
| Relational compute | DataFusion 55.1.0, Arrow 59.3.0, existing source-bound provider fork a41da22 | Use built-in joins/projection/aggregation; charged maps only for repeated semantic indexes. No federation or alternate SQL semantic owner |
| Topology | petgraph 0.8.3 with serde-1; borrowed visit traits/views; Postcard 1.1.3 alloc snapshots | Keep Graph for immutable compact topology. StableGraph mutation semantics are unnecessary; rustworkx/graphops only if a named algorithm cannot be supplied correctly by current mechanisms |
| Communities | leiden-rs 0.8.1, no features; explicit GraphDataBuilder | Default adapter cannot preserve ArcId/mixed-layer policy; sequential execution avoids extra thread-pool coordination |
| Weighted rank | Existing bounded weighted kernel; Leiden compute_flow can be an independent numerical control under matching parameters | petgraph page_rank lacks the required weight/convergence contract; no library swap for name similarity |
| Boolean conditions | biodivine-lib-bdd 0.6.3 through domain::conditions::Diagram | OxiDD would change manager/storage lifecycle without a measured need. Raw BDD APIs cannot bypass variable reconciliation/work/node limits |
| Recursive composition | petgraph SCC plus domain-specific deterministic worklist | Ascent 0.8.1 and datafrog 2.0.1 provide generic recursion but do not replace condition/witness/residual/resource semantics. No added production dependency |
| Finite theory | Existing model-owned bounded scalar/class operations | Z3 is not selected for restoration; escalation needs a concrete unsupported theory consumer and a separate decision |
| FCA/RCA | fixedbitset 0.5.7; existing bounded kernel; fcars 0.2.2 + bitvec 1.0.1 dev oracle | Library enumeration alone does not supply current budgets, lineage and RCA policy; compare concept sets, not order |
| Typed Arrow/storage | serde_arrow 0.15.1 arrow-59; SQLx 0.9.0; pgpq 0.12.0; existing SeaQuery lowering | Derive schema once; no JSON-domain or handwritten DDL copy. Physical vector adaptation remains explicit |
| Synthesis/client | unicode-segmentation 1.13.3; reqwest 0.12.28 via lctx-embed | Preserve existing focused contracts rather than new sentence/client frameworks |
| P5 contract dependencies | Schemars 1.2.2; jsonschema 0.58.2 dev-only; pgvector Rust 0.4.2/extension 0.8.6; existing bm25s | Verify input/output schema contracts separately; no new vector engine, ORM or lexical policy in P4 |

Pinned source anchors: workspace Cargo.toml/Cargo.lock and docs/pins.md; the rust-graphs,
rust-reasoning, datafusion and sqlx-postgres skills' versioned corpus; existing
domain/conditions/kernel.rs, lctx-analytics/{ranking,communities,concepts,summaries/worklist}.rs,
cpg-schema/embedding.rs and lctx-embed. Exact APIs supporting adoption are inspected again when a
pin or feature changes.

Documentation references:
[petgraph visit](https://docs.rs/petgraph/0.8.3/petgraph/visit/index.html),
[petgraph algorithms](https://docs.rs/petgraph/0.8.3/petgraph/algo/index.html),
[BDD 0.6.3](https://docs.rs/biodivine-lib-bdd/0.6.3/biodivine_lib_bdd/),
[Ascent 0.8.1](https://docs.rs/ascent/0.8.1/ascent/),
[DataFusion 55.1](https://docs.rs/datafusion/55.1.0/datafusion/),
[pgvector](https://github.com/pgvector/pgvector#readme),
[Schemars](https://github.com/GREsau/schemars/blob/master/docs/3-generating.md).
No new speed, RSS or retrieval-quality claim is established by this inspection.

## 10. Stage graph, frontiers and execution packages

### 10.1 Public interfaces

Extend the existing mandatory --through argument to **facts | normalized | analysis | catalog**.
Keep --profile catalog|behavioral, default catalog; keep explicit generation selection separate.
New frontier enum values are Analysis and Catalog, each declared by FrontierDescriptor only when
its full production/validation envelope is implemented. Unsupported serving still refuses before
acquisition or database effects.

Both new frontiers include lower layers. Analysis includes declared analysis outputs and explicit
NotRequested outcomes for inactive capabilities. Catalog includes mandatory catalog, selection
domains, optional synthesis, retrieval inputs and analysis closure. Neither is serving-ready.
Retain current technique configuration semantics and defaults; expose --techniques default or the
existing comma-separated +/- technique vocabulary only for analysis/catalog. Reject contradictory
flags before effects. Embedding enablement/spec remain explicit existing configuration.

generation show and compile JSON add typed analysis/catalog capability outcomes and stage
diagnostics. Existing fields keep their current meaning. No tool reports “ready to serve” from
selection or catalog publication alone.

### 10.2 Dependency graph

```mermaid
flowchart TD
  I[Capture and declared model context] --> F[Facts and vocabulary epoch 0]
  F --> N[Entities, links and callable surfaces]
  N --> D[Dispatch and receiver derivation]
  D --> B[Complete events, policies and bindings]
  B --> G[Build and store graphs once]
  G --> NC[Normalized checkpoint]
  NC --> C[Mandatory catalog and original evidence]
  NC --> E[Base evaluation, type and entry witnesses]
  E --> X[Source-call and completion enrichment]
  X --> M[Model, context and action evidence]
  M --> S[Finite SCC summaries and discharge]
  NC --> V[Optional analytic text and embedding realization]
  V --> T[Structural and optional topology analytics]
  NC --> T
  S --> A[Analysis coverage and checkpoint]
  T --> A
  C --> Q[Selection domains and pure classifier inputs]
  A --> Y[Programmatic findings and optional briefs]
  Q --> Y
  C --> R[Retrieval units and optional vector realization]
  Y --> R
  R --> P[Catalog coverage, validation and publication]
```

Vocabulary closes occur at the dependency boundaries in §3; each publication group drains readers
before its exclusive transition. Conditional behavioral nodes produce explicit NotRequested
outcomes for catalog profile rather than disappearing from the expected capability inventory.
Catalog core construction can precede optional behavior; publication still validates the selected
frontier's complete scheduled envelope.

### 10.3 Ordered packages

| Package | Concrete scope and owner | Prerequisite and completion evidence |
|---|---|---|
| D0 — design lock | Resolve/adopt proposed ADRs, inspect current baseline and partition retained expectations; model/design owners | Independent design/target review; every required choice settled before production edits |
| R0 — vocabulary publication | Model epoch declarations, store delta/group transaction, prefix views/permits/receipts and final seal | D0; real PG prefix isolation, FK, dedup/conflict, rollback/cancel and old-receipt tests |
| R1 — derived result foundation | Analysis definitions/invocations/coverage, FiniteF64, nominal obligations, emitter/proof contracts, contribution APIs | R0; pure ownership/identity controls and real-store invalid-result refusal |
| N0 — model context | Typed authored catalog, dependency requirements, exact exceptions/MRO capture, explicit catalog digest | R1; native model-context positive/missing/version-change controls; catalog Flow unchanged |
| N1 — normalized consumer corrections | Derived override alternatives, ClassOf receiver binding, complete policy/binding verification before N6 | N0; diamond/open-world/classmethod controls and override-induced SCC edge in stored graph |
| G0 — graph algorithm access | Borrowed, generatively branded native views and lifetime/resource ownership; canonical SCC schedule | N1; wrong-view/escape compile-fail, no edge reconstruction, native roundtrip, isolate/parallel/cycle and reservation controls |
| B0 — local semantic foundations | Entity-owned transfers; shared entry witness, guard/type derivation and source contribution identities | R1/N1; rebinding, provider/context, nested reach, scalar/MRO and receiver twins |
| B1 — execution/completion | Base then enriched evaluation, default/body/frame-exit evidence, handlers/finalizers and source-call preparation | B0; retained independent evaluation/completion/frame controls; no circular premise |
| B2 — models and protocols | Context values, modeled identities, actions/phase postconditions, exact exception applicability | N0/B1; invocation-vs-normal and resource-identity controls |
| B3 — summary/discharge | Typed complete alternative outcomes, finite witness DAG, SCC worklist/residuals, coverage and final verdicts | G0/B2; recursion, cap, sibling, proof-cycle and shuffle controls |
| A0 — structural analytics | Pass A–C meanings, direct usage, public paths, qualified conclusions and evidence for S0's emitter | G0/B0; independent graph/source controls, no heuristic promotion |
| E1 — embedding foundation and analytic realization | One spec/codec/service contract, normalized analytic text and early AnalysisEmbeddingUse; no catalog/brief input | R1/N1; token/spec/shape, immutable winner and service-free replay controls |
| A1 — optional analytics | Communities, weighted rank, FCA/RCA and retained kNN/layers with explicit selection and consumers | A0; E1 only when vectors requested; oracle/shape/determinism/limit controls; defaults stay off |
| C0 — mandatory catalog | Public slots/contracts/constructors/options; consume normalized authority without flow | N1/R1; no-flow/no-brief/no-seed and signature/constructor/default controls |
| C1 — contextual evidence | Exact field associations, original scenarios/deployment/checks and complete evidence roots | C0; two-field, original-byte, intent/check and outside-seed controls |
| C2 — selection | Finite domain/witness vocabulary, closure and pure classify/joint operations | C1; overload/context/empty/missing-domain/whole-conjunction controls |
| S0 — synthesis | Shared finding/assertion policy, per-value outcomes, optional grounded briefs and source patterns | A0/B3/C2 and A1 where selected analytics affect output; condition, evidence, status and warning/limit retention controls |
| E0 — retrieval realization | Typed units/fragments/rendering, optional RetrievalEmbeddingUse, canonical exact consumed-value replay | C1/S0 and E1 shared foundation, never its optional analytic output; dedup/anchors/spec/cold-replay/corruption controls |
| F0 — assembly and CLI | Analysis/catalog FrontierDescriptors, profile/technique preflight, capabilities and generation diagnostics | All selected capability producers; both profiles through real PG, lower-frontier refusal controls |
| X0 — ownership retirement | Remove migrated legacy authority and move independent dormant expectations; inventory P5 survivors | F0; active-path dependency/search audit, no compatibility routes |
| Q0 — assembled exit | Full authorized-scope functional acceptance, independent target review, current owner docs and handoff | X0; precise composite receipt and explicit unqualified boundaries |

This is dependency order, not a requirement for artificial one-file commits. Catalog C0–C2 can
develop after N1 independently of behavioral kernels; assembly waits for all retained scope.
Reviews are scheduled after R0/R1, after B3 with its prerequisites, and once over assembled P4.
Ordinary slices use compile checks and targeted tests, not repeated architecture reviews.

## 11. Migration and preservation inventory

A table count is not a target schema. The historical 121 analysis/catalog families preserve
obligations; their independent expectations move to the new authorities before source removal.

| Legacy family/consumer group | Replacement and deletion boundary |
|---|---|
| CatalogCompilation/Members/Bindings/Signatures/Constructors/Parameters/Types/TypeArgs/TypeObservations/Surfaces | C0 public slot/invocation records plus references/views over normalized signatures/types; remove duplicate semantic copies |
| CatalogConfigurations/FieldLinks/Evidence | C1 typed option/link/support conclusions; source/effective uncertainty retained |
| CatalogArtifacts/Spans/Scenarios/Deployments/Associations | Canonical original artifacts/spans plus C1 context/subject/basis relationships; no copied original store |
| CatalogSelectionDomains and wire requirements' semantic types | C2 typed domain/member/closure relations and pure classifier; only transport DTOs remain P5 |
| PublicPaths, Delegations, Handoffs, ArgumentFlows, Guards, ParameterReads | A0/B0 typed structural observations, transfers/influences and exact path evidence |
| Operations/OperationFacets/OperationFacetStatus | Typed per-subject/per-value analysis conclusions and scoped availability; no free-form classifier |
| ValueFlows/Contributions/ReachBoundaries/PredecessorCandidates/Compatibility | B0/B3 transfer witnesses, complete alternative domains, outcomes and obligations |
| AnalysisConditions/AnalysisConditionNodes and FlowTestValueLinks/ExactOrigins | One shared BDD vocabulary/epoch mechanism and current normalized operand links; remove parallel condition authority |
| SummaryComponents/Flows/FlowSteps/Boundaries/OriginCoverage/BehaviorDischarges | B3 finite witness DAG, aggregate publication, typed residuals and central discharge |
| SourceParameterIdentities/SourceModeledIdentities/ModeledAssignmentReturnPaths | Shared entry-value witnesses and B2 modeled identity evidence |
| ModelTargets/Applications/FormalPaths/ArgumentBindings; ModelTransfers/Effects/Callbacks/Resources/Exceptions | Typed model catalog/applicability and channel-specific application records; raw facts stay attributed |
| ModeledCallback/Resource/Transfer/ExactValueTransfer/ArgumentEvaluation/Effect/Exception sites | B2 explicit source-call/model evidence and independent action/value/phase outcomes |
| ExpressionEvaluations/Steps; CallExecutions/Steps; SourceCallBindings/Normals/HeaderSteps | B1 acyclic base/enriched execution relations over stored normalized bindings |
| SourceBodyCompletions/Steps/ReleaseInputs; ReturnCompletionCertificates; ModelFrameExits/Arguments/Steps | B1 body/frame completion certificates, exact premises and refusal outcomes |
| ExitSites, ReturnExit/Entry statuses/steps, StatementCompletions/Steps | B1 ordered completion/pending outcome proof relations |
| HandlerClauses/Types/Actions/ReturnNoneSites; ModeledExceptionHandlerCandidates/Walks/ReturnNonePaths | B1/B2 handler/exception evidence and cleanup boundaries |
| ModelContextProtocols/SourceContextSites/Arguments/ValueIdentities | B2 typed protocol/lifecycle/value evidence |
| ModeledActionAssessments/Postconditions | B2 separately admitted trigger and exit-phase postconditions |
| FieldAccesses/AmbientReads/DynamicAccesses/RaiseSites/Singletons/NegativePremises/Behaviors/Steps | B0–B3 local observations, supported derivations, scoped negative premises and obligations |
| ConceptAttributes/Incidences and analytic invocation/member/witness tables | A1/R1 typed contexts, concepts, metrics, shared emitter and invocation lineage |
| Findings/Evidence/Assertions/AssertionSupport/AssertionPolicy | R1/S0 one model-owned emitter and derived support/status policy |
| Briefs/BriefAssertions/Members/Documents | S0 optional canonical briefs and reproducible rendering |
| EmbeddingSpecs/UsedEmbeddings/EmbeddingUses; retrieval artifacts outside the legacy table macro | E1/E0 shared spec/value operation with separately owned analytic/retrieval consumption records and addressable units |
| Legacy Nodes/Edges/ProjectionSpec and provider/call identity maps | N1/G0/A0 current normalized projections/admission; delete when last active P4 consumer moves |
| cpg-schema query/wire/bundle/serving tables and native/Python consumers | Explicit P5 recovery-only survivors; no active P4 dependency or adapter |

Audit both the legacy analysis macro and consumers outside it. Each removed field must either map
to a current typed meaning, be mechanically derived, or have a documented consumer-based retirement.
Deleting a producer does not retire a capability. Do not delete the whole cpg-schema crate while
P5 still needs its independently partitioned recovery expectations.

Dormant test migration:
- analysis/behavior/compile/syntax/function_implementation/resolved_attribute_access: recover
  P4 transfers, completion, handlers, type/guard and behavior expectations.
- graph: recover remaining analytic/SCC/FCA/community controls; P3 already owns normalized shape.
- catalog/catalog_evidence: recover public contracts, two-field nonleakage, original context,
  candidate associations and checks.
- catalog/pr4 and selection_evidence: split pure classification/rendering/realization controls
  into P4; leave cursor, native, byte pagination and request hydration expectations in P5.
- bundle and serving-specific tests remain named P5 obligations. Their Delta/import mechanics
  never return.

Schema/model/producer/projection revisions change during implementation. Rebuild only project-owned
generations after inspecting state and quiescing readers; preserve retained services, evaluation
assets and benchmark captures. No store mutation is part of this design-document task.

## 12. Verification and acceptance

### 12.1 Independent controls required during implementation

| Area | Discriminating controls |
|---|---|
| Vocabulary lifecycle | Reader at epoch 0 cannot see epoch 1; later row cannot alter old payload or meaning; identical duplicate dedups; FK to future/foreign row refuses; failed/cancelled close exposes nothing; lost commit is Unconfirmed; published readers see final closed content |
| Admission/coverage | Forged event/binding owner/receipt refuses; absent/partial/unrequested/complete-empty stay distinct; no observation does not suppress expected capability coverage |
| Entry/conditions | Formal rebinding, nonlocal reach, loop carry and wrong provider defeat entry proof; implicit/explicit receiver twins agree; mutation-sensitive predicates cannot use binding-only stability |
| Dispatch | Complete/incomplete diamond MRO; unknown subclasses remain open; override-induced cycle appears before graph construction; class-of and object are distinct |
| Recursion/proofs | Guard-free depths 0/1/2; one/two-site opaque guards; g and not-g across invocations; no-base cycle; equal-cost/cheaper witnesses; capped sibling stays open; shuffled input; injected derivation cycle or removed premise refuses |
| Completion/models | Short-circuit skipped operand; normal/failing default; pending return versus replacement finalizer; nested handler/bare re-raise; invocation effect versus normal postcondition; symbolic acquisition versus resource identity |
| Topology/heuristics | Isolates/parallel/self-loop/reconvergence/cross-module cycles; selector does not shrink universe; invalid weights; canonical partitions; FCA oracle sets; explicit non-convergence and work refusal |
| Catalog | No flow/seeds/brief/vector; source/effective uncertainty; ordered overloads, aliases and constructors; None versus unknown default; two fields/unrelated reader; candidate association stays unresolved |
| Evidence/checks | Original MDX/fence byte coordinates, enclosing context, expected-failure intent, API outside brief seeds, release-only requirements, absent/failed execution, unrelated distribution |
| Selection | Existential/universal/empty domains, unloaded types, incompatible overloads/configurations, comparable conflicts, three-way conjunction with pairwise-only overlap, foreign IDs |
| Synthesis/retrieval | Same-generation support closure, identity versus derived wording, per-value outcomes, full warnings, family text dedup retaining anchors, wrong spec/width/nonfinite vector, immutable cache winner and service-free cold replay |
| Assembled store | Real disposable PG18 for both profiles/frontiers; exact shared validation, read/write refusal, stage failure/abort and reconstruction from captured inputs |

Test expectations must not be generated from the same production rules they are meant to challenge.
Use existing fixture answers, hand-constructed semantic twins and existing independent FCA/numerical/
runtime oracles where they test the selected contract. Gold/held-out material never enters compilation.

### 12.2 Commands and timing

During production implementation, use the build-environment wrapper with cargo check on changed
crates and focused release-profile tests. Add real-store cases through the existing disposable
PostgreSQL harness. Review changed schema snapshots before accepting them.

After all functional packages and retirement are complete, run the repository's integrated
functional acceptance and required checks under its then-current command/after-turn ownership.
Do not run formats/lints/full gates after each package. Fix failed checks and report composite
receipts honestly. The assembled design review evaluates source and exercised contracts;
documentation link success is not semantic acceptance.

For **this design-document task**, documentation publication and ADR metadata/index checks belong
to the automatic end-of-turn hook under the current repository instructions; do not run or inspect
those checks manually. Record only receipts actually available, without anticipating hook success.
No production code gate, actual FastMCP compile, embedding campaign, store reset, full-library output
comparison or benchmark is required. The user-stopped Phase 3 pilot/RSS/hydration measurements remain
not_run until explicitly reactivated. Future Phase 4 real-library smoke/measurements must be separately
identified in qualification; their omission cannot support a performance or product-quality claim.

### 12.3 Phase 4 exit

Phase 4 implementation exits when:
- both profiles publish the declared analysis/catalog frontiers through the real store;
- every retained capability has its model contract, producer, validator, explicit availability,
  independent controls and consumer route;
- shared vocabulary, graph hydration and all proof/coverage closures obey their generation boundary;
- no active P4 path imports legacy semantic authority;
- assembled review has no unresolved design/fidelity gap in claimed scope;
- exact check receipts and the remaining P5 contracts are recorded.

Performance measurements, full-library comparisons and served journeys have separate scopes.
A bounded acceptance must name any excluded qualification; it cannot label P5 or PR6 complete.

## 13. Findings, review and change scenarios

The parent cutover §8 remains the current cross-phase disposition owner. The table below maps
its open obligations to packages and closure evidence; it does not close them by accepting a plan.

| Source obligation | Package | Closure evidence |
|---|---|---|
| semantic-data-model F01/F03/F06 | C0/C1/B0 | Explicit default uncertainty, association basis and exact field/reader twin |
| semantic-data-model F02/F09; core C04 | N1/G0/A0/B3 | One normalized policy/admission owner through stored graph and real composition |
| semantic-data-model F04/F05/F07/F08 | R1/B0/B3/S0 | Transfer algebra, one condition owner, typed obligations, emitter/invocation/proof closure |
| semantic-data-model F13 | N0/C1/E0; P5 remainder | Named registration/model applicability and policy/spec identity |
| C4/C5 F02/F04/F06–F09; R1/R4; P0 exit F03 | R0/B0/B3 | Entry witness, private admission, finite recursion residuals, subject-specific outcomes, influence and two-variant guard controls |
| A6–A8 F02/F04/F07 | N1/B0 | Complete dispatch membership, symbolic class-of binding and structural type/refutation coverage |
| core C13 | R1/S0; P5 explanation remainder | Nominal derivation graph and generated index, then bounded serving lookup |
| P3 exit O2 | G0 | Algorithms borrow hydrated native graphs without reconstruction |
| P3 normalization/exit O1 and resource F07/F08 | R0/R1/Q0 | Preserve current reservations/allowances; measurement remains separately unqualified |
| Forward plan product/PR3/PR4 findings | C0–E0; P5 wire remainder | Re-home retained independent controls; historical PR closure is not current P4 qualification |

### 13.1 Change scenarios used for architectural review

| Scenario | Expected change locality and acceptance |
|---|---|
| Add a model using existing channels | One model declaration and actual new domain behavior; context requirements/digests/validation follow existing operations; no new core/Python classifier |
| Add a new analysis over existing facts | Declare inputs, capability/outcomes, method parameters and kernel; store/receipts derive; no extractor change or hand-authored serving schema |
| Replace a graph/numeric kernel | Consumer contract states universe, weights, precision, diagnostics and limits; adapter replacement must preserve them and pass independent controls |
| Add a selection predicate | One domain predicate with dependencies, witness and closure semantics; query/render layers consume its result |
| Add evidence association/rendering | Association uses typed basis and originals; renderer cannot upgrade evidence or alter selection semantics |
| Recollect a changed library/model | Full fresh capture/derivation, one graph construction and self-contained publication; old reader stays on its lease; no incremental repair |

### 13.2 Current design status

Full D0–Q0 execution was authorized on 2026-09-30, including ADR-0106's conditional operator-review policy.
D0 adopts ADR-0105/0106 and preserves the independently reviewed target and §11 migration inventory.
The implementation baseline is clean main `24e86d5`; production sources and dependency manifests
were unchanged from the inspected design baseline at execution start. R0 is implemented with focused
pure, real-store and source-bound provider controls. The independent
[R0/R1 foundation review](../design_review/reviews/design_review_phase4-foundation_2026-09-30.md)
is **Accept scoped, 2026-10-01**, at frozen `0a60954`, with no material in-scope finding.
Its O1 retains downstream selector activation prerequisites; O2 distinguishes scoped preparation
controls from full assembly. Parent §8 owns their current dispositions.
ADR-0108's additional close boundaries require separating named publication identity from the
schedule's contiguous physical prefix ordinal; that refinement is integrated as `5262be0`.
Ordinary outputs now preserve that authority transitively through acknowledged declared inputs,
and publication refuses vocabulary references outside the inherited or narrower explicit prefix.
R1's finite metrics, sixteen nominal producing-owner families, exact native premise inventory,
source-bound expected coverage and cross-owner discharge are integrated. Actual scheduled publication
replays shared model callbacks against acknowledged inputs and the registry profile before granting
read access. LocalTransfers, Catalog, CatalogEvidence and AnalyticEmbedding have their scoped
producer selectors; other nonempty owner invocations refuse until their packages supply one. The testing-only
unscheduled Harness exercises ordinary model invariants but cannot qualify source-sensitive publication.
Those controls use scheduled `begin_conformance` attempts and actual completed inputs.

**R0 validation-view correction, Implemented / focused-Tested, 2026-10-01.** B0 substitution
controls exposed an earlier validator replaying its facts-derived normalization against later
vocabulary. Validation inputs now declare their immutable boundary explicitly, and the store
resolves that boundary in the registered schedule, verifies its closed receipt and refuses reads
beyond the particular relation's acknowledged source. Stage-input, publication, group-close and
final invariant runners preserve `(relation, prefix)` when dispatching inputs. Normalization and
its binding preparation remain Facts-bound; global nominal/condition integrity still checks the
full candidate or final union. Memory validation retains charged immutable snapshots. Independent
bounded static advice supported this correction to ADR-0105's existing contract; it is not a new
compatibility authority. Memory controls qualify group/final invariant routing; actual publication
and input-grant checks are qualified through PostgreSQL.

N0's immutable authored catalog, retained parser reservations, native dependency requirements and
early configuration/native-inventory producers are integrated. N1 receiver proofs and captured open
dispatch members are integrated; normalized alternatives use Native/DerivedDispatch sources and
projection snapshots are VERSION3, including exact lexical source-definition edges. Open override shape bindings retain SourceInspection authority
and do not become Summary/composition admission. Preparatory G0 borrowed graph views and canonical
SCC scheduling are integrated. Collection-owned preparation hydrates selected stored snapshots once,
retains their reservations and admits later borrowers only against identical completed sources and
the same budget; F0 still must own that lifetime in the assembled pipeline. B0 entry-value/guard,
nominal Local/Model/Summary transfer ownership, private call-binding admission and finite replayed
Summary witnesses are integrated with focused native/store controls. The actual direct Local producer
and execution-domain-qualified entry sources are integrated and focused-Tested against the assembled
tree. Local remains Partial/IncompleteDomain in behavioral mode and explicitly
NotRequested in catalog mode. Type theory, exact field state, crossed-call semantics and scheduled
B3 Summary publication remain in progress. The removed legacy PostgreSQL stability fixture is
replaced only in its Entry/Local scope: actual scheduled Summary GuardSubstitution roundtrip,
unsubstituted-bound-guard refusal and forged-substitution refusal remain explicit B3 obligations.
C0 catalog is integrated with focused pure/native
PostgreSQL controls: stable public slots, source/effective contracts, constructors, total defaults,
source aliases and normalized callable metadata. The normalized pipeline now executes its metadata
stage. E1 shared embedding configuration,
codec and realization service are implemented with focused controls: `lctx-model` owns the sole Spec,
the exact f32 byte recipe and typed early configuration; the retained cache and HTTP client consume
that owner. Actual stored selection and declared Embedding effect admit realization. Analytic text is now
Implemented / focused-Tested: version 2 preserves captured parameter and original document bytes,
uses exact declaration/docstring syntax links, and publishes charged UTF-8 windows or named
unavailability from normalized/source inputs. Nominal AnalysisEmbeddingUse is now Implemented /
focused-Tested through native extraction and actual PostgreSQL. Its exact source-derived invocation
and window universe, selected effect, availability, consumed bytes and cold replay are validated.
The shared winning-byte replay kernel and RetrievalEmbeddingUse/cross-consumer validator are
Implemented / focused-Tested; final E0 nominal/vector activation remains open. The dormant global receipt writer remains an X0 deletion obligation.
C1 source/document/deployment evidence is integrated and focused-Tested, preserving original
coordinates, exact C0/native frames and release ownership. Runtime field promotion remains pending
actual Local/B1 evidence. C2 declaration selection is integrated and focused-Tested: exact contextual
domains, finite classify/joint operations, four outcome groups and cold replay preserve missing
evidence and scope. Runtime-dependent field/state facets remain open. Downstream execution,
analytics, synthesis/retrieval, CLI assembly,
ownership retirement and Q0 remain pending. Existing descriptions marked Proposed specify accepted
targets whose behavior is not yet implemented or tested; they are not runtime claims.
The [independent assembled review](../design_review/reviews/design_review_phase4-plan_2026-09-30.md)
is **Accept scoped, 2026-09-30**, at Proposed design level. A1–A3 and the document-target gates are
satisfied in the stated envelope; this is a static design judgment, not runtime qualification.

Review F01 identified that a lifetime alone cannot identify a graph. The corrected §4.3 chooses
invariant generative graph brands; the reviewer reinspected that contract. The parent
[§8 disposition](semantic-model-cutover-plan_2026-09-29.md#8-findings-disposition) owns current status:
design addressed, G0 implementation and compile-fail controls pending. The direct-usage counting
rule and cross-consumer vector-winner invariant were also clarified during assembled review.

| Task receipt | Outcome, 2026-09-30 |
|---|---|
| A0 context and bounded traversal controls | passed, 2026-10-01: the documentary integration command below also passed actual Structural publication in both PG profiles and 3 pure replay controls. Native entry reads and suppression checks are restricted to the matching analysis context. A native forwarding chain beyond the configured depth retains an explicit traversal stop; `/tmp/phase4-documentary-control-limits1.log`. |
| S0 documentary preparation / schema migration | passed, 2026-10-01: frozen `3123758` integrated; `python3 scripts/build_environment.py -- cargo test --release --no-fail-fast -p lctx-model -p cpg-core --test synthesis_documentary --test structural`, `/tmp/phase4-documentary-control-limits1.log`: actual documentary PG publication/seal/validation passed alongside 4 structural controls. Documentary conclusions retain exact native literal source spans and independent source qualifications. Full assertions, seeds, briefs and retrieval publication remain open. |
| A0 Pass B/C and qualified structural publication / schema migration | composite passed, 2026-10-01: the wrapped Cargo command in the source-body receipt below passed 3 structural replay controls and one actual native/PG18 control across both profiles, `/tmp/phase4-body-controls-integrated3.log`. The four-method Structural frame now retains canonical binding/Local identity forwarding, one exact reaching-definition alias, literal and unfollowed/unpacked arguments, bounded per-formal paths and conditional raise observations; official nested and named producer-result handoffs retain exact site/phase/modality, source-role ordering and witness caps. Catalog retains nested handoffs while named Flow proofs and Controls remain unavailable/NotRequested. Native controls reject literal-as-producer identity and suppress claims along caught/value-tested paths or locally swallowed raises. Initial controls failed the Local RHS/LHS target join (corrected in `206ef98`) and an overly strong substitution-stability requirement for static raises; the latter now uses an exact Local Use entry read inside the native test plus branch implication, without broadening guard substitution. Exact source qualifications remain separate from static S0 conclusions. Full seal/replay and reservation release pass; assembled acceptance remains open. |
| B1 source-body publication / schema migration | passed, 2026-10-01: frozen `69e3d50` integrated with canonical Local configuration; `python3 scripts/build_environment.py -- cargo test --release --no-fail-fast -p lctx-model -p cpg-extract -p cpg-core --test base_execution --test execution_channels --test execution_outcomes --test structural`, `/tmp/phase4-body-controls-integrated3.log` (B1: 4 real PG + 10 native + 3 outcome controls; A0: 4 additional controls). BaseCompletion publishes independently replayed source-body certificates, ordered native/statement premises, mandatory frame-release inputs and refusals. Async functions and unreachable-yield generators cannot inherit ordinary synchronous body completion. Coupled status/member changes and inventory erasure refuse. Fresh source calls, enriched execution and B2/B3 remain open. |
| B0 native assignment target correction / schema migration | composite passed, 2026-10-01: `python3 scripts/build_environment.py -- cargo test --release --no-fail-fast -p cpg-core -p cpg-extract -p lctx-model --test local_semantics --test structural --test base_execution`, `/tmp/phase4-assignment-controls-tests2.log`: Local scope 13 native + 2 real PG; Base regression 4 real PG passed. Local Definition contributions now join the native RHS value to its exact definition observation and support, retaining both premises and the target place. Rebound inputs and missing target support refuse. A0 first exposed the erroneous RHS-to-LHS join; its alias positive now passes, but the same combined run failed its subsequent conditional-raise control, which remains open. This is Local qualification, not a clean combined or phase gate. |
| B1 BaseCompletion production / schema migration | passed, 2026-10-01: frozen `c27bf6f` integrated; `python3 scripts/build_environment.py -- cargo test --release --no-fail-fast -p lctx-model -p cpg-extract -p cpg-core --test base_execution --test execution_channels --test execution_outcomes --test structural`, `/tmp/phase4-completion-handoffs-integrated1.log` (4 real PG + 9 native + 3 outcome controls for B1; 4 additional structural controls). BaseCompletion reads actual earlier BaseEvaluation results and publishes its complete statement/refusal universe, exact parents, finalizer outcome order and actual-profile coverage. Coupled inventory removal and stored premise corruption refuse replay. Unsupported exact attribute readiness retains `HeapFieldStateUnavailable` code58; this does not replace retained per-origin field/global observations or finite field transfers. Canonical Local fixtures and current nominal coverage selectors are preserved. Source-body certificates, fresh calls, enrichment and B2/B3 remain open. Worker `/tmp/phase4-completion-production-pg2.log` passed the same four PG profiles after correcting an initial wrong capability selector; that earlier failure is retained. |
| B0 finite theory/field evidence and B1 BaseEvaluation production / schema migration | passed, 2026-10-01: frozen `9f60ec0` and `bab1be7` integrated against main; `python3 scripts/build_environment.py -- cargo check -p lctx-model -p cpg-core`, `/tmp/phase4-local-base-integrated-check1.log`; the same wrapper `cargo test --release --no-fail-fast -p lctx-model -p cpg-extract -p cpg-core --test domain_analysis_owners --test domain_stability --test domain_guard_rebase --test execution_outcomes --test structural --test local_semantics --test execution_channels --test transfer_composition --test entry_value_witnesses --test base_execution --test entry_value_publication`, `/tmp/phase4-local-base-integrated-tests1.log` (28 model + 35 native + 7 actual PG controls). Canonical Local definitions bind finite scalar/nominal type rules and exact field-location evidence to code/limits; open class alternatives cannot establish complete negatives. BaseEvaluation reuses one charged syntax index, recomputes the complete Local/native expression and refusal inventory, preserves actual profile and exact parents, and rejects coupled inventory erasure. Both profiles publish through the generation store. Field locations do not establish heap-state stability; Completion/SourceCall/enrichment/B2/B3 remain open. This rerun also closes the pending A0 candidate-qualification/digest PG correction. Worker precursor receipts retain narrower scope and earlier repaired failures. |
| E0 mandatory rendering and exact consumed values / schema migration | passed, 2026-10-01: frozen `116e71b` integrated with current A0/C1/C2; `python3 scripts/build_environment.py -- cargo test --release --no-fail-fast -p lctx-model -p cpg-core --test domain_retrieval --test retrieval_consumption --test retrieval_preparation --test analysis_preparation` (12 pure + 2 real PostgreSQL controls), `/tmp/phase4-e0-integrated1.log`. All four mandatory families have native nonempty examples, canonical original anchors and immutable early rendering definitions. UTF-8 fragments deduplicate text while preserving occurrences, subjects and source context; lexical IDs do not depend on vector selection. Shared winning-byte replay retains signed zero, refuses spec/request/value corruption and preserves lexical data when vectors are unavailable or unrequested. The scoped helper performs actual source reads/build/publication and seals its scoped model; it is not the final E0 nominal producer. Final invocation/coverage, actual S0 brief units, vector effects and complete cross-use PG integration remain open. Worker receipts `/tmp/e0-pure5.log` and `/tmp/e0-pg3.log` retain the preceding scoped evidence. |
| A0 sealed structural conclusions / schema migration | composite passed, 2026-10-01: wrapped Cargo `test --release --no-fail-fast -p lctx-model -p cpg-core --test structural` (3 pure + 1 real PG control across both profiles), `/tmp/phase4-structural-conclusions-tests1.log`; final candidate-qualification and semantic-digest correction passed wrapped Cargo `test --release -p lctx-model --test structural --test domain_analysis_owners` (12), `/tmp/phase4-structural-conclusions-model2.log`. Exact source rows support static source-graph conclusions, with per-hop execution conditions retained separately. Forged finding kind and coupled source/conclusion removal refuse replay; public candidates stay Candidate/Over. Structural epoch owns its new qualification rows; S0 proof slot12 consumes the sealed conclusion through the shared emitter. No caller-execution or normal-completion promotion. Final PG after the qualification/digest correction passed in the B0/BaseEvaluation combined rerun above; Pass B/C conclusions remain open. |
| A0 structural publication / schema migration | composite passed, 2026-10-01: `python3 scripts/build_environment.py -- cargo test --release --no-fail-fast -p lctx-model -p cpg-core --test structural` (2 pure + 1 real PostgreSQL control across both profiles), `/tmp/phase4-structural-tests4.log`; wrapped Cargo `test --release --no-fail-fast -p lctx-model --test analysis_expected --test domain_analysis_owners` (12), `/tmp/phase4-structural-owners2.log`. Canonical settings-bound Delegation/DirectUsage invocations borrow prepared graphs; C0 owns public candidates, exact source/release membership owns subsystem scope, and replay recomputes complete inventories and exact Local/C0 parents. Missing usage stays absent, traversal limits and conditional/native step qualifications remain visible. Input-grain graph coverage was separated from artifact-grain callable/public evidence after a failing PG control. Local now consumes normalized dispatch directly; unused Dispatch analysis parent code4 stays reserved. Its former test now exercises a real BaseEvaluation/Local predecessor. A definition-only canonical S0 helper is a downstream prerequisite, not activation. Final seed selection belongs to S0; Pass B/C and qualified conclusions remain open. |
| A0 delegation and source-definition topology | composite passed, 2026-10-01: `python3 scripts/build_environment.py -- cargo check -p lctx-model -p cpg-core`; wrapped Cargo `test -p cpg-extract -p lctx-model --release --test normalized_projections --test domain_analysis_owners` (5 native + 9 model), final `/tmp/phase4-source-definition-tests2.log`. Borrowed canonical traversal preserves parallel call-site witnesses, shortest-path ties, open dispatch, explicit subsystem/dependency/source-role boundaries, unresolved sites, depth/vertex/arc limits and charged retained outputs. A failing nested-function control exposed missing source-definition edges: projection VERSION3 adds SourceDefinition from exact normalized declaration ownership, independent of native call phases; factory-to-nested is never Direct. Initial expected edge count and retained VERSION2 test expectation were corrected; earlier failures are retained in `/tmp/phase4-native-delegation-tests*.log` and `/tmp/phase4-source-definition-tests.log`. The model owns the pure transformation for later shared replay; analytics reexports it. Nominal publication, public seed selection, Pass B/C and A0 activation remain pending. |
| B1 expected lower contracts | passed, 2026-10-01: `python3 scripts/build_environment.py -- cargo test -p lctx-model --release --lib domain::analysis::expected::tests` (4), `/tmp/phase4-base-expected-tests.log`. Base Execution/Completion require Python Syntax, Signatures, Lexical and Flow plus normalized Symbols/Callables; catalog requests remain NotRequested. Missing native dependencies and foreign method/capability pairs refuse. No speculative FlowLinks/Calls/Bindings requirement or producer-selector activation. |
| B1 integrated base evaluation/completion records | passed, 2026-10-01: `python3 scripts/build_environment.py -- cargo test -p lctx-model -p cpg-extract --release --test execution_channels --test execution_outcomes --test domain_analysis_owners` (5 native + 3 outcome + 9 ownership controls), `/tmp/phase4-b1-integrated-tests.log`. Frozen `96eef881` integrated with current C1/E1/C2 contracts. Opaque exact expression and ordered completion proofs retain native/entry premises, pending return/raise/finalizer behavior and allocation ownership; replay refuses value/status/disposal/order and coupled membership forgeries. Configured parameter/semantic-version binding, source-call/default/body/frame-release evidence, actual producers and scheduled publication remain pending; this is no B1 exit. |
| Independent document/source target review | Accept scoped; review above records evidence, exclusions and required future controls |
| R0 model publication controls | passed: wrapped `cargo test -p lctx-model --release --test vocabulary_epochs --quiet` (4); profile-filtered writers, per-epoch uniqueness and inherited-prefix receipts |
| R0 PostgreSQL publication controls | passed: wrapped `cargo test -p lctx-postgres --features testing --release --test vocabulary_epochs --quiet` (9); immutable views/grants, mixed/future-prefix refusal, atomic rollback, cancellation, unconfirmed acknowledgement and frozen-content controls |
| R0 source-bound provider and normalized regression controls | composite passed: wrapped `cargo test -p cpg-core --release --test vocabulary_read --test normalized_generation --test stage_checkpoint --quiet` passed normalized (4), checkpoint (1) and old-prefix read; its new canonical-condition fixture initially expected zero native True nodes incorrectly. Corrected fixture and wrapped `cargo test -p cpg-core --release --test vocabulary_read --quiet` passed (2), including shared orphan-node refusal. |
| R0 independent implementation review and wrapper correction | Original three findings corrected and reinspected: inherited receipts, same-epoch writer uniqueness and inactive-profile filtering. Follow-up found an existing extraction test sink lacked new group forwarding; corrected and wrapped `cargo test -p cpg-extract --release --test typed_calls variants_keep_channels_phases_and_native_owners --quiet` passed (1). Foundation acceptance still awaits R1. |
| Initial R1 semantic slice | passed in isolated derived checkout at `7b04bcf`: wrapped Cargo with `--config 'build.build-dir="/home/paul/.cache/library-context-phase4-derived/build"' test -p lctx-model --release --test domain_analysis --test domain_assertions --test domain_derivation --test domain_transfer --test domain_composition` (8+3+3+7+11). Composite after fixture/condition/diagnostic corrections; producer ownership and bounded BDD allocation corrections remain pending. |
| R1 native inventory model controls | passed in isolated native-inventory checkout at `478ddf7`: wrapped Cargo with `--config 'build.build-dir="/home/paul/.cache/library-context-phase4-native-inventory/build"' test -p lctx-model --release --test native_inventory` (6), after a test-only ambiguous trait method was qualified. Exact native pairs, shared status/qualification projection, missing/extra/forged evidence, permutation stability and retained reservations; early stage wiring remains pending. |
| R1 native inventory store controls | passed at isolated `b3528fa`: same wrapped Cargo `test -p lctx-postgres --features testing --release --test native_inventory` (1 real PG18 test). Valid native fixture projection roundtrip; omitted pair/companion and forged status refuse through the generation store with original native support validators. This is not a native extraction or complete R1 qualification. |
| Finite metric model controls | passed: `python3 scripts/build_environment.py -- cargo test --release -p lctx-model --test finite_metrics` (2 tests); canonical signed zero, finite extremes, Arrow refusal and normalization |
| Finite metric store controls | passed: `python3 scripts/build_environment.py -- cargo test --release -p lctx-postgres --features testing --test finite_metrics` (1 real PG18 test); roundtrip and raw invalid-value refusal |
| Finite metric independent implementation review | no material findings against isolated commit `882f86a`, whose patch is integrated as `a44e3f8`; this is a bounded review, not R1 acceptance |
| Preparatory G0 graph controls | passed in the isolated graph checkout with `python3 scripts/build_environment.py -- cargo --config 'build.build-dir="/home/paul/.cache/library-context-phase4-graph/build"'`: `build --release -p lctx-model`; `test --release -p lctx-model --doc with_native_graph` (one positive and two compile-fail controls); `test --release -p lctx-model --lib native_borrow` (1); `test --release -p lctx-analytics --lib native_schedule` (3). Includes hydrated native topology, parallel arcs, isolates, canonical SCC ordering and reservation ownership. The public-entrypoint control initially lacked its test-owned scope, corrected before the passing rerun. An earlier shared-worktree build loaded divergent macro artifacts; isolated build directories resolved that collision without cleaning the shared cache. |
| Preparatory G0 independent implementation review | no material findings on the six-file adapter/kernel slice; its identified direct-entrypoint coverage gap is covered by the added public scheduler control. N1 dispatch-induced topology and generation preparation lifetime remain pending. |
| Preparatory N0 model controls | passed in the isolated authored-model checkout: `python3 scripts/build_environment.py -- cargo --config 'build.build-dir="/home/paul/.cache/library-context-phase4-models/build"' test --release -p lctx-model --lib domain::models` (12), and the same wrapped Cargo `test --release -p lctx-model --doc domain::models::Catalog` (one positive, two compile-fail). Retained parser/channel/phase/identity controls, complete typed path dependencies, immutable source/meaning binding, explicit pins and protocol/exception requirements, and shared stored-declaration refusal. This is not native model-context qualification or N0 completion. |
| Preparatory N0 independent review | N0-F01 found mutable parsed meaning could retain old source identity; private validated catalog/entry fields and borrowed getters correct it. N0-F02 found omitted global/raised-class path requirements; shared typed traversal and independently enumerated missing-package requirements correct it. Both reinspected closed at Interface-checked strength against `bee3e19`; native resolution and retained runtime parser accounting remain pending. |
| R0 named boundaries and ordinary-prefix integration | composite passed on main: `python3 scripts/build_environment.py -- cargo test --release -p lctx-model -p lctx-postgres --features lctx-postgres/testing --test vocabulary_epochs` (7 model, 12 real PG18); the same wrapper `test --release -p cpg-core --test vocabulary_read --test normalized_generation --test stage_checkpoint` (2+4+1). Includes physical order independent of named codes, transitive stored/handoff inheritance, explicit narrow input, no-input Facts bound, future-reference refusal without a consumer, rollback and zero invalid receipts. An initial overbroad non-vocabulary completion check failed two normalized controls; restricting this new check to vocabulary references restored all focused tests. |
| R1 immutable-owner model migration | passed in isolated derived checkout at `01a05e8`: wrapped Cargo with its stable private build directory, `test --release -p lctx-model --test domain_analysis_owners --test native_inventory` (7+6) and `--test domain_analysis --test domain_findings --test domain_assertions --test domain_derivation` (4+3+3+3). At `6cb94f4`, `--test domain_transfer --test domain_composition --test domain_stability` (7+11+3) passed. Concrete finite owner families and shared proof policy are implemented; acknowledged input receipts, expected-domain admission and cross-owner discharge remain in progress. |
| R1 behavior store migration | composite passed in isolated PG-derived checkout: `python3 scripts/build_environment.py -- cargo --config 'build.build-dir="/home/paul/.cache/library-context-phase4-pg-derived/build"' test --release -p lctx-postgres --features testing --test domain_transfer --test domain_composition --test domain_stability` (1+1+1 real PG18). Fixtures use actual native inventories and Local derivation/support families. Initial full-schema retirement exhausted default PostgreSQL lock capacity; disposable servers now start with `max_locks_per_transaction=256`, and all three reran successfully. The production cluster remains at 64; F0 must configure adequate capacity during quiesced maintenance before qualifying the full model. |
| N0 native context and independent review | passed in isolated native-context checkout at `7d82034`: wrapped Cargo with its stable private build directory, `check -p cpg-extract -p cpg-core --all-targets`, CLI check, `test --release -p cpg-extract --test native_model_context --test bundle --test producer_fingerprint` (6+6+1), core facts admission (2), extraction short-budget control (1) and retained typed-call control (1). Independent bounded review found no material issue; early catalog publication and retirement remain integration obligations. |
| N0 additional native controls | passed: isolated wrapped Cargo `test --release -p cpg-extract --test native_model_context` (8), adding wrong distribution ownership and inconsistent/cyclic MRO controls. An initial test assumed an Any base makes Pyrefly MRO incomplete; replaced with actual conflicting C3 and cycle fixtures before the passing rerun. |
| R1 acknowledged-source publication | passed, 2026-09-30: wrapped Cargo `test --release -p lctx-postgres --features testing --test analysis_publication` (one test, six real PG18 cases). Both profiles execute Local publication over actual completed sources. Missing source receipts and coupled removal of requirement/member/outcome rows refuse before acknowledgement or grants. Empty requested roots remain NoScope; catalog Flow remains NotRequested. |
| R1 source/discharge integration | passed, 2026-09-30: wrapped Cargo `test --release -p lctx-model --test analysis_discharge --test analysis_expected --test analysis_sources --test domain_analysis_owners` (4+3+3+9). Behavioral discharge refuses documented or heuristic support; catalog documentation retains its own channel. Dispatch closure excludes future normalization coverage and transfer ownership. |
| R1 scheduled versus unscheduled store boundary | composite passed, 2026-09-30: wrapped Cargo `test --release -p lctx-postgres --features testing --test analysis_publication --test domain_composition --test domain_stability --test domain_transfer --test publication_checks` (1+1+1+1+1; publication test contains sixteen ordinary/group/profile/forgery cases). Initial unscheduled behavior fixtures lacked acknowledged sources; the testing Harness now explicitly exercises ordinary invariants, while every scheduled completion retains source-sensitive callbacks. No unscheduled receipt qualifies actual publication. |
| N1 receiver/dispatch checkpoint | passed in isolated receiver checkout at `5b1b51e`, 2026-09-30: wrapped Cargo with its stable private build directory, `check -p lctx-model -p cpg-core -p cpg-extract --all-targets`; `test --release -p lctx-model --lib --test domain_calls --test domain_sites` (19+17+11); `test --release -p cpg-extract --test normalized_bindings --test normalized_events --test normalized_projections --test normalized_recovery` (9+2+4+3); `test --release -p cpg-core --test normalized_relations` (two real PG18 profiles); `test --release -p lctx-model --doc Applicable` (one positive, three compile-fail). The diamond SCC control uses a native serialized/hydrated canonical snapshot; it is not a PostgreSQL diamond-SCC receipt. Integrated as `00df7e7` with optional-attribution refusal in `b3d0ef5`; integrated rerun pending. |
| Early authored/native preparation | composite passed, 2026-10-01 at `f8266cc`: wrapped Cargo `test --release -p lctx-model -p cpg-core --test analysis_preparation` (two model controls and one core test across both profiles), with all early-boundary schema/invariant checks, native evidence parity, revoked INSERT grants and reservation release. Initial catalog scheduling required an inactive Flow writer; profile-specific input closure corrected it. A subsequent test incorrectly validated unproduced normalized owners in the full schema; the test now installs facts/configuration/native relations and still seals and validates every invariant in that boundary. The model fixture used unsupported format 1; corrected to format 7 before the passing combined rerun. This is preparation qualification, not F0 assembly. |
| R0/R1 independent foundation review | Accept scoped, 2026-10-01: review above at frozen `0a60954`, source/type inspection plus attributed focused receipts. No material in-scope finding. Downstream selectors/producers, complete P4 assembly and retirement remain open. |
| Integrated N1 and G0 preparation controls | composite passed, 2026-10-01: wrapped Cargo `test --release -p cpg-core --test normalized_relations` (two real PG18 profiles), `check -p cpg-core --all-targets`, and `test --release -p cpg-core --doc PreparedGraphs` (one compile-fail). Initial final validation incorrectly included unproduced native-analysis inventory; the control now uses the normalized manifest and retains all its boundary invariants. A new negative fixture needed an explicit Option type before the final passing rerun. Both profiles hydrate four stored projections, reuse the same graph objects and canonical SCC outputs across two declared consumers, refuse missing graph permits and foreign budgets, and release all retained reservations. The compile-fail control prevents borrowing beyond consumer access. F0 lifecycle wiring and downstream algorithm qualification remain open. |
| E1 shared embedding foundation | composite passed, 2026-10-01: wrapped Cargo `check -p cpg-core -p lctx-postgres -p lctx-embed -p lctx-semantics --all-targets`, followed by focused checks after effect changes; `test --release -p lctx-model --test embedding_contract` (2), `--lib domain::embedding` (1), `test --release -p cpg-core --lib embed` (6), `--test embedding_configuration` (1 real PG18), `test --release -p lctx-postgres --features testing --test services cache_roundtrip_and_spec_conflict` (1 real PG18), and `test --release -p lctx-embed --test client` (7 contract controls plus one environment-guarded live test). The live branch was not_run; fixture/stub results are not live-service qualification. Initial typed-batch fixture used an array instead of Vec; the new PG consumer probe first omitted a declared output and then its StageOutput declaration; corrected before passing reruns. Preserves immutable cache winners, spec/token/shape/digest checks, signed zero, budget release, endpoint/spec selection and pure-stage refusal. E1 analytic production and E0 consumption remain open. |
| R0 declared validation views | composite passed, 2026-10-01: wrapped Cargo `test -p lctx-model -p cpg-core --release --test validation_views` (one memory and two real PG18 controls), then `test -p lctx-model --release --test validation_views` (two, adding malformed-frame refusal). Earlier/current views of the same relation remain distinct through stage admission, publication and final replay; a Facts grant cannot widen to later vocabulary whether that later group is open or closed. Duplicate identical frames and non-vocabulary prefix bindings refuse model admission. Reservations return to zero. Initial model test incorrectly required Tokio; corrected to the existing ready-future test pattern without adding a runtime dependency. Logs: `/tmp/phase4-validation-views-tests3.log` and `/tmp/phase4-validation-views-metadata.log`. Actual B0 Summary publication remains pending its B3 capability contract. |
| R0 inactive nullable target control | composite passed, 2026-10-01: wrapped Cargo `test -p cpg-core --release --test validation_views` (3 real PG18 controls, `/tmp/phase4-validation-views-nullable2.log`). Group-close validation permits an empty nullable reference to an unavailable relation, but rejects every non-null reference even when a future ordinary target is physically present without its acknowledged receipt. Both native normalized profiles then passed full seal/model validation in `/tmp/phase4-analytic-text-pg5.log`. The negative fixture initially tried private GenerationId bytes; corrected to its public hexadecimal form. Diagnostic instrumentation was removed. |
| E1 analytic source text | composite passed, 2026-10-01: wrapped Cargo `test -p lctx-model --release --test analytic_text --test embedding_contract` (5+2, `/tmp/phase4-analytic-text-final-model2.log`); `test -p cpg-core --release --test normalized_relations -- --test-threads=1` (2 real PG18 profiles, `/tmp/phase4-analytic-text-pg5.log`). Native controls retain Unicode, original defaults, positional/keyword markers, nested declarations, exact docstring linkage and original document passages; complete generation validation and graph preparation remain enabled. Pure controls cover explicit unrequested selection, missing source/entity, charged UTF-8 windows and forged stored replay. Failed controls led to explicit Facts/Dispatch schedule boundaries, BYTEA text decoding, the exact StmtExpr-to-literal docstring link, preservation of captured star markers and the R0 nullable-reference correction above. Analytic vectors, their nominal consumption and live-service qualification remain unimplemented/not_run. |
| C0 catalog integration and schema migration | composite passed, 2026-10-01: worker checkpoint `8b07f13` integrated with normalized pipeline declaration/execution; wrapped Cargo `test -p lctx-model --release --test domain_catalog --test domain_callable_aspects --test analysis_expected` (11+6+3), and `test -p cpg-core --release --test catalog_core --test normalized_relations -- --test-threads=1` (1+2 real PG18, `/tmp/phase4-c0-integrated-pg2.log`). C0's fixture initially installed unproduced E1 text owners; it now installs the complete facts/normalized/early/Dispatch/CatalogCore/catalog boundary and still seals and validates every installed invariant. Native C0 retains source aliases, exact source-parameter bridges, original None/default/factory uncertainty, overloads, constructors, accessor/context-manager/registration metadata, exact source receipts and unrequested Flow; reservations return to zero. C1 and broader assembly remain open. |
| Per-grain downstream coverage foundation | composite passed, 2026-10-01: wrapped Cargo `test -p lctx-model --release --lib domain::analysis::expected::tests` (2) and the C0/expected model suite above (20; `/tmp/phase4-c0-grains-model.log`). Python, document and input scopes retain their own exact lower contracts; Deployment stays input-grained. E1 request selection reads the single immutable text definition. CatalogEvidence appends method/capability code20. C1/E1 publication selectors remain unbound pending their actual producers; these controls do not qualify execution. An initial test used nonexistent TextDefinition::default; corrected to builtin before passing `/tmp/phase4-expected-grains2.log`. |
| B0 entry and transfer prerequisite integration / schema migration | composite passed, 2026-10-01: integrated worker checkpoint `27da5f2`; wrapped Cargo `test -p lctx-model --release --test domain_transfer --test domain_stability --test domain_guard_rebase --test analysis_discharge --test domain_analysis_owners` (34, `/tmp/phase4-b0-integrated-model2.log`). Wrapped Cargo `test -p lctx-model -p cpg-core -p cpg-extract -p lctx-postgres --features lctx-postgres/testing --release --test analytic_consumption --test entry_value_witnesses --test transfer_composition --test entry_value_publication --test domain_transfer` passed B0 native controls (4+12), real PG entry/stability controls (2), model transfers (10) and PG transfers (1), `/tmp/phase4-b0-e1-integrated-tests.log`. Entry PG now installs and fully validates normalized/native/Entry/Stability owners, including callable metadata. Initial model compile exposed the new ClassOf match; normalization now explicitly preserves its unresolved runtime class identity. Private composition, retained charges, native defaults/receivers/guards, captured-state refusal and forged witness replay are covered. Retired raw composition fixtures were replaced by native controls. Actual Local production and scheduled Summary publication remain pending their downstream contracts. |
| E1 analytic consumption and shared winning-value replay / schema migration | composite passed, 2026-10-01: wrapped Cargo `check -p cpg-core --lib`, then `test -p lctx-model -p cpg-core --release --test analytic_consumption --test analytic_embedding -- --test-threads=1` (5 pure controls; 3 actual PG18 tests across available, unrequested, service-unavailable and token-limit cases), `/tmp/phase4-analytic-consumption-final1.log`. The external effect uses a deterministic contract fixture, not a live provider; live-service behavior/quality is not_run. Original text and normalized producers are real. Every expected source frame/window retains its own nominal use, exact spec/request/codec/digest/bytes, and availability. Cold replay rejects changed requests, signed-zero conflicts, missing uses and coupled frame removal; transient/retained charges return to zero. Initial PG1/diagnostic runs failed required input admission; declarations now include complete text and configuration prerequisite references, without changing store validation. Temporary backend diagnostics were removed. E1 coverage selector is bound; E0 cross-consumer integration, optional analytics and F0 assembly remain open. |
| A1 bounded nearest-neighbour kernel prerequisite | passed, 2026-10-01: wrapped Cargo `check -p lctx-analytics --lib` and `test -p lctx-analytics --release --test native_neighbours` (3 pure controls), `/tmp/phase4-native-neighbours-tests.log`. The new nominal kernel replays admitted E1 consumption before decoding; equal dimensions, finite values, exact selected universes/context, top-k/threshold/self selection and canonical item/window ties are enforced. It preserves missing-window availability, preflights the explicit window-pair work bound and retains input/output reservations. Analytically known vectors cover maximum window similarity, permutation, corrupt shape/NaN/digest rejection and budget release. This is a kernel prerequisite; A1 nominal publication, community layers and F0 activation remain unimplemented. |
| C1 original contextual evidence / schema migration | passed, 2026-10-01: integrated frozen worker `0db6026`; wrapped Cargo `test -p lctx-model -p cpg-core -p cpg-extract --release --test domain_catalog_evidence --test catalog_core --test catalog_evidence --test typed_comprehension_routes -- --test-threads=1` (10 pure, 2 actual PG18 and 1 native extractor control), `/tmp/phase4-c1-integrated-tests.log`. Both catalog generations seal and validate their full installed boundary. C1 retains original MDX and separate extracted-code coordinates, explicit setup dependencies, declaration/context-scoped scenarios, exact intent identity, release-owned deployment and distinct reported/acquired environments. Actual native frames and exact C0 parents prevent coupled removal of empty evidence frames. Source field candidates stay Unknown until Local/B1 proves exact receiver/init/read semantics. Worker failed receipts (pre-capture derivation, BYTEA test decoding, omitted fixture member) remain preserved; final validation was not weakened. Runtime field integration, C2 and F0 remain open. |
| C2 prerequisite contracts | passed, 2026-10-01: wrapped Cargo `test -p lctx-model --release --lib domain::analysis::expected::tests` (3), `/tmp/phase4-c2-contract-tests.log`. CatalogSelection appends method/capability code21 and requires the union of C0/C1 lower contracts at Python/document/input grains. C1-only lower evidence cannot close missing public exposure/ancestry/type coverage. The unactivated selection family now has only its actual CatalogEvidence predecessor; unused structural/analytic predecessor codes5/6 are reserved and not reused. This corrects R1 wiring to §10.3 without weakening shared validation or requiring optional analytics. The producer selector stays unbound until C2 implementation and exact frame/domain closure are ready. |
| A0 direct-usage aggregation prerequisite | composite passed, 2026-10-01: wrapped Cargo `test -p lctx-model -p cpg-extract --release --test analysis_usage --test direct_usage` (3 pure and 1 actual native catalog-profile control), `/tmp/phase4-usage-native-tests2.log`. Each official Example/Test/DocBlock call site counts once across duplicate target evidence, phases and native origins; its distinct admitted Usage-policy targets determine shares before callable output filtering. Definition-only references do not contribute calls. Exact policy/admission/alternative receipts, open target sets and uncertainty remain visible; missing targets do not become zero scores. The actual captured Markdown fence retains aliases and four source call sites without requesting Flow. Short-budget, foreign/duplicate selector, incomplete membership and retained reservation controls pass. The initial native test compile had an ambiguous Record/Key encode call; corrected before the passing rerun. Structural producer publication and full Pass A–C remain pending A0/B0 integration. |
| A1 weighted-ranking kernel prerequisite | passed, 2026-10-01: wrapped Cargo `test -p lctx-analytics --release --test native_ranking` (3), `/tmp/phase4-native-ranking-tests.log`. Nominal selected entities and directed positive integer weight pairs feed the retained uniform teleport/dangling power iteration; duplicate pairs aggregate with checked arithmetic before canonical accumulation. Independent Leiden compute_flow agrees on asymmetric weighted/parallel/dangling/isolate controls. Permutation, unknown endpoints, duplicate vertices, zero/overflow weights, finite parameter admission, empty universe, explicit iteration/work stop and reservation release are covered. Unperformed iterations have unavailable residuals; no fabricated convergence. The model-owned graph/layer conversion, nominal publication and optional activation remain A1/F0 obligations. |
| A1 finite-concept kernel prerequisite | passed, 2026-10-01: wrapped Cargo `test -p lctx-analytics --release --test native_concepts` (3), final `/tmp/phase4-native-concepts-tests2.log`. Retained frequent NextClosure and Duquenne–Guigues enumeration now use nominal generic object/attribute identities with private equal-length FixedBitSet 0.5.7 state. Context, scratch, basis and returned results retain attempt reservations, including geometric vector capacity. Independent fcars concept sets agree for five generated contexts at three support thresholds; exhaustive attribute subsets check implication-basis closure. Permutation, empty contexts, enumeration cap, foreign/duplicate incidence domains and short budgets are covered. Model-owned attribute/incidence construction, one-step RCA, nominal publication and optional activation remain open. |
| B0 actual Local producer and execution-domain entry integration / schema migration | composite passed, 2026-10-01: worker checkpoint `00603df` integrated; wrapped Cargo `test --release --no-fail-fast -p lctx-model -p cpg-core -p cpg-extract --test domain_stability --test domain_guard_rebase --test local_semantics --test entry_value_witnesses --test entry_value_publication --test execution_channels --test domain_analysis_owners` (4 real PG18, 13 native, 22 model controls), `/tmp/phase4-local-integrated-tests3.log`. Use/Value/Guard sources retain exact native and statement-region domains; Guard identity holds before both arms, and bare-name evaluation refuses an otherwise valid Value proof. Actual Local publication writes source receipts, transfers, guard influences/selections and explicit boundaries; catalog is NotRequested without Flow, behavioral remains Partial/IncompleteDomain. Initial integrated compile restored imports removed from tests before their new consumers existed; initial PG seal installed an unscheduled E1 owner, so the fixture now installs the complete Local envelope and runs callable metadata. A fixture call used the wrong module before the final passing rerun. Production validation was not weakened. The removed old PG stability test's scheduled Summary substitution and forgery obligations remain B3 work, as stated above. |
| B1 canonical rule binding and source-body prerequisite | composite passed, 2026-10-01: worker `84eb034` integrated; wrapped Cargo `test --release --no-fail-fast -p lctx-model -p cpg-extract -p cpg-core --test analytics_settings --test execution_channels --test execution_outcomes --test domain_analysis_owners --test analysis_preparation`, `/tmp/phase4-b1-settings-integrated2.log`. B1 scope passed 6 native + 12 model controls. Exact Use-source replay now retains the native evidence floor; changed semantic versions and limits refuse. Source-body completion consumes independently entered, ordered statement proofs, stops after abrupt completion and retains mandatory frame cleanup. It proves behavior under entry, not caller continuation. Initial integration moved a module doc comment with its import; restored before the passing rerun. Actual Base/SourceCall/Enriched producers and B2/B3 remain open. |
| C1 public original sources and C2 declaration selection / schema migration | passed, 2026-10-01: frozen `739e41b`/`c56b994` integrated; `python3 scripts/build_environment.py -- cargo test --release --no-fail-fast -p lctx-model -p cpg-core --test analytics_settings --test analysis_preparation --test domain_catalog_evidence --test domain_selection --test domain_selection_catalog --test catalog_selection --test domain_analysis_owners`, `/tmp/phase4-c2-settings-integrated.log` (44 total; C1/C2 scope 29 pure + 1 actual native/PG18). C1 preserves original artifacts for admitted public members outside scenario roots. C2 publishes exact C0/C1-owned domains, typed defaults and signatures, predicate-specific closure and Supported/Contradicted/Unresolved/Conflicting groups; discovery preserves uncertainty and strict selection admits only support. Shared replay refuses missing/forged/coupled domain and invocation erasure. Reserved unused transfer code2 is removed from the unactivated family closure; no fabricated transfer prerequisite. Native conditional types and foreign contexts cannot establish unconditional claims. Runtime field/state facets remain pending actual Local/B1 evidence; no full C2 or Phase 4 exit is claimed. |
| A0 resolved settings and early publication / schema migration | passed, 2026-10-01: the preceding integrated wrapped Cargo command passed 2 settings, 2 configuration-model and 1 actual PostgreSQL control across both profiles. The pure version-1 parser preserves authored roots, ordered primary/distractor seeds, bounds, budget and the entire resolved optional technique set; it rejects contradictory flags, invalid bounds and missing dependencies before effects. Selected settings are charged, immutable, included in the configuration digest and read back from PostgreSQL. Structural/analytic activation and final S0 seed selection remain open. |
| Integrated functional gate, actual-library pilots and performance experiments | not_run; full functional scope and X0 retirement are not complete; previously excluded measurements remain outside scope |
| ADR index/lint, docs publication and other automatic hygiene | not_run by this agent; current repository instructions assign them to the end-of-turn hook; no future result is presumed |

The next implementation boundary is actual B0/B1 production and B2/B3 composition, with
C2 and A0/A1 progressing independently over their completed lower owners. Acceptance of the design is distinct from
implementation, semantic qualification, pilot measurements and serving/product acceptance.
