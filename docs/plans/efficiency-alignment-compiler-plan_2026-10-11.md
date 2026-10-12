# Efficiency alignment: compiler, capture and reuse

**Proposed, 2026-10-11.** Companion to the new
[coordinator](efficiency-alignment-plan_2026-10-11.md), the sole owner of progress, finding
disposition, baseline and operator decisions. Sources are original O-F07/O-F16–O-F30/O-F46 and
audit A-F02/A-F04/A-F05/A-F07/A-F16/A-F20. Existing plans were not consulted or superseded.
No product implementation occurred. The bounded 2026-10-11 decision investigation and its evidence are recorded in §4; finding dispositions remain Planned.

## 1. Target and foundation assessment

Compile over an immutable, completely described captured input. Reuse eligible completed provider
results before invoking providers; attach admitted native contributions by reference; demand only
the exact relation keys needed by the consumer. Separate relation value identity from exact evidence
identity so unchanged values can cut off propagation without attributing old receipts to new inputs.
Keep portable reuse enabled by default while reducing its encoding, crossing and retention costs.

The existing provider sink, declaration and stage contracts already support composition. A new generic
provider framework is unnecessary. The lower contract boundary must make semantic inventories usable
by cold admission without importing executable analyzer constructors. Native owns physical installation,
incarnation and protection; the compiler owns request/spec equality and outcome eligibility. The model
owns availability, coverage, identities and provenance, including explicit empty and NotRequested.

Current native view members already have direct selected-view/relation/key addresses and a unique
view/key index; N04 will bind these to the exact physical realization/member namespace (§4 C03/C04). The required foundation is a durable, validated installation-complete fact, not a new lookup
architecture. Existing sorted full-run access remains appropriate for dense demand. Current profile
and configuration keys are conservative but too broad; narrowing them is safe only after dependencies
and evidence-bearing outputs have explicit contracts.

A patch release with unchanged source bytes but a different input role is a revealing case. Capture
must distinguish byte identity from result-affecting acquisition metadata. A second revealing case
changes source spans while retaining relation values: a downstream value-only computation can be
reused, but receipts claiming the old exact sources cannot be relabelled. These cases drive the design.

## 2. Packages

<a id="C01"></a>
### C01 — Separate provider contracts from executable composition

**Sources:** O-F46; A-F12. Entry points include `cpg-extract` provider `bundle.rs`, `cpg-core`
workspace/facts admission and `scripts/producer_fingerprint.rs` (locate the current fingerprint
implementation before editing). Preserve `ProviderSink`, declarations and `ProviderStage` unless
an actual consumer requires a change. Move captured input/configuration contracts and the semantic
provider inventory below executable adapters. Composition remains with the compiler entrypoint.

Expose the narrow native candidate/held-inspection/attach/completion operations needed by C03/C04.
The native layer must not decide whether two captures are semantically interchangeable. Cold facts
admission must not construct or link analyzers merely to learn their output contracts.

D02 owns physical crate migrations and dependency validation; C01 owns the semantic boundary. The
fingerprint walker currently follows normal/build local dependencies and source directories. Feature
gates alone do not narrow that identity. Retain all responsible model, ingestion, codec and derivation
implementation in their owning fingerprints; a reader-only change should not invalidate extraction.

**Closure:** publisher, embedding and contract-only compiler consumers build through their actual lower
contracts without unrelated analyzer implementations. An analyzer/ingestion/model change still changes
the responsible producer identity; an unrelated reader change does not. Verify actual dependency and
fingerprint behavior, not a diagram alone. C02 can specify complete keys before all crate moves finish.

<a id="C02"></a>
### C02 — Describe complete captured requests and eligible outcomes

**Sources:** O-F16/RC02, O-F19/RC05; A-F04. One authoritative inventory constructs both the product
request and contribution specification. Include selected bytes, additions/deletions and absence,
provider/supplier implementation, settings, interpreter/platform/environment context, upstream exact
or value dependencies, declared output contracts and requested availability.

**Planning discovery PD01:** `CapturedSourceBinding` alone is insufficient. Acquisition emits origins,
roles, ownership, distributions/RECORD and derivations from `AcquiredInput`/`LibraryInventory` in
addition to bytes. Two captures with identical files but different tree labels can emit different
InputOrigin; changing selected roles changes ArtifactUse. Include all result-affecting acquisition
metadata in capture/request identity, separately from byte content identity. Start from acquisition's
actual emission inputs rather than assuming a binding digest is complete.

Initially admit reuse only for complete results and valid explicitly NotRequested results whose
specification guarantees substitutability. Failed, abandoned, poisoned and resource-partial outcomes
are not generally reusable. Any future provider-specific partial policy must establish equivalent
coverage and outcome semantics. Start at whole-provider grain; per-input reuse requires independent
input/context correspondence and is not a prerequisite for this track.

**Closure:** unchanged capture avoids provider execution; changing bytes, absence, origins, roles,
settings, implementation or a relevant upstream view misses. Same bytes under a new role must not
reuse old attributed facts. Fresh attribution is honest, not a new receipt attached to old claims.
The exact declaration inventory includes empty outputs. Use controlled inputs, not protected evaluation
payloads, to distinguish these cases.

