# Evidence/retrieval and primary evaluation — assembled implementation review

**Baseline decision: Revise, 2026-10-06.** The assembled implementation has useful ownership
boundaries: source/catalog facts govern applicability, full embedding values are separate from
consumer projections, serving owns bounded pinned delivery, and an offline evaluator supplies
independent contextual witnesses and frozen comparisons. The failures below occur at their seams.
Part coordinates are interpreted in the wrong representation, context membership is inferred from
a global role, native limits precede distinct-target aggregation, and evaluator compatibility
confuses evidence provenance with task context. Two other gaps concern readable qualification and
the independently checked final delivery map. These are concrete supported scenarios, not style
findings or a demand for additional accounting, proof machinery or a compatibility runtime.

This is the completed dated assessment of `24a5968ce9810ac705fadbf62830bcb4d4c34975`. Integration is
continuing; a focused follow-up will inspect the exact correction diff and record resolution or
remaining limits. This baseline judgment is not a verdict on uninspected later edits. The final
status read observed later HEAD `e7581be521668bd376934edad7e790368995f852`; it is not this review's
production baseline.

## 1. Scope and evidence

| Item | Boundary |
|---|---|
| Reviewer | Independent delegated design reviewer, 2026-10-06 |
| Standard | Core/template 3.3, heuristics 1.0, code-intelligence profile/review additions 1.5, library-context binding; shared worker/design-reviewer contracts and both review skills |
| Tier / purpose | Design / target, assembled ER1–ER4 and EV1–EV4 with adjacent native/value/transport consumers |
| Functional target | Find built-in library capabilities and correct invocation/configuration/source/example/deployment information; retain compatible qualification and actual readable delivery; improve system and evaluator through a primary independent programmatic loop and grounded outer feedback |
| Current owners | [Combined coordinator](../../plans/evidence-retrieval-and-evaluation-plan_2026-10-06.md), [evaluation plan](../../plans/programmatic-evaluation-plan_2026-10-06.md), ER extensions in the four graph-native supporting plans, product §14.8–§14.12, synthesis/serving §11, semantic model §15 and validation/evaluation §12 |
| Workload | One local operator, immutable pinned releases; long multi-section documents and many windows for one member, skewed source/relation degree, concurrent native reads, bounded optional inference; finite offline development cases and stored-vector references |
| Method | Static inspection of declarations, construction/admission, embedding identities/values/projections, publisher lowering/reconciliation/restore, native nomination/rescoring, readable closure, delivery/continuation, independent decoder/kernel, experiment/feedback and runner |
| Limits | No Cargo, runtime probes, gold/heldout/confirmation reads, service activation, operator changes, paid trials, legacy parity or performance measurement by this reviewer. Broader graph-audit F01–F12 remain with their existing coordinator. Source existence is Implemented; architectural remedies below are Proposed until inspected. |

The nominated source review's F01–F03 retain their source-qualified identities and sole disposition
owner. The findings in this document are new implementation-review evidence; they do not renumber
or automatically close that review, the graph audit or earlier analytical/product findings.

## 2. Responsibilities and domain fidelity

| Owner | Meaning and contract | Consumers / legitimate change |
|---|---|---|
| Source, catalog and document owners | Defining source differs from public access; qualified options/scenarios/document associations and captured spans have nominal identities | Retrieval construction; new declaration/document evidence belongs here |
| Model retrieval | Unit/origin, Primary/Context parts, source maps, whole-part window membership and qualified target binding | Compiler, admission, publisher and serving; new interpretation dependency requires owned semantics (F02) |
| Model embedding | Actual encoder/input identity, immutable full winner, declared normalized projection and exact consumer references | Cache, E0/E1, transport/native restore; query policy can change without document inference |
| Native publisher | Stream admitted windows/bindings, shared projection cohorts and occurrence lineage; reconcile before sealed publication | Native discovery/restore; physical indexing changes cannot invent applicability |
| Serving | Nominate eligible contexts, hydrate readable closure, preserve mandatory core, describe actual fields/omissions and retain immutable ranked order | Rust/PyO3/MCP; rendering must preserve coordinate/status meaning |
| Private evaluator | Independent readable predicates, compatible All/Any/Exists witnesses, finite-world determinacy and explicit unscorable outcomes | Primary runner, sensitivity controls and grounded feedback; it must not inherit producer truth |
| Experiment | Freeze actual task/request/oracle/kernel/observation meanings and realization settings, route feedback and rejudge exact retained packets | Development optimization; changed meaning starts a new baseline |

