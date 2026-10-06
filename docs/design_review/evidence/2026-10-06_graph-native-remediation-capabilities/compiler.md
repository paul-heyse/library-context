# Compiler remediation foundation advice

Proposed advice for the plan author, 2026-10-06. Examined clean main
`e52e6312367140a173e8aef338e10a6c69efbeb6`. No tracked edits or probes. This develops
audit F01/F02/F03; it does not change historical user acceptance or imply new tested evidence.
The root owns the target, plan and disposition. The architectural basis remains the two
2026-10-05 graph-native target/SurrealDB capability reviews. Core 3.3, CI 1.4 and the efficiency
heuristics inform this focused foundation assessment.

## Recommended combined target

Make semantic admission a required operation over exact completed compiler inputs. Keep source
model authority over the checks, while the workspace supplies ordered, joined, partitioned
streams. Establish necessary predecessor properties once before their consumers rely on them,
carry their validity within the immutable attempt, and finalize the complete referenced graph
before exporting an admitted artifact. Replace whole-corpus typed collectors with bulk joins,
ordered streaming reducers and dependency-closed local kernel inputs. Keep whole-topology
materialization only for an algorithm's explicitly named universe, with compact IDs/incidence
and no unrelated native facts or original corpus payloads.

These corrections must be designed together. Calling `Workspace::validate()` wholesale would
restore checks but also invoke exact producer replay invariants, retain broad resident inputs
and worsen F02/F03. Removing every verifier would eliminate replay but also destroy the
binding/composition assurances those verifiers currently mint. Neither is a suitable pivot.

## F01: one semantic admission, with necessary checks separated from diagnostic replay

Current source: production `compilation.rs:531,758` completes providers/stages without running
model invariants; `artifact.rs:1095-1122` checks compilation, coverage, row shape, nominal
references and derivation cycles. `assertion.rs:990-1000` requires matching support context,
provider/family and scope; `assertion.rs:1309-1313` requires support for every assertion.
These properties are not implied by graph reference closure. Publisher reconciliation does not
establish them later.

The semantic owner should distinguish actual admission obligations from test/debug replay.
Use ordinary Rust declarations/functions in the existing model validation owners, not a new
proof framework, generic rule DSL, persistent receipt ledger or stage-grant mechanism. Move
normalization `stored.matches(normalize(...))`, finite-summary replay and similar differential
checks to explicit test/debug routes where they challenge implementation changes. Preserve and
implement independently checkable production properties:

- Assertion/support total membership; provider/run/family/context consistency; acquired-input
  and scope ownership of subjects, conditions and evidence; exact source bounds.
- Provider run family inventory/digest and coverage ownership; complete expected coverage keys
  derived from captures/profile rather than from observed results.
- Canonical conditions, referenced atom/context and assumption basis; nominal roles/subtypes,
  reference closure and derivation acyclicity across the completed graph.
- Exact required invocation/outcome/coverage domains from captures, method configuration and
  declared predecessor scopes; typed missing/partial/unavailable/NotRequested distinctions.
- Binding/signature/receiver/event premises sufficient for the specific bound-call and
  composition authority; structural/path claims retain only their declared witness meaning.
- Exact vector text/specification/value consumption and provenance of analytic results.

Existing anchors include `attribution.rs:324,437`, `assertion.rs:457,541,591`,
`assumptions.rs:303`, `analysis/expected.rs`, `analysis/frontier.rs:249-313` and
`embedding/analytic.rs`. Inventory their actual declared checks and applicable owners rather
than introducing an independent handwritten checker list in orchestration. For each replay
checker retired from production, identify the property its consumer still needs and the
replacement owner operation establishing it. A deterministic rerun or digest is not that
replacement property.

The compiler may internally carry a private acknowledgment/checked view bound to the existing
completed descriptors, profile, policy revision and exact selected vocabulary boundary. Its
constructor must be unreachable before the actual owner checks finish; an empty descriptor
set cannot authorize an unperformed check. No persisted certificate is required. Mutable
pending streams never mint this state. Final admission consumes the checked state and covers
cross-element/global obligations, while exported manifest content binds the result. Failure
or cancellation leaves the attempt without an admitted artifact. If native batch boundaries
can contain forward references, defer only those closure checks until their full declared
predecessor set is complete.