<a id="C03"></a>
### C03 — Preflight held candidates, then attach by reference

**Sources:** O-F16/O-F17/RC03; A-F04. Consume N04's exact-incarnation hold and installation-complete
contract. Find and hold a candidate, check request/spec/outcome, admission and recovered closure,
then validate the receiving attempt's prerequisites before any effects. Attach/bind its immutable
content through native operations. Keep protection until durable handoff or completed rejection cleanup.

Terminal previously-unadmitted results enter admission-for-reuse under N09's recovery owner. Prior
attempt admission is not required, but current complete candidate admission is. Distinguish ordinary
ineligibility (miss), corruption (failure) and unresolved effects (recoverable/unknown). Never turn a
partial prefix or a row count into evidence of complete installation.

Replace N-row attachment copies with view-local logical-to-physical contribution references. Migrate
all contribution enumeration, selected backup, detached export/import, recovery, audit and retirement
consumers together. A first global contributor match is not authority for the selected view.

**Closure:** a hit attaches without copying all row pointers; rejected candidates leave no receiver
mutation; a retiring or incomplete candidate cannot pass; shared content survives one borrower's
release; imported and recovered candidates establish destination-local protection. Actual native
controls are required for physical claims, with pure controls for semantic eligibility.

<a id="C04"></a>
### C04 — Use exact view-member demand without reconstructing the universe

**Sources:** O-F21/O-F22; A-F02. Recognize mandatory id equality/IN demand in the actual pinned
DataFusion expression/coercion contract; retain residual predicates. An OR branch is not mandatory
key demand. Verify expression handling against the locked library before changing it.

Use `H(native-view-member/v2, held.view_realization_id, relation, key)` for bounded multi-get after
N04 establishes validated completion of the selected view/dependency closure. Logical view equality
alone does not identify the held physical member namespace. Small exact requests must not grow with foreign attempt history or scan
all sorted membership. Key-only requests return compact keys without payload hydration; hydrate only
where the consumer or residual needs values. Dense demand may still use one sorted run.

Retain deterministic sorted/distinct output, explicit empty requests, view/shape validation, virtual
and absent relation meaning, alias resolution, nominal conflicts, charges and drain. An empty input
set does not permit an invalid view to pass unnoticed.

**Closure:** sparse, dense, duplicate, missing, virtual, foreign-backing and incomplete-view cases;
mandatory versus non-mandatory filters; key-only versus payload demand; equal results/refusals against
an independently enumerated small admitted view. Query-shape/source evidence establishes selectivity;
latency measurement is external. This replaces the earlier owner-by-key open design question.

<a id="C05"></a>
### C05 — Separate value change from exact provenance and profile meaning

**Sources:** O-F18/O-F19/RC04/RC05; A-F05. Split unrelated configuration outputs before narrowing
reuse. Define relation-local canonical value identity, output contract/outcome/completeness, exact
source evidence, and current attachment as distinct facts. Dependencies declare whether they consume
exact views or proven value equivalence. Metadata/provenance consumers retain exact dependencies.

`AnalysisInvocationID` and source receipts include exact source identity, and their checks verify it.
Receipt-bearing outputs cannot be reused by refreshing a spec label. The §4 C05 matrix keeps these
outputs ExactView, and selects separate CatalogCore/Retrieval payload, preparation/scoring/inference
computations with freshly generated exact receipts. Migrate each validator with that chosen separation.

Use explicit profile-neutral versus profile-specific declarations. Removing one profile field from a
hash is insufficient: native context includes profile/requirements and changes availability; provider
runs retain genuinely requested families. Only providers and downstream outputs whose complete inputs,
coverage and output meaning are neutral can cross catalog/behavioral profiles.

Give cutoff one actual owner: the per-relation dependency/request construction and product lookup
path above. The existing `compilation/reuse.rs::DependencyGraph::affected` currently feeds a debug
count, not execution. Remove that unused cutoff graph as consumers migrate; preserve its real selected-
premise and double-binding conflict checks at the authoritative request/binding boundary, with equivalent
controls. Keep useful hit/miss tracing there. Do not retain charged graph traversal as a second apparent
cutoff authority or delete its meaningful validation along with its diagnostic-only reachability work.

**Closure:** unrelated config changes stop invalidating unaffected values; exact span/evidence changes
still update receipts; profile-neutral work reuses while requested availability and provider attribution
remain correct. A value hit cannot turn NotRequested into complete coverage or preserve a stale claim.

<a id="C06"></a>
### C06 — Form needed unions and compact uniform non-requested outcomes

**Sources:** O-F23/O-F24/O-F25/RC08; A-F05/A-F07. Completed descriptors always list every declared
output, including empty ones. Form union views when a declared consumer needs them, not after every
vocabulary contributor. A completed empty relation is a first-class result, never an absent relation.