Dependency direction is appropriate: production does not import private expected-answer contracts.
`lctx-eval` has its own finite meanings and no serving/native dependency. Python projects public
calls through production wire admission and communicates with one bounded JSONL evaluator worker.

| Fact family | Fidelity, identity and uncertainty | Relevant consumer |
|---|---|---|
| Source/declaration/options | Provider-attributed extracted/resolved alternatives; source default differs from effective runtime override | Construction and readable closure |
| Parts/windows/bindings | Derived, nominal unit/part/window/binding identities; Context does not nominate a member; exact/candidate bases remain encoded | Search lowering and delivery |
| Full/projection values | Numerical heuristic representation, exact winning bytes plus encoder/input and projection-source identity; NotRequested/refused remain explicit | E0/E1, rescore, replay without live inference |
| Served original/condition/default | Actual bytes/readable literals and setup under release/analysis/signature/variant; opaque operands are not meaningful delivery | Consumer and independent observer (F01/F05) |
| Private witness/world/result | Independently authored accepted meanings and complete finite domain; unsupported/incomplete/inconsistent/budget/operation outcomes separate | Primary comparison, never production input |

## 3. Contracts, composition and execution

Inspected strengths constrain the remedies. `embedding/spec.rs::Spec::hash` separates actual
encoder identity from query/document recipes. `FullValue` keys encoder/input rather than digest;
competing bytes cannot legally create different winners for one input. `ProjectedValue::verify`
uses the selected F64-norm/F32-output policy, and `ValueIndex` retains compact checked identities
for later use admission. Publisher search lowering batches companion reads, shares projection
arrays by library-input/family cohort and preserves exact binding lineage instead of rebuilding
the old fragment × member × anchor product. Backup transports canonical content and originals;
restore rebuilds derived search rather than requiring live inference or the mutable cache.

`ranked_results.rs` retains bounded serialized rows and final metadata in a session-owned immutable
entry. Resume uses retained channels/order, validates cursor bindings and rejects foreign sessions,
expiry, eviction and a changed supplied query value. It does not rerun discovery. Delivery protects
core/signatures/interpretation and checks actual MCP renderer size while pruning optional bundles
or the ranked tail; transport still owns final envelope admission. These are credible mechanisms
for pinned multi-call reads. This review establishes their static route, not runtime qualification.

The finite witness kernel naturally joins compatible assignments and projects Exists only after
the complete conjunction. Its exhaustive reference enumerates proof choices independently. World
judgment is satisfiable-first and reports inconsistent/incomplete/unsupported cases separately.
Experiment freezing binds actual task inventory and kernel source; feedback requires an independent
basis and new evaluator revision where meaning changes. Numerical reference reports its supplied
finite population and caller-provided nomination basis honestly; it does not claim native ANN
readiness or global library completeness.

| Stage / question | Universe and method | Output and limits |
|---|---|---|
| Construct searchable evidence | Captured root/context, primary/context parts, semantic whole-part partition and local exact tokenizer | Nominal windows/maps/bindings; oversized lexical-only; F01/F02 prevent faithful delivery/scaling |
| Nominate useful contexts | Native exact fields, match-zero lexical and declared1024 ANN; compatible occurrence filters | Heuristic channel witnesses and bounded widening; F03 leaves a pre-group cap |
| Rescore | Retained nominated occurrence union, shared full4096 values, F64 scalar cosine | Orders known eligible candidates; cannot recover missing nomination |
| Deliver | Selected whole context bundles and final renderer envelope | Readable fields plus omission/expansion map; F05/F06 require correction |
| Judge | Exact final bytes, independent predicates and compatible finite assignments/worlds | Scoped sufficient/insufficient or explicit unscorable result; F04 affects legitimate composite witnesses |

## 4. Findings and correction boundaries