Implementation ownership stays split: model functions define relation/role/evidence meaning;
`cpg-core` executes bulk joins, ordering and streams and manages private files; artifact
lowering transports the checked result; publisher verifies its persisted realization.
`lctx-model/Cargo.toml` is explicitly pure and has no DataFusion dependency. Use typed resolved
tuples or ordered owner inputs at that boundary instead of embedding an SQL runtime in the
model. Generated column/key plumbing can lower mechanically; handwritten semantic predicates
remain with their model owner.

### Detached export/import and portable restore

Do not let a self-authored manifest inherit the live compiler's checked state. Distinguish:
(1) same-attempt export verified against its actual admitted manifest/content, which carries
established validity through immutable bytes; and (2) detached `publish-artifact`/portable
restore, where bytes have no live semantic acknowledgment and require necessary semantic
re-admission before a target becomes ready. Hash/format/policy compatibility and canonical
payload reconstruction only establish transport consistency. This distinction must appear
in the public API/types and publication/restore consumers, not just trusted-export comments.

The canonical graph retains enough typed inputs for native support semantics: observations,
support payloads (`graph/supportvalue.rs`), captures/sources, ProviderRun and RunFamily,
qualifications, conditions/atoms, assumptions, evidence, ArtifactUse and corpus-library links.
`lctx-surrealdb/src/codec.rs:20-22` and `batches.rs:55-68` already demonstrate mechanical
graph-to-typed-record reconstruction. Reuse/share a neutral typed dispatch based on the existing
graph inventory; do not make the compiler depend on the SurrealDB adapter merely to decode its
own graph. Decode bounded batches into attempt-owned temporary typed streams and execute the
same admission owners over those streams. No acquisition/provider/service call is required.

For detached complete artifacts, derive capture/profile and method/configuration obligation
universes from actual typed input declarations and compare the actual outcomes/support domain.
Require all semantic admission inputs applicable to the declared frontier. A missing source
family is a refusal or a declared unrequested/partial case justified by the independent input
domain, never a reason to skip a checker. Reconcile original bytes/vector receipts as now.
Restore may stage bytes privately, but must pass necessary semantic re-admission before
sealing/publication, using the same owner checks as fresh/imported compilation.

The graph intentionally does not transport workspace ProjectionSnapshot/Chunk formats and
some ephemeral producer descriptors. This is another reason not to revive the old complete
invariant/replay runner for import. Required projection properties should be checked from
canonical entities/typed arcs/gaps and source lineage, not by importing retired topology
serialization. Any necessary admission input not presently represented must either be derived
mechanically from retained canonical meaning or become a declared artifact semantic input;
resolve that specific gap before claiming that frontier portable. Do not fabricate old grants
or retain a legacy decoder.

The supported importer accepts complete canonical artifacts, not arbitrary diagnostic or
projection exports. A partial neighborhood cannot become a complete compiler input simply
because it decodes. The same pure semantic checks can reject malformed support/ownership and
missing obligation keys, but cannot authenticate a dishonest producer or establish unrestricted
Python truth; preserve the target's controlled-compiler trust scope. No signing infrastructure
or hostile-compiler proof requirement is implied.

## F03: prepare immutable normalized authority and migrate real consumers

`normalized/binding_normalization.rs:879-898` already names the useful prepared product:
bound calls, binding shapes, source shapes, effective invocation and composition admissions.
Today `verify()` at :899-907 obtains it by rerunning binding normalization and
`verify_upstream()` at :1589-1634 reruns entity/relation/callable/event normalization.
Receiver verification at `normalized/receiver.rs:599-628` recursively reruns callables;
event verification at `normalized/event_normalization.rs:204-218` reruns event evaluation.

