# Native execution efficiency and ownership — design review

**2026-10-09 · design / target · independent review · decision: Revise**

The native execution architecture needs revision for execution fit. The current tree has substantive ownership, batching, exact-view and admission strengths, but bounded requests still expand into avoidable native crossings, owner/key combinations, repeated query construction and rich decoding. These are structural FP-07/A4 defects. They do not establish the cause or relative contribution of the observed timeouts, and they do not establish semantic corruption.

The recommended target retains immutable completed inputs, typed semantic authority, independent admission and coherent native publication. It shares the physical preparation and typed representations needed by compatible operations, uses native access paths that follow actual membership, and narrows coordination to the work whose completion is relevant. Integrating the existing domain execution declarations can remove reconstructed dependency decisions. Turning all lifecycle, cache and database operations into one universal execution graph is not justified by the inspected consumers.

## 1. Scope, evidence and qualification boundary

| Field | Reviewed boundary |
|---|---|
| Subject | `/home/paul/library-context`, HEAD `85e8b9cb`, with the existing substantial dirty GK/GR/native/compiler/serving baseline. Source inspection describes that mixed tree on 2026-10-09. |
| Standard | Core principles/template 3.3, Efficient Architecture Heuristics 1.0, code-intelligence principles/review 1.5 and repository binding, loaded through `standard.toml`. |
| Reviewer | Fresh independent `design-reviewer`, supported by a `code-mapper` and `library-research`; root reconciled decisive access, assurance, lifecycle and receipt evidence before publication. Helper messages supplied leads, not substituted judgments. |
| Purpose | Design/target review of the composed native execution route, not conformance to current ADRs. Existing plans and accepted decisions describe constraints and migration work, not acceptance criteria for a better target. |
| Functional scope | Catalog and explicitly selected behavioral compilation; provider ingress; completed-input selection and normalization; necessary support/invariant admission; selected product reuse; artifact admission/export/import; adjacent publication, serving preparation and fixture observation. |
| Workload premise | A pinned library can contain many facts, producer owners and semantic kinds; roots can share support and have skewed/high-degree neighborhoods. Cold and warm execution, empty/deleted input, independent concurrent reads, cancellation and retirement must retain complete/refused/failed distinctions. Ordinary output bounds do not bound examined membership or intermediate work. |
| Exclusions | No operator database inspection/activation, real-library pilot, protected evaluation, runtime tuning, new accounting framework or migration implementation. No builds, tests, probes, formatting or repository edits were performed by this reviewer. |
| Owners read | Semantic-model §15, especially §15.10/15.11; acquisition/extraction; storage/publication; validation/evaluation; API/evidence product; GK/GR and persisted coordinator. Adjacent source boundaries and locked dependency contracts are assessed below. |
| Coverage limits | The review does not certify every analyzer relation, behavioral kernel, generated schema, published evidence journey or backup format. Their preserved contracts constrain the corrections. Lack of examination is not itself a finding. |

The existing `just qualify` receipt at `build/runs/20261009T170649.056Z-032c30` was canceled after about 21 minutes, exit 143. It records model 798 passed, analytics 14 passed and flow passed; extraction 45 passed, one authentication failure and 21 timeouts at 300 seconds; core 147 passed and two timeouts at 300 seconds before cancellation caused 32 SIGTERM failures and 289 not_run. SIGTERM failures are not independently demonstrated assertion defects. The cache matrices were still in their **first cache-off attempts**: logged catalog relation/aspect phases were about 121/50 seconds and behavioral relations about 200 seconds. Tiny logged CPU regions are not total CPU consumption. A separate run, `20261009T163757.029Z-9b0811`, had two normalized-artifact timeouts at 900 seconds inside invariant admission. These receipts establish incomplete/failed qualification, not an attribution to one mechanism.

The dirty SDK stream patch in `lctx-surrealdb/src/prepared.rs` retains the existing authenticated client's `Arc` rather than constructing an owned SDK client through `Query::into_owned`. It is **Implemented, uncompiled and unverified** in this review baseline. Locked SurrealDB 3.3.0 source establishes the mechanism: `query.rs:79–85` converts the borrowed client to owned; `lib.rs:337–349` creates a new session identity and enqueues a clone; `engine/remote/grpc.rs:535–542` attaches/replays the session, `:548–578` can replay sign-in, and `:1025–1059` serially handles session lifecycle events. Retaining an `Arc` of the existing session has a credible ownership rationale. This does not prove the observed server authentication failure was caused by session replay, prove the patch's terminal/lifetime behavior, or establish that all slow paths are repaired.

All corrective routes below are **Proposed**. Current mechanisms are **Implemented / source-inspected, 2026-10-09**; their structural assessment is static. Historical receipts keep their original scope; this review adds no Tested or Measured performance claim.

## 2. Responsibilities, semantic ownership and fidelity

| Component | Owned responsibility and hidden decisions | Consumer contract / allowed direction | Expected reason for change |
|---|---|---|---|
| `lctx-model::domain` | Nominal fact/role/coverage identities, scope programs, stage inputs/outputs, invariants and analysis semantics | Mechanisms lower declarations; callers do not recreate meaning from table layout | New domain concept, invariant or declared method |
| Provider adapters / acquisition | Explicit pinned source/configuration, independent provider meanings, bounded typed ingress | Provider-private representations stay within adapters; completed contributions carry attribution | Provider upgrade or new supported fact |
| `cpg-core` workspace and compilation | Attempt ownership, exact completed input bindings, stage invocation, semantic admission and outcome composition | Core → native mechanism → model; effects stay private until admitted publication | New composition/frontier or lifecycle requirement |
| `consumed_rows` / normalization | Declared selected domains and partitioned kernels over exact inputs | Logical root partitions may share physical discovery/hydration without changing ownership | New scope/normalization semantics or selected access mechanism |
| `lctx-surrealdb` | Native envelopes/indexes, immutable memberships, streamed terminality and acknowledged effects | No unrestricted compiler admin escape; no native mechanism invents semantic validity | Native query/transport/finality substitution |
| Product reuse | Optional portable results keyed on complete selected dependencies, validated under the current owner | Cached bytes carry no ingress capability; misses preserve ordinary computation | Product contract/settings or physical reuse strategy |
| Publisher / serving | Private admitted authority → coherent publication; readers pin one complete realization and executable definitions | Publication differs from selection; serving is derived, pinned and rebuildable | Published physical realization or evidence interface |
| Validation / evaluation | Independent semantic controls; actual row admission; private evaluation truth | Necessary admission is distinct from producer diagnostic replay and gold expectations | Invariant/evaluator meaning change |

