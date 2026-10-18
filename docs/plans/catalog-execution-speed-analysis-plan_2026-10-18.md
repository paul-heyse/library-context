# Catalog execution speed: analysis and reusable products

**Proposed implementation plan · 2026-10-18 · baseline `9c457a0c`.** This companion develops
analysis-side execution changes for the [execution-speed coordinator](catalog-execution-speed-plan_2026-10-18.md).
It implements the corrective direction of review F01, F06, F08 and F09, contributes the upper
kernel portion of F07, and resolves the relevant I03/I04 questions. The coordinator's
[§6 checkpoint](catalog-execution-speed-plan_2026-10-18.md#6-sole-disposition-and-current-checkpoint)
is the sole owner of disposition and current evidence. Package descriptions below are planned
work, not a second progress ledger or claims that the replacements already exist.

## 1. Purpose, authorities and implemented foundation

The [comparative review](../design_review/reviews/design_review_catalog-execution-speed-comparative-target_2026-10-18.md)
and [recommendation crosswalk](../design_review/evidence/2026-10-18_catalog-execution-speed-comparative/recommendations.md)
correct the premise that the compiler is simply serial or rebuilds every graph for every root.
Dependency-ready owned work and shared hydrated graphs already exist. The target removes specific
reconstruction, rich copying, eager contextual work and optional foreground retention while
preserving the product's exact code-intelligence meaning.

The [semantic model](../design/sections/semantic-model.md) and executable `lctx-model::domain`
own projection universes, closure rules, catalog dependencies, method/configuration meaning,
coverage and value identity. The compiler companion owns private immutable completed sources,
resource/spill lifetime, selected roles and visibility. The native companion owns durable
requested-frontier materialization, admission, independent receiving and publication. This plan
consumes those contracts; it does not add another editable graph, certificate authority or cache.

Inspected foundations at the baseline:

- `domain/projection/normalization.rs::encode_input` constructs a materialized graph, encodes
  canonical snapshots and drops the materialization. `cpg-core/src/analysis_graphs.rs::load`
  reads selected snapshots and reconstructs shared graphs for upper consumers.
- `lctx-compiler-store/src/consumed_rows.rs` already uses indexed native frontiers, but contextual
  pair queries execute their source-kind universe to EOF and build shared ordering/adjacency.
  This work is shared per edge source; it is not a separate whole-graph reconstruction per root.
- `cpg-core/src/catalog_core.rs` hydrates one root-window union, then `selected_partition`
  creates owned rich copies. `domain/catalog/build.rs::selected_copy` clones selected rows.
- Existing `RowsView` provides borrowed whole/exact selected membership, ordered unique-ID
  checking and refusal of absent premises. Use this boundary rather than inventing a universal
  data-view framework.
- `catalog_products.rs` computes value dependencies and performs lookup after rich preparation;
  C0 and Retrieval await pure-product retention before required emission. Whole-producer
  retention elsewhere already uses accepted owned children.

Core 3.3, efficiency heuristics 1.0 and code-intelligence profile 1.5 govern the work. H5–10,
H12–17 and H21–28 motivate the physical route: scope necessary work, reuse unchanged preparation,
keep one charged backing where useful and avoid paying optional durability on private hops.
These are qualitative execution-fit conclusions; no numerical speedup has been measured.

## 2. Target behavior and preservation

Normalization may hand a compatible upper consumer the exact charged materialized projection it
already built. Bind this representation to the compiler-owned completed source and its projection
assessment/context, universe, model and revision. A consumer cannot use a similarly named graph
from another role or generation. Encode canonical projection outputs when the requested frontier
or a justified retained product requires them. Retain neither encoded and hydrated copies nor
all named projections indefinitely merely to avoid a future decode.

Contextual closure has exact model-owned rules irrespective of physical strategy. A selective
implementation binds the current frontier into the authored relation operation and returns every
qualifying pair, including required intermediate nodes. A shared bulk adjacency remains useful
for dense/all-root consumers. Choose between these routes from real consumer shape and reuse,
not by replacing graph semantics with a generic reachability filter or by adding a cost planner.
A rule whose negative/nearest/optional semantics cannot yet be expressed exactly keeps the valid
bulk route until its exact replacement is established; it must not silently narrow its universe.

C0 kernels consume borrowed typed root partitions over one owned union for each active window.
Selected IDs and the union remain charged through all borrowers; independently owned output and
fresh invocation/evidence links outlive input release as required. A deliberate small copy may
release a disproportionate backing allocation, but it needs a concrete lifetime/locality reason.
Existing row-level conflicts, exact empty-domain meaning and missing-premise refusal remain.

Cache lookup precedes rich hydration where sufficient exact value premises are available. Carry
model-owned semantic dependency tokens; do not substitute physical full-row hashes for the current
intentional coverage projection over scope/context/family/status. Equal neutral values can reuse
with fresh provider/run provenance, which is reconstructed for the current invocation. Additions,
deletions, negative lookups, policy, configuration, coverage and output contracts invalidate only
where their declared value meaning changes; unknown dependencies remain conservative.

Retained products exist for named consumers and actual reusable work. Decline optional population
before accepting effects when it has no useful consumer. Required output may proceed independently
of accepted population, but the enclosing owner still joins accepted effects and preserves their
original error/Unknown outcome. Moving work after readable output is not automatically an earlier
command end. Private recomputation after interruption does not promise arbitrary-stage resume.

**Approved rule directions:** operator approval on 2026-10-18 includes RC01/RC02 for private
handoff and durability, RC06 for retention, and RC07 for contextual strategy. SP00 owns decision
routing before dependent physical changes; these companion notes do not independently amend an
ADR. Required fact families, optional techniques and standalone requested frontiers remain intact.

## 3. Work packages

### AA00 — Decide contextual strategies and useful materializations

**Consumes:** existing source and review I03/I04 immediately; SP00's consumer/durability slice
before fixing any changed persistence obligation. No implemented private-source prerequisite.

Inventory actual contextual rule consumers and reusable products, including projection topology,
C0 payloads and Retrieval rendering. For each, identify selected/dense scope, repeated preparation,
exact key and negative premises, backing lifetime, reconstruction/spill, independent receiving and
who will read a persisted result. Include the current no-cache/recompute alternative; do not add
read-only/read-write flag families solely for experimentation.

Deliver two bounded decisions in the owning design discussion: (1) which contextual operations
receive frontier-bound execution and which consumer uses shared bulk preparation; (2) which
products justify retention and their exact granularity. Compare sparse selected roots with dense
all-root/high-degree/cyclic scope, and cold lookup/population with useful repeated consumers.
Source reasoning and minimal finite examples can settle semantics. A focused probe is appropriate
only if its result changes the choice; an unresolved latency forecast does not block an obvious
copy or scan correction. Record any unresolved strategy condition and the consumer it constrains.

**Closes locally when:** every selected operation has an exact semantics-preserving route, each
retained product has a concrete consumer and complete key, and dependencies on SC00/SN00 are
specific implemented capabilities rather than whole-document barriers. I01 benefit remains SP95.

### AA01 — Transfer existing charged projections directly to compatible consumers

**Consumes:** implemented SC00 completed-source backing/borrow/resource contract and the relevant
SC01 normalization-to-analysis migration slice; SP00 accepted durability decisions. Independent
mapping and API preparation can begin earlier without creating a temporary second authority.

Change normalization's production handoff and upper graph preparation together. Retain the
already materialized graph with its exact identity and earned properties; allow each compatible
analysis to borrow it under the source owner. Preserve the portable canonical projection encoder
for requested-frontier native materialization and independent imported artifacts. Cold receiving
can still hydrate canonical snapshots under its own assurance; it cannot borrow a nonexistent
producer-local graph or the producer's stale ownership.

Remove the live producer encode→persist→scan→decode handoff after all affected upper consumers
migrate. Release encoded chunks after transfer when they have no current consumer, and topology
at last borrower. Partial construction, resource refusal and cancellation publish no usable
completed graph. Spill uses SC00's owned backing rather than an independent cleanup subsystem.

**Focused closure:** selected versus disabled named projections; isolates, parallel relations and
unresolved targets; exact output/coverage at Normalized and Analysis frontiers; changed context,
source or role refusal; shared multiple-consumer lifetime; resource/spill/cancellation; independent
receiving from canonical output. Use an independently specified tiny topology, not only agreement
between the new encoder and decoder.

### AA02 — Execute contextual closure at the scope its semantics require

**Consumes:** AA00's I03 decision and existing model-authored rule definitions. It can proceed
against current exact providers; migration to private sources follows implemented SC00/SC01 slices.

Introduce frontier parameters at the model-owned contextual operation and provider execution
boundary. Bind exact typed IDs and role-qualified sources in bounded windows. Preserve optional,
coalesce, negative and nearest-binding semantics, including relevant rows outside the returned
public subset. Reuse existing prepared correspondence/query capability across a valid input scope.
No ad hoc SQL string filter may become a second definition of the contextual rule.

Migrate selected-root consumers and retain shared bulk preparation only for the consumers AA00
justifies. Avoid eager full-pair sort/IPC adjacency on the selective path. If the selected rule
requires broad premises, state and read those premises rather than treating an empty match as
absence. Preserve canonical membership/reduction and existing bounded-prefix semantics.

**Focused closure:** explicit expected contextual edges for optional/negative/missing bindings,
reconvergence, cycles, high degree and intermediate nodes outside output roots; sparse roots with
large unrelated private scope; dense equivalence; alternate physical providers; cancellation and
resource refusal without partial Complete output. Inspect actual examined scope or access plan
when establishing removed eager work; returned-row counts alone do not establish that change.

### AA03 — Borrow root premises and perform useful lookup before rich hydration

**Consumes:** existing `RowsView` and exact membership now; AA00 product decisions for early-key
work. Private source tokens consume the implemented SC00/SC02 portions that establish their exact
value/coverage meaning. Borrowed partition migration need not wait for the whole private pivot.

Migrate C0's builder, invocation-link construction, value-dependency construction and affected
validators together to borrowed exact views over an owning window. Replace `selected_copy` and
per-root rich clones where no consumer needs ownership. Keep ordered selected IDs charged and
owned while jobs execute through existing owned-work facilities.

Compute compact qualified requests for a root window and perform compatible multi-lookup before
rich hydration. Hydrate the miss union once; hits reconstruct only fresh links and required
provenance. Reuse source-owned value preparation where it exactly matches C0's semantic contract.
If an exact key still requires rich fields, share their preparation instead of inventing a weaker
key or claiming all hydration was avoided. Do not install a parallel cache beside existing reuse.

**Focused closure:** overlapping inherited/aliased roots, explicit empty versus absent rows,
missing or duplicate membership refusal, mixed hits/misses, equal values with fresh provenance,
changed coverage/context/negative premises, cancellation and last-borrower resource release.
Kernel expectations must include exact content and links, not counts or hashes alone.

### AA04 — Integrate justified retention and independent upper work

**Consumes:** AA00 selected products; AA03 key/window behavior where retention uses it; relevant
SC03 owned-work/visibility and SN02 retention installation capabilities where changing effects.
Local removal of unaccepted optional population does not require unrelated scheduling changes.

Move useful C0/Retrieval population off required emission paths through existing owned children,
and batch compatible exact product requests without merging keys or neutral/fresh provenance.
Batch the retained dependency mapping construction through existing native owners. Admission,
accepted-effect joins, refused/Unknown writes and actual command completion retain their owners.
Remove obsolete foreground-only population and per-product rediscovery after their consumers move.

Inspect C2 selection and S0 synthesis for independent per-root pure kernels. Use existing owned
CPU work for genuinely independent domains and deterministic ordered integration; do not cross
C0→C1→C2→S0→Retrieval value dependencies. Preserve global reductions/constraints as such. Where
parallel decomposition would duplicate more shared work or alter semantics, keep the exact kernel
and carry a reasoned decision rather than adding tasks for their own sake.

**Focused closure:** uncached/read-only-use when needed, useful hit and miss, optional decline
before effects, accepted population failure and cancel with no abandoned work, changed dependency
mapping, deterministic C2/S0 output under shuffled completion and genuine prerequisite refusal.
Readable-frontier completion and enclosing cleanup are distinct observations.

## 4. Integration, migration and completion

AA00, borrowed-root work and contextual semantic work may start together where edits do not
conflict. AA01 requires real source/resource ownership; a settled interface alone is insufficient.
AA03 early tokens require their exact semantics to exist, and AA04 must use the accepted-effect
protocol rather than a detached cache task. Coordinate shared catalog/build and consumed-row
editing with compiler/native owners; logical independence does not permit conflicting edits.

A new projection or optional analysis extends model semantics and consumer selection, not an
unconditional persistence stage. A new cache product supplies a concrete consumer/key/lifetime;
it does not automatically become a durable checkpoint. Changing physical strategy cannot change
the declared graph universe, relationship identity, method coverage or same-generation evidence.

Implementation controls are compile checks and revealing affected cases under normal available
parallelism. Native scenarios use existing fixture/service owners. SP90 owns integrated functional
acceptance, including genuine requested-frontier admission/publication and optional configurations.
SP95 separately owns later representative benefit evidence; full FastMCP, broad qualification and
numeric forecasts are not local prerequisites. Embedding inference, submitted MCP runtime and
operator adoption are outside this plan series' execution boundary.

At cutover, remove replaced copies, eager paths and unused cache representations after their
consumers migrate; regenerate project-owned derived products through SN04. Retain no compatibility
reader or dual authority solely for old data. Functional closure requires complete target output,
resource/failure semantics and consumer migration. It does not follow from an accepted decision,
an interface inspection, a warm cache hit or an improved timing alone.