Split useful authority construction from broad result replay. Construct the necessary
receiver/event/binding admissions during their owning normalization operation from already
checked predecessors and the actual newly constructed outputs. Preserve their exact semantic
eligibility checks. Carry a borrowed or indexed immutable prepared view into the upper owners.
Use the normalized attempt files for large derived indices and materialize only a requested
dependency-closed group, not one additional global `VerifiedBindings` map duplicating every
binding. A small shared view is enough; this does not require a universal cache service.

Migrate these consumers in the package that changes the authority constructor:

| Consumer | Existing repeated work | Required replacement |
|---|---|---|
| SourceCalls | `execution/source_call_records.rs:318` verifies bindings in Behavioral | Exact call/event/binding admissions from checked normalized inputs |
| Models | `execution/model_production.rs:538` verifies bindings | Same normalized authority with model applicability's actual required source scope |
| Summary | `execution/summary_production.rs:1618` verifies bindings in Behavioral | Same authority plus borrowed named invocation topology and completed Local/Model/Enriched/SourceCalls |
| Receiver/Event preparation | Receiver/callable and event evaluation replay | Checked upstream applicability and newly constructed owner admissions |
| Enriched -> Models | `execution/model_production.rs:365` reruns `enriched_production::enrich_all` and compares many frame inventories | Checked completed Enriched context/argument/native-call views; retain narrowly necessary model-specific applicability checks |
| Other upper owners | Broad `Data`/`Rows` loaders currently copy related inventories | Read exact scope/columns from prepared normalized sources; retain only method-specific state |

The Enriched replay is important: replacing only the three binding verifier call sites leaves
an additional full predecessor rerun in Models. Search nested verifier/replay calls during
implementation, including those hidden inside model-owned kernels, and migrate actual callers.
Historical debug replay may remain explicit; no compatibility consumer or emulated old store
grant is needed.

## F02: bounded normalization across all nine bindings

Use the existing attempt-owned IPC streams and DataFusion runtime. Perform key joins,
membership, deduplication, grouping and ordering before rich typed decoding. Build reusable
compact lookup inputs or secondary ordered files only for actual consumers. Stream output
into the stage's canonical writer; release a processed group and its rich evidence before
advancing. A different grouping order may require an external sort, but a stage need not
persist another semantic authority or retain all result rows merely to emit them later.

The following grains are recommended starting points, not permission to drop cross-partition
dependencies. Each owner must define the complete key and imported dependency closure before
the caller partitions it.

| Binding | Bulk preparation and local kernel scope | Cross-group obligations |
|---|---|---|
| Entities | Join source/module/qualification ownership once; process source spans in lexical order, retaining an active owner stack rather than cloning all source rows | Provider/context correspondence, external symbols and complete entity universe; source identity survives imported references |
| Relations | Separate reference/import/type/mention branches; join canonical source/entity/provider keys and emit ordered resolved tuples | Cross-module imports, corpus-library associations, type/reference candidates and all alternatives; same-input membership is not association proof |
| Callables | Group effective callable/declaration and signature families; retain all overload candidates, parameter slots and typed source correspondence | Imported target resolution; complete signature enumeration and ambiguity across providers; decorator/overload families must not be split midway |
| Receivers | Group call target/syntax and exact applicable effective-callable inputs | Preserve expression-relative ClassOf and source/context constraints; no intrinsic instance identity inference |
| CallableAspects | Group assessment/declaration/class-field with joined decorator/trait/default inputs | Cross-declaration setter/accessor, ancestry and field associations; metadata recognition still admits no body/signature authority |
| Events | Group source call/evaluation occurrence and qualified provider alternatives; stream ordered members | Source/native correspondence and exact phase/dispatch alternatives, unfollowed inventory and condition/coverage boundaries |
| Bindings | Group event/invocation alternative plus imported complete signatures/receiver admissions; emit bound/shape/composition products once | Keyword/default/vararg/overload combinations, complete candidate and required outcome keys, native Bound/unattached remains distinct |
| Projections | Select vertices independently; stream typed arcs/gaps and side lineage by named input/context projection | Isolates, parallel/self arcs, missing internal endpoint refusal, canonical IDs; SCC/global methods need their full named universe, not an output selector |
| Coverage | Join independently derived capture/profile obligations with immutable output descriptors and native evidence; ordered scoped reduction | Empty expected universe versus missing observations; exact membership, declared complete/partial/unavailable/NotRequested and referenced producer content |

