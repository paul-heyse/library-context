# Phase 3 assembled implementation: final architecture review

## 1. Scope, outcome and coverage

| Field | Assessment |
|---|---|
| Subject | R1–R3, N1–N7 and X: normalized semantic relations, cumulative generation lifecycle, persisted computational graphs and downstream ownership boundary |
| Revision | Shared `main` worktree, 2026-09-30; N6/N7/X and review corrections are uncommitted during inspection |
| Standard | Core/template 3.2, code-intelligence profile 1.3, repository binding through [standard.toml](../design_principles/standard.toml) |
| Tier · purpose | design · target; scheduled assembled Phase 3 checkpoint |
| Reviewer | Independent `design-reviewer` agent, 2026-09-30 |
| Target authority | [DESIGN §15](../../design/sections/semantic-model.md), ADR-0101/0103 and [detailed plan](../../plans/semantic-model-phase3-detailed-plan_2026-09-30.md) |
| Included | Semantic ownership and totality; stage eligibility/freeze and publication; model-owned projections; actual petgraph construction, serialization, hydration and traversal; migrated expectations and retained P4/P5 consumers |
| Excluded qualification | Q full gate, schema acceptance, fresh FastMCP profiles, deterministic repeat, end-to-end and graph-specific measurements remain primary-agent work; this review does not supply those receipts |
| Method | Independent current source/type/test inspection, pinned petgraph implementation inspection and bounded review of adjacent consumers; no new runtime probe or product gate run by this reviewer |

The relevant journeys are discovering supported callable surfaces without turning uncertainty into
absence, retaining inspectable source contracts under unknown effective behavior, traversing a
complete declared program universe, and handing later analysis one coherent validated generation.
P4 owns transfer, SCC analysis, summary/catalog conclusions and algorithm consumers. P5 owns serving
and evidence-backed product answers. No retrieval-quality or behavioral-result claim follows from
successful normalization.

