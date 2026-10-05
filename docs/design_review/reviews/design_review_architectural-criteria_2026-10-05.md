# Architectural criteria for efficient, assured systems

**Assessment: revise the design standard. Proposed amendment package, 2026-10-05.**

The principles protect meaning, ownership and change locality much better than they select an
efficient physical realization. They already contain useful efficiency rules. The missing piece
is an independent architectural judgment of **what work the composed system performs, where it
performs it, and how that work grows**. A design can currently satisfy the main judgments while
turning each semantic distinction into another table, transaction obligation or retrieval hop.

Add one foundation and one acceptance judgment for execution fit; strengthen the existing rules
on layout, library composition, reuse, assurance and lifecycle. Do not replace semantic rigor with
database conventions. Make the standard explicitly distinguish semantic authority from physical
representation, execution placement and enforcement frequency.

The intended result is **strong assurance with less repeated work and fewer moving parts**.
The most consequential correction is to make avoidable operational amplification a sufficient
reason to revise an architecture *before* a benchmark fails. Measurements still establish actual
speed, capacity and retrieval quality; static reasoning can establish a bad execution structure.

No finite principle set guarantees a globally optimal architecture. Performance, memory, build
cost, flexibility and assurance have real tradeoffs. “Apply maximally” should mean that every
supported scenario survives the architectural judgments—not that every property receives the
largest possible amount of machinery. This review supplies concrete criteria for making those
tradeoffs and rejecting clearly inferior realizations.

## 1. Scope and evidence

| Field | Boundary |
|---|---|
| Baseline | Clean `main` at `166764f9`; source inspection on 2026-10-05 |
| Subject | Core 3.2, code-intelligence profile 1.3, their review instructions, and the architectural choices they encouraged or failed to reject |
| Purpose | Design/target assessment of the standard; current implementation and proposed SurrealDB realization are revealing cases |
| Functional constraints | Validated immutable pinnable catalogs, attribution, evidence closure, typed uncertainty, deterministic declared analytics, bounded serving and the pinned embedding specification |
| Method | Current source and standard inspection; existing operational receipts; independent principles assessment and bounded source mapping; targeted primary literature and current documentation |
| Evidence strength | Diagnosis is Interface-checked source reasoning; amendments and replacement mechanisms are Proposed. Historical timings retain their original scope. No new runtime probe or performance measurement |
| Changes in this review | Assessment and handoff only. Governing principles, accepted ADRs, production code, dependencies and stores remain unchanged |

The [graph-native target review](design_review_graph-native-target_2026-10-05.md) establishes
the replacement direction. The [capability assessment](design_review_surrealdb-capabilities_2026-10-05.md)
establishes the relevant SurrealDB 3.3 mechanisms and limitations. This review consumes those
assessments rather than repeating their capability catalog or reopening the selected store.

The existing standard remains useful. Its explicit unknown states, assertion identities,
provenance, declared graph projections, independent controls and semantic ownership are preservation
constraints. The failure is not that the system became too understandable or too well typed.
The failure is that operational decomposition was allowed to mirror semantic decomposition too
closely, and additional enforcement was easier to justify than removing redundant enforcement.

## 2. What the existing principles do and do not explain

**Interface-checked, 2026-10-05.** The user's observation establishes that the standard and its
application were insufficient in practice. It does not establish that every expensive choice was
required by the standard, or that changing database brands alone fixes those choices.

### 2.1 Existing criteria that should already have helped

The [core](../design_principles/core/design-principles.md) explicitly permits derived physical
representations (FP-04, DP-01), layouts chosen for access patterns and coarse crossings (DP-10),
library capability use (DP-13/14), proportional machinery (DP-16), bounded resources (DP-20), and
risk-directed verification (DP-23). The [template](../design_principles/core/design-review-template.md)
already mentions cardinality/cost in its execution slot. These are not wholly missing concerns.

However, A1–A3 judge change locality, domain meaning and composition. None directly asks whether
the resulting physical plan is appropriate for the supported workload. DP-10 is a SHOULD with no
gate; G8 can pass when every local component uses a library, even if their composition repeatedly
moves and rebuilds the same data. G5 can pass when an operation fails honestly after exceeding a
resource limit. DP-22 prevents an unsupported speed claim but does not require a credible execution
shape when nobody claims speed. The template's priority text puts “measured costs” after other
architectural concerns, inviting reviewers to defer a structural cost finding until measurement.

The standard also gives incomplete positive selection guidance. It can reject duplicate semantic
owners without helping choose between two perfectly typed implementations whose physical work
differs by many database calls, full scans or materializations.

### 2.2 Concrete counterfactuals

| Inspected mechanism | Existing protection and remaining gap | Criterion that would change the decision |
|---|---|---|
| `ddl.rs:191` lowers each selected relation to a physical table; grants/receipts follow that granularity | FP-04/DP-01 permit other layouts, but one generated declaration-to-table path looks authoritative and easy to extend | Semantic distinctions need not multiply physical objects or lifecycle operations. Choose grouping by access, indexing and mutation behavior |
| `stage_validation.rs:179` issues reference anti-joins inside a loop over applicable reference fields | DP-03 gives every invariant an enforcement point; it does not compare the complete cost of those points | Resolve/check references in bulk over admitted indexes or shared streams; identify the necessary passes and eliminate incidental repetition |
| `validation_session.rs:457–497,615–651` finishes validators while the database transaction stays open | Effects and memory budgets are explicit, but expensive CPU work extends transaction lifetime | Separate long preparation/certification from the short visibility transition; transaction duration follows atomicity needs |
| `evidence_service.rs:557–627` scans and hashes whole derivation relations during a bounded explanation | Node/edge result bounds and evidence checks are correct locally | Bound examined work separately from returned output; reuse immutable certification and retrieve the relevant indexed frontier |
| Broad serving preparation and chained typed lookups | DP-10 already argues against fine-grained crossings, but each lookup has a clean contract | Expose complete request operations and preserve engine-side filtering, traversal and hydration opportunities |
| `lctx-model/build.rs:18–46` captures all model/macro Rust files, manifests and `Cargo.lock` | DP-09 favors fine invalidation but also accepts conservative safety; no cost attached to the conservative scope | Separate semantic contract, computation dependencies, build provenance and physical realization identities |
| `MemoryGeneration` concatenates and orders whole relations and collects all keys | DP-20 charges allocations and fails honestly; this does not establish a workable large-input design | Bound live representations and use streaming/spillable preparation where the supported workload exceeds memory |