For uniform NotRequested over an exact declared domain, retain a compact outcome with explicit owner,
exact source/definition and domain-total meaning (ADR-0160). Do not manufacture per-kind invocation/output work
merely to repeat that outcome. Mixed or partial scopes retain their distinctions; no-scope and unknown
are different from domain-total non-request. The model owns scoped answers without inferring them
from missing rows.

The contribution is physically Complete when those exact declarations and every declared empty output
have been produced. Semantic NotRequested belongs to the independently admitted compact domains.
Reuse follows the ordinary complete-candidate protection/admission route and preserves those semantic
answers; it does not require contribution outcome4. Embedding retains its separate Fresh/cache policy.

Migrate frontier enumeration/coverage premises, `FrontierCheck.finish` scoped-set reconstruction,
compilation publication assessments and scoped serving availability together. Prepare expected inputs
once per compile using indexed inputs/profile/method extensions. This is invocation-local preparation,
not a persistent cache over arbitrary source-owner sets; cold admission remains independent.

**Closure:** equal final unions with explicit empty and conflict controls; exact complete, partial,
NotRequested and absent/unknown scope answers at all four consumer boundaries; changed domains cannot
inherit old total coverage. Scheduling fewer units must not suppress requested semantic work.

<a id="C07"></a>
### C07 — Reuse provider preparation while preserving capture and correspondence

**Sources:** O-F07/O-F26–O-F30; A-F07/A-F16/A-F20. Build path/use/derivation indexes once with
multi-owner meaning. Reuse Pyrefly span/path session preparation while preserving owned versus inherited facts; retain
the existing OnceLock AllThreads joined pool across dependency waves. No new pool or fixed worker cap.

Index Ruff canonical events by site/name and preserve multiplicity, unique-site fallback, ambiguity
and unlocated facts. Retain the existing separate diagnostics and semantic-observe walks; reuse the parsed tree and indexed
correspondence. No diagnostic-walk fusion is selected by this track. A single walk is not worth an
unestablished change to noqa, deferred-rule or diagnostic key-multiset behavior. Per-root serial post-processing remains until merged-input/lifetime parity is shown;
a new long-lived analyzer process is unnecessary for the current target.

Fuse SHA-256 with capture copying where the exact copied bytes are the hashed bytes. Use the complete
captured-tree bracket selected in §4 C07; a private/read-only directory alone is not immutable ownership. Corpus
origin describes the selected captured tree: exact revision equality or base revision plus modified
content identity. Neither checkout HEAD nor unrelated dirty files decide selected-source provenance.

N02 owns scheduling against acknowledgement of persisted work; provider completion retains accepted
sends through sink failure, cancellation and drain. Do not move an unbounded queue behind a new batch API.

**Closure:** inherited/multiple-name/ambiguous correspondence; suppressed and deferred diagnostics;
joined-worker failure/drain; selected-file tamper versus unrelated dirty work; changed input roles;
same bytes through fused hash/copy; faithful source origin. Retain CLI parity oracles as independent
controls, not production facts providers. Do not open protected evaluation inputs.

<a id="C08"></a>
### C08 — Keep portable reuse, reduce its physical overhead

**Sources:** O-F20/RC18; A-F05. Keep the portable cache enabled by default. Batch lookup and retention
through charged root windows and shared lifecycle operations, preserving per-entry holds, generations,
readback and refusal. Distinguish quota-fenced admission from SUM-based eviction planning; their current
roles do not establish two competing admission authorities.

Replace JSON byte-array expansion with the borrowed/preflighted Postcard 1.1.3 envelope selected
in §4 C08, preserving exact opaque section bytes under a deliberate format cutover. Carry
one checked canonical batch until its consumer/native ingress no longer needs it instead of decoding
and canonicalizing equivalent forms repeatedly. The cache remains optional: missing entries recompute;
corrupt entries are diagnosed under the existing policy, never accepted as valid compilation.

Bound eviction planning without making unmeasured claims about host capacity. A native pointer cache
is an alternative for a future need, not a prerequisite for correcting this cache. Preserve ordinary
warm reuse and lifecycle ownership rather than removing the feature to remove its costs.

**Closure:** bounded batch shape, stale generation and corrupt-section handling, quota races,
retained/absent/virtual outputs, equivalent cold compilation, and consumer release. Changing portable
encoding requires destination validation and explicit retirement of obsolete project formats.

## 3. Integration and extension boundaries

C01/D02 settle lower contracts; C02 establishes semantic keys. C03 requires working N04 protection and
completion, then enables genuine provider hits. C04 consumes the same native completion guarantee.
C05 is required before broad downstream cutoff/profile reuse. C06/C07/C08 have independent useful
slices and do not wait for the whole native plan. Shared workspace/ingress edits need one integration
owner even when their semantic prerequisites are independent.

A new provider declares its real acquisition/settings/dependencies and eligible outcomes. A new fact
family extends the model inventory and its consumer union without multiplying unrelated unions. A new
validator changes its admission identity without pretending old protected content was checked by it.
These extensions use existing owners, not another pipeline orchestrator.