Source/source-context partitioning alone is insufficient for Relations, Callables and Bindings.
Prepare cross-module dictionaries and candidate joins from the full completed input and pass
the actual dependency closure to local kernels. Do not replicate the full project dictionary
into every partition. A dependency can be obtained with a bulk semijoin/indexed range fetch,
then joined before decoding. Absence claims require a closed candidate universe, not just
what happened to be in the current chunk.

Handle skew explicitly. A single module, overload family or high-fanout target can exceed a
coarse partition. Use ordered incremental reductions for membership/enumeration, tiled bulk
joins for real many-to-many output, and nested finer semantic groups where independent.
Stateful source ownership can be an ordered active-span sweep. Where a kernel truly requires
whole-group state, retain only the irreducible compact structure and stream its surrounding
payloads. Refusing every oversized ordinary partition is not completion of this migration.
No runtime cost estimator, detailed work-accounting service or numerical capacity proof is
needed; the plan should identify the concrete skew mechanism and its focused control.

## Upper migration and analytical lifetime

Keep the finite 21 `UpperStage` bindings in `compilation.rs` as the completeness inventory.
Configuration, Native and EmbeddingConfiguration are small immutable definitions/inventories;
do not widen them into fact hydration. Text/Embedding group by canonical text request/spec and
retain exact winners; CatalogCore/Evidence, Selection, Synthesis and Retrieval consume scoped
source/catalog joins and stream output. Local/Base/Completion/SourceCalls/Enriched/Models use
source/evaluation/call scopes and the prepared normalized authority. Summary uses completed
predecessors plus compact named invocation topology. Structural/Analytic borrow compatible
projections and hydrate evidence only for resulting claims. AnalysisFrontier/CatalogFrontier
reduce independently declared frames/methods/coverage and cannot derive their expected set
from output rows.

Migrate loaders with consumers, including `normalize/mod.rs`, `catalog_core.rs::aspects`,
`consumed_rows.rs`, `local_semantics.rs`, `semantic_execution.rs`, `semantic_models.rs`,
`semantic_summaries.rs`, `structural.rs`, `analytic.rs`, `catalog_*`, `synthesis.rs`,
`retrieval_preparation.rs`, `retrieval.rs` and `final_coverage.rs`. A partitioned normalization
producer whose upper consumer recollects every rich record has not closed F02.

Retain full topology only for global algorithms. `analysis_graphs.rs` is the right lifetime
owner for compatible borrowed graphs, but its current load collects all assessment/header/chunk
records before hydration. Narrow to selected keys and release encoded assembly after hydration;
prepare each topology once and release after its last actual consumer. Do not change the
semantic computation universe for an output selector. Optional methods/layers stay off by
default; disabled methods still get their declared NotRequested outcomes without building
method-specific rich input. Selected PageRank/communities/FCA/RCA/kNN retain settings,
seed/convergence/partial behavior and exact vector specifications. A global method's compact
memory requirement is legitimate and distinct from retaining a whole fact graph.

## Pinned DataFusion capability boundary

The current lock matches the skill: DataFusion 55.1.0, Arrow 59.3.0, object_store 0.13.2.
Read skill routes/capabilities `df.consume`, `df.relations`, `df.source`, `df.storage-reuse`
and `topics/physical-execution`; their exact-release contracts support the proposed built-ins.

- `execute_stream` avoids terminal result collection but does not bound operator or consumer
  state. `execute_stream_partitioned` preserves partition streams, not global ordering.
- Explicit ORDER BY provides required order. Many-to-many joins preserve multiplicity;
  null equality and null-safe equality differ. Do not use UNION/DISTINCT to collapse semantic
  alternatives unless the owner's exact identity/payload deduplication permits it.
- Spill is operator-specific. The existing pool/disk manager/sort settings are useful;
  prefer suitable sort/merge joins for large equality joins, but inspect the actual plan and
  exercise skew/empty-side cases. A configuration preference is not evidence every join spills.