Current scheduled disposition should be linked from the [combined coordinator §6](../../plans/evidence-retrieval-and-evaluation-plan_2026-10-06.md#6-finding-disposition-investigation-outcomes-and-completion).
This document owns dated diagnoses, not a second live execution ledger.

<a id="F01"></a>
### F01 — Serving confuses part-local, window and encoder-input coordinates

**High; FP-02/04, DP-02/08, CI-11; A2 and G2/G6/CI-G2 violated.**
`retrieval/partition.rs:55` constructs `WindowPart { start:0,end:part.text.len() }`.
`retrieval/construction.rs::verify` enforces those part-local whole-part intervals. Its source-map
validation independently uses complete encoder-input coordinates, adding the renderer prefix and
preceding part lengths/separators. `lctx-serving/src/evidence.rs:239` instead slices `window.text`
with the part interval; lines 245–262 intersect complete-input maps with that same interval.
For context then a different primary, the primary repeats the window prefix. A nonempty template
also shifts source maps for the first part. Valid compiler content can therefore return incorrect
text or lose correct source maps without changing the nominal binding.

**Proposed correction:** serving consumes the authoritative part-local text and composes maps
through a model-owned coordinate operation, or hydrates canonical PartSourceMaps. Keep renderer
prefix/separators separate and produce wire-relative map offsets only after translation. Do not
change an existing interval's meaning in just one consumer. Closure requires inspection and a
meaningful final search delivery case with Unicode prefix/context and a distinct second primary.

<a id="F02"></a>
### F02 — Context role does not model which primary needs that context

**High; FP-04/07, DP-02/08/10, CI-05/08; A2/A4 violated.**
`retrieval/construction.rs:187` marks all whole-document headings as Context. Partition lines
25–46 collect every Context part and render all of them into every window; admission requires
all context parts in every window. A page's section A therefore includes unrelated B/C headings
before its body. Growing sibling sections changes A's input and can force every otherwise-small
primary over2048 tokens into lexical-only availability. It also multiplies unrelated context
bytes by window count. Safe refusal is honest but cannot establish fit for ordinary long documents.

**Proposed correction:** own per-primary interpretation dependency membership, for example a
small typed Primary→Context part relation. Use existing document passage level/heading/path and
captured spans/components for relevant enclosing heading/table context; source primaries retain
declaration/enclosing-control context; scenario primaries retain admitted setup and qualification.
Partition unions only dependencies of selected primaries and preserves their original order.
Validate that selected closure rather than a global all-context condition. Sharing context parts
does not require copying bodies or introducing another service. Setup-only grains retain explicit
navigation meaning. An unrelated sibling addition must preserve A's window/admission, while
removing a real enclosing heading/setup dependency must refuse. Narrowing scenario setup without
an independent authority would be an invalid remedy; conservative genuinely shared setup remains.

<a id="F03"></a>
### F03 — Native limits still precede distinct contextual target aggregation

**High; FP-07, DP-08/10, CI-08; A4 and G6 violated.**
`lctx-serving/src/search.rs:70–71` and `166–167` group by target,context,**in** before
`LIMIT1024`. This retains one candidate per document/vector per target. Distinct-target counting
happens after that cut. With1024 high-scoring windows for A, a qualifying exact-path B at zero
BM25 can be discarded even though it was admitted by the exact-field predicate; widening cannot
recover it at the same ceiling. Rust family fusion cannot restore an already discarded witness.

**Proposed correction:** choose the best contribution for target/context/family/channel before
the final target quota, retaining its exact window/binding witness; preserve independently admitted
exact identifier priority. Raw ANN/document tiers may remain declared approximate discovery bounds,
but are different from a final contextual target quota. Closure is the actual native many-A-windows
plus exact/zero-score B case, including exhausted tier behavior and retained lineage.

<a id="F04"></a>
### F04 — Evaluator joins evidence-instance provenance as semantic context

**High; FP-04, DP-02/05/08, CI-12; A2, G2/G6 violated.**
`lctx-eval/src/mcp_observer.rs:197–214` inserts window/part identity and artifact/start/end into
shared Assignments. The get_evidence decoder similarly inserts source coordinates. In
`witness.rs::compatible`, every shared key must agree. Two independently valid source fragments
from different parts/ranges in the same release/analysis/member cannot satisfy an All witness,
even when their semantic contexts are compatible. The evaluator turns evidence location into a
false incompatibility and can guide optimization away from legitimately sufficient packets.

**Proposed correction:** keep evidence-instance identity and source coordinates in provenance/
anchor fields with independent integrity checks; join only meaningful task-context variables.
Where a task needs the same formal/field across roles, that shared variable must remain explicit;
removing all binding distinctions would permit false default/name combinations. Use independently
authored two-span positive and foreign-release/analysis/variant negative cases. Multi-parameter
tasks need deliberately scoped variables rather than globally equating every parameter identity.

<a id="F05"></a>
### F05 — Class predicates label an opaque operand as readable meaning

**High; FP-04, DP-02/22, CI-04/11; A2, G2/G7 and CI-G1/2 violated.**
`lctx-serving/src/defaults.rs:126–127` formats IsInstance/TypeIs class_expression as a readable
string. The model operand is not captured readable class-expression evidence. read_originals
marks an atom Available when that nonempty predicate and the tested subject source are present.
The independent qualification decoder accepts the string verbatim. A condition can consequently
claim complete interpretation while the class operand remains opaque.

**Proposed correction:** retain explicitly unreadable/partial qualification until the class
operand has an admitted readable interpretation and its applicable binding. Do not merely hide
the subject source or guess class spelling from a nominal/string key. A condition with source
subject and missing operand must not be Available or satisfy immediate readable qualification.

<a id="F06"></a>
### F06 — Final delivery map is ignored rather than independently checked

**Medium; DP-03/08/23, CI-11/12; G3/G6 violated for map-conformance claim.**
The current MCP decoder derives actual information independently, which is valuable. It does
not validate `structuredContent.delivery`. Its search test
`actual_source_maps_and_context_roles_are_independently_decoded` deliberately injects a false
delivery-field binding and still decodes successfully. Window source-map checks cover a different
contract. The target requires a schema-valid false final PacketEvidenceMap to fail; ignored map
metadata can misdescribe delivered field context/dependencies/expansion while witness text remains.

**Proposed correction:** independently check map pointers, binding/role/source associations,
dependencies and omission/expansion claims against exact final fields. Keep this result separate
from evidence truth; producer map labels must never authorize their own sufficiency. Valid map
plus independently sufficient alternate should pass; corrupt schema-valid binding/pointer/omission
should report observation-conformance failure without fabricating task insufficiency or source truth.

## 5. Scenarios, alternatives and library fit

| Revealing change | Result on inspected baseline |
|---|---|
| Add unrelated sibling section to a long document | Rewrites every window's context/input and admission, F02; exceeds semantic dependency propagation |
| Add configuration-precedence evidence assembled from two source fragments | Semantic source owns new information, but delivery coordinates and evaluator assignment join defeat valid composition, F01/F04 |
| Add many windows for one member | Shared values/cohorts are useful; per-document pre-group cap can suppress another target, F03 |
| Change only query instructions | Encoder/document keys remain stable; query/ranking/cursor identities change, a sound local contract |
| Replace ANN nomination with exact eligible-set scoring | Applicability/bindings and independent oracle need not change; numerical/channel policy changes explicitly. Actual planner behavior remains runtime evidence, not a static performance claim |
| Restart/evict session during pagination | Explicit continuation-unavailable; retained order is not recomputed, sound inspected mechanism |
| Revise evaluator meaning after grounded outer feedback | New evaluator revision/baseline and same-packet rejudgment route exist; private truth remains offline |

The adopted tokenizers adapter avoids endpoint round trips while preserving Unicode offsets and
complete-input accounting. The native query engine owns scalar filters/search/grouping; fixing
F03 there preserves bulk execution rather than moving the whole index into Rust. Standard finite
sets/joins and the independent exhaustive kernel are proportionate to the declared finite lane;
no solver, generative judge or universal workflow framework is needed. The bounded session cache
has a concrete pagination consumer. Full-value/projection and source-map domain transformations
remain specialized owned code; generic library adoption cannot decide their semantic contracts.

The simplest conforming correction preserves these foundations and repairs their contracts.
Increasing token/candidate limits or ignoring semantic metadata would mask the causes. Retaining
old runtime generations or dual readers would add lifecycle burden without a named consumer.
Advanced solver/reranker/index replacements remain conditional on a concrete loss or resource
problem, rather than prerequisites for fixing present source/delivery/evaluator semantics.

## 6. Gates, architectural judgments and completion boundary

| Gate | Baseline verdict | Scope evidence |
|---|---|---|
| G1 Authority | pass, static | Source/retrieval/value/native/private-evaluator responsibilities are distinct; findings concern missing distinction or wrong lowering, not competing writable stores |
| G2 Semantic fidelity | fail | Coordinates, context dependency and observer compatibility/readability, F01/F02/F04/F05 |
| G3 Validity | fail for delivered-map conformance | False schema-valid final map is ignored, F06; canonical shared admission otherwise has inspected enforcement |
| G4 Hidden behavior | pass, static | Explicit acquisition/cache/inference/publication effects; private task projections reject production-unknown fields |
| G5 Consistency/recovery | pass for inspected pin/continuation route | Immutable snapshot/session result, refusal on incompatible resume; no runtime qualification inferred |
| G6 Transformation/reuse | fail | F01/F03/F04/F06; sound split embedding identities do not offset these failures |
| G7 Truthful capability claims | fail for complete readable class qualification | F05; numerical/finite/scope limits otherwise explicit |
| G8 Library leverage | pass, static scoped comparison | Adopted tokenizer/native search and ordinary finite library containers; no unjustified generic replacement required |
| CI-G1 Fidelity | fail for class-readability claim | F05; heuristic ranking remains separate from structural evidence |
| CI-G2 Evidence closure | fail for actual mapped delivery | F01/F05; pinned identities alone do not establish delivered readable meaning |
| CI-G3 Evaluation integrity | pass for private isolation/frozen comparisons, static | No protected inputs inspected; oracle separated from production, meaning revisions explicit. This does not certify observer adequacy (F04/F06) |

FP-01/03/06 are satisfied for the inspected ownership/composition/local-kernel scenarios;
FP-02/04/05 are violated at coordinate/context/observer contracts; FP-07 is violated by unrelated
context amplification and premature target cuts. Applicable DP/CI groups follow the findings and
the independent gate evidence above, rather than an aggregate score.

| Judgment | Baseline verdict | Required next action |
|---|---|---|
| A1 Localize change | satisfied for declared owner boundaries | Keep corrections with retrieval, native search, serving and private observer owners |
| A2 Encode domain meaning | violated | Establish dependency membership/coordinate representation, semantic versus provenance variables and honest readability |
| A3 Extend through composition | satisfied for owned primitives; failing transformations assessed independently | Preserve independently testable model/value/evaluator and native/transport composition |
| A4 Fit execution | violated for long documents and skewed window counts | Relevant context closure and target aggregation before quota; no measurement campaign or accounting system needed |

**Rule impacts: none.** The remedies implement the functional meanings already requested; they
require no change to programmatic primacy, protected populations, hard current-only cutover,
native-store selection, synthesis policy or targeted acceptance rules. Any new semantic dependency
declaration is implemented through the existing model/graph/transport owner, not a new process gate.

**Checks:** static `rg`, `sed`, `cat`, `git status --short`, `git rev-parse HEAD` and source inspection
completed on the named baseline. Cargo/family controls, actual native/MCP journeys, live inference,
gold/heldout/confirmation, operator adoption and quantitative measurements are **not_run** by this
reviewer. Root owns targeted acceptance on the integrated source/native tree. Existing historical
receipts remain attributed, and no stopped broad suite is required by this review.

**Bounded decision: Revise. Enclosing architecture:** the assembled ER/EV scope needs the named
corrections before architectural acceptance for these supported scenarios. Other graph-audit,
real-library usefulness, live checkpoint quality and release qualification remain with their owners.
The next step is coordinator integration of the material corrections, then independent inspection
of the exact final diff and root's chosen functional evidence. Consequence priority is high for
wrong delivered meaning and false witness judgments; dependency membership precedes partition/
delivery refinements even when its immediate consequence is reduced search availability.