Functional exit requires producer-hit/miss, receipt/coverage, direct selected access, portable-cache
and lifecycle controls through migrated consumers. Full real-library cold/warm and memory/crossover
campaigns remain external follow-ups in the coordinator. No performance result is claimed here.


## 4. Decisions resolved for implementation (2026-10-11)

**Proposed design; bounded investigation, not implementation acceptance.** Baseline
`439f3d2dd73135cfada43a58e90361a56a96668e`. The
[compiler/analysis decision evidence](../design_review/evidence/2026-10-11_efficiency-design-decisions/compiler-analysis.md)
contains actual input inventory, source contracts, probes and limits. The coordinator continues to
own disposition; all C01–C08 findings remain **Planned**. These choices replace open alternatives in
§2 without widening the authorized execution or qualification scope.

### C01: existing semantic owner; executable composition stays above it

Extend `lctx-model::domain::producer_contract`, which already owns `ProducerContract`,
`SupplierRole`, `CapturedProducerBinding` and `CapturedSourceBinding`. Move the pure facts inventory
currently reached through `cpg-extract::contracts::facts` into that owner. The same table supplies
cold recorded-facts admission and executable declaration binding. Keep concrete constructors,
`CapturedInput` filesystem ownership, native analyzers and build fingerprints in `cpg-extract`;
compiler scheduling remains in `cpg-core`. Do not make model consumers instantiate adapters to
retrieve contracts. A captured descriptor is a value; it grants no filesystem or native access.

D02 owns physical native/compiler separation justified by independent reader/publisher consumers,
and the embedding service trait/future contract with model-level errors. C01 does not create another
provider-contract crate: the existing owner already reaches all relevant typed declarations. Reject
feature-only separation because the fingerprint walker follows declared normal/build dependencies
including optional dependencies. Retain the complete responsible source closure, file membership and
runtime inputs. Extraction fingerprints follow the extraction adapter closure, never orchestration or
serving readers. Compiler-store fingerprints remain conservative until D02 establishes their smaller
responsible physical closure; changing a hash to omit an unresolved dependency is forbidden.

**Migration/control:** model inventory → extractor binding → `facts::recorded_coverage` → publisher,
embedding and cold admission callers. Build actual lower consumers and mutate synthetic source,
manifest, runtime-file and membership inputs to establish the stated dependency/fingerprint boundary.

### C02/PD01: one captured request, whole-provider terminal eligibility

Construct `CapturedRequest` once before executing or looking up a provider. It contains a sorted exact
file manifest (relative names, bytes/digest, additions/deletions, import-readable closure and absent
selected paths), semantic contract/revision/output inventory, supplier roles/build identities,
producer limits/settings, interpreter/platform/environment selection, authored model catalog and
requested availability wherever the provider actually consumes them. Each field is the provider
contract's declared projection, not an implicit whole-compile profile/catalog binding; §4 C05 fixes
those projections per provider. Include ordered upstream exact views or explicitly declared pure-value
inputs. Never hash an absolute temporary capture path as semantic identity.

Add a separate canonical **acquisition descriptor** to the existing captured binding. It contains
origin (installed library/requirement/lock/installer, corpus repository/revision or tree label), every
artifact's role set, distribution ownership/unowned status, distribution name/version/first-party
role/artifact SHA256 set/RECORD verification digest, interpreter/platform and derivations. Corpus
library links name the actual input identity rather than the caller's vector index. Sort sets;
preserve ordered inputs and multiplicity wherever the emission contract distinguishes them. Distinct
acquisitions of identical bytes cannot be silently deduplicated by `InputRevision` alone. Derive both
request identity and emission inputs from this descriptor; no separately maintained digest inventory.

The current `CapturedSourceBinding` is insufficient: it records input/context/configuration, while
Acquire also emits origins, distribution verification, ownership, roles and derivation facts. A new
label or RECORD attribution can change rows without changing those three fields.

Eligibility is a total operation returning `Hit`, `Miss(reason)` or `Corrupt(error)`:

| Candidate | Eligibility |
|---|---|
| Complete with equal full request, exact complete declared inventory and valid admission | Hit after C03 protection/admission |
| Complete with an explicitly empty declared relation | Eligible; retain the empty descriptor |
| Complete compact output with explicit semantic NotRequested, equal exact domain/request and declared whole output uniformly unrequested | Eligible only after the model's exact compact-owner/domain validation and C03 protection/admission |
| NotRequested with any currently requested family, missing domain or mixed/partial scope | Miss; never synthesize complete coverage |
| Partial, Unavailable, Failed, abandoned, poisoned, resource-truncated or unfinished | Miss; no initial partial-reuse policy |
| Equal key but inconsistent bytes/output inventory or admission evidence | Corrupt; do not silently treat corruption as ordinary key ineligibility |

A stage-level NotRequested value does not bypass current `admission::reconcile`, which rejects it.
Compact declaration production completes physically; its model-owned semantic NotRequested proof
is independently reconstructed before reuse. Stage outcome4 alone remains ineligible. Retain
whole-provider grain; independent per-file incremental analyzer sessions are rejected
for this track because import cycles, module lookup and inherited facts require wider correspondence.