The exact source is under `crates/lctx-postgres/src/generations/` except the explicitly named
model files; the source map is summarized in [§9](#9-evidence-map-and-limits). Existing validation
sessions already share compatible streams and cache frames. This is not a claim of one full scan
per invariant or of all 878 relations being processed by every stage.

The previous review records roughly an hour of reference checks after extraction, then a terminated
lifecycle connection. It does not isolate one expensive `finish()` experimentally. Source inspection
nevertheless establishes the transaction/CPU coupling and per-reference work. Likewise, 243 serving
hop call sites are an inventory, not calls per request; 8–10k lines are an estimated compensation
surface, not measured deletion savings. Domain `Id<T>` values are structurally keyed: the broad
source digest concerns model compatibility, not every entity's identity recipe.

### 2.3 Four different corrections are necessary

1. **Missing acceptance criterion:** operational fit of the assembled physical design.
2. **Underweighted existing criteria:** batching, actual-access layout, proportional verification,
   and total integration cost must be capable of determining acceptance.
3. **Misleading interpretation or wording:** semantic authority must not imply host execution,
   one physical form, per-relation scheduling, or revalidation at every layer. CI-07's categorical
   placement of relational work in query engines and topology in graph libraries needs refinement.
4. **Application failure:** local passing slices do not establish assembled fitness. This is
   already stated in the review rules; the assembled review must also examine work growth and
   interactions, not repeat only the local ownership analysis.

## 3. The criteria that should guide architectural choices

**Proposed.** These twelve lenses define concrete architectural properties. They are a coherent
coverage set for substantial design work, not twelve new checklists, services or mandatory records.
Only relevant lenses apply to a bounded change. Each includes an exception so that the cure does
not become another rigid recipe.

### 3.1 Separate semantic authority from physical organization

Declarations decide what values and operations mean, which combinations are valid, and which
identities and evidence must survive. Physical plans decide tables, indexes, batches, storage
families, operator order, process placement and scheduling. Derive the latter where useful; do not
require a one-to-one mapping. An ownership/module boundary need not be an I/O or materialization
boundary. A declaration is not inherently a unit of work, privilege or transaction.

This permits many semantic kinds in one indexed physical family, one kind in several specialized
read structures, and a model-owned operation executed by SurrealQL or a shared Rust kernel.
Conversely, a hot family may deserve its own table. Neither maximum consolidation nor maximum
normalization is the objective. Repeated physical bytes are acceptable when derived and useful;
independently writable meanings remain unacceptable.

**Audit:** Can an ordinary kind be added without adding a store protocol? Can the layout or
execution placement change without redefining meaning? Which physical distinctions earn their cost?

This is the general form of “the types judge the graph; they do not run its operations.” Data
independence is not peculiar to graph databases: Codd's original paper explicitly separates user
meaning from internal organization. The inference here is to apply that separation to operational
granularity as well as storage. [Codd, 1970](https://www.seas.upenn.edu/~zives/03f/cis550/codd.pdf)

### 3.2 Choose work that grows with the necessary input

For a material operation, understand the inputs it actually needs, its access paths, fan-out,
passes, sorting/hashing, serial dependencies and completion conditions. Compare equivalent
realizations by the work they remove, not just by how cleanly they package it. A small output is
not evidence of small work. A single RPC can conceal a full scan; a fixed graph depth can conceal
enormous width. A budget that promptly refuses every realistic input is safe but unfit.

For compilation, full-input processing is expected. Work over all admitted rows/references and
necessary sorts is defensible; rescanning them separately for incidental lifecycle boundaries is
not automatically defensible. For a small evidence request, indexed lookup plus the examined
frontier is a plausible shape; corpus-wide integrity reconstruction on each request is not.
Global analytics may legitimately require the full declared graph. Do not promise every query is
output-linear or every compilation is linear.

Useful variables include rows, bytes, relation kinds, reference fields, graph degree, input width,
retained snapshots and concurrent requests. A schematic cost decomposition is:

`necessary computation + repeated scans + serialization/transfer + synchronization + recovery work`.

It is a reasoning aid, not a new estimator to implement. Static source can show a nested factor
or serial round trip; measurements determine its coefficient and practical breakpoints.

**Audit:** What doubles when the corpus doubles? What grows when only the declaration count grows?
What remains on the critical path? Does a small answer become slower because unrelated data grew?

### 3.3 Select libraries for composed capabilities and preserve optimizer visibility

Choose the mechanism that can perform the complete useful operation with the least additional
machinery. Evaluate interacting capabilities, not a list of individual checkmarks. Graph traversal,
eligibility, full-text/vector candidates and evidence hydration in one engine can be more valuable
together than four individually good components joined by application loops.

Keep filtering, projection, grouping, ordering, limit and traversal structure visible to the
selected engine where their semantics are supported. An abstraction that reduces every operation
to fetch-one-record can prevent an otherwise capable engine from planning bulk work. An opaque UDF
or WASM module may remove transport yet hide predicates from the optimizer; place a kernel after
selective retrieval unless its own contract supports efficient selection. A backend-neutral API
that discards needed native capability is not successful replaceability.

DataFusion's provider contract is a concrete positive example: projection, filter and limit can
reach the scan, while the provider states which filters it handles. A wrapper that eagerly loads
everything would discard that opportunity. This is evidence of the architectural mechanism, not
a claim that the current integration implements every pushdown.
[Official DataFusion documentation](https://datafusion.apache.org/library-user-guide/custom-table-providers.html)

**Audit:** What can the engine optimize across the boundary? Which application loops disappear?
Does the built-in actually preserve the required semantics, or merely have a matching name?

SurrealDB's native ranking is a useful counterexample: the capability review found that built-in
fusion does not by itself implement member/family collapse and witness semantics. A small owned
semantic fold may be better than adopting the wrong built-in or creating a general translation DSL.

### 3.4 Put computation near its data and cross expensive boundaries deliberately

Minimize repeated movement and re-encoding of the working set. Co-locate selection and reduction
with indexed data; move reduced results to consumers. Use batches or complete operations across
storage, process, language and device boundaries. Keep local function/module boundaries where they
improve reasoning without imposing expensive transport.

Placement considers both data residence and computational suitability: a database-local operation
can be ideal for selective traversal; one bulk export to a dense Rust graph can be better for
repeated whole-graph SCC or fixpoint work. Existing libraries, predictable resource behavior,
isolation and debugging may outweigh avoided transport for a particular kernel. Rust ownership is
compatible with generated query fragments, an owned SurrealQL implementation, or qualified shared
WASM. It does not require executing everything on the application host.

**Audit:** How many copies and crossings does the same data incur? Can selection/reduction happen
before transport? Does moving the kernel create a less controllable runtime or duplicate semantics?

### 3.5 Design the live working set and intermediate representations

Choose representations for the operation: columnar batches for scans, compact adjacency and local
indices for graph algorithms, indexed records for navigation, and documents for values consumed
together. Keep canonical identity mappings. Avoid carrying full source bytes, diagnostic strings
and wide provenance payloads through an inner loop that needs only IDs and weights. Hydrate them
when the consumer needs them.

Bound simultaneously resident inputs, intermediate results, copies, indexes, caches and output.
Pipeline compatible work in chunks; materialize when it enables reuse, ordering, atomic visibility
or algorithm requirements. Use spill/external ordering when justified by the supported size, with
backpressure through the producer chain. “In memory” and “zero copy” are not substitute designs.
Small fixed inputs can reasonably use simple collection and sorting.

The X100 paper demonstrates why both tuple-at-a-time overhead and full-column intermediate
materialization can be poor choices, motivating bounded vectorized pipelines. It supports this
tradeoff, not a speed prediction for library-context.
[Boncz, Zukowski and Nes, 2005](https://www.cidrdb.org/cidr2005/papers/P19.pdf)

**Audit:** What is live at peak? Which intermediate exists only because of a component boundary?
Does the chosen representation support the access pattern without repeated decoding or allocation?

### 3.6 Amortize stable work and make invalidation no broader than justified

Separate acquisition, preparation, index construction, certification and query execution. Reuse
immutable prepared inputs and their established properties. Share compatible validation streams
and graph topology rather than reconstructing each for every consumer. Distinguish a semantic
contract revision, result-affecting kernel dependencies, physical realization and full build
provenance; one all-source fingerprint should not answer all four questions.

Soundness still wins over an unsupported narrow dependency claim. A conservative fingerprint is
legitimate where dependencies are opaque, but its rebuild scope is an explicit cost/tradeoff to
improve when material. Do not remove algorithm/version/configuration inputs that affect a result.
Introduce incremental computation when repeated changes make it worthwhile; an immutable full
rebuild may be simpler for the first product. A cache is justified by reuse, not merely by the
availability of a caching library.

**Audit:** What actual change invalidates this artifact? What can remain prepared after a layout,
source-comment or unrelated dependency edit? Does cache bookkeeping cost more than recomputation?

### 3.7 Design assurance to detect distinct failures with economical checks

Place checks where the relevant facts and trust assumptions are available. Prefer validated
construction, shared admission, immutable artifacts and compact checkable witnesses to repeated
reconstruction of an entire producer. Each additional enforcement boundary must address a named
failure class or materially reduce failure/recovery cost. Repeated execution of the same logic
does not by itself supply an independent semantic oracle. Repeated enforcement can legitimately
cover a new trust transition, changed state or transient execution failure.

For this product, the proposed chain is: admit typed graph content; check relational/global
invariants over that artifact; persist privately and finish required indexes; quiesce/drain writers
and freeze content plus answer-affecting definitions; reconcile the frozen stored realization
through the actual reader/codec and physical adjacency; publish. An equivalent protected immutable
view can supply the same guarantee. Reuse that certification while its premises remain valid.
Request-time work checks pinning, qualification and the relevant evidence path.
It should not recertify unrelated immutable content. Cheap schema/endpoint checks still catch
different bugs and can remain valuable defense in depth.

A digest establishes equality to a particular expected representation, not semantic correctness.
A certificate must bind the relevant graph, operation, parameters and verifier assumptions, and
its checker must establish the claimed property. A path witness proves that path exists; it does
not prove absence of all other paths. A claimed fixpoint needs the actual closure/stability check;
some obligations may still require global work. Where no cheaper sound check exists, retain the
necessary validation. Independent semantic controls continue to challenge producer and model.

The end-to-end argument provides useful grounding: lower-layer checks can improve efficiency and
recovery, but they do not automatically replace an application-level correctness check. Their
placement and additional cost are design choices. Applying that reasoning here is an architectural
inference, not a theorem about this validator.
[Saltzer, Reed and Clark](https://web.mit.edu/6.033/2002/wwwdocs/papers/endtoend.pdf)

**Audit:** What distinct failure does each check catch? What can invalidate a prior check? Can the
same guarantee be established by admission plus one reconciliation and a smaller independent check?

### 3.8 Match transaction and recovery units to effects

Keep long computation outside the transaction that makes a result visible unless the operation
truly requires a live transactional read/write set. Private immutable construction permits a small
publication transition. Separate durable progress from visibility; idempotent ingest batches or
rebuildable segments can bound retry work without per-relation leases and epochs.

The atomicity boundary must still cover the promise. Sealing includes mutable executable
definitions when they affect answers. If the chosen store cannot atomically publish all physical
parts, use private completed artifacts plus a visible handle, with an explicit crash argument.
Already visible snapshots must not depend on unfinished indexes or uncommitted external blobs.
Trust the actual operator scope; do not invent a hostile administrator threat model for a
single-operator rebuildable catalog.

**Audit:** What does a crash just before/after visibility leave? How much completed work must be
repeated? Why must this lock/connection/lease live for the duration of this computation?

### 3.9 Coordinate capacity, queues and contention across the operation

Bound admitted work, not only worker count or returned bytes. Budget CPU, memory, I/O, connection
occupancy, queue length and retained state together where they are material. Propagate cancellation
and backpressure; define which work can be interrupted safely and when its resource charge ends.
Avoid nested independently maximized pools and a connection held while unrelated CPU work runs.

Bulk compilation and interactive serving have different scheduling needs. Separate or prioritize
their resource use when they contend; this need not mean separate services. Partition mutable
work by independent attempts/keys where useful. Prefer immutable shared readers to unnecessary
coordination. More threads, replicas, sharding or distributed queues are responses to a demonstrated
need and suitable partitioning—not evidence of a better architecture on their own.

**Audit:** What happens when requests arrive faster than completion? Can one high-degree query or
build starve small requests? Does a cancelled opaque kernel keep running after its budget is released?

### 3.10 Reuse a semantic artifact through purpose-built physical views

A canonical artifact should make new consumers inexpensive: petgraph analytics, inspection,
Neo4j exploration, Arrow/DataFusion computation, or a relational export can share admitted identity,
provenance and meaning. Define export by its universe, endpoint roles, multiplicity, isolates,
typed uncertainty and declared losses. A projection is a consumer-specific realization, not a
second authority and not necessarily another persistent database.

Precompute compact repeated joins, indexed evidence access or stable classification where this
reduces repeated work enough to justify construction and storage. Snapshot immutability makes
these views easier to reason about. Do not eagerly materialize every transitive path, all-pairs
relation or imagined destination. Keep the format sufficient to reconstruct needed views; build
each when a real consumer exists. Even a graph-native store still benefits from a compact
algorithm-specific projection.

**Audit:** Can another consumer use the artifact without reconstructing its meaning from lifecycle
tables? Which read pattern pays for this extra index/view, and what is its build/update cost?

### 3.11 Make work bounds and exactness coexist honestly

Keep semantic scope, operational effort and response size distinct. A query scoped to three hops
can be complete for that question; a work budget exhausted at two hops is partial. An approximate
candidate search may still return exactly checked evidence for each selected result. That does
not make the candidate ranking exhaustive. Decide where exactness is necessary for meaning and
where declared approximation serves discovery without weakening evidence.

Eligibility must precede a top-k cutoff when the question is top-k eligible results. Collapsing
retrieval units into members can change the ranking universe; bounded overfetch alone does not
establish exact member top-k. Preserve complete mandatory claim/qualification structure; refuse
an unrepresentable packet instead of trimming away its meaning. Optional source/explanation
expansion can be independently partial. These are operation contracts, not UI-only labels.

**Audit:** What question is complete when the operation stops? Are approximate ranking and exact
evidence separated? Are limits applied before or after the semantic operation they must preserve?

### 3.12 Minimize the total machinery needed to build, operate and evolve the product

Evaluate code plus engines, copies, configuration, roles, migrations, retries, telemetry, build
dependencies, deployment and upgrade/recovery burden. A powerful library can remove several
subsystems; a small library can add an awkward lifecycle. Source-line reduction and feature count
are weak proxies. An experimental optional extension should not become a mandatory dependency of
the basic service merely because it is attractive.

Prefer operational choices appropriate to one operator and regenerable data. Preserve focused
testability, fast local iteration and useful diagnostics: long dependency builds, broad invalidation
and mandatory infrastructure for local semantics also slow improvement. Use selected engine plans
and metrics to expose scans, candidate counts, fan-out, queueing, spills and retries. Instrument
at useful boundaries; do not make one receipt/span per tiny fact the price of explainability.

**Audit:** What operations and failure modes disappear with this choice? What new obligations
appear? Can a native capability be upgraded or isolated without rebuilding unrelated authority?
Which mechanism could be removed while preserving the actual supported guarantees?

## 4. What SurrealDB teaches—and what it does not

**Proposed application; specific capabilities Interface-checked in the capability review.**

| Architectural opportunity | SurrealDB realization worth using | General criterion and limitation |
|---|---|---|
| Persist useful connected structure | Identity-bearing relation records, typed documents, role-labelled participants | Preserve the domain while choosing navigable physical forms; not every declaration is a binary edge |
| Compose retrieval near data | Graph paths, filters, lexical/vector indexes and coarse functions | Preserve planning opportunities and reduce crossings; one query can still scan or explode |
| Select before expanding | Supported indexed eligibility, 3.3 bitmap fusion, selected INLINE fields/adjacency | Reduce examined candidates and bytes; verify the supported predicate shape, index build/version and degree behavior |
| Reuse extraction and admission | Native stored artifact feeding petgraph, inspection and explicit exports | Multiple physical consumers with one semantic authority; export adapters and projection contracts still exist |
| Flexible kernel placement | Host Rust, SurrealQL, or qualified shared Rust/WASM | Co-design ownership and placement; an extension's ABI, isolation and resource behavior are part of its cost |
| Less publication machinery | Private snapshot, index completion, writer drain/seal, stored reconciliation, short handle publication | Exploit immutability; include functions/analyzers/module artifacts in the served realization |

The first, fourth and sixth rows do not require SurrealDB. PostgreSQL can support a decoupled
compiler, bulk/set processing, derived read models and short publication. The graph/search/function
combination makes SurrealDB a promising fit for this product, not a universal architectural rule.
Current Rust algorithms, typed unknowns and independent semantic controls are capabilities the
database does not automatically supply.

The same criteria reject a bad SurrealDB implementation: mechanical one-table-per-declaration
lowering without access/lifecycle justification, per-record RPCs, broad startup loading,
unbounded recursive traversals, global integrity scans per
answer, or a query function that only conceals those costs. They also reject using `INSERT IGNORE`
or successful transport status as proof that all requested data was stored. Native permission,
truncation, write-result and transaction semantics remain part of qualification. A database choice
cannot discharge the application contract.

The second-order benefit is a reusable semantic substrate: better debugging, alternative analytics,
additional destinations and focused replacement without new interpretation layers. It is valuable
because the representation is admitted, interpretable and exportable—not because every possible
integration is enabled at once. No export promises destination parity without a mapping.

## 5. Concrete proposed amendments to the standard

**Proposed wording, not adopted policy.** Keep FP-01–FP-06, DP-01–DP-24 and G1–G8 stable.
Add FP-07 and A4; modify the existing relevant rules instead of adding twelve new MUSTs. A4 is an
architectural judgment, not a benchmark command or a ninth correctness gate.

### 5.1 Governing objective and new foundation

Add to core §1:

> Design the semantic contract and its physical realization together. Preserve correctness,
> fidelity and the declared assurance scope while choosing mechanisms that minimize unnecessary
> work, data movement, coordination and lifecycle machinery for the supported workload. Domain
> authority does not determine physical layout, execution placement or enforcement frequency.
> Compare the composed system, including preparation, normal execution, failure and change.
> Prefer a simpler conforming realization that removes substantial work or machinery; state a
> concrete benefit when retaining the more expensive choice.

Revise §1's technology sequencing: establish functional intent and semantic obligations, then
co-design boundaries and candidate library realizations. Do not freeze a backend-neutral interface
before examining the capabilities it must preserve. Meaning remains authoritative; implementation
capabilities inform how best to realize it.

**FP-07 — Execution fits the workload**

> Choose representations, algorithms, access paths, execution placement and lifecycle granularity
> for the operations and input sizes the system supports. Work should follow necessary data and
> dependencies, with explicit treatment of skew, intermediate size, concurrency and failure.
> Avoid incidental per-item crossings, repeated whole-input processing, unnecessary materialization
> and coordination. Preserve optimization opportunities in composed libraries. Reuse stable work
> and established immutable properties; every extra enforcement or recovery mechanism addresses
> a concrete failure or operational need. Semantic ownership and physical decomposition remain
> separately changeable. A bounded refusal is honest, but does not by itself make the architecture
> fit a workload it claims to support.
>
> **Audit:** What work, movement and coordination does a complete operation require? How do they
> grow with relevant input dimensions and concurrency? Which costs are required by the result or
> guarantee, and which arise only from the chosen boundaries? What simpler conforming realization
> was considered?

**A4 — Fit execution to the supported workload**

> For the selected operation and growth/failure scenarios, establish a credible physical execution
> route: access paths, necessary versus repeated work, important crossings and intermediates,
> resource/transaction lifetimes, and recovery scope. The composed architecture must not introduce
> unjustified amplification or predictably fail its supported workload. Compare a materially
> simpler or more native realization where one is credible. Static evidence can establish a
> structural violation; measured latency, throughput and capacity claims require measurements.

A4 starts from a brief workload premise drawn from functional intent: representative operations,
relevant size/skew/growth, concurrency/deployment constraints, and resource envelope where material.
For this assessment that includes multi-million-fact compilation, hundreds of semantic kinds,
repeated small evidence requests, high-degree outliers and a single-operator deployment. It does not
invent a capacity SLA or require every future scale. Supported scope cannot be reduced to convenient
fixtures while still claiming the real-library product. This premise belongs in the existing scope
and scenario discussion, not a new document or exact cost model.

A4 uses the existing satisfied/violated/unresolved/not-applicable judgments. An unresolved
physical premise material to supported use prevents architectural acceptance of that use. It does
not require measuring every implementation constant. Scope exclusions must state what is excluded;
“performance unmeasured” cannot excuse a known bad work shape. Proposed acceptance remains distinct
from release/runtime qualification.

### 5.2 Amend existing rules at their owners

The following are targeted additions or replacements, not parallel policy definitions.

| Owner | Proposed substantive amendment |
|---|---|
| FP-01, FP-03, FP-04 | Clarify that semantic/module composition need not introduce process calls, store operations or materialization. Lowerings may group or split declarations and fuse operations while preserving owned contracts |
| DP-03 | Add: “Select enforcement points by the failure classes and trust transitions they cover. Reuse established immutable validity where its premises remain true; additional enforcement should contribute distinct protection or cheaper recovery.” Keep every critical invariant's honest enforcement |
| DP-08 | Add work/completeness semantics to material operation contracts: relevant input universe, supported scope, operational versus output limits and failure behavior. Do not require a new operation registry or exact cost model |
| DP-09 | Distinguish semantic identity, preparation/result dependencies, physical realization and full build provenance. State material over-invalidation as a tradeoff; retain conservative keys where finer dependencies cannot be justified |
| DP-10 | Strengthen the architectural requirement to choose layouts and granularity for actual operations. Explicitly cover per-declaration/per-stage crossings, shared scans, late hydration and avoidable intermediate materialization; do not prescribe one batch size or ban scalar operations |
| DP-13/14 and §F | Evaluate composed library capabilities, optimizer visibility, data movement, physical access, lifecycle and failure behavior. A built-in is preferred when its semantics and total integration fit; do not force a wrong built-in or lowest-common-denominator wrapper |
| DP-16 | Include generated and library-induced runtime machinery in the economy test. One generated definition can expand into excessive objects/operations. Minimize required mechanisms and repeated work as well as independent semantic decisions |
| DP-19 | Separate preparation, durable progress, sealing and visibility where the contract permits. Match transaction/lock lifetime and retry unit to the actual effect; coherent publication need not mean one long transaction |
| DP-20 | Distinguish work, resident-state, queue, transport and output budgets. Include backpressure, admission, cancellation/drain and contention between workload classes. Explicit refusal satisfies truthfulness, not supported-scale fitness |
| DP-21 | Prefer diagnostics that explain complete operations and their cost drivers. Preserve provenance through compact references and selective hydration; avoid observability whose cardinality dominates useful work |
| DP-22 | Add: “Unmeasured speed is a hypothesis; a demonstrated avoidable scan, crossing or amplification is architectural evidence. Lack of a speed claim does not exempt a supported operation from A4.” |
| DP-23 | Separate admission/semantic verification, persistence reconciliation, certificate checking and independent audit/testing. Repeated producer execution is not automatically stronger assurance; preserve checks that address different failures |

DP-10's concrete techniques can remain SHOULDs: not every efficient algorithm needs every technique.
FP-07/A4 supplies the acceptance consequence when their omission creates material operational harm.
This is preferable to elevating every optimization idiom into an unconditional MUST.

### 5.3 Correct the domain profile's mechanism assumptions

| Profile owner | Proposed correction |
|---|---|
| CI-05 | A canonical artifact may itself be graph-native. Each analytic/query view still declares its topology and universe; it need not reconstruct or persist another graph to satisfy the principle |
| CI-07 | Retain distinctions among relational algebra, topology and program-analysis transfer/fixpoint semantics. Choose host library, query engine or qualified shared kernel by semantic fit **and** physical cost/locality. Do not assign whole task categories permanently to one runtime |
| CI-08 | Add examined work, branching/degree, intermediate cardinality and bytes to output bounds. Permit compact reused views/certificates when they reduce repeated work; preserve explicit partiality and avoid unnecessary all-path/all-pairs expansion |
| CI-13 | Pin the complete served realization for the declared consumer lifetime, including multi-call evidence retrieval, resources and continuations. Bind returned references, cursors and cache tokens to that realization, including semantic content and answer-affecting functions, analyzers and embedding/index specifications. Incompatible reuse is refused or deliberately reselected. Process-lifetime pinning is one implementation, not the domain law |

Proposed CI-07 replacement core paragraph:

> Place each analysis where its semantic operations and physical access pattern fit. Set processing,
> topology algorithms and program-analysis transfer/fixpoint operations remain distinct, but may be
> implemented by a suitable query engine, graph library, columnar runtime or shared kernel. Account
> for data movement, preparation, optimizer support, resource behavior and reuse across the complete
> operation. Plain reachability does not establish a dataflow result; moving execution does not move
> semantic authority or relax its contract.

Do not weaken CI-01–04, CI-06, CI-09–12 or the profile gates. Pinned embeddings remain the same
semantic requirement under a different physical search realization.

### 5.4 Change what a review may accept, without adding a process

Use the existing review slots. Slot 4 assesses physical work as well as semantic composition;
slot 5 selects relevant growth/skew/concurrency/failure scenarios beside extension scenarios;
slot 8 compares composed capabilities; slot 12 settles A4 beside A1–A3. No new artifact type,
standing code audit, exhaustive flow trace or benchmark gate follows.

Replace the template's priority wording with:

> Prioritize correctness/fidelity failures and architectural choices that make a supported
> workload infeasible, unstable or operationally disproportionate. Structural cost evidence can
> establish such a failure before measurement. Assess change barriers, repeated semantic ownership
> and other material complexity in the same functional context; prioritize by consequence rather
> than by whether the evidence is a benchmark.

Add to the template's finding consequences: avoidable repeated full-input work, unnecessary
crossings/materialization, excessive live state, unbounded queues/fan-out, overbroad invalidation,
and resource/transaction lifetimes mismatched to their effects. Update acceptance from A1–A3 to
A1–A4. Preserve independent correctness/fidelity gates and the distinction between a local slice
and the assembled architecture. A documented tradeoff must explain the benefit retained; a note
that a cost exists does not automatically justify it.

## 6. Scenarios that distinguish good application from slogans

**Proposed counterfactual assessment, not executed tests.** These scenarios are examples to choose
from during a substantial review, not a mandatory test suite.

| Scenario | Consequence the revised standard should require |
|---|---|
| Add 100 ordinary assertion kinds without increasing the fact volume | Model changes and required query/codec definitions follow their owners; no automatic proliferation of per-kind lifecycle objects or serving startup work |
| Increase unrelated catalog content tenfold; ask for the same bounded evidence packet | No request-time global rehash merely to retain confidence in an unchanged immutable artifact; indexes and examined frontier drive retrieval, with real limits |
| Encounter one very high-degree node | Degree/edge/byte/work bounds and an explicit partial/refusal outcome; depth alone is insufficient. INLINE/caching choices account for high-degree behavior |
| Compute global SCCs or a finite fixpoint | Use the complete declared universe even for a small output; prepare reusable compact topology and retain deterministic ordering/certificates. Do not push an unsound selector into the analysis |
| Change only a layout/index or unrelated source comment | Relevant realization/build provenance may change; semantic identity and recompilation change only where their contract/dependencies require it |
| Crash after bulk loading but before publication | A completed old snapshot remains valid; private work is resumable/idempotent where useful or safely discarded. No partly indexed new snapshot is advertised |
| Serve while a new snapshot compiles | Shared capacity has admission/prioritization or a stated no-concurrency scope; no hidden assumption of unlimited pools/connections |
| Replace host retrieval with a database function | Same eligibility, witnesses, uncertainty and limits; fewer crossings only count as an improvement if server-side examined work and effects remain appropriate |
| Export to petgraph, Neo4j or PostgreSQL | Preserve canonical IDs, isolates, parallel assertions, roles and attributed meaning through an explicit consumer mapping; no independently mutable source of meaning |
| An optional WASM extension is unavailable or incompatible | Basic admitted storage and supported serving remain independently usable unless the product deliberately made that kernel mandatory; no silent semantic fallback |

The optimization order should normally be: remove unnecessary work and obligations; choose suitable
algorithms/access paths; improve placement/representation; amortize stable work; then tune batching,
parallelism and low-level kernels where needed. This is a useful default, not a prohibition on a
clearly dominant local optimization. It prevents adding concurrency to compensate for unnecessary
scans or adding caches to conceal an unsuitable query interface.

## 7. Findings, alternative remedies and disposition

These findings concern the standard. Existing product findings remain with their current review/
plan owners; the rows below do not create a second product defect ledger.

<a id="f01"></a>
**F01 — Architectural acceptance omits physical execution fit.** A1–A3 can all hold while work
scales with incidental declarations, unrelated corpus data or long-lived transaction state.
DP-10/20 and the template's cost prompts are insufficiently decisive. Add FP-07/A4 and the scoped
work/growth scenarios. Closure is an amended standard/template that can reject the counterfactuals
in §6 without waiting for a latency claim. **Disposition: Deferred to standard adoption; owner:
core/template. Trigger: adopting this assessment.**

<a id="f02"></a>
**F02 — Authority, physical granularity and runtime placement are not distinguished strongly
enough in selection guidance.** Existing permissions for derived layouts coexist with an
authority-first sequencing and categorical CI-07 placement that can favor mechanical one-to-one
lowering or host-only execution. Clarify core §1/FP-01/04 and CI-05/07; evaluate composed native
capabilities and optimizer-visible contracts. Closure is a scenario where one meaning has several
legitimate efficient realizations without a second owner or mandated universal abstraction.
**Disposition: Deferred to standard adoption; owner: core/profile. Trigger: adopting this assessment.**

<a id="f03"></a>
**F03 — Assurance rules do not sufficiently distinguish additional protection from repeated
work.** Invariant enforcement and evidence closure are explicit; the physical cost and distinct
failure coverage of repeated enforcement are not. Refine DP-03/19/23 with admission, immutable reuse,
reconciliation and certificate semantics; preserve independent controls. Closure is a coherent
failure argument for removing repeated work without claiming that a digest proves correctness.
**Disposition: Deferred to standard adoption; owner: core with profile applications. Trigger:
adopting this assessment.**

<a id="f04"></a>
**F04 — The review's prioritization and growth scenarios underweight operational architecture.**
The template emphasizes semantic extension scenarios and “measured costs”; finite/bounded failure
can obscure inability to serve a supported input. Amend slots and priority text as in §5.4, using
existing review cadence. Closure is a review that considers assembled cardinality, skew, concurrency
or recovery where material and distinguishes structural evidence from measured speed.
**Disposition: Deferred to standard adoption; owner: template/profile review guidance and skills.
Trigger: adopting this assessment.**

| Alternative | Judgment |
|---|---|
| Add only “performance matters” or mandatory benchmarks | Insufficient: identifies an outcome or detects damage late without guiding representation, placement or assurance choices |
| Apply existing DP-10/13/16/20/23 more forcefully, no standard change | Necessary but insufficient: retains the missing acceptance judgment and ambiguous placement/evidence priority |
| Add a separate mandatory checklist/gate for every cost dimension | Reject: replaces one form of operational burden with another and turns context-sensitive techniques into universal rules |
| Add FP-07/A4 and targeted amendments to existing owners | Recommended: comprehensive physical criteria with one coherent new architectural judgment and unchanged bounded review cadence |
| Prescribe graph databases, native execution everywhere, or one universal physical form | Reject: overfits this case, loses algorithm/semantic fit and can recreate the same amplification inside another engine |

F01 is the acceptance gap; F02 and F03 supply important architectural content; F04 makes that
content consequential in review. Adopt them coherently rather than adding A4 while leaving the
template's priority and the profile's placement assumptions unchanged.

## 8. Judgment and authority route

For the **proposed amendment package**, A1 is satisfied: changes belong to existing core/profile/
review owners, with repository bindings retaining local technology choices. A2 is satisfied at
Proposed scope: the package explicitly models semantic authority, physical realization, work,
trust transitions and operation completeness. A3 is satisfied: it enables recomposition and
substitution without requiring another universal framework. These judgments assess the proposal;
they do not recertify the current product.

For the **current standard's sufficiency**, the decision is **Revise**, supported by F01–F04.
FP-01–06 and fidelity principles remain useful preservation constraints. DP-10/13/16/20/23 need
the refinements above; existing wording already permits several recommended improvements.
Proposed A4 is not retroactively an adopted requirement, and its absence is not called a historical
conformance violation.

G1/G2/G4/G6/G7/G8 pass for the proposed guidance's own scope: it preserves one semantic authority,
fidelity and explicit effects, retains reuse contracts, labels its evidence and considers established
capabilities. G3/G5 and CI-G1–CI-G3 remain preservation requirements for the product; this document
changes no executable admission/publication/answer path and supplies no new runtime qualification
against those gates. They are not applicable as executed acceptance claims in this document-only
scope. Current product acceptance remains with its existing coordinators.

The next decision is adoption of this package. Follow the existing ADR route for the standard
revision; update the core, template, manifest, profile/guidance, binding and process skills together,
preserving existing IDs and historical assessment versions. Amend the owning DESIGN review policy
where its meaning changes. Keep generic principles repository-agnostic; the SurrealDB applications
remain in this review and the product architecture. Other repositories are not implicitly changed.
No new register, universal cost model, per-change review or compulsory probe is proposed.

The recommendation would change if the added judgment could be satisfied by relabelling the current
work without changing its physical shape, or if it systematically demanded complex machinery for
small bounded tasks. The audit questions, exceptions and concrete scenarios are intended to prevent
both. Actual implementation measurements may change a selected index, batch size, kernel placement
or engine; they would not remove the need to judge operational architecture explicitly.

## 9. Evidence map and limits

**Read/assessed 2026-10-05.** Source paths identify the inspected baseline; line numbers are navigation
hints, not permanent interfaces.

| Evidence | What it establishes |
|---|---|
| [Core principles](../design_principles/core/design-principles.md), §1, FP-01–06, DP-01/03/09/10/13–16/19–23, §E | Existing concern coverage and the A1–A3 acceptance gap |
| [Core template](../design_principles/core/design-review-template.md), priority/decision text and slots 4/5/8/12 | Current evaluation emphasis, existing cost prompts and correction points |
| [Code-intelligence profile](../design_principles/profiles/code-intelligence/principles.md), CI-05/07/08/13; [review additions](../design_principles/profiles/code-intelligence/review.md) | Graph/placement/pinning assumptions and existing semantic constraints |
| `crates/lctx-postgres/src/generations/ddl.rs:191`, `stage_validation.rs:179`, `validation_session.rs:457,615` | Per-declaration physical lowering, per-reference checks, shared scans and CPU completion within transactions |
| `crates/lctx-postgres/src/generations/evidence_service.rs:557`, `crates/lctx-postgres/src/connection_options.rs:25` | Whole-relation hashing within bounded explanation; configured transaction lifetime constraint |
| `crates/lctx-model/build.rs:18`, `crates/lctx-model/src/domain/memory.rs:165–220` | Broad model fingerprint; charged whole-relation concatenation/order and key collection |
| [Target review §2](design_review_graph-native-target_2026-10-05.md#2-what-does-not-scale-in-the-present-operations) and its source inventories | Previously mapped operational surfaces and attributed failed real-library receipt; not a new timing run |
| [Capability review §§2–5](design_review_surrealdb-capabilities_2026-10-05.md#2-target-responsibilities-and-composed-operations--slots-24) | SurrealDB mechanisms, exact limits, second-order uses and proposed application boundaries |

Primary external grounding is linked at the relevant criteria: Codd for data independence, X100
for execution/materialization tradeoffs, Saltzer/Reed/Clark for economical placement of assurance,
and official DataFusion documentation for optimizer-visible scan contracts. Context7 resolved
`/apache/datafusion` and queried provider pushdown on 2026-10-05; current docs are illustrative,
not a qualification of this repository's locked integration. The detailed SurrealDB version/source
evidence remains in its capability review. These sources support mechanisms, not a promised speedup.

Independent assessment separated missing criteria from violations of existing intent and recommended
one operational-fit judgment rather than many gates. Bounded source mapping confirmed the distinction
between model fingerprint and domain identity, output limits and examined work, historical timing
and scale evidence, and existing compatible validation-stream reuse. The principal review integrates
those judgments; no agent performed a new product run.

Final independent review: **Accept scoped** for the proposed amendment package, 2026-10-05.
Corrections preserve reconciliation after writer drain/sealing, pinning across the declared
multi-call consumer lifetime, and a workload premise derived from the actual functional intent.
The standard itself still requires revision through its adoption route; this acceptance does not
adopt the amendments or qualify the product.

Documentation verification: `just docs-check` **passed** (309 canonical pages, zero link errors).
The pre-existing unclosed HTML-tag warning in the earlier native-realization evidence README
remains outside this change. Product tests, real-library compilation and benchmarks are **not_run**
for this assessment-only scope.