Representation flow is typed provider batch → private native contribution → immutable completed view → compact selected projection / charged typed kernel input → fresh private output → independent frozen admission → native seal → pinned serving. The **semantic fact graph**, the **producer/stage dependency graph**, and a **selected analytical graph** have different universes and edge meanings. They must not become interchangeable authorities.

| Fact / relation family | Provider and revision authority | Fidelity | Coverage / unknowns | Identity and consumers |
|---|---|---|---|---|
| Syntax/lexical/source correspondence | Independently pinned latest Ruff; acquisition and captured run context | Extracted canonical syntax | Unsupported/failed/not-requested stays explicit; provider-local parsing is not another canonical source | Model nominal records and source/role correspondence; normalization, catalog evidence |
| Typing and declared runtime-flow views | Pinned Pyrefly/embedded Ruff and independent ty families | Provider typing / declared runtime view; no inference-to-runtime relabeling | Disagreement and unresolved targets retained | Exact producer/model/profile/configuration/source binding; selected normalization and behavioral enrichment |
| Receiver/event/binding/entity support | Model-owned support scope and normalization operations | Resolved or derived under declared model | Explicit missing physical root versus virtual root and empty support; negative domains are not dropped | Exact completed view and nominal role; validators and products |
| Behavioral summaries | Selected behavioral profile and finite model | Exact/conservative under stated abstraction, not unconditional runtime truth | Five nominal verdicts and coverage; unavailable is not absent | Model/settings/dependency identity; explicitly requested enrichment |
| Analytics and programmatic assertions | Named projection/method/settings and synthesis consumer | Derived or heuristic as declared | Partial/refused outcomes retain status; heuristic cannot assert behavior as fact | Projection/source/run linkage; ranking or attributed findings |
| Served API/evidence answers | Rust-owned contracts over one pinned native realization | Preserve the stored fact/result fidelity and evidence status | Missing evidence is reported, not synthesized | Snapshot/content/executable identities include returned evidence references and continuations |

The domain model is adequate for this bounded correction: scope, exact views, stage dependency visibility, coverage and proof ownership are explicit. The reviewed issue is their physical realization, not absence of a shared semantic model. `domain/stages.rs:164–210,768–825` owns stage declarations and dependencies, including invariant inputs and epoch visibility. Model support admission uses `OwnershipRows` and `OwnershipSet` (`domain/admission_scope_program.rs:30–58,293–350`), rather than validators rediscovering ownership from arbitrary SQL.

## 3. Contracts and enforcement boundaries

| Operation | Preconditions / meaningful outcomes | Effects, lifetime and enforcement | Substitution / isolated control |
|---|---|---|---|
| Complete/select an input | Exact producer group and immutable view; complete-empty, not-requested and missing physical rows remain distinct | Pending rows cannot widen selected universes; workspace remembers exact premises, not a global validity bit | Typed selected-value controls plus native membership controls |
| Discover selected support | Declared root/namespace/role and exact dependency views | Compact union discovery may be shared, but each logical root retains its partition and absence-sensitive dependencies | Independent known-answer selected scopes, overlapping roots and absent targets |
| Admit support/invariants | Actual physical rows under current profile/model and full required negative domains | Independent check objects reject invalid rows; ownership scans may include Full inputs | Shared validator implementation, independent adversarial rows; no producer expected-row oracle |
| Reuse a product | Complete dependency/configuration key, canonical portable encoding and current semantic acceptance | Reject before ingress; install into a fresh contribution; resource/uncertainty failure cannot silently turn into recompute after effects | Canonically valid but semantically wrong cached rows are revealing controls |
| Freeze/publish/restore | Actual frozen descriptor agreement and independent semantic/frontier/outcome checks | Content closes irreversibly; acknowledged identity survives cancellation; standalone transport re-admits independently | Native tamper/finality/restore controls, not cache-hit or producer receipts |
| Stream/read/retire | Bound owner/viewer and terminal-success protocol | Rows provisional; readers/writes remain charged until actual terminality; final global closure drains owned descendants | Early cancellation, late error, retained reader and retired realization controls |

These are necessary distinct assurance boundaries. They need not entail re-reading and re-decoding the same immutable support once per root or rebuilding the same query catalog once per grain. `workspace.rs:1065–1083,1482–1530` already skips checks only when the **exact invariant/model/profile/input-view premises** match; `:1703–1811` separately memoizes exact reference constraints. These strengths constrain F02: replace repeated physical work without broadening semantic validity or trusting a producer certificate.

The final-content boundary also remains substantive. `workspace.rs:468–542` fences completion and mutations, freezes content and compares actual owners to expected bindings. `artifact.rs:964–992` independently verifies restored inventory/contracts, semantics, frontier and outcomes. Export's original-chunk positional/count/byte checks (`artifact.rs:266–405`) have an independent integrity purpose. They are not removable merely because other identities are hashed.

## 4. Composition, physical execution and graph integration

Several earlier sharing corrections are present and useful. `consumed_rows.rs:608–821` prepares compact dependency projections once per exact input scope; `:1310–1476` groups compatible roots, batches root presence and discovers their union before propagating distinct owner partitions. `normalize/mod.rs:554–615` prepares once across four entity root families; `:912–980` hydrates the callable miss union once per group. Native providers already expose projection/pushdown rather than forcing every kernel through a rich graph. These mechanisms should be extended selectively, not replaced with whole-graph residency.

| Stage / question | Projection and method | Exactness / limits / result linkage | Physical assessment |
|---|---|---|---|
| Selected root support | Declared namespaces, edge direction/roles, exact views; union frontier discovery then per-root membership | Exact under model; duplicate logical roots and absent physical roots retained; charged compact state | Sound sharing already exists; owner/key access and product token queries remain separate amplification points |
| Normalize entity/receiver/event/binding | Model-owned scope and typed kernels over selected support | Exact model semantics; independent candidates/stored-row roots protect negative evidence; fresh typed outputs | Preserve partitioned behavior; amortize preparation and input decoding where compatible |
| Support/invariant admission | Actual selected/full ownership inputs and stable ordering; model validator | Complete semantic rejection, never a sample or producer agreement test | Per-grain queries and rich replay remain disproportionate; F02 |
| Behavioral/global analytics | Declared complete projection/SCC/BDD or analytic method, settings and consumer | Stated abstraction, seeds/tolerances and partial/refused policy remain authoritative | No algorithm change proposed. A selected display filter must not narrow a complete global-analysis universe |
| Product hit / miss | Selected content domain plus profile/model/code/configuration identity | Optional equivalence under current owner; input absence is key material | Union token access and one typed replay representation fit overlapping warm roots; F03/F04 |
| Canonical publication / serving | Frozen exact content and executable definitions; pinned derived projections | Complete publication only; same pinned realization for evidence/resource/continuation journey | Direct sealing avoids self-transport; serving generation pressure still splits initialization ownership, F07 |