**Final decision: Accept scoped for R1–R3/N1–N7/X, after correction and reinspection, 2026-09-30.**
The initial decision was Revise: F01 concerned missing normalized scope outcomes and F02 concerned
lost whole-event uncertainty in graph availability. Both have satisfactory corrective source
reinspection and attributed focused receipts. This accepts the assembled architecture within its
stated semantics, without granting the still-pending Q phase exit. Unrelated dirty
AGENTS, agent and settings changes were preserved. The current execution disposition is owned by
[plan §11](../../plans/semantic-model-phase3-detailed-plan_2026-09-30.md#11-current-package-status-and-finding-routes).

## 2. Responsibilities, dependencies and semantic ownership

| Owner | Responsibility / authoritative operation | Consumer and reason for change |
|---|---|---|
| L0 native adapters and typed facts | Provider-qualified syntax, declarations, signatures, types, calls, flow and coverage | N1–N7 consume attributed records; provider-private API changes stay in the adapter |
| `domain::normalized` | Nominal correspondence/owners, total relationships, effective surfaces, complete events, five call policies and binding admission | Each semantic operation has one definition shared by construction and validation |
| `domain::projection` | Finite endpoint roles/specs, universe, typed arcs, gaps and projection assessments | Named program graph meanings; new topology semantics belong here |
| `projection::snapshot` | Immutable petgraph representation, pinned binary wrapper, chunks, hydration and typed traversal | Repeated analysis consumes a computational materialization; canonical relations retain authority |
| `domain::admission` / `stages` / `normalized::coverage` | Frontier closure, exact scoped evidence outcomes, capability eligibility, execution identity and receipts | Store/runtime consume model contracts; corrected F01 supplies one normalized availability operation |
| `lctx-postgres::generations` | Freeze/grants, read proof, checkpoint, seal/validate/publish/select/retire and leases | PostgreSQL is the sole durable store, including graph bytes |
| `cpg-core::{model_runtime,generation_read,normalize,stage_runtime}` | Source-bound compute, scan admission, charged transport/operations, combined schedule and telemetry | Composes existing owners without reclassifying call or graph meaning |
| P4/P5 consumers | Later analyses/catalog and serving; retained legacy declarations have named consumers | Their migration is not activated by publishing a normalized generation |

Dependency direction remains mechanics → model meaning. Model declarations derive schema,
references, codecs, validator inventories and stage wiring. The added role metadata validates that
each named role has the declared canonical relation; it is not another mutable graph registry.

| Family | Origin and fidelity | Coverage / unknowns | Identity and consumers |
|---|---|---|---|
| L0 assertions | Pinned native providers; original qualification and supports | Exact admitted provider scope rows distinguish Complete/Partial/Unavailable/NotRequested | Input/context/provider-native identities; normalized operations |
| Entity and relationship outcomes | Deterministic model operations over L0 | Tentative/conflicting correspondence and unresolved candidates retained | Declaration occurrences and qualified external/synthetic identities; N2–N7/P4 |
| Effective callables and binding | Bounded supported semantics, separate source inspection and invocation authority | Conditional/uncertain evidence, wrappers, unavailable signatures and body exclusions cannot confer context-wide admission | Callable/context, raw variant/actual/formal IDs, full member digests; P4 token boundary |
| Events and policy membership | Whole attributed event then one model policy evaluator | Open remainders, dispatch, disagreement and unknown receiver are N4 outcomes | Site/origin/context, raw relationship IDs; policy views and graphs |
| Graph snapshots | Exact computational materialization of named normalized relationships | Unresolved side records, dependency coverage and selected events' whole-event uncertainty | Typed `ArcId` and nominal entity weights; private runtime indices |
| Normalized scope outcomes | Model-owned transformation-level result over admitted declared input scopes | Complete/Partial/Unavailable/NotRequested/NoScope; completion does not strengthen evidence | Capability × scope × context, producing/output receipts and shared exact lower-evidence sets |

## 3. Contracts, constraints and testing boundaries

The R1–R3 source boundary still requires frozen completed outputs, matching attempt/model/schema
and exact declared input invariants. Store read-check receipts bind the frozen premise set;
each provider connection rechecks authorization while holding a generation lease. The prepared
query owns scan admission and retained source reservations. No public unrestricted DataFrame
execution path was introduced by normalization.

N1–N5 corrections from the [earlier review](design_review_phase3-normalization_2026-09-30.md)
remain visible in the current source: establishment requires sufficient qualification, N3 checks
each context-wide knowledge component, and private binding admission replays N1–N4 plus exact
binding results. CompleteEvent verification is internal; public event validation returns unit.
Successful source binding, unique effective binding and positive body admission remain distinct.

Graph construction is structural, not a call/composition proof. `ProjectionInput` explicitly
documents that supplied records still require publication validation. Hydration validates format,
key, expected counts, accepted entity/arc kinds and canonical order. Publication additionally
recomputes the canonical projection and compares all vertices/arcs and side records. A private
field is not substituted for this shared validation boundary.

The cumulative schedule preflights a writer for every normalized relation, including explicitly
empty outputs. All facts finish before the private L0 checkpoint; subsequent computation closes
its reader before output completion. Final publication is one generation and never selects it.
Failure aborts the private attempt and preserves the selected prior generation. The corrected
coverage operation distinguishes completed computation from available normalized evidence;
final admission checks both the scope outcomes and exact frozen producer/output receipts.

## 4. Composition and execution

| Question / stage | Universe, relations and scope | Method / semantic strength | Budget and failure | Output/evidence |
|---|---|---|---|---|
| What objects and links are supported? N1/N2 | Complete admitted L0 input, nominal entities and source/context relationships | Deterministic typed operations; no spelling equivalence or heuristic winner | Charged complete indices/rows; explicit refusal | Total normalized outcomes with raw candidates/premises |
| What contract can execute? N3–N5 | Complete event/variant/actual sets, exact supported callable premises | Separate identity/signature/descriptor/body knowledge; one policy evaluator and binder | Full lower replay charged; unknown variant prevents unique admission | Stored attempts plus private verified P4 admission |
| What invocations are recorded? CallableInvocation | Input entities/context targets and Invocation-admitted typed alternatives; directed, parallel, self-loops, isolates, no weights | Exact over selected recorded alternatives, not runtime reachability or complete behavior | Allocation/capacity checked; whole-event uncertainty retained without deleting known arcs | Original alternative IDs; event/source assessments, coverage and gaps |
| What defines/contains what? DefinitionContainment | Definition-phase alternatives plus occurrence ownership; directed typed roles | Distinct definition and containment arcs; no invocation relabeling | Same charged snapshot contract | Relationship IDs identify canonical evidence |
| What imports/references what? ImportReference | Complete accepted source entities and referenced destinations; separate Import/Reference roles | Ambiguous candidates survive; builtins/external modules have explicit side outcomes | Unsupported destinations do not fabricate graph nodes | Candidate/assessment IDs and gaps |
| What is publicly exposed? PublicExposure | Access modules and accepted exposed entities, preserving alternative identity | Exposure is an attributed relationship, not entity equivalence | Unresolved/ambiguous exposure retained | Public exposure candidate IDs and source coverage |
| How is topology reused? Snapshot adapter | One actual directed petgraph Graph per built-in spec/input/context | Canonical insertion, serde-1, pinned Postcard wrapper, generation-owned chunks; hydration reconstructs library vectors without SQL edge insertion | Shared reservations cover graph/encoding/decoding/index state; partial snapshots cannot publish | Immutable graph, private dense indices, typed outgoing/incoming arcs |
| What can traversal conclude? `reachable` | Complete graph universe before output selector; optional typed edge role and direction | petgraph DFS with filtered/reversed borrowed view; structural reachability only | Charged traversal/result; resource refusal | Sorted domain entity IDs; no dataflow/behavior conclusion |
| What evidence is available? Final coverage writer | Seventeen finite capability meanings/grains, admitted scope anchors and whole-input/context dependency families | Scoped anchor status plus exact dependency membership; no numeric enum maximum; same operation available before final writer | Charged indices and shared membership sets avoid per-outcome copies of every raw coverage row | Computation, scope outcome, exact evidence membership and frozen output receipt records |
| What is published? Combined pipeline | One captured closure; facts checkpoint, N1–N6, final coverage writer and normalized admission | Model-owned schedule and existing transactional store lifecycle | Abort on failed stage/validation; no intermediate selection | Self-contained normalized generation and operational measurements |

Snapshots are built sequentially, but complete stage inputs, outputs and validation closures are
materialized. This is a reservation-controlled implementation, not streaming relational joins or
an absolute RSS bound. O1 remains Q's measurement obligation.

## 5. Change and failure scenarios

| Scenario / kind | Expected owner and propagation | Inspected consequence / evidence |
|---|---|---|
| Add another analyzed release — instance | Captured manifest/context and fresh generation; existing stage/graph machinery | Every collection rebuilds snapshots; no graph reuse cache or cross-generation retention |
| Unavailable calls in a module with no reported events — incomplete evidence | Normalized call/binding capability outcome for the declared scope | F01 corrected: expected scopes derive from admitted coverage, so an outcome exists independently of observation rows |
| One known call target plus an open remainder — partial evidence | N4 retains closure state; N6 preserves it without dropping the known arc | F02 corrected: the selected event's uncertainty changes availability and retains its exact assessment premise |
| Add a named projection or new endpoint role — domain extension | Model spec/role relation, selection operation and independent semantic controls | Store lowering and snapshot transport follow declarations; no SQL graph schema duplication |
| Add SCC/centrality over an existing graph — analytic composition | P4 chooses projection/model/algorithm and obtains an owned read-only graph adapter | Current API supports typed traversal only; O2 records the concrete P4 adapter trigger |
| Replace serialization format — mechanism | Snapshot owner/pin/format version, rebuild project generations | Canonical IDs/relations stay unchanged; incompatible wrapper refuses, no compatibility reader |
| Missing/extra/wrong graph materialization — invalid data | Shared projection validator and generation publication | Exact canonical comparison and complete header/chunk closure refuse corruption |
| Filter output to one distant node — query selector | Traverse the declared universe before applying selector | Diamond/cycle control retains intermediate nodes and returns the reachable requested node |
| Cancel or fail after facts checkpoint — lifecycle | Existing attempt cleanup and reader drain | Real-store focused control preserves selected prior generation and removes private attempt |
| Migrate legacy composer/catalog — ownership change | P4 consumes normalized entities and private admissions; removes old authority at its boundary | X retains named consumers and independent recovered expectations; no active legacy compile path |

## 6. Correctness and fidelity gates

| Gate | Corrective source verdict | Evidence / remaining qualification |
|---|---|---|
| G1 Authority | pass scoped | Canonical typed relations, one policy/binder, derived snapshots, model-owned frontier and store effects |
| G2 Semantic fidelity | pass scoped | Corrected capability scopes and exact dependency sets retain no-observation/partial/unrequested outcomes; event gaps retain open remainders |
| G3 Validity | pass scoped | Final admission reconstructs exact normalized scope/premise/output-receipt records from admitted facts and private execution receipts |
| G4 Hidden behavior | pass | Declared stage effects, source-bound reads, pinned inputs, pure semantic operations and separate telemetry |
| G5 Consistency/recovery | pass within inspected lifecycle | Freeze/checkpoint/atomic publication, retained leases, explicit failure and charged state; Q resource qualification remains pending |
| G6 Transformation/reuse | pass scoped | Canonical projection and persisted graph preserve typed arcs, complete universe and relevant whole-event uncertainty |
| G7 Truthful claims | pass within this report | Implementation/focused tests separated from pending full gate, measurement and product outcomes |
| G8 Library leverage | pass scoped | petgraph/Serde/Postcard, DataFusion and PostgreSQL own established generic mechanisms; domain policy stays in model operations |
| CI-G1 Fidelity | pass scoped | Normalized availability and graph availability preserve their distinct evidence and open-remainder premises |
| CI-G2 Evidence closure | n.a. to serving | P5 serving excluded; same-generation relationship references and canonical graph comparison inspected as prerequisites |
| CI-G3 Evaluation integrity | pass scoped | Independent fixtures are tests; no gold/skill/evaluation references enter compiler semantics |

## 7. Findings and applicability

<a id="F01"></a>
### F01 — normalized computation completion has no scoped evidence outcome

**Corrected and reinspected · FP-02/04/05, DP-02/03/08, CI-04, A2/A3, G2/G3, CI-G1.**

The initial `Frontier::Normalized` descriptor in
[`admission.rs`](../../../crates/lctx-model/src/domain/admission.rs) adds relation closure and a
Facts checkpoint but retains only `FACTS_REQUIREMENTS`. `AdmissionCheck` and
`ScopedAvailability` consume `ProviderCoverage`; no NormalizationCoverage, normalized capability
scope operation or final normalized coverage writer exists. Computed producers finish with
`ProviderOutcome::Complete`. Their exact-output invariants are valuable but are total over
originating observations, not an independently declared scope universe.

Consequently an unavailable family with no observations has no normalized group result, and
catalog flow-derived capabilities have no normalized NotRequested result. A P4 consumer must
reconstruct which raw families and scope premises establish each normalized capability. This
is the explicit remaining N7 contract in plan §6.5 and in the foundation review's exclusions;
passing stage receipts do not establish it.

**Correction:** add the owned capability × scope × context outcome or a semantically equivalent
owned operation. Derive expected scope membership from admitted captured/coverage premises,
retain exact lower evidence, policy/producer identity and completed output receipts, and enforce
the contract through final admission. Pre-final completed readers need the same operation.
Keep computation completion separate from Complete/Partial/Unavailable/NotRequested/NoScope.
Do not fabricate source observations or strengthen partial evidence because normalization ran.

**Closure:** independently check no-observation/unavailable and catalog NotRequested cases;
missing, duplicate, wrong-scope and wrong-receipt results must refuse publication. A complete
positive must retain exact scope/premise identities. Current disposition: plan §11.

**Corrective source reinspection, 2026-09-30: satisfied.** The new
[`normalized::coverage`](../../../crates/lctx-model/src/domain/normalized/coverage.rs) introduces
finite capability/grain contracts, exact provider-coverage premises, canonical producer identity
and frozen per-output receipts. One final writer emits the records; the shared AdmissionCheck
reconstructs them from admitted facts plus the private execution receipt and compares exact
contents. Seventeen documented capability meanings distinguish PublicExposure from entity/
correspondence outcomes and N2 operand/place links from N4 FlowEvents, including the correct
producer receipts. The audited dependency families include Types/Lexical for callable surfaces
and effective bindings, Syntax for source-backed mentions, and Types/Lexical for the exposed-field
projection universe. Coverage anchors follow the originating family grain, including Exports for
import observations.

Shared `NormalizationEvidenceSet(input, context, family)` membership retains each original
ProviderCoverage ID, including its reason and scope, without copying whole family vectors for
every outcome. A local anchor controls Unavailable/NotRequested; Complete additionally requires
every admitted scope in each declared dependency family of that input/context to be Complete.
Thus a complete caller cannot hide an unavailable body source elsewhere in its input. This
conservative policy may retain Partial when an individual result could be established; it never
promotes evidence. Pre-final readers expose the admitted ScopedAvailability and the same scoped
operation. Exact admission comparison rejects omitted/extra/altered outcome, evidence or receipt
records. Focused controls cover missing anchor/signature evidence, unavailable dependent Types/
Lexical, another unavailable source, and real-store catalog Flow NotRequested with receipt
equality. The final focused receipts and their source-revision limits are recorded in §10.
**Closure accepted within this review's scope**; Q remains separate.

<a id="F02"></a>
### F02 — a known arc can erase its event's open remainder from graph availability

**Corrected and reinspected · FP-04/05, DP-02/07/08, CI-02/04/05, A2, G2/G6, CI-G1.**

The initial [`projection_inputs!`](../../../crates/lctx-model/src/domain/projection/inventory.rs)
collects events, alternatives, policy assessments/admissions and raw targets, but no
`EventAssessment` or raw resolution completeness. In
[`describe_indexed`](../../../crates/lctx-model/src/domain/projection/normalization.rs), an admitted
Invocation alternative with a mapped entity becomes an arc. Gaps cover missing alternatives,
unselected policy, mapping/universe failures and raw family availability. None preserves the
whole-event open remainder recorded by N4.

For a complete provider extraction that reports one known target and an incomplete resolution,
N4 correctly retains OpenResolution. Invocation may legitimately retain its known arc, but N6
can produce CompleteUnderStatedModel with no uncertainty gap. Complete extraction coverage
does not establish complete target resolution. A later consumer inspecting graph availability
can mistake omitted alternatives for an exhausted graph boundary.

**Correction:** consume the owning event assessment and retain relevant open/incomplete event
premises as typed projection gaps/availability. Preserve known edges and the distinct Invocation
policy; do not turn it into Summary or discard useful partial evidence. Keep definition-only or
otherwise excluded alternatives scoped to the selected projection's meaning.

**Closure:** an independent complete/open pair with the same known mapped target must retain
the same known arc, while the open case exposes the event premise and Partial availability;
deleting that gap must fail shared validation. Current disposition: plan §11.

**Corrective source reinspection, 2026-09-30: satisfied.** ProjectionData now includes EventAssessment,
and projection gaps can retain that exact assessment with EventUncertainty. The new native twin
retains the known arc count while adding an open-remainder gap, and omission fails shared
validation. Event uncertainty is relevant only when the existing model-owned Invocation
membership selects an alternative. The outside-policy twin verifies that removing that selection
also removes its event-uncertainty gap; no second Invocation predicate is introduced. Existing
explicit empty-event handling remains. The gap also retains inexact, unknown-receiver,
unresolved, dispatched and disagreeing selected events without claiming P3 dispatch expansion.
The three native projection controls passed in the final corrective suite. **Closure accepted
within this review's scope**; exact-current-source integrated qualification remains Q.

**O1 — measurement remains required.** Complete charged collections, graph snapshots and replay
closures can coexist. Stage reservation/RSS telemetry and separate graph construction, encoding
and hydration measurements are Q work. Inspected bounds/refusals do not establish an end-to-end
memory limit, throughput, speedup or pilot scalability. Optimize measured excessive residency or
generic relational work with the existing compute engine where the evidence warrants it.

**O2 — P4 needs a read-only algorithm boundary.** `MaterializedGraph` currently exposes typed
entities/arcs and `reachable`, while the actual petgraph graph is private. The analytics reexport
does not yet let SCC/centrality consumers borrow it. At the first P4 algorithm migration, add a
scoped read-only adapter or owned operation preserving domain-ID translation and lifetime;
avoid reconstructing all edges merely to call a library algorithm. This is a named future seam,
not a request for a speculative framework or a claim that P3 implements those algorithms.

FP-01–06 are satisfied within the inspected scope after corrective source reinspection.
DP-01–24 have scoped support from declared dependencies, shared operations, qualified evidence,
explicit lifecycle and library mechanisms; resource/performance claims remain unmeasured.
CI-01–05/10/12 are supported for these normalized/graph transformations. CI-06/07/08/09 behavioral/analytic conclusions and
CI-11/13 served answers remain P4/P5 obligations, not silently accepted by this review.

## 8. Library fit and total complexity

| Capability | Inspected choice / alternatives | Fit and tradeoff |
|---|---|---|
| Immutable multigraph | petgraph 0.8.3 Graph; compared with StableGraph, GraphMap and hand-built adjacency | Parallel typed arc identity and compact immutable universe fit Graph. StableGraph deletion stability has no current consumer; GraphMap collapses endpoint-parallel identity |
| Persist actual graph | petgraph serde-1 with Postcard 1.1.3; compared with per-request SQL reconstruction and custom binary adjacency | Existing graph serialization retains topology; wrapper/version/chunk logic is a small domain boundary. No custom graph file format or additional durable store |
| Hydration validation | Pinned library deserializer plus canonical structure/source checks | Inspected petgraph rejects holes and out-of-range endpoints; fixed-width nominal ID visitor does not allocate from untrusted length hints. Decode reservation is conservative and still needs Q calibration |
| Traversal | petgraph DFS, EdgeFiltered and Reversed views | Complete universe and post-traversal selector preserve intermediates. These answer structural questions, not program transfer semantics |
| Compute/store | Existing DataFusion/Arrow and PostgreSQL/SQLx/COPY | Built-ins own planning, schema and transactional transport. Current domain grouping remains in charged Rust collections, not claimed as DataFusion streaming joins |
| Further graph analyses | petgraph / rustworkx and specialized libraries when a P4 algorithm needs them | No new algorithm library is justified by persistence alone; O2 records the adapter obligation |

The pinned graph skill and current lock/features were inspected, together with petgraph's
serialization implementation. No latest-release API claim is made. Postcard is an intentional
versioned implementation detail; the format can change by rebuilding pinned generations.

## 9. Alternatives and tradeoffs

| Alternative | Authority / locality | Judgment and revisit |
|---|---|---|
| Build graphs per request from canonical rows | Canonical authority is simple; repeats SQL reads/insertion for an immutable generation | Superseded by the operator's explicit build-once requirement and ADR-0103 |
| Persist graphs alongside canonical rows | One semantic authority, one disposable computational materialization; publication checks equivalence | Appropriate for full collection rebuilds; measurement must justify operational envelope |
| Make graphs canonical or add cross-generation cache hashes | Adds semantic editing/invalidation authorities without a current lifecycle consumer | Correctly rejected |
| Reuse stage Complete as normalized availability | Less state, but cannot answer group/scope evidence questions and requires consumer inference | F01; not a sufficient substitute for an owned normalized outcome operation |
| Drop unresolved/open events from graph output | Produces convenient topology while concealing incompleteness | F02; keep known arcs and typed uncertainty side records |
| Expose unrestricted mutable petgraph | Easy algorithm calls but permits topology mutation and escaping indices | No current need; P4 should use a scoped read-only adapter with domain-ID results |

## 10. Verification and uncertainty

These are author-run focused receipts reported in plan §11 and inspected test sources, dated
2026-09-30. They are not new executions by this reviewer and do not replace Q.

| Command / scope | Outcome / strength and limitation |
|---|---|
| Wrapped release model projection unit tests | passed, 4 controls: deterministic diamond/cycle/parallel/self-loop/isolates, malformed wrapper/chunks/capacity/budget, empty snapshot set and 70,000-node multi-chunk hydration |
| Wrapped release `cpg-extract --test normalized_projections` | passed, 2 controls: independent native topology and shared publication tamper rejection |
| Wrapped release `cpg-core --test normalized_relations` | passed, 2 real PostgreSQL profiles through projection persistence |
| Wrapped release `cpg-core --test normalized_generation` | passed, 3 controls: cumulative self-contained publication/repeated content, failure/selection preservation, producer preflight |
| Wrapped release `lctx --test compile_facts` | passed, 1 CLI control covering five profile/frontier compiles |
| Wrapped release `cpg-extract --test normalized_recovery` | passed, 3 independent recovered source/stub, Unicode/re-export/variant, dataclass/conditional declaration controls |
| N1–N5 corrected focused controls | Historical current-day receipt in the [normalization review](design_review_phase3-normalization_2026-09-30.md); decisive corrected code rechecked here |
| Corrective core/native/CLI command below | passed: core normalized generation 3, native projections 3, CLI compile 1; inspected [raw receipt](../evidence/2026-09-30_phase3-qualification/raw/review-final-controls.log) |
| Corrective model command below | passed: all 5 model library controls, including scoped availability/dependencies and four snapshot controls; inspected [raw receipt](../evidence/2026-09-30_phase3-qualification/raw/model-final-controls.log) |
| Initial primary-agent `just fmt` / `just test-all` | fmt passed; test-all failed in local Clippy issues before integrated qualification. Corrections/lint repairs and full rerun remain primary-agent Q work |
| Fresh FastMCP facts/normalized pilots, graph measurements and final full gate | not_run by reviewer; pending Q qualification at final review disposition |

Exact author-run corrective commands:

```sh
python3 scripts/build_environment.py -- cargo test --release -p cpg-core --test normalized_generation -p cpg-extract --test normalized_projections -p lctx --test compile_facts
python3 scripts/build_environment.py -- cargo test --release -p lctx-model --lib
```

The final PublicExposure projection dependency declaration (Types/Lexical) landed while the
core/native/CLI command was building. Subsequent model controls include it; this reviewer
reinspected it directly. The first receipt therefore does not certify every final source change.
Q must run the complete exact-current-source suite. These are composite focused receipts, not an
initially clean gate or completed phase qualification. The
[qualification evidence owner](../evidence/2026-09-30_phase3-qualification/README.md) records Q's
current execution and measurement conditions.

No additional runtime probe was necessary to establish the two source-visible contract gaps.
Independent shapes challenge semantics separately from shared construction/validation equality.
Physical performance, cancellation timing under pilot load, schema qualification and complete
phase exit require their named receipts; neither ADR acceptance nor this source review supplies
them.

## 11. Authority changes and dispositions

ADR-0103 deliberately supersedes the prior per-request graph construction decision. DESIGN §15
and plan §5 own that change; canonical relational authority and the single PostgreSQL store remain.
F01/F02 are corrections within accepted coverage/fidelity contracts, not reasons to add another
store, compatibility route or graph policy interpreter. Their current disposition belongs in
plan §11; this review owns the stable source findings.

X's inventory identifies each surviving legacy family and its P4/P5 consumer. Current normalized
compilation imports no `cpg_schema`, legacy IDs, `lctx_id` or old derived SQL. Recovered meaningful
assertions replace obsolete schema-count expectations; named transfer/handler/summary/protocol
expectations remain P4. Retire each old declaration with its last migrated consumer rather than
claiming whole-crate deletion at P3. Final docs/STATUS and runtime-store migration receipts remain
the primary execution owner's responsibility.

## 12. Architectural judgment and decision

| Judgment | Final verdict | Scope / remaining qualification |
|---|---|---|
| A1 Localize change | satisfied within inspected scope | Model operations own semantic changes; store/transport and graph representation have explicit owners |
| A2 Encode domain meaning explicitly | satisfied after corrective source reinspection | Normalized scope outcomes, exact evidence sets, event uncertainty and private admission govern supported behavior |
| A3 Extend through composition | satisfied after corrective source reinspection | Shared capability assessment, exact publication validation and existing graph operations compose without independent policy inference |

**Bounded decision: Accept scoped for the assembled R1–R3/N1–N7/X architecture**, following
F01/F02 correction, independent source reinspection and the attributed focused receipts above.
No blocking semantic or architectural finding remains in the reviewed supported scope. The
persisted graph architecture fits the stated immutable collection lifecycle. O1 remains Q
measurement and O2 remains the named P4 algorithm handoff.

**Enclosing architecture:** the normalized semantic/store/graph path is accepted at the stated
source-review and focused-test strength. Phase 3 exit still requires Q's complete current-source
gate, pilots, deterministic repeat and resource/graph measurements. P4/P5 and product qualification
are not established. The primary execution owner proceeds with Q and records its receipts in plan
§11; any material semantic change or exposed claim defect returns to the relevant review boundary.