**Controls:** real native providers on owned deterministic fixtures, full typed populations/outcomes,
repeat/relocation/reordering/transfer sizes under normal AllThreads, and mutations of role/origin,
RECORD, settings, bytes, import cycles, membership and upstream identity. A fake provider cannot
establish producer determinism. Eligibility itself has independent finite terminal-state cases.

### C03: exact held installation, read-only preflight, then atomic binding

Use N04's `HeldInstalledView` containing logical view/completed identity, exact physical view record
and incarnation, `view_realization_id`/member namespace, member codec, durable installation-completion
identity, selected logical-contribution → physical-record/incarnation mapping and hold owner. Logical
content equality does not identify a physical installation. N04 owns capability construction and
protection; C02 owns request/outcome equality. Check full inventory, recovered closure and receiver
prerequisites without receiver effects, then ask native ownership to attach references atomically.
The capability freezes only the selected completed view/contributor/dependency closure, with
`view_closure_identity`; unrelated producers may continue. N06 final attempt freeze is separate,
so intermediate C04 reads do not wait on final admission. Hold through durable handoff or terminal
rejection/recovery cleanup. Missing completion is a
rebuild/refusal; it is never evidence that a selected member is absent.

Reject a naked digest/row-count capability, replay-copy of all membership pointers, and prior-attempt
admission as a substitute for current candidate admission. Migrate enumeration, backing lookup,
backup/export/import, audit, recovery and retirement to the selected exact mapping together. Native
controls must establish retiring/incarnation races and cross-borrower protection; a memory sink cannot.

### C04: conservative mandatory key demand plus unchanged residuals

At DataFusion **55.1.0**, extract a finite mandatory `id` set from the *actual coerced Expr*:
`id = typed literal` (either operand order), non-negated `id IN (typed literals)`, and conjunctions.
Intersect two finite conjunct demands; keep one when the other is unrecognized. Sort/distinct the set.
Positive equality to NULL has no true matches; discard NULL members of a positive IN list, and retain
an explicit empty set for empty/all-null demand. OR, NOT IN, subqueries, runtime values and arbitrary
column casts produce no finite demand. Do not strip casts generally: recognize exact identity casts
only after the locked expression/type contract proves equivalence, or use the already constant-folded
literal. Unsupported coercions take the ordinary scan route rather than approximating byte keys.

Keep residual predicates and necessary columns. Extraction alone advertises Inexact; a predicate may
remain Exact only where the existing native evaluator fully implements its typed SQL semantics.
Never push limit below residual evaluation. Resolve nominal relation/aliases and validate the held
view even for empty demand or zero-column projection. Multi-get member IDs are
`H(native-view-member/v2, held.view_realization_id, relation, key)`, **not** bare logical-view IDs;
the realization commits exact physical mapping/incarnations and codec. Key-only scans avoid payload
hydration. Dense requests may use one existing sorted run. No new global owner-by-key index is needed.

**Controls:** actual Expr/scalar coercion with reversed equality, duplicate/NULL/empty IN, nested
AND/OR, lossy/failed casts, alias/projection/residual/limit combinations; compare full outputs and
refusals against independent small enumeration. Native incomplete/foreign/virtual/absent cases need
the N04 installation control; finite SQL reference probes establish only candidate-set logic.

### C05: exact by default; separate a pure computation before cutting off provenance

The dependency declaration has one authoritative per-consumer policy:

| Consumer/output | Dependency and profile decision |
|---|---|
| Acquisition | Exact acquisition descriptor/capture settings; ProfileNeutral (no profile-dependent emitted rows) |
| Full Pyrefly native-provider output | Exact semantic context plus behavioral model-context selection; ProfileSpecific because requirements widen retained dependencies and MROs |
| TyFlow and Assembly vocabulary | Behavioral-only requested Flow versus catalog explicit NotRequested; ProfileSpecific |
| Documents and Deployment | ProfileNeutral under the context split below; retain exact declared inputs and supplier attribution |
| Normalized entity/callable/aspect products | CompleteDomain plus canonical typed input values; retain all source/qualification/nominal IDs carried in outputs |
| Structural/Local/Summary/automatic synthesis receipt-bearing outputs | ExactView; invocation, source, proof and definition receipts remain exact |
| CatalogCore and Retrieval computation payloads | RelationValue for explicitly selected relation content/configuration groups; emit current exact source/invocation receipts separately |
| Condition lowering, immutable topology/index preparation | PureValue only for the precisely indexed condition/topology domain and kernel contract; invocation-local |
| Neighbour scalar/window scoring | Pure numeric admitted vectors + ordered selectors/parameters/context; regeneration produces fresh exact windows/uses/invocation receipts |
| Embedding inference winners | Exact encoder + exact fully preprocessed text; document/projection/service binding and use receipts remain separately checked |
| Coverage, availability, frontier/publication and serving metadata | ExactView plus exact domain/request/definitions; never narrowed on equal payload rows |