- `DataFrame::cache` materializes in memory and is unsuitable as the blanket cure for F02.
  A reused logical view reuses a plan, not necessarily prepared data. Reuse immutable files
  when a second ordering/index pays for a concrete repeated consumer.
- Existing Arrow scans can feed this work; file-scan automatic repartitioning is documented
  for Parquet/CSV, not an automatic promise for the current IPC files. Use sorted sequential
  streaming first; introduce partitioned files/Parquet only for a specific access requirement.
  A custom TableProvider/ExecutionPlan is unnecessary unless built-in composition cannot express
  the required scan or streaming reducer. If added, its ordering/partition/boundedness claims
  must be accurate.

## Sequencing, acceptance and remaining decisions

Start by separating semantic admissions from producer replay and defining exact checked
predecessor interfaces. Close F01 for native support/coverage immediately with bounded owner
checks. Then migrate a coherent normalization dependency chain and its consumers together:
Entities/Relations -> Callables/Receivers/Aspects -> Events/Bindings -> SourceCalls/Enriched/
Models/Summary and product consumers. Projection/coverage inputs follow their actual completed
dependencies. Final admission consumes all applicable acknowledgments and global closure.
No temporary dual semantic authority, whole-resident fallback or old-format reader should
become the accepted completion route.

Focused independent acceptance should expose these failures, not just reproduce old outputs:

1. Unsupported native observation; support redirected to wrong context/input/provider/family;
   evidence outside declared acquired scope; valid conflicting provider assertions retained.
2. Missing required outcome key replaced by another key with the same count; partial/empty/
   unavailable/NotRequested distinction; current final-frontier method membership preserved.
3. Two modules referring across partitions, conflicting provider targets, ambiguous overloads,
   keyword/default/vararg cases and Bound/unattached source inventory; hand-authored expected
   identities, roles and consequences, then shuffle partition/batch order.
4. Synthetic many-module normalization under small memory; one skewed source/target/signature
   family; known canonical result equality and successful completion, not merely honest refusal.
5. Prepared source/binding views rejected for foreign attempt/content/profile/vocabulary;
   semantic changes invalidate affected use; SourceCalls/Models/Summary no longer rerun
   unrelated predecessor normalization; Enriched is not rerun by Models.
6. Named graph with isolate, parallel/self arcs, external/unresolved boundary, selector-only
   subset and cross-scope traversal; independently known SCC/weighted/incidence/vector answers.
7. Four-frontier/two-profile routing through focused tiny fixtures, default optional methods
   NotRequested, separately selected methods/layers with actual partial/vector/convergence
   outcomes. No real library, live Qwen, operator database or historical compiler-suite rerun
   is implied by these planned controls.
8. Cancellation or late input/output failure leaves no completed admitted artifact, and a
   transported valid artifact still preserves originals, participant roles and exact vectors.
9. Detached canonical import and portable restore reject an unsupported observation or
   wrong-context support even when all declared file/payload hashes match their self-authored
   manifest; same-attempt immutable export preserves already checked semantics. Partial
   diagnostic exports refuse full-artifact import; no provider/service execution occurs.

Compile checks and explicit affected `verify-model`/`verify-compiler`/`verify-analytics` controls
are the execution route, with filters selected by the implementer. These are proposed new
controls; no command has been run here. User-directed focused verification governs execution;
do not silently expand it into broad qualify or revive the stopped historical suite.

Decisions for the root to settle in the plan: (a) explicit production-versus-diagnostic validation
declaration ownership; (b) exact checked normalized interfaces and their foreign-source refusal;
(c) dependency-closure keys and skew strategy for each kernel family before migrating its callers;
(d) whether a given reusable secondary file is justified by repeated consumers or a streamed
join suffices; (e) global topology lifetime and which optional methods actually need it.
Recommended defaults are source-owned Rust checks, immutable attempt descriptors, sorted bulk
joins plus local reducers, sequential bounded admission/preparation, and no new persistent
proof/cache/workflow infrastructure. Material alternatives can change these choices on concrete
source evidence; detailed cost/proof services are outside this scope.
