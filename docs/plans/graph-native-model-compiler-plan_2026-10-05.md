# Graph-native model and compiler

**Persisted follow-up implementation paused, 2026-10-07:** the [persisted graph execution plan](persisted-graph-execution-plan_2026-10-07.md) owns the current combined execution, catalog-speed F01–F05 and additional same-pattern corrections. PG1–PG4 replace the store-free workspace with exact native completed views, shared access and demand-driven preparation. Model meanings and pure kernels remain owned here; the prior IPC/export receipts below describe their original implementation boundary; the current compiler uses native completed views. Original dated receipts and prior audit findings retain their scope; only that new plan owns these follow-up findings.

The [supporting correction plan](persisted-execution-corrections-plan_2026-10-07.md) supplies the Proposed continuation: PC1/PC2 complete operation outcomes and captured-provider admission; PC3/PC4 complete shared lowering and candidate execution. [Persisted coordinator §8](persisted-graph-execution-plan_2026-10-07.md#8-sole-finding-disposition) owns the correction-causes findings; this document adds no second disposition. Plan authoring does not resume production.

**ER production scopes Implemented / focused Tested, 2026-10-07:** this plan owns the model/compiler contracts below. [Graph coordinator §7](graph-native-pivot-plan_2026-10-05.md#7-sole-finding-disposition-and-optional-capabilities) remains the sole graph-audit disposition owner; its [§8.1 checkpoint](graph-native-pivot-plan_2026-10-05.md#81-current-remediation-acceptance--2026-10-07) and [combined coordinator §6](evidence-retrieval-and-evaluation-plan_2026-10-06.md#6-finding-disposition-investigation-outcomes-and-completion) own current receipts and completion. Earlier dated receipts retain their original source and scope; Q1 remains pending.

**Complete — implemented and user-accepted, 2026-10-05 (ADR-0128).** Supporting plan for the [replacement coordinator](graph-native-pivot-plan_2026-10-05.md).
It develops M1, C1, C2-N and C2-U. The coordinator owns package state, cross-plan dependencies
and finding disposition. The design basis is its two named reviews, not another model review.

**Current continuation Implemented / focused Tested, 2026-10-07:** §5 develops semantic admission and bounded preparation.
Prior implementation/acceptance labels below describe the initial pivot, not closure of the audit.

The [2026-10-06 implementation audit](../design_review/reviews/design_review_graph-native-pivot-implementation-audit_2026-10-06.md) identifies remaining coordinated-plan gaps;
[coordinator §7](graph-native-pivot-plan_2026-10-05.md#7-sole-finding-disposition-and-optional-capabilities)
owns their current disposition. Earlier stage acceptance and dated receipts below are preserved;
this document's implementation label does not establish closure of the audit findings.

## 1. Foundation assessment and resulting boundary

`lctx-model::domain` already owns attributed observations, nominal identities, source/context
correspondence, five-way verdicts, conditions, typed catalog/selection, projection semantics and
wire contracts. Keep these meanings and useful pure kernels. They are foundations to reshape,
not a requirement to preserve every declaration, relation, stage, epoch or validator attachment.

**Implemented; compiler-stage completion accepted by the user, 2026-10-05:**
`cpg-core::compilation::compile` builds immutable completed Arrow IPC streams in a private,
spillable workspace. Explicit semantic predecessor selectors keep native Facts separate from
Local, Model and Analytic vocabulary. Typed graph lowering and artifact admission/export are
separate from publication. The PostgreSQL compiler, runtime grants and `MemoryGeneration` are
retired; canonical declarations and semantic policy revisions own model compatibility.
The [coordinator's current checkpoint](graph-native-pivot-plan_2026-10-05.md#8-current-checkpoint)
owns actual verification outcomes. The user explicitly stopped further tests and accepted this
plan scope as complete. Model and analytics controls passed; compiler verification was interrupted
with partial passing evidence and a repaired obsolete test not rerun. This is an Implemented
completion boundary with scoped Tested evidence, not an all-checks-passed or Measured claim.

The current compiler receives explicit captured inputs, profile/method settings, semantic
definitions, resource/runtime services and optional embedder/cache effects. It owns a private
STRICT native database and returns an admitted graph backed by that exact completed state.
Immutable contribution views and frozen semantic-boundary bindings select predecessor inputs;
model-owned pure kernels remain independent of the store. The publisher seals that same database
through its [realization contract](graph-native-surrealdb-realization-plan_2026-10-05.md).
Selecting a snapshot remains outside both operations. The persisted plan owns current acceptance.

## 2. Graph semantics and identity — M1

### 2.1 Author the smallest sufficient graph

The Rust model declares typed kinds/properties, identity recipes, endpoint/participant roles,
provenance, qualifications, coverage, domain operations and admission rules. An attributed
assertion is distinct from its subject entity. Provider disagreement adds separately supported
assertions; it does not overwrite an entity property. A public exposure is distinct from its
declaration, invocation variant and contextual occurrence.

| Graph meaning | Required representation |
|---|---|
| Entity | Independent identity and typed intrinsic data; exists without incident edges |
| Binary assertion | Its own ID, kind, ordered endpoints/roles, scope, qualification and evidence; parallel assertions survive |
| N-ary assertion / derivation | Addressable assertion with role-labelled participants; derivation additionally names rule/revision, ordered premises, assumptions and outcome |
| Source / evidence / run | Shared immutable context, exact original bytes/spans and provider/environment identity; compact links from assertions |
| Coverage / outcome | Expected domain and observed result with complete-empty, partial, unavailable, failed, refused and NotRequested distinctions |
| Retrieval / vector use | Text/unit identity, contextual occurrences, family/member and exact embedding consumption; heuristic witness separate from requirement evidence |

Small exclusively owned values can remain nested typed payloads. Independently cited or reused
values remain addressable. Give n-ary structures real roles instead of flattening them to binary
edges. Classic relation assertions can be referenced by other assertions where the adapter's
encoding supports it; reification is also available through the same semantic mapping.

Retire relation-registry-as-published-schema and operational vocabulary history. Typed record
declarations and Arrow batches may remain useful internal computation views; their number does
not dictate physical families or execution stages. Do not automatically wrap all old tables in
generic payload nodes. For each current producer, map its required semantics to these graph forms,
merge incidental bookkeeping, and remove data without a current semantic/serving consumer.

Conditions retain canonical evaluation-atom/BDD meaning, not library-local indices. A private
attempt may assemble a deterministic vocabulary with dependency-local completed views. Final
admission checks the complete referenced vocabulary. No persisted epoch/grant/receipt choreography
is needed to enforce a compiler dependency. Codebook allocations remain append-only.

### 2.2 Separate identity scopes

| Identity | Owned inputs and invalidation |
|---|---|
| Logical entity/assertion | Declared semantic key, contextual/source identity where meaningful; physical database names never enter it |
| Captured source | Exact bytes, release/environment/acquisition identity and source coordinates |
| Semantic contract | Canonical declaration structure, explicit semantic policy/invariant revisions, codebooks and operation meanings |
| Producer implementation | Provider/compiler implementation and relevant dependency/configuration identities; provenance and safe reuse dependency |
| Graph content | Canonical complete payloads and links, outcomes/coverage, sources and consumed embedding values/spec |
| Physical realization | Codec/layout, functions, indexes/analyzers, engine and optional module artifacts; owned by publisher |

Current `Id<T>` is structurally keyed, not directly a source-text hash. Preserve the nominal/key
principle while deliberately revising recipes where the new meaning needs it. Remove broad
source/manifests/lock hashing from **semantic contract compatibility**. Keep implementation
fingerprints where a producer/reuse result actually depends on them. An unrelated comment or
storage edit must not rename unrelated entities or invalidate semantic installation; a changed
semantic rule must invalidate its consumers. No old-ID reader or translation table is added.

Use explicit canonical encodings with stable ordering and discriminants. Compare complete
payloads when keys collide: identical observations deduplicate, conflicting same-key payloads
refuse unless a named model merge owns that case. Human formatting/Debug/ordinary JSON and
engine numeric coercions do not define identity. The physical adapter maps IDs mechanically to
typed canonical-string record keys.

### 2.3 Admission product

The manifest identifies input captures, contract/implementation/settings, requested frontier and
profile, graph-family content/counts, coverage/outcomes, originals, selected projection/analysis
definitions and exact consumed embedding spec/values. The admission result binds that immutable
content and the checks completed by their semantic owners. It is not a certificate of unrestricted
Python truth or a performance proof.

Construction enforces local shape/ranges, nominal roles and key consistency. Completion checks
reference closure, required domain outcomes, coverage and cross-element invariants once over
completed inputs. External unresolved Python targets are typed uncertainty; absent internal
endpoints are invalid. Forward/cyclic references stay private until closure succeeds. Decodeable
partial producer output is not admitted merely because its records are well formed.

Retain a cheap algorithm-specific certificate/check only when it establishes a property that
publication genuinely needs. For SCCs, component order alone does not establish strong
connectivity; for total outputs, compare actual keys with the independently defined obligation
universe, not just counts. A post-fixpoint check does not establish a least fixpoint. Witnesses
establish their declared witness claim, not path absence. Preserve exactness where required;
keep a narrowly necessary validation when no adequate cheaper replacement exists. Do not invent
a generic proof system or replay every producer on every compile. Independent small controls
primarily establish algorithm correctness.

## 3. Bounded workspace and producer migration — C1/C2

The initial IPC workspace mechanics below are displaced by [persisted execution PG1–PG4](persisted-graph-execution-plan_2026-10-07.md#7-execution-packages-and-dependencies). Preserve exact typed inputs, outcomes, semantic validation and pure operation boundaries; do not reimplement the old physical recipe.

### 3.1 Physical compilation route

Use model-owned typed contributions and exact completed native views for authoritative
compiler products. Native keyed/reference selection reaches scoped rows before hydration;
projected Arrow/DataFusion batches serve bulk joins, grouping, sorting and semantic kernels.
One attempt owns budgets, streams, writer admission and cancellation/drain. Necessary spill
runs are derived scratch, not a second completed IPC relation registry.

A producer declares all outputs, including complete-empty products, and binds its exact
predecessor views before execution. Full-payload equality governs duplicate nominal keys;
completed memberships freeze once. Current views can advance without rewriting preceding
contributions, while frozen lower boundaries remain unchanged. Admission retains exact
unchanged validity premises and checks altered ones. Explicit export includes the native
completed-state family; ordinary compilation has no portable self-roundtrip.

Whole-topology algorithms necessarily materialize their named topology; that allocation is not
made scalable merely by refusing it. Reuse one compact prepared projection across compatible
methods, and keep unrelated evidence/payloads out of it. The [projection plan](graph-native-projections-plan_2026-10-05.md)
owns that representation and its analytical lifetimes. Discard temporary state once no consumer
needs it. Restart a failed disposable compile; persistent arbitrary-stage resumability is not
required.

### 3.2 Producer closure

| Slice | Result and migrated consumers |
|---|---|
| C1 facts | Acquisition and independent Ruff/Pyrefly/ty capture emit attributed typed graph records and original chunks; profile outcomes remain explicit. Facts admission uses exact completed native views; independent model validation remains pure. |
| C2-N normalized | Occurrences/entities/places, exposure/declaration correspondence, signatures, call targets/bindings, conditions and projection bases consume completed facts/workspace views. Preserve all alternatives, roles and unresolved inventory. |
| C2-U semantic upper | Local, execution/transfer, finite models/Summary, Structural and their selected definitions/outcomes/premises consume explicit predecessor views. Remove PG checkpoint adapters and blanket stored replay. |
| C2-U product | C0 public universe/options/contracts, C1 source/scenario/deployment associations, C2 predicate-domain selection basis, S0 assertions/briefs, contextual retrieval and embedding uses construct the graph. Catalog does not depend on optional analytics or brief availability. |
| C2-U selected analytics | A1/A2 supply projection inputs/native kernels. Disabled methods emit NotRequested; actual selected failures/partial outcomes retain their meaning. |

These names identify existing semantic responsibilities, not mandatory new crates/stages.
Migrate producers **and all their consumers** with each boundary. `CompletedStore`, `StageAccess`,
store-bound DataFusion registrations, persisted stage grants, vocabulary delta tables and callback
replay factories disappear. Ordinary explicit completed-input views replace their dependency
meaning. No adapter emulates an old database grant over an in-memory graph.

For corpus evidence, preserve original enclosing context and declared corpus/library links;
same-input or same-database presence does not establish association. Retain native Bound/unattached
formulas without manufacturing Entry proof. Source field location, Terminal/raised typing,
navigation and source similarity remain their stated evidence, not runtime guarantees. Negative
claims retain complete scoped premises. This is domain fidelity, not preservation of old storage.

Embedding is an explicit effect after admitted token/text preparation and outside transactions.
The cache may be available, but facts/normalized and embedding-unrequested catalog compiles do not
open it. Exact consumed cache/service winners enter the artifact; later reconstruction uses those
bytes, never a live service lookup. Batch duplicate texts by complete spec/text identity and share
winning values across analytics/retrieval consumers without equating their semantic roles.

## 4. Packages, controls and completion

| Package | Implementation boundary and focused control |
|---|---|
| M1 | Revise declarations/identity/admission and generated wire/codec consumers together. Hand-authored graphs challenge conflicting supports, parallel assertions, isolates, n-ary roles, ordered premises and internal/external missing targets. |
| C1 | Workspace streaming/spill, captured originals and facts producers. Compile a fixture with no DB configuration; forced-small workspace gives the same canonical artifact as an ordinary workspace, and cancellation exposes no completed artifact. |
| C2-N | All normalization adapters use workspace inputs. Independent signature/binding/unresolved/source-context cases preserve known answers and unknowns; insertion/batch order does not change canonical meaning. |
| C2-U | All selected semantic/product consumers and A1/A2 integration. Missing obligation keys, partial coverage, damaged provenance and unavailable optional analyses stay distinct; catalog works independently of briefs. |

Identity controls distinguish comment/storage edits from semantic rule changes, and complete
payload changes from unchanged keys. Artifact controls include missing/reordered/tampered chunks,
non-UTF8 original bytes and exact source-span expansion. Certificate controls challenge their
actual claim: incorrectly merged SCC, valid post-fixpoint labelled least, missing required outcome,
and path witness promoted to absence are revealing small cases.

During slices run touched-crate compile checks and selected `verify-model`, `verify-providers`,
`verify-analytics` or `verify-oracles` controls. New pure-domain tests prepare no database/Python
adapters unless they actually exercise one. Reuse existing independent fixture expectations;
do not copy old stage/receipt tests mechanically. Review changed `.snap.new` before acceptance;
graph/identity/wire changes are explicit contract migrations.

Initial-pivot completion supplied store-free admitted frontiers for both profiles, upper catalog content and
selected analytical outcomes, not only a tiny facts demonstration. The publisher's full-content
roundtrip and actual serving are additional integration obligations at Q0. Real-library scale,
live vectors and activation remain the coordinator's explicitly authorized Q1 work.

Package state, actual current-tree verification and finding disposition remain at the coordinator.
The persisted plan now owns combined compiler, native publication and shared-serving execution.

## 5. Audit remediation: semantic admission and bounded shared preparation

**Implemented / focused Tested, 2026-10-07.** F01/F02/F03 are a coupled correction, not permission to restart the
stopped compiler suite. [Coordinator §9](graph-native-pivot-plan_2026-10-05.md#9-remediation-and-improvement-execution)
owns execution and §7 owns disposition. The [foundation assessment](../design_review/evidence/2026-10-06_graph-native-remediation-capabilities/compiler.md)
grounds this design in current owners and the matched DataFusion skill. The current workspace
now owns exact native completed views and spillable compute. The persisted plan replaces the
prior IPC authority and repeated rich reconstruction while preserving these admission obligations.

**Acceptance checkpoint:** Necessary admission/input transport and the scoped compiler repairs are integrated. The earlier 2026-10-06 SourceCalls/Enriched, paired normalization, exact upper Place endpoint and Facts-only import receipts retain their original source boundaries. Fresh graph lowering adds exact Place aliases only at Normalized/upper frontiers; detached import remains strict. Current coordinated native acceptance is in progress; the checkpoint owners linked above record actual outcomes without closing findings or claiming whole-suite acceptance.

### 5.1 R-C0 — Necessary semantic admission, distinct from producer replay

Production admission checks compilation completion, coverage, graph shapes/references and
acyclic derivation, but it does not invoke assertion support/qualification/ownership checks.
Calling `Workspace::validate()` wholesale would also invoke broad producer replay. Split these
responsibilities at the existing model validation owners: required semantic admission operations
and explicit diagnostic/differential replay. Use ordinary typed Rust functions/declarations;
`lctx-model` remains pure and does not gain DataFusion or a validation DSL.

Required checks include assertion/support total membership; context/provider/run/family/fidelity
agreement; acquired-input and source/scope ownership of subjects, conditions and evidence; source
bounds; canonical condition/assumption basis; nominal roles/subtypes and reference closure;
coverage and required invocation/outcome key domains; exact vector/spec/text consumption; and
applicable binding/receiver/event composition premises. Derive these obligations from existing
owner declarations and input captures/profile/methods, not an orchestration-maintained duplicate
checker list. Preserve the existing independent final-frontier obligation logic. Digest equality,
matching counts or a successful producer return cannot replace these semantic properties.

Execute relational checks using joined/ordered completed inputs and source-owned typed predicates.
Local construction checks run when rows enter; predecessor checks finish before consumers rely
on them; forward references and truly global closure finish when their declared domain closes.
Avoid a new whole-resident support map per assertion family. Bulk semijoins, sorted reducers and
attempt-private ordered membership streams carry the same owner semantics. Failure/cancellation
leaves no completed checked view or admitted artifact.

Carry established validity with a privately constructed checked completed-input view bound to
its immutable descriptors, capture/profile/settings, applicable semantic policy and selected
vocabulary boundary. Reuse existing content descriptors: do not rehash all unchanged input to
authorize every consumer, mint validity from empty descriptors, persist receipts or recreate
store grants. Mutable/pending output cannot construct it. Each consumer receives the properties
it needs; final admission combines applicable checked views and remaining global closure.
For a replay removed from production, preserve its genuinely necessary consumer premise through
this owner operation and retain an independent small test of the producer. Do not remove an
assurance merely because it currently shares a function with replay.

All four frontiers and both profiles use this path. Unrequested optional operations have explicit
NotRequested outcomes without preparing their rich inputs. Valid unresolved/conflicting provider
observations remain admitted as their declared uncertainty; wrong support attribution refuses.

### 5.2 Detached transport and restore do not inherit live checked state

Distinguish live `AdmittedArtifact` export checked against its actual immutable manifest from
`publish-artifact`/portable restore of detached bytes. The live path preserves established
properties under exact transport equality. A detached file's self-authored manifest establishes
neither support validity nor completeness: perform the necessary semantic re-admission before
sealing or publishing it, without acquisition, providers, normalization replay or embedding.

The canonical graph retains observations/supports, ProviderRun/RunFamily, qualification,
conditions/assumptions, capture/source/evidence ownership and corpus associations. Share neutral
mechanical graph-to-typed-record dispatch at the model/codec owner; existing native codec/batch
conversion shares the model-owned mappings through the current native compiler dependency.
Decode bounded batches into attempt-private typed streams and execute the same admission owners.
Expected coverage/outcome keys come from retained input/profile/method declarations, not observed
output rows. Missing applicable inputs refuse; an omitted ephemeral workspace projection format
is not an excuse to skip a semantic obligation. Reconstruct required projection/lineage checks
from canonical roles, vertices and gaps, or add a specifically necessary declared semantic input
before claiming portability of that frontier. No retired topology serialization or grant is imported.

Affected consumers: artifact construction/export verification, detached CLI publication, publisher
input types, portable backup restore and graph/projection dispatch. These share one semantic
admission implementation with two validity lifetimes. Arbitrary partial diagnostic/projection
exports are not complete artifact inputs. The supported boundary remains controlled compiler
output plus semantic/transport checks; it does not authenticate dishonest producers or prove
unrestricted Python truth. No signing/certificate service is introduced.

### 5.3 R-C1 — Partitioned normalization that retains global dependencies

Select DataFusion bulk projection, semijoin/join and external ORDER BY followed by model-owned
ordered reductions and dependency-closed local kernels. Stream outputs directly into the current
canonical stage writer. Do not collect `SELECT *` into every `Rows` field or collect all outputs
before writing them. The lock matches DataFusion 55.1.0 / Arrow 59.3.0 / object_store 0.13.2.
`execute_stream` bounds neither operators nor consumer collections; spill is operator-specific.
Use actual spillable sorts/appropriate merge joins and the current pool/disk manager. Do not use
`DataFrame::cache()` as a blanket replacement: it materializes memory. Partition streams do not
promise global order, and IPC scans do not automatically inherit Parquet repartitioning behavior.

The physical partition grain follows the semantic dependency closure, not file size alone:

| Binding | Selected processing grain and required closure |
|---|---|
| Entities | Ordered source/span ownership sweep plus joined module/provider/context; retain complete external-symbol correspondence and entity universe |
| Relations | Separate source-reference/import/type/mention joins; obtain cross-module target candidates and corpus-library association before typed reduction |
| Callables | Complete callable/declaration/signature family, including all overload/provider candidates and ordered parameter slots; imported targets remain joined |
| Receivers | Call target/evaluation occurrence plus applicable complete callable inputs; preserve expression-relative ClassOf and source/context admission |
| CallableAspects | Assessment/declaration/class-field group with decorator/default/trait and ancestry/accessor links; metadata is not body/signature proof |
| Events | Qualified call/evaluation occurrence and complete alternatives/phase/dispatch; retain native correspondence, conditions and unfollowed boundaries |
| Bindings | Event/invocation alternative with complete signature/receiver candidate closure; emit bound/shape/effective/composition products once |
| Projections | Independent vertices, ordered typed arcs/gaps and lineage per named context/universe; global methods retain their actual full universe |
| Coverage | Ordered join of independently derived capture/profile obligations, native evidence and completed outputs; exact key domains and all declared outcome states |

Cross-module/candidate dictionaries remain compact or file-backed, not copied into every partition.
Use a bulk semijoin or indexed range fetch for each required dependency closure. Absence claims
require the closed universe of alternatives, not the rows present in the current chunk.
One source, signature family or highly connected target can be skewed: use incremental ordered
membership/enumeration, tiled joins for genuine many-to-many output and finer independent semantic
groups. Keep only irreducible compact state for a truly whole-group kernel and stream surrounding
rich evidence. A page count or refusing every large ordinary partition does not satisfy F02.

Execute coherent chains: Entities/Relations and their consumers; Callables/Receivers/Aspects;
Events/Bindings and their consumers; projection/coverage reductions. No old-resident fallback
is accepted as completion. Introduce reusable secondary orderings only for actual repeated
consumers; otherwise a streamed join is simpler. A built-in-composed provider/plan is preferred;
any necessary custom reducer must accurately expose ordering/partition/boundedness rather than
claiming automatic spill. No universal optimizer or accounting service is added.

### 5.4 R-C2 — Prepare authority once and migrate every upper consumer

Build receiver/event/binding admissions during their owning normalization operations from checked
predecessors and actual new outputs. Separate this authority construction from
`binding_normalization::verify`/`verify_upstream`, receiver/callable reruns and event replay.
Expose a borrowed/indexed immutable view of bound calls, shapes, source forms, effective
invocations and composition admissions. Large indexes reuse normalized attempt files; do not
add a second global resident `VerifiedBindings` map. Scope use to the actual consumer's closure.

Migrate SourceCalls, Models and Summary together with this constructor. Models also currently
reruns `enriched_production::enrich_all`; consume completed checked Enriched context/argument/
native-call inputs instead, keeping the narrowly necessary Model applicability checks. Inspect
nested verifier calls as part of these concrete consumers. A named wrapper that still normalizes
upstream is not migration. Prepared views reject foreign attempt/content/profile/policy/vocabulary.
Do not cache across an input lifetime merely to avoid constructing this view.

The complete upper inventory remains the 21 `UpperStage` variants. Configuration/Native/
EmbeddingConfiguration use small definitions; Text/Embedding group exact requests/spec/winners;
CatalogCore/CatalogEvidence/Selection/Synthesis/Retrieval consume scoped joined inputs; Local/
Base/Completion/SourceCalls/Enriched/Models use exact source/evaluation/call closures; Summary
uses completed Local/Model/Enriched/SourceCalls and shared invocation topology; Structural/Analytic
borrow their required projections; AnalysisFrontier/CatalogFrontier reduce independent expected
method/frame/coverage domains. Default Catalog must not prepare unrequested behavioral inputs.

Actual loader/edit scope includes `normalize/mod.rs`, model normalized admission functions,
`catalog_core::aspects`, `consumed_rows`, `local_semantics`, `semantic_execution`, `semantic_models`,
`semantic_summaries`, `structural`, `analytic`, catalog adapters, synthesis, retrieval preparation/
retrieval, final coverage and `analysis_graphs`. Scope these consumers alongside producer migration:
a streaming normalization followed by recollection of the whole rich fact graph is incomplete.
Global algorithms may legitimately need a compact complete topology; that is separate from
unrelated native facts/original payloads. The projection plan owns that consumer slice.

### 5.5 Focused acceptance and execution boundary

R-C0 starts with small hand-authored invalid supports: absent support, wrong context/input/provider/
family/fidelity and out-of-scope evidence with otherwise existing referents. Also replace a required
outcome key with another at equal count and preserve valid provider disagreement. Exercise both
fresh admission and detached import with internally consistent hashes; invalid semantics still
refuse. Valid transport/restore retains exact originals/vectors and invokes no providers.

R-C1/R-C2 use cross-partition imports, ambiguous overloads, receiver/default/keyword/vararg cases,
native Bound/unattached inventory, shuffled batches and a skewed source/target. Independently
specify known identities, roles and uncertainty, then compare canonical outputs for two memory/
partition configurations on a synthetic workload that forces external ordering. Both must finish
successfully; honest refusal alone is insufficient. Test prepared validity mismatch and cancellation/
late-output failure, and inspect that SourceCalls/Models/Summary/Enriched no longer replay unrelated
predecessors. Do not introduce runtime invocation counters solely to demonstrate removed replay.

Cover four-frontier/two-profile routing with targeted tiny fixtures. Select positive behavioral
and optional method cases needed by changed consumers, preserving explicit partial/NotRequested/
unavailable outcomes and method/vector settings. Reuse independent model/analytics known answers.
Run touched-crate compile checks and explicit affected verification-family filters. This is new
remediation validation, not the waived historical compiler-suite/parity work, a live-library
pilot, assembled qualify or performance measurement. Completion still requires migration of the
whole applicable binding/consumer inventory, not only the representative first fixture.

## 6. ER1/ER2 — Evidence construction and actual encoder dependencies

**Implemented / focused Tested, 2026-10-07.** The [combined coordinator](evidence-retrieval-and-evaluation-plan_2026-10-06.md)
owns ER package state and the new review findings; its §2 fixes shared meanings and representation
policy. Combined execution completed §5's targeted graph-native acceptance; the coordinator records each finding's closure evidence.

### 6.1 ER1 — Honest roots, parts and binding operation

Extend the existing retrieval model/compiler operation, using source/catalog/association authority.
Resolve defining-source alternatives independently of public access/import provenance. Construct
roots for callable/non-callable/ambiguous source definitions, public invocation variants, source and
effective options, standalone documents, scenarios and deployment/release material. Existing Member
and Release origins alone cannot erase unsupported roots. Source unavailable/native cases keep
qualified boundary information rather than guessing a body.

Construct ordered ContentParts with primary versus interpretation-context purpose, semantic scope,
original artifact UTF-8 ranges and synthetic-segment maps. SearchWindows bind a selected unit/view,
part intervals, complete rendered input and render/partition policy. WindowBindings nominate exact
or candidate targets from primary information and qualified association basis. Context-only imports,
headings and helper mentions remain navigation/context; publisher no longer interprets broad parent
membership as fragment applicability. Mandatory interpretation dependencies include conditions,
signatures/variant/configuration, headings/table keys, setup and original qualifications.

The owned operation receives completed source/catalog/association inputs and returns admitted
unit/part/window/binding/map streams with explicit omissions/unknowns. Shared validators establish
range/parent/context/association correctness without replaying every upstream producer. Diagnostic
replay remains an independent control. Construction is per immutable documentary grain with bounded
batches; release rich source after its final use, retaining compact shared source/association indexes.

Register each new canonical record in typed declarations, nominal identities, graph kinds/roles,
relation inventory, loaders/frontiers, admission and neutral transport before assuming derived
codecs cover it. Pure validators and explicit omission controls protect detached import/restore.
Source maps and qualifications are semantic content; private evaluation expectations never appear.

### 6.2 Semantic partitioning and tokenizer adapter

Reuse latest canonical Ruff nodes/coordinates, current Markdown hierarchy and Unicode boundaries.
Choose semantic splits around complete signatures/options, enclosing predicates, source blocks,
document headings/tables and scenario setup. Prefer1024 complete-input tokens, normal hard2048;
mandatory context and synthetic headers count. No truncation or invented negative evidence.
An indivisible oversized unit emits explicit lexical-only/partial-vector availability while full
original expansion remains reachable. A larger-token admission policy is conditional ER5 work.

Use a narrow Rust tokenizers adapter loaded once from exact acquired encoder assets/configuration.
Disable truncation and count actual special-token/input preprocessing behavior. Rust tokenizer
byte offsets describe rendered input; compose them with part maps to original artifacts. Synthetic
and empty special-token ranges have no fabricated source spans. Keep endpoint tokenization for
focused parity/admission controls, not every tentative split. Qualify service assets beyond one
JSON file where additional configuration affects encoding.

### 6.3 ER2 — Separate values from their consumers

Replace the format2 all-purpose embedding spec at the model boundary. Encoder realization identifies
actual checkpoint/tokenizer/custom-engine/pooling/precision/full-output behavior. Input identifies
exact final encoder bytes/processing. Render/partition and query recipes have their own dependencies;
projection identifies source full-value digest, dimension/algorithm/precision; each E0/E1 consumption
identifies its actual value/projection and method/policy. These distinctions may be nested records,
not a new registry or service. Identical actual input can share a winner across source occurrences.

Accept immutable normalized full4096 F32 winning bytes, derive normalized1024 by F64 prefix-norm
accumulation and one F32 rounding, reject invalid norm/components and use shared validation.
Initial E1 analytics and ANN share that projection; full values support candidate rescoring and
references. Both document and query clients request full output under admitted launch semantics.
Never reconstruct missing dimensions from old1024 or infer bitwise endpoint-prefix parity.

Compiler preparation, explicit cache/inference effects and analytical consumers migrate together.
Query-only changes do not re-key unchanged document/E1 inputs. A projection change derives from
existing admitted full bytes; changed encoder/input requires fresh inference. Canonical consumption
binds exact winners, availability, projection recipe and compact/full bytes needed for service-free
reconstruction. Current immutable-winner semantics survive; no live cache lookup repairs canonical
content. Token/refusal/NotRequested cases remain distinct and do not erase original source content.

### 6.4 Functional controls and changed consumers

ER1 migrates retrieval/build, cpg-core retrieval preparation and source/option/document/scenario
construction, graph admission/frontiers and typed artifact loaders. ER2 migrates embedding spec/text
admission, cpg-core shared effect/cache consumption, lctx-embed and the Python numerical client,
all E1 consumers and canonical transport. Native cache/schema/index/restore changes are at the
realization plan §7 and projection plan §7, part of the same package rather than deferred cleanup.

Use independent sibling/re-export/non-callable/ambiguous/option/release and setup-only cases;
Unicode/synthetic/special-token maps; boundary splitting and indivisible refusal; canonical omission,
wrong-context/range refusal and detached roundtrip. Numeric controls use hand-computed tiny vectors,
zero/nonfinite norms and source/dimension/receipt mutation. Identity controls change query, rendering,
source binding, encoder, projection and analytical policy independently to establish actual reuse.
No live service/database is needed for pure contracts; fake vectors establish that seam only.