`AnalysisInvocation.sources` participates in its key; `AnalysisInvocation` plus `InvocationSource`,
`AnalysisInput`, SourceReceipt and validators must all use current exact snapshots. The current
`PortableProduct::value_identity` hashes section bytes: if canonical rows contain old source IDs,
that identity changes with those IDs. Removing request provenance from its hash does **not** turn
source-bearing rows into pure values. Keep these products exact unless an explicit adapter separates
an actually pure result (for example scores keyed by immutable numeric inputs) from receipt emission.
The adapter regenerates current attribution and is independently checked; it never relabels old rows.

Split configuration relations by actual consumers before narrowing keys. The concrete context split is:
`CapturedSemanticContext` contains canonicalized Pyrefly resolution/configuration, interpreter/platform,
exact environment/lock identity and Ruff interpretation settings. `ModelContextSelection` contains the
authored catalog/derived requirement inventory and requested behavioral model/Flow selection. The
former owns the shared AnalysisContext used by source/diagnostic/document/deployment observations;
the latter is a separate exact dependency of Pyrefly's behavioral external-definition/MRO enrichment
and TyFlow. Acquire needs neither analysis-context output: its descriptor already captures its real
acquisition emission inputs. Remove implicit whole-profile/context attachment from its request.

Pyrefly's full contribution remains ProfileSpecific: `requirements` adds imported definition roots and
transitive MRO symbols, so equal parser settings alone cannot establish equal full typed output. We do
not split that provider into two sessions or promise cross-profile whole-provider hits. Documents and
Deployment have no profile branch; after their context source migrates they are ProfileNeutral with
all actual declared input views/settings still exact. Extra Pyrefly inputs may conservatively cause a
miss; a neutral declaration never asserts those inputs equal. Assembly remains ProfileSpecific because
it explicitly emits catalog Flow NotRequested and merges the selected vocabulary.

For downstream normalized kernels, declare neutrality of the algorithm/configuration while retaining
exact CompleteDomain source/qualification IDs. Upper receipt/publication requests keep exact requested
method/domain/configuration and receipts; shared pure payload/preparation and embedding inference can reuse
under their narrower matrix entries, without making receipt-bearing rows profile-neutral. Attempt and
publication profiles remain explicit. This specifies the O-F19 configuration/context remedy and its
consumer migration rather than leaving neutrality to a later design decision. Current native-context
hash includes profile/catalog/Ruff together; merely deleting its profile field is rejected.

**Relation-local values and actual attribution are different dependencies.** A declared value consumer
uses `RelationValue = H(relation-local semantic result contract, outcome, completeness/domain,
ordered OutputContent)`. OutputContent includes every canonical row field/ID for that relation and
explicit zero-row completion. It excludes sibling relations, contributor/build/request identity and
whole producing Spec/configuration lineage. The relation's semantic result contract includes schema,
interpretation, applicable supplier semantics and output validity rules; it is not the whole stage's
output inventory or configuration digest. Represent this as a model-declared PureValue relation token,
with distinct role/selector and its own domain tag. A consumer cannot request it merely because two
observed bodies happen to agree. Changed completeness, nominal/source IDs or this local contract
invalidate it. Persist/derive it only from an admitted complete current relation, not unvalidated bytes.

`ActualSource` separately names the exact selected immutable snapshot and is held through current
invocation/source/proof receipt emission and admission. Cache lookup for a separated computation uses
RelationValue; the emitter's request and validators use current ActualSource. Never bind both as
computation edges, or exact lineage would undo the cutoff. A miss/hit always reconstructs and validates
current receipts. `AnalysisInvocation.sources` remains exact and can therefore change on a compute hit.
This replaces the current `selected_product_dependencies` requirement that every compute dependency
is ExactView, and `completed_product_tokens`' use of provenance identity as the only output token.
Retain authoritative selected-role/epoch and double-binding conflict checks in this split.

**Configuration owners and Spec grain are selected now.** Replace the one `analysis_configuration`
product whose configuration hashes every definition/settings row with five configuration groups:
(1) authored ModelCatalog/Target/Model/ContextProtocol; (2) MethodParameters + AnalysisDefinition;
(3) ProjectionDefinition; (4) AnalyticsConfiguration; (5) RetrievalDefinition. Each group has its own
request over exactly its authored inputs, local implementation/result contract and output inventory.
The method/parameter collection retains one relation writer but assembles per-method subgroup products;
subgroup requests and selection tokens include that method's exact definition/parameter closure, not
sibling methods. Explicit empty groups remain completed. EmbeddingSpec, ServiceConfiguration,
DocumentRecipe and embedding projection are separately selected configuration products within their
existing owner; service endpoint identity belongs to transport/use admission, not encoder inference.

`PreparedCompilation` remains an orchestration container, not a computation identity. A product Spec
contains only the selected group parameters, local code/policy/result contract and its declared
RelationValue/ExactView premises. The schedule/attempt still carries the complete profile/configuration
inventory for final admission. Do not hash the whole Configuration/PreparedCompilation into each Spec.
CatalogCore replaces `configuration_relations()`'s blanket grant with the Catalog method's definition
and parameters plus the authored/projection groups its algorithm actually consumes. Retrieval selects
RetrievalDefinition and the method/parameter groups needed by its mandatory/evidence/synthesis routes;
it does not depend directly on structural seed/budget or optional technique configuration. Collection
lookups may use exact selected method IDs; admission retains the full actual snapshot needed for
references without making unrelated rows computation premises.