The stage declarations can improve orchestration **without becoming a new runtime framework**. `compilation.rs:479–500` resolves a declared completed input by scanning the schedule; `:503–579` checks selected product dependencies and freezes completed producer groups. `:719–807` invokes typed normalization authority handles, releases them after use and admits the normalized frontier. `compilation/reuse.rs` binds dependency tokens in a petgraph, but its affected traversal is consumed mainly as a tracing count at `compilation.rs:838–840`; actual keys and exact dependencies decide reuse. This is not currently a unified lifecycle or cross-run invalidation engine.

The simplest viable improvement is to resolve each declared input selector/predecessor binding once into an immutable typed binding object, then pass it to workspace provider registration, product keys and native view resolution. Its origin remains the model schedule. This removes repeated reconstruction while preserving ordinary Rust lifetime/proof owners. Use an existing compact dependency graph for an actual reverse-closure or inspection consumer when needed; do not add scheduling, recovery, invalidation or transaction semantics merely because nodes and edges exist. A semantic relationship edge does not authorize an effect dependency.

## 5. Findings

Each finding is a dated source-review obligation. On2026-10-09, plan authoring transferred F01–F08 to
[the persisted coordinator's sole disposition table](../../plans/persisted-graph-execution-plan_2026-10-07.md#native-efficiency-findings-transferred-on-2026-10-09).
The [native-efficiency companion](../../plans/native-execution-efficiency-plan_2026-10-09.md) develops
the proposed corrections and investigation routes. This transfer changes status ownership, not
the review's original source assessment or its evidence strength.

<a id="f01"></a>
### F01 — Exact membership demand expands into absent owner/key combinations

**High · FP-07, DP-10/14/20, CI-07/08 · A4 violated.** In `lctx-surrealdb/src/compiler.rs:3286–3369`, `MembershipLookup` generates the Cartesian product of every selected owner and every requested key into 128-record ID windows (`:3339–3358`) and executes each window. `SelectedRows` additionally shrinks its key window by owner count (`:3394–3399`) before membership discovery and payload hydration (`:3425–3433`). A view with many contributions and a sparse actual match incurs work on absent combinations and more crossings despite a small requested result. The full scan loops owners (`:3081–3089`); atomic field selection executes an indexed query per value (`:3065–3079`). These are related demand-to-physical-work manifestations, not proof of one timeout cause.

**Owner and proposed correction.** Native reader/access lowering should select actual immutable memberships using a bounded relation/key prefix with owner residual, or a compact derived exact-view membership index. Compare bounded atomic set selection with the scalar loop where the chosen index supports it. Preserve exact selected owners, nominal relation identity, duplicate/conflict detection, deterministic ordering, physical-ID dedup before payload hydration, explicit empty validation and terminal streams. Remove the discarded Cartesian enumeration only after the replacement provides those guarantees. The existing `member_keys(relation,semantic_key,contribution)` index (`compiler.rs:340`) makes a native route credible but does not itself qualify a query.

**Challenge and closure.** Many owners can share a key, owners outside the view can contain it, and conflicting payloads under one nominal key must still fail. A compound `IN owners AND IN keys` query is not automatically the cure: locked SurrealDB 3.3.0 `idx/planner/plan.rs:323–365` also constructs Cartesian equality/union values. Closure needs inspection showing actual-membership work rather than relocated Cartesian generation, an actual selected plan where the planner decision matters, and focused independent equivalence cases for sparse/missing/overlapping/conflicting memberships. No measured benefit is asserted.

<a id="f02"></a>
### F02 — Independent support admission repeats physical preparation and rich inputs per grain

**High · FP-07, DP-03/10/23 · A4 violated.** `cpg-core/src/scoped_admission.rs:152–196` creates a fresh validator per grain and builds/executes an ordered SQL query for each invariant input. `validate_support` lowers the support program once (`:199–219`), but each scope/context or 128-root grain invokes that path (`:243–263,282–293`). `OwnershipRows::Full` can query full ownership rows (`:88–119`); the model support program intentionally includes released distributions and corpus/library dependencies (`admission_scope_program.rs:293–350`). Exact bounded roots therefore can repeatedly plan and hydrate overlapping support, not just newly required rows. `CompletedInputs.session` (`workspace.rs:2572–2619`) creates a context/registers providers each request, another repeated preparation boundary.

**Owner and proposed correction.** Core support admission should group compatible immutable premises, prepare a checked logical input template/catalog once, hydrate the input union once, and feed separately owned check objects borrowed typed selections for their grain. Model-owned ownership programs and Full negative domains remain unchanged. Shared physical input is not shared semantic acceptance. Current exact-premise and reference memoization remains, and candidate/stored-row roots continue to detect orphans and missing qualifications. DataFusion typed logical templates/provider scans are credible tools; exposing SQL PREPARE statements is unnecessary.

**Challenge and closure.** An invalid stored row unreachable from the producer's expected positives, a released distribution with no direct root, or a context-qualified orphan must still be rejected. Batching must not merge ordered streams or confuse per-context validator state. Closure is a trace of one compatible input preparation/decoding route with exact partitioned enforcement plus meaningful negative/admission controls. The 900-second invariant-admission receipts make this boundary important; they do not identify its dominant expense.

<a id="f03"></a>
### F03 — Overlapping cached root products fetch overlapping content tokens separately

**Medium · FP-07, DP-09/10 · A4 violated for optional lookup enabled.** `consumed_rows.rs:1791–1826` computes each logical product domain by requesting native content tokens for each input (`:1800–1812`) and rebuilding role/presence members. Native token access itself issues key windows (`compiler.rs:2869–2888`). Callable normalization invokes selected-domain/key lookup per root (`normalize/mod.rs:931–939`) while already sharing hydration for the miss union. Repeated overlapping roots therefore pay repeated native token reads before hits are known.

**Owner and proposed correction.** Prepared root batches should fetch the union of required tokens once per exact input/relation and derive each separate root domain from the shared compact result. Batch optional cache lookup where useful. Preserve absence-sensitive domains, root roles, full payload/content identity, profile/model/configuration and exact dependency distinctions; never let a positive token set certify completeness. Do not retain rich whole closures or require caching for operations whose lookup costs exceed avoided work.

**Challenge and closure.** A deleted/missing member or unrelated contribution must invalidate only the appropriate domain; shared contexts can belong to several roots without merging their logical identity. Trace equivalent independent root keys from one union lookup and retain hit/miss/empty/deletion controls. **This finding cannot explain the first cache-off matrix attempts**, which skip optional lookup. Its consequence is specific to enabled optional reuse.

<a id="f04"></a>
### F04 — A warm product becomes several encoded/rich representations before replay

**High · FP-07, DP-08/10/19/20 · A4 violated.** `workspace_products.rs:62–72` retains portable cached data and clones sections into a private candidate; `:135–149` decodes JSON/Arrow and re-encodes to check canonical rows. `:99–114` decodes again into replay batches. Normalization's current semantic hit predicate independently decodes the candidate again into a MemTable (`normalize/reuse.rs:25–49`). Cold capture additionally scans completed native output into full values (`workspace_products.rs:158–181`). Each step has a purpose, but independent assurance does not require independent copies of the same representation.

**Owner and proposed correction.** Product/private replay ownership should decode once into a charged typed candidate, validate portable canonical shape/order and current semantics against those values, then transfer the same validated batches to fresh ingress. Portable data must not carry capability; the current owner grants replay only after rejection checks. If all candidate validation must finish before any effect, retain that rule and the necessary charged values rather than streaming partial acceptance. Selective eligibility or disabled reuse remains a valid route for cheap products; remove redundant intermediates rather than moving them into another cache layer.

**Challenge and closure.** A canonically valid, coherent cached row can have the wrong receiver or qualification. It must fail under the current model before mutation, as the meaningful adversarial shape in `normalize/reuse.rs:155–161` requires. A resource failure after effects or uncertain acknowledgement must not trigger fresh fallback. Closure needs one typed decode/ownership route, equivalent canonical and semantic rejection and preserved pre-effect/finality behavior. This is not a proposal to trust cached validity.

<a id="f05"></a>
### F05 — Repeated synchronous native crossings poll readiness at a fixed interval

**High · FP-07, DP-10/14/20 · A4 violated.** `native_bridge.rs:21,103–109,123–133` uses 20-ms sleep polling for response readiness and full submission queues. `workspace.rs:2702–2706,2730–2738` crosses this bridge for batch writes/flush; declaration always traverses it (`:2870–2874`), including the already-registered contribution fast path (`:2825–2833`). `NativeBatches` similarly polls an empty bounded handoff (`:2242–2271`) after an async native stream (`:2171–2217`). Repeated fine crossings inherit readiness quantization and sync blocking even where no semantic boundary requires another hop.

**Owner and proposed correction.** Bridge/provider plumbing should use completion/event wakeups suitable for its synchronous callers, with cancellation wakeup, and give async consumers a coarse async route where warranted. Register a kind once under its producer owner. Coalesce compatible writes only where failure attribution and acknowledgement remain explicit. The dedicated router/runtime and bounded handoff are legitimate lifetime mechanisms (`native_bridge.rs:33–75`); deleting them blindly or merely reducing the sleep constant does not supply the contract. No thread/job caps are proposed.

**Challenge and closure.** A provider canceled while queue-full, a submitted write reporting a late error, or a stream dropped after provisional rows must drain to its actual terminal result while retaining client/runtime and charges. Closure is wake-driven/coarse boundary inspection with focused cancellation/full-queue/late-error controls. The number of polls and their contribution to observed elapsed time are unmeasured.

<a id="f06"></a>
### F06 — Completing one contribution waits for every native scan

**High · FP-03/07, DP-08/19/20 · A3 and A4 violated for independent composition.** `compiler.rs:1881` calls global `wait_scans` at contribution completion. That wait (`:1264–1288`) requires the store-wide scan count to be zero; every retained scan increments that count (`:1120–1138`). An unrelated immutable completed-view reader can therefore delay finishing a new independent contribution. This barrier couples otherwise valid composition to unrelated retained reader lifetimes. A final content/seal/abandon drain has a broader purpose; a routine producer completion does not automatically have the same scope.

**Owner and proposed correction.** Native/core completion should own and await the producer's accepted mutations and relevant producing read descendants. Keep global failure propagation and the final freeze/seal/abandon drain. This may be ordinary scoped ownership, not a graph scheduler. If a concrete reader can mutate the completing contribution or its definitions, its relationship must remain explicit and included.

**Challenge and closure.** An unrelated reader may fail after another producer becomes complete. Its failure must still prevent final admission/publication where that failure compromises the attempt; local completion is not a release of global certainty. Closure needs a trace that independent immutable reads do not block local completion, while own work/late errors and final global drainage remain correctly enforced. No observed deadlock is claimed.

<a id="f07"></a>
### F07 — Optional serving retention replacement also replaces in-flight coalescing

**Medium · FP-07, DP-09/10/20, CI-13 · A4 violated under pressure.** `lctx-serving/src/preparation.rs:43–79` holds a Moka cache whose entire generation is replaced by `release_optional_retention` (`:67–72`). `:108–146` spawns retained initialization using a cloned cache generation and `try_get_with`; old initialization can still be active when a new same-key request uses the replacement generation. Moka coalesces within one generation, not across these objects. Pressure intended to release optional residency can duplicate the same complete selected preparation.

**Owner and proposed correction.** Serving preparation should retain one stable per-key in-flight owner independently of optional completed-value retention, or defer generation replacement until it cannot split active initialization without preventing admission progress. Prefer a small ownership correction composed with Moka, not a general workflow engine. Current exact keys/pins and the owner that continues and drains canceled work remain authoritative; charges stay with the last retained value. Closing/retirement still fences new requests and drains retained work (`:148–175`).

**Challenge and closure.** Pressure while the first initializer is suspended, a second same-key request, cancellation of either waiter and concurrent retirement must not produce two ungoverned loaders or release a live charge. Closure needs a deterministic generation-pressure/in-flight case and source ownership inspection. This is an execution-fit finding, not evidence of partial published results or a violated G5.

<a id="f08"></a>
### F08 — Typed extraction inspection reconstructs the whole relation inventory

**Medium · FP-06/07, DP-10/17/23 · A1/A4 violated for selected controls.** `cpg-extract/tests/typed_driver/mod.rs:65–82` holds the observer mutex while scanning every completed relation, collecting batches and concatenating them. The typed inspector macro (`:85–95`) does not translate its requested type into selected observed relation demand. `run_profile_with_budget` already compiles and validates the workspace before observation (`:248–273`). A control interested in a small fact family therefore couples its observation to full relation scanning and rich retention.

**Owner and proposed correction.** The extraction harness should declare inspector demand and retain selected bounded/borrowed batches through an async or coarse native read. Tests whose guarantee is complete inventory, whole-output determinism or complete admission must still request that universe. Preserve native-backed coverage; do not weaken test filters to make timeouts disappear. Tests of pure transformation should retain an explicit-value seam independent of acquisition/publication setup.

Fixture setup also has independent ownership questions: core's `tests/fixtures/native.rs:9–34` owns an additional multi-thread runtime, and `NativeCompilerStore::begin` (`compiler.rs:353–412`) authenticates, creates a private database and installs schema per attempt. Fresh native database isolation and actual DDL are required for the relevant tests; stable process transport/runtime/readiness or prepared schema lowering may be shared only within a suitable owner. Logged setup phases around 3.8/6.3 seconds do not prove setup is the dominant timeout cause. Default Tokio tests are not all multi-threaded, and there is no measured thread exhaustion.

**Challenge and closure.** A test that must detect an unexpected relation or orphan cannot become a selected-positive test. Explicitly distinguish that full-inventory guarantee from the inspector-specific control; trace reduced observation for the latter and preserve an independent complete-universe control. Do not share attempt-private output databases or weaken native persistent realization.

## 6. Synthesis and expected change scenarios

F01–F05 share a physical pattern: a correct logical partition is repeatedly turned into a separate query, decoded candidate or synchronous crossing. Their remedies fit together around immutable prepared inputs, union physical demand and borrowed partitioned typed values. They remain separate closure obligations: union tokens do not fix Cartesian membership, one decoder does not remove admission queries, and wakeups do not make a scalar access path bulk. F06 concerns coordination scope; F07 concerns initialization ownership under retention pressure; F08 concerns observation scope. A global graph or a cache cannot safely paper over all of them.

| Scenario / kind of change | Expected owner and contract propagation | Current observation / proposed consequence | Settling evidence |
|---|---|---|---|
| Cold compile; execution mechanism | Exact inputs → scope/kernel → fresh native output → admission | Necessary computation remains; avoid Cartesian absent memberships and repeated support reads. Product capture itself can add work; reuse stays optional | F01/F02/F04 route inspection and minimal negative controls |
| Warm overlapping roots; instance/composition | Product contract and exact root domains, without changing model semantics | Shared union discovery exists; token access/decoding repeat. Share physical tokens/typed candidates, retain distinct partitions | F03/F04 hit/miss key equivalence |
| Add/delete/empty data; instance | Complete-empty and missing physical roots, absence-sensitive universes | Positive-only reuse/admission would be wrong; proposed grouping must preserve deleted support and orphan checks | Empty, deletion, missing qualification and orphan cases |
| Configuration/model/provider revision; binding/policy | Captured source/provider/configuration/model/code identity | Exact premises/products must miss or re-admit; never reuse by relation name alone | Different context/pin changes only affected keys and checks |
| More unrelated content or semantic kinds; instance/domain extension | Existing selected operation retains universe; new kind has declaration and genuine new semantic behavior | Envelope/typed lowering avoids vocabulary-wide body programs; F01/F08 still expose owner/inventory expansion | Existing selected path unchanged by irrelevant kind/content; full analysis remains complete |
| New fact family / ownership rule; domain concept | Model declaration, fidelity/coverage and invariant; adapter and genuine consumer semantics follow | Mechanically derived backing/provider metadata should follow declaration; no new per-kind scheduler, cache or observer classifier | Trace one family through declaration/ingress/admission and selected inspector; independent semantic edits have named owners |
| Replace native reader with indexed-set lowering; mechanism substitution | Native exact-view read contract unchanged | F01 requires real planner fit, duplicate/conflict and terminal preservation; no compatibility reader needed | Locked API/query-plan evidence and native membership shapes |
| High-degree/reconvergent roots; instance/skew | Complete support and separate partitions, charged bounded discovery | A small output can examine many actual edges; shared union helps, but cannot falsely cut required complete discovery | Overlap, isolate, parallel edge, cycle and missing target shapes where affected |
| Concurrent independent read/compile; composition | Exact immutable views and local contribution ownership | F06 global wait over-couples; scoped completion must leave final closure global | Independent retained read vs local completion and final drainage |
| Cancel/full queue/late error/retire; lifecycle | Retained owner drains actual work; new requests fenced | F05 wakeups and F07 coalescing must preserve lifetime/certainty | Focused blocked/canceled/late-error/retire cases |
| Damaged detached import; external binding | Transport integrity plus independent restored semantic admission | Shared preparation cannot inherit producer validity; byte/body/scope/outcome tampering still fails before publication | Existing native restore/tamper controls retained |
| Trace served claim / new realization; instance/mechanism | One pinned content and executable definition set across evidence/resource/continuation | No new graph/cache representation may resolve citations in another realization | Targeted pinned journey; full served breadth not certified here |
| Evaluation run; policy | Private truth remains separate; evaluator revision explicit | Efficiency corrections must not change gold, tuning population or verdict meanings | Evaluation contracts unchanged; no protected data opened |

## 7. Library fit and alternative comparison

The loaded capability skills match SurrealDB 3.3.0, DataFusion 55.1.0 and petgraph 0.8.3; Moka is locked at 0.12.16. Current Context7 documentation gathered by the researcher is discovery guidance, not version proof. Locked registry source is decisive for API and execution claims. The Salsa skill is 0.28.4 while the workspace locks 0.28.5; no Salsa contract is transferred or adoption recommended from that skill.

Context7 first resolved the libraries, then queried `/surrealdb/docs.surrealdb.com` for record-ID versus indexed-set selection, `/apache/datafusion` separately for provider optimizer visibility and prepared-plan reuse, `/petgraph/petgraph` for traversal reset/move semantics, and `/websites/rs_moka` for future initialization coalescing/cancellation. None supplied the exact locked-version documentation identity. For example, current DataFusion documentation uses a different projection-argument signature than locked 55.1.0; no current signature is transferred into this target.

| Capability / owner | Existing and library-owned route | Resolved fit / burden | Recommendation |
|---|---|---|---|
| Native selection | Exact record IDs, relation/key membership index, bounded indexed sets | Surreal 3.3 can express set selection, but compound unions can still expand Cartesian products. Correct dedup/terminal handling remains adapter-owned | Compare actual access paths; retain native authority without sanctifying point enumeration |
| Relational admission | DataFusion TableProvider projection/pushdown, parameterized checked logical plans | Existing providers already expose Exact/Unsupported pushdown (`compiler_provider.rs:161–269`). PREPARE stores logical plans (`session_state.rs:207–209`); execution builds physical plans (`dataframe/mod.rs:1601–1604`), not result caching | Reuse immutable catalogs/logical input templates and typed values. `cpg-core/sql.rs:8–20` intentionally forbids statement effects; no blanket PREPARE opening |
| Graph selection / dependencies | Existing compact adjacency, model scope programs and petgraph | Dfs reset/move operations can reuse traversal state after selected discovery; dense bitsets require compact index/length identity and explicit charges. None authorizes canonical identity substitution | Use library graph algorithms where an actual algorithm consumer exists; avoid whole-graph residency and generic lifecycle graph |
| Same-key preparation | Moka future `try_get_with`, typed application lifetime owner | Coalescing/error contract is per cache generation; detached app task owns continuation/drain | Correct generation/in-flight ownership, retain library coalescing |
| Incremental computation | Current exact immutable keys; Salsa candidate | No reviewed workload needs fine-grained mutable query-session dependency tracking. Adopting it adds cancellation/durability/lifetime integration before fixing concrete physical work | No substitution justified now; reconsider only for a named dynamic incremental consumer |
| Typed record transfer | Existing Arrow/DataFusion bounded internal values | Shared Arrow contract is intentional; richer wrappers need a semantic ownership reason, not merely an abstract interface | One charged typed candidate/borrowed view where it removes copies; no parallel canonical store |

| Alternative | Ownership / local reasoning | Physical route and total machinery | Decision / premise that would change it |
|---|---|---|---|
| Current composed baseline | Strong explicit model, exact views, owned effects; local mechanisms are understandable | Repeated absent combinations, queries/decodes/crossings and broad completion/observation remain | Revise; correct output alone does not establish A4 |
| Recommended target | Exact immutable bindings, shared selected preparation, partitioned typed checks, scoped lifecycle owners | Correct access paths and event-driven/coarse handoffs; optional cache cannot own correctness. Existing compiler/serving components absorb changes | Preferred Proposed target; no universal scheduler or new accounting system |
| Suitable library-owned alternative | Surreal set/index access + DataFusion provider/logical templates + Moka/petgraph under typed owners | Library bulk/optimizer/coalescing surfaces reduce bespoke loops when qualified. Same semantic rejection and native terminality still require integration | This can coincide with the recommended target; choose per actual operation rather than one library for every responsibility |
| Simplest viable alternative | Share resolved exact inputs and one typed representation in ordinary functions; bypass optional products where not useful | Fix specific loops/barriers without adding a graph runtime, mutable invalidation engine or another persistence authority | Start here; a new abstraction must earn an actual consumer |
| Materially different intermediate placement | Typed/spilled compute outputs followed by one final native materialization | Could reduce native crossings, but must reconstruct exact epochs, absence domains, charged spill, canonical/original integrity, independent admission and publication authority | Credible comparison, not rejected by current ADR. No decisive evidence now that its added placement/transfer obligations beat the simpler target. Reopen if required native memberships remain structurally disproportionate after qualified access sharing; supersede current placement decisions if selected |

A pinned personal deployment does not make repeated crossings acceptable; conversely, a timeout does not prove a store replacement is the best remedy. The recommended correction deletes duplicated physical work rather than installing another mechanism to mask it. Preserve useful preparation and direct sealing. No license constraint or catalog completeness is used as a library rejection reason.

## 8. Independent principles and gates

| Foundation | Verdict | Scoped evidence |
|---|---|---|
| FP-01 Separation of concerns | Satisfied | Model semantic ownership, native mechanism, core admission and publication visibility remain distinct; F08 is a selected harness boundary correction |
| FP-02 Stable contracts / replaceable implementations | Satisfied | Exact-view/typed-operation/terminal contracts permit reader and preparation substitution without domain redefinition |
| FP-03 Composition over entanglement | Violated | F06 couples one contribution's completion to unrelated immutable readers |
| FP-04 Explicit domain model / semantic authority | Satisfied | Stages, scope programs, nominal identities, coverage and exact bindings govern reviewed behavior; fact schema alone is not the evidence |
| FP-05 Explicit structure and constraints | Satisfied | Private ingress, exact premises, mutation fence and final admission reject unsupported states; no silent partial success proposed |
| FP-06 Local reasoning | Violated in selected test observation | F08 selected controls depend on whole relation reconstruction; no claim every test has this defect |
| FP-07 Execution fits workload | Violated | F01/F02/F04/F05 and scenario-specific F03/F06/F07/F08 establish structural amplification independently of elapsed-time attribution |

Supporting-rule assessment is qualitative and grouped by the evidence that settles it. DP-01/02/04/05/06/07/11/12/18/21/24 are **satisfied in the examined contracts**: explicit typed meaning, identities, graph kinds, stage visibility, effects and outcomes are retained. DP-03/08/19/23 are **satisfied for semantic enforcement/finality**, with physical enforcement frequency/replay and isolated observation **violated** in F02/F04/F06/F08; those distinctions do not justify deleting independent admission. DP-09 is **satisfied for exact dependency semantics**. F03/F07 concern execution fit and preparation frequency; they do not establish an incomplete dependency key or a semantic reuse violation. DP-10/20 are **violated** by repeated work/crossings and coordination scope. DP-13/14/16 are **unresolved for the final replacement access route**, because a native set plan/logical-template/ownership alternative must be qualified; they are not generic-library-absence defects. DP-15/22 are **unresolved for the dirty SDK stream correction's qualified claim**; its implementation is not a Tested remedy. DP-17 is **satisfied for production owners**, with F08's observation responsibility requiring correction.

CI-01/02/03/04/05/06/10/13 are **satisfied in the inspected selected-domain and pin contracts**, subject to preserved root/coverage/graph semantics; CI-07/08 execution use is **violated under A4** by F01/F02 despite explicit refusal/limits. CI-09's heuristic policy and CI-11's complete served-evidence breadth are **not independently certified** by this efficiency review; no heuristic-as-fact or cross-snapshot defect was found. CI-12 is **unchanged/not applicable to a functional correction here**: no evaluator or private truth is changed or accessed.

| Gate | Verdict | Independent evidence / limit and required action |
|---|---|---|
| G1 Authority | Pass, scoped static | Typed declarations own facts/operations; derived preparation is not another authority. Proposed remedies must preserve this; no independently mutable duplicate definition identified |
| G2 Semantic fidelity | Pass, scoped static | Exact roles/coverage/empty/missing and partition semantics remain explicit. F01/F02/F03 challenges prevent physical grouping from flattening them |
| G3 Validity | Pass for inspected enforcement contract; changed SDK execution qualification unresolved | Typed ingress, independent actual-row admission and exact premise caching have defined rejection. Historical admission timeouts do not prove invalid state escaped; current composed qualification remains incomplete |
| G4 Hidden behavior | Pass, scoped static | Effects are private/declared; SQL helper excludes statements and final publication is explicit. Reuse is not a license for undeclared effects |
| G5 Consistency / recovery | Unresolved for qualification of current dirty native transport/lifetime boundary | Strong retained owner/freeze/terminal contracts inspected; uncompiled SDK ownership change and canceled integrated acceptance prevent qualified claim for that changed route. F06/F07 are not evidence of partial publication |
| G6 Transformation / reuse | Pass, scoped semantic contract; proposed replacements unresolved | Exact keys/current semantic replay predicates and graph partitions inspected. F01–F04 alternatives need independent equivalence/negative controls before acceptance |
| G7 Truthful capability claims | Unresolved for enclosing native readiness | The SDK patch is not a proven speed/auth repair; qualification is canceled/incomplete. Any current “running” checkpoint must reflect the actual receipt. No new runtime readiness claim follows this review |
| G8 Library leverage | Unresolved for selected physical alternatives; no independent demonstrated generic reinvention failure | Libraries offer plausible bulk/planning/coalescing operations, but planner fit and lifecycle integration matter. Generic availability alone does not fail the gate; F01/F02/F05/F07 require a qualified composed choice |
| CI-G1 Fidelity | Pass, bounded static | No examined route relabels provider meanings or turns unknown into absence; complete negative domains and partition semantics are remedy constraints. Full semantic family qualification not established |
| CI-G2 Evidence closure | Unresolved outside inspected pin boundary | Serving preparation keys/pins preserve one realization; complete API/evidence/resource/continuation journey was not independently traced. No cross-snapshot defect inferred |
| CI-G3 Evaluation integrity | n.a. to this correction | No evaluator meaning, production gold input, protected-data tuning or comparison baseline is changed; existing segregation must be retained |

Architectural failures above stand independently of gate verdicts. G8 alone is not used as a substitute for the demonstrated A4 problem. A passing static contract row is not a passed native test suite.

## 9. Verification, uncertainties and disposition

| Claim / boundary | Evidence label / date | Outcome and limit |
|---|---|---|
| Source structural findings F01–F08 | Implemented mechanisms / source-inspected assessment, 2026-10-09 | Direct source inspection; the named scenarios establish mechanisms, not measured runtime shares |
| Existing sharing, exact premises, freeze and restored admission | Implemented / source-inspected, 2026-10-09 | Identified implementation paths; no new runtime qualification |
| Locked library API/planner/lifetime descriptions | Interface-checked / source-inspected, 2026-10-09 | Registry source for exact versions; current documentation discovery not pinned proof |
| `just qualify` / above run IDs | Historical receipts, 2026-10-09 | Failed/canceled enclosing acceptance; scoped passed controls do not certify the composed dirty baseline |
| Compile/focused native controls for SDK patch and proposed remedies | not_run by this review | No build/test authority in this delegated read-only assignment; focused checks belong to implementation and existing acceptance timing |
| Latency/throughput/capacity improvement | Unmeasured | No numeric benefit, root-cause share, thread exhaustion, backend superiority or whole-pilot acceptance is claimed |
| Root publication checks | Tested documentation only, 2026-10-09 | `just docs` **passed**, 358 canonical pages and zero link errors; `git diff --check` **passed**. `just docs-check` **failed** before publication at the existing dirty ADR-0140/Design §B3 decision-line mismatch. This is a mixed-tree documentation boundary, not a product or architectural qualification result |

The consequential open choices are the actual native membership plan in F01, partitioned complete-input admission in F02, and ownership/finality of the stream/bridge changes. These affect the corrective direction; settling them can use static implementation and narrowly revealing controls, with planner inspection where query shape is decisive. They do not require a benchmark campaign, runtime accounting system, new checklist or always-on telemetry. Quantitative claims later require measurement. Existing plan timing owns full functional qualification; no new test caps, timeouts or acceptance dilution are proposed.

### Rule impacts after the architectural judgment

The operator explicitly accepted RC01–RC05 during plan preparation on2026-10-09.
[Companion §2](../../plans/native-execution-efficiency-plan_2026-10-09.md#2-operator-decisions-and-authority-route)
records each outcome and its implementation route. The recommendations below retain their
original review meaning; this confirmation does not amend an accepted ADR or establish closure.

| ID | Proposed change / rule impact | Required route and owner | Source findings / status |
|---|---|---|---|
| <a id="rc01"></a>RC01 | Replace §6.2's physical requirement to “point-read deterministic memberships” with exact immutable membership selection, qualified actual access paths and deterministic conflict/dedup semantics. The current physical wording unnecessarily commits the contract to Cartesian point enumeration | Storage/publication owner; update governed architecture through the appropriate ADR route. Keep model/exact-view semantics; accepted ADR-0138 is not edited in place | F01; **Proposed decision at review time; operator accepted direction2026-10-09, route in companion §2** |
| <a id="rc02"></a>RC02 | Distinguish local contribution completion from final global read drainage in the native lifecycle contract | Semantic-model §15.10/15.11 and storage lifecycle owners; decide explicit relevant descendant/failure ownership, then amend owning architecture if the current global completion guarantee is deliberately changed | F06; **Proposed decision at review time; operator accepted direction2026-10-09, route in companion §2** |
| <a id="rc03"></a>RC03 | Share prepared relational/typed representations while retaining independent admission, exact premise identity and reject-before-ingress | Existing model/core owners; implementation/refactor under current authority unless chosen lifetime/assurance meaning changes. SQL statement prohibition stays; opening PREPARE is not required | F02–F04; **No additional semantic rule relaxation proposed** |
| <a id="rc04"></a>RC04 | Separate optional value retention from stable in-flight ownership; replace polling with actual completion wakeups | Serving/native/provider owners; implement inside current retained-owner/terminal contracts. Any altered cancellation/acknowledgement contract requires the existing decision route | F05/F07; **No caps, scheduler or retry framework proposed** |
| <a id="rc05"></a>RC05 | Declare selected versus complete-inventory observation in typed extraction controls | Harness/validation owners; preserve independent full-universe controls and actual native realization | F08; **No acceptance-scope weakening proposed** |

No alternate canonical authority, operator activation, old-format compatibility reader, source of truth ledger or universal execution graph is authorized by these rule impacts. If the materially different intermediate placement wins a later comparison, it can supersede §B7/§B12 and related decisions through the existing ADR/owner route rather than conforming to them by construction.

| Findings | Current disposition owner | Transfer / reopening trigger |
|---|---|---|
| F01–F08 | [Persisted coordinator §8](../../plans/persisted-graph-execution-plan_2026-10-07.md#native-efficiency-findings-transferred-on-2026-10-09), transferred2026-10-09 | [Native-efficiency companion](../../plans/native-execution-efficiency-plan_2026-10-09.md) schedules corrections with original IDs, owners and closure evidence; this dated review does not keep another mutable status table |
| Existing GK/GR/PJ obligations and canceled qualification | Their existing coordinator/plan owners | This review neither closes nor duplicates their mutable status; root reconciles current receipt truth and explicit transfer |

## 10. Architectural decision and priorities

| Judgment | Verdict | Evidence / action |
|---|---|---|
| A1 Localize change | Violated for selected extraction observation; otherwise satisfied in inspected production ownership | F08 couples narrow controls to whole inventory. Preserve coherent model/native/core/publication boundaries; improve observer demand without certifying unexamined breadth |
| A2 Encode domain meaning explicitly | Satisfied in bounded reviewed scope | Scope programs, stage dependencies, coverage/roles/exact views and lifecycle owners govern behavior. No universal graph is needed to supply missing meaning |
| A3 Extend through composition | Violated for independent native read/producer composition | F06 store-global drain defeats independent completion. Typed resolved stage bindings are the simpler integration target; new scheduling machinery needs a named consumer |
| A4 Fit execution to supported workload | Violated | F01/F02/F04/F05 and conditional F03/F06/F07/F08 demonstrate avoidable examined work, representations/crossings and lifetime coupling. Safe refusal and bounded windows do not settle fit |

**Bounded target decision: Revise. Enclosing native execution architecture: needs revision for the declared compile/admission/warm-reuse/concurrent-read scenarios.** Static evidence suffices for the structural decision; measured runtime diagnosis and the current native lifetime patch remain unresolved. Preserve semantics and finality strengths while changing physical mechanisms. The proposed target is not implemented or accepted by this review.

Consequence-based priorities are F01/F02/F05/F06 for the main selected/native admission and coordination route, F04 for warm replay and charged state, F07 for serving under pressure, and F03/F08 for enabled lookup and observation scope. This is not implementation order: exact binding/typed candidate/lifetime contracts can enable several remedies, while F03 cannot repair a cache-off attempt. The next consequential decision is how exact membership and independent support admission share physical preparation. Transfer only chosen work to the existing coordinator, qualify the changed transport/lifetime route, and preserve independent negative/finality controls. Do not infer closure from an accepted ADR, one fast phase or a passing cache matrix alone.


## Source navigation and retained evidence

Line numbers above identify the inspected dirty baseline, not immutable line addresses after later implementation. These links lead to the responsible current files.

| Evidence / responsibility | Current source owner |
|---|---|
| Exact membership, content tokens, native lifecycle / F01/F03/F06 | [Native compiler store](../../../crates/lctx-surrealdb/src/compiler.rs) |
| Provider pushdown/projection | [Native compiler provider](../../../crates/lctx-surrealdb/src/compiler_provider.rs) |
| SDK session retention | [Prepared streams](../../../crates/lctx-surrealdb/src/prepared.rs), [stream reader](../../../crates/lctx-surrealdb/src/reader.rs), [new unverified native control](../../../crates/lctx-surrealdb/tests/native.rs) |
| Root union/partition/domain preparation | [Consumed rows](../../../crates/cpg-core/src/consumed_rows.rs), [normalization](../../../crates/cpg-core/src/normalize/mod.rs) |
| Shared assurance / F02 | [Scoped admission](../../../crates/cpg-core/src/scoped_admission.rs), [workspace](../../../crates/cpg-core/src/workspace.rs), [SQL effect boundary](../../../crates/cpg-core/src/sql.rs) |
| Typed replay / F04 | [Workspace products](../../../crates/cpg-core/src/workspace_products.rs), [normalization replay](../../../crates/cpg-core/src/normalize/reuse.rs), [selected kernel products](../../../crates/cpg-core/src/normalize/kernel_products.rs) |
| Polling / F05 | [Native bridge](../../../crates/cpg-core/src/native_bridge.rs), workspace native batch/writer owners |
| In-flight/retention ownership / F07 | [Viewer preparation](../../../crates/lctx-serving/src/preparation.rs) |
| Selected observation / F08 | [Typed driver](../../../crates/cpg-extract/tests/typed_driver/mod.rs) |
| Operational graph/declarations | [Compilation](../../../crates/cpg-core/src/compilation.rs), [dependency graph](../../../crates/cpg-core/src/compilation/reuse.rs), [stage declarations](../../../crates/lctx-model/src/domain/stages.rs), [admission scope](../../../crates/lctx-model/src/domain/admission_scope_program.rs) |
| Independent transport admission | [Artifact](../../../crates/cpg-core/src/artifact.rs) |
| Architecture owners | [Semantic model](../../design/sections/semantic-model.md), [storage/publication](../../design/sections/storage-and-publication.md), [acquisition/extraction](../../design/sections/acquisition-and-extraction.md), [validation/evaluation](../../design/sections/validation-and-evaluation.md) |
| Scheduled scope / historical receipts | [Persisted coordinator §9.1](../../plans/persisted-graph-execution-plan_2026-10-07.md#91-current-execution-checkpoint-2026-10-07), [GK plan](../../plans/graph-compilation-kernels-and-hashing-plan_2026-10-09.md), [GR plan](../../plans/graph-compilation-reuse-and-invalidation-plan_2026-10-09.md) |

Exact locked library sources inspected are under `/home/paul/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/`: `surrealdb-3.3.0/src/{lib.rs,method/query.rs,engine/remote/grpc.rs}`; `surrealdb-core-3.3.0/src/idx/planner/{plan.rs,tree.rs}`; `datafusion-55.1.0/src/{execution/session_state.rs,execution/context/mod.rs,dataframe/mod.rs}`; `petgraph-0.8.3/src/visit/traversal.rs`; `fixedbitset-0.5.7/src/lib.rs`; and `moka-0.12.16/src/future/{cache.rs,value_initializer.rs}`. These local sources qualify the version claims, including Surreal Cartesian union expansion, DataFusion logical versus physical planning, and Moka per-generation coalescing.

Documentation discovery references are [SurrealDB multi-record selection](https://github.com/surrealdb/docs.surrealdb.com/blob/main/src/content/build/migrating/from-other-databases/from-postgresql.mdx), [DataFusion prepared statements](https://github.com/apache/datafusion/blob/main/docs/source/user-guide/sql/prepared_statements.md), and [versioned Moka initialization](https://docs.rs/moka/0.12.16/moka/future/struct.Cache.html#method.try_get_with). They do not establish the replacement native query's selected plan.

Raw run data remain local under `build/runs/20261009T170649.056Z-032c30/` (`record.json`, `summary.json`, `output.log`, `verify/*.log`) and `build/runs/20261009T163757.029Z-9b0811/`. The first `record.json` records explicit cancellation and exit143. No new evidence folder, runtime recorder or performance gate is created by this review.