Add separated CatalogCore construction and Retrieval rendering payload products to the initial
PureValue subset. They store semantic payloads and typed `(source-role, row-key)` bindings, excluding
invocation/source/proof receipt rows; the current emitter resolves those bindings against held
ActualSource, creates current IDs/links and runs existing closure/admission validators. No stored
receipt row is relabelled. Retrieval's exact text/spec is additionally the embedding inference key.
Other receipt-bearing upper algorithms stay ExactView until similarly split; this is an explicit
initial scope, not a promise that arbitrary output rows are neutral.

**Required change control:** with techniques off, changing only unrelated AnalyticsConfiguration
seed/bounds changes that group's exact product/receipts but leaves CatalogCore and Retrieval's
unaffected configuration RelationValue tokens and computation requests stable. Regenerated exact
invocations can differ and must pass current-source validation. If a selected synthesis/evidence
payload actually changes, its RelationValue changes and Retrieval correctly misses. Repeat with a
sibling method definition, identical relation content under a different contributor/build, explicit
empty/Partial, and changed consumed parameter/source ID to show both cutoff and invalidation.

Validator/model changes invalidate admission/result contracts independently of value equality. Retire
the debug-only DependencyGraph cutoff after migrating real selected-premise/double-binding conflict
checks to authoritative request construction; keep tracing there.

### C06: compact NotRequested is domain-total, explicit and checked at all consumers

Introduce one model-owned compact uniform outcome over an exact admitted domain and exact
method/definition set. It records exact input/context/source/definition identities and NotRequested;
its scoped operation checks domain membership. Do not materialize an invocation per kind merely to
repeat uniform unrequested status. This is a deliberate model/schema cutover, with append-only codes
and migrated validators; no compatibility reader is implied.

| Exact scoped state | Frontier enumeration | FrontierCheck.finish | Publication assessment | Serving availability |
|---|---|---|---|---|
| Requested + Complete, including explicit empty values | Complete | Expected domain matches | Complete | Complete |
| Requested + Partial | Partial with residual | Exact partial scopes | Partial | Partial |
| Unrequested + total NotRequested descriptor, member in exact domain | NotRequested | Accept same exact total domain | NotRequested | NotRequested |
| Requested + NotRequested | Invalid | Refuse | Refuse | Refuse |
| Subject outside descriptor domain; missing descriptor; no-scope/unknown | Unknown/no-scope as declared | Never invent expected coverage | Unknown/no-scope | Unknown/no-scope |
| Changed domain/definitions/request with old total descriptor | Stale | Refuse | Refuse | Refuse |

All four call the model's same scoped operation; consumer-local inference from missing rows is
removed. Mixed domains use ordinary scoped records. Needed unions are constructed once at the
first real consumer from a complete contributor inventory, including zero-row contributions. An
explicit empty demanded union exists and validates its view; no consumer demand creates no union.
Duplicate identical values coalesce; same nominal key/different payload remains conflict.

### C07: retain existing joined native pool; optimize preparation and acknowledgment lifetime

Pyrefly already has a process-wide OnceLock AllThreads pool and joins `transaction.run` before
serial result extraction. Dependency waves currently use the joined inline transaction at
`pyrefly_stage.rs:1587`; migrate their independent bulk work onto that same pool and join before
ordered extraction. Keep it; another pool or a long-lived analyzer process adds no value.
CanonicalSyntax already parses once. Select continued separate Ruff diagnostics/semantic-observe walks over the same CanonicalSyntax parse;
no fused-walk prototype or unresolved fusion choice remains. Index site/name with multiplicity and
unique-site fallback; preserve ambiguous, unlocated, owned/inherited, noqa and deferred-rule populations.
Keep per-root ordered post-processing; the actual-provider probe establishes baseline population
stability, not parity for an unimplemented walk rewrite.

N02's enqueue acknowledgment is `IngressReceipt(sequence, charged_bytes)`, transferring charge lease
to the sink. `await_committed(receipt)`/`finish` yields a NativeWriteReceipt with original request and
terminal/reconciled disposition. Acceptance closes before drain; every accepted sequence retains its
charge through acknowledged write/recovery cleanup, including cancellation/failure. Provider completion
cannot be reported merely because enqueue returned. No extra unbounded queue or worker cap is allowed.
**Captured-tree integrity is a complete bracket, owned by the run and provider closure.**
The capture owner produces the exact manifest while copying/hashing the bytes and retains its private
tree through all joined analyzer reads and ingress cleanup. A `CapturedTreeRunGuard` performs one
complete opening check before the first provider: exact file membership (including missing/extra files,
derived artifacts and import-readable support files), regular/no-symlink paths, sizes and content
hashes against all captured manifests. This moves `cpg-core/facts.rs:24–26`'s leading byte pass into the
capture/run owner and strengthens it to closed-tree membership; it does not claim that pass disappears.
The native resolver may read only this captured closure plus explicitly pinned built-in analyzer
resources in the supplier contract, never ambient environment/project files.

For Pyrefly, Documents, Deployment and behavioral TyFlow, `StageContext::close` performs a complete
closing check of the captured closure after the provider returns with all native dependency waves
joined and before flushing final writers/`sink.finish`, outcome acceptance, contribution completion
or portable product staging. Checking only analyzed-root CanonicalSyntax text or Natives' artifact
index is insufficient: Pyrefly resolves supporting/dependency files from the tree. The complete check
also validates absent membership, so an injected importable file refuses. Assembly reads only admitted
rows/metadata and needs no tree closing pass. Acquire already streams every manifest artifact through
`ArtifactCapture::finish` digest/length checks; remove `acquisition.rs:856`'s redundant post-chunk full
byte pass. Its run-guard opening membership check and per-artifact emission check remain mandatory.
No complete-byte check is replaced by a path/use index or source-root-only hash.

Any mismatch, missing/extra file, changed bytes or forbidden path poisons the current contribution and
overall attempt. Accepted ingress still drains/reconciles under N02; no completed reusable view/product
is emitted for that provider. Earlier exact contributions remain protected staging/recovery objects,
not admitted output of the failed attempt. Cancellation cannot skip closing verification and then
report Complete; error/cancel instead follows terminal refusal/drain.

This contract assumes ordinary exclusive owner control: no supported writer mutates the captured tree
between checks, and all consumers share the retained lease. It detects persistent accidental/concurrent
mutation and refuses it; it does not claim filesystem permissions are immutable or defend against a
hostile writer that changes and restores bytes between checks. A verified VFS/read adapter is rejected
for this increment because native analyzers resolve their own trees; it is the explicit future route
if that stronger threat model is required. Preserve original captured bytes/origin/derivation identity;
fused capture hashing covers the actual copied stream, not a second read of the source checkout.

**Controls:** mutate an unanalyzed import dependency, an unused captured support file, a selected
root, add/remove an importable file, inject a symlink, and mutate after a native transaction but before
close; each refuses before completion and drains accepted writes. An unrelated source-checkout dirty
file must not affect the captured population. These bracket controls remain future implementation work.

### C08: Postcard envelope, unchanged canonical section bodies and existing lifecycle

Choose the already locked **postcard =1.1.3** (`alloc`, no default heapless feature) for a versioned
opaque binary envelope over the existing ProductRequest/outcome/ordered section inventory. Declare
it directly at the cache codec owner; this adds no version upgrade. Keep each section's exact canonical
typed-body bytes unchanged. `Vec<u8>` encodes length plus bytes, avoiding JSON's decimal integer arrays.
Use a format-2 borrowed frame: fixed format/outcome, canonical encoded request bytes, exact section
count, then ordered `(name, rows, body-bytes)` sections. Decode the frame into borrowed slices with
`take_from_bytes`; do not deserialize an untrusted owned PortableProduct first. Preflight entry byte
length against the actual retained buffer and configured capacity, compare canonical request bytes
against the current trusted lookup request, and require section count/names/order equal that request's
exact declared outputs before any variable owned allocation. Each declared name/body length must fit
the remaining buffer, checked total section bytes/count arithmetic and per-section transfer/row limits;
malformed count/length/truncation/trailing data is rejected while still borrowing input. These checks
must precede Serde Vec/String creation, including malicious count hints.

Admit a separate decoded reservation for `request.retained_bytes` plus section containers/names and
the sum of copied body lengths (including capacities) before cloning the current trusted request or
owning sections; the charged input buffer does not cover those clones. Prefer borrowed section slices
through checked typed ingress to avoid that copy entirely. Existing typed-body decoding has its own
preallocation bound; schema/IDs/order/conflicts and exact row count remain checked before one canonical
batch is admitted. Reject noncanonical encoded metadata as well as malformed framing. Envelope decode
never establishes semantic admission. Deliberately retire format-1 project entries at the coordinated
cutover; no compatibility reader is selected.

Reject bincode (not currently locked), JSON base64 (4/3 expansion and outer JSON parsing), RawValue
(JSON-only nested payload coupling), and an Arrow IPC section conversion (changes canonical section
bytes/schema authority and adds another codec migration). Postcard is a physical envelope, not a new
semantic value-identity algorithm. Its Serde/schema constraints are checked on these concrete types.

Keep cache enabled by default and optional misses recomputable. Batch lookup/staging/acknowledgment
for bounded charged root windows; preserve per-entry leases, exact generation, readback, corrupt-entry
policy and cleanup. Quota `used` under fenced transactions is admission authority; replace ordinary
SUM planning scans with that counter and a `(partition, created, key)` ordered eviction access path.
Eviction skips held entries and resumes by stable key, not mutable offset; reconcile counters in explicit
recovery, not every insertion. Never delete warm retained generations merely because implementation
identity changed. Native quota/lease races and actual cache readback remain future integration controls.
