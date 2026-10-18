# Catalog execution speed: independent comparative design review

**2026-10-18 · design / target · decision: Revise.** Core/template3.3, efficiency heuristics1.0
(H1–H28), code-intelligence profile1.5 and the library-context binding. Baseline:
`b0dd321eb78fa5ca90c5107e887145f85f774b52`. Reviewer: Codex coordinator, with an independent
source-led design reviewer, retained-trace auditor and library-capability researcher.

## Integrated judgment

Both external reviews identify substantial avoidable physical work around useful computation.
The first is generally careful about workload equivalence, existing parallelism, independent
admission and measurement limits. The second makes a valuable architectural challenge: private
compiler computation need not use the database as its stage-to-stage transport. Its numerical
argument substantially overstates what the retained trace establishes. Correcting that argument
does not vindicate the current execution architecture.

**Proposed target:** one compiler-owned, immutable typed working set, with explicit charged spill,
exact completed-input and validity capabilities, and operation-shaped durable materialization at
the **requested frontier**. Preserve SurrealDB's final canonical graph, originals, publication,
indexed consumers and independent receiving. Persist intermediate products only for a concrete
consumer whose reuse benefit warrants their cost. This is a hybrid execution design, not two
editable canonical stores, an all-RAM requirement, or an alternative database recommendation.

The source supports this direction independently of a speed forecast. Current pure owners write
through physical lowering and ownership protocols, then consumers reconstruct typed data. Even
with substantially improved operation pages and packing, those crossings are unnecessary when
the only consumer is the same live compile. The durable outputs requested by `--through facts`,
`normalized`, `analysis` and `catalog`, plus genuinely useful cross-attempt products, remain real
consumers. Removing every intermediate persistence mechanism without preserving those outputs
would be an incorrect pivot.

**No 20–35s FastMCP result, 10× improvement, fixed percentage of database time or removable
117.5s retry penalty is established.** Recommend the simpler conforming execution route from
its removed work and required semantics; measure representative end-to-end speed later.
A complete tracing platform or benchmark framework need not precede source-established repairs.

The independent pass additionally found broad contextual-pair preparation, overlapping rich-root
copies before cache lookup, a metadata registry lock held over stream acquisition, an avoidable
singleton block-inspection loop, and work performed even when phase diagnostics are disabled.
These are integrated below rather than placed in a separate backlog.

## 1. Subject, evidence and boundaries

The two supplied inputs are [review E1](../../external_review_execution_speed_and_diagnostics.md)
and [review E2](../../catalog-compile-performance-review_2026-10-18.md). Their original files are
unchanged. The [complete recommendation crosswalk](../evidence/2026-10-18_catalog-execution-speed-comparative/recommendations.md)
assigns stable E1/E2/D references to every substantive design, diagnostics, experiment and sequence
recommendation, including compound proposals. Dispositions are Retain, Correct, Improve, Omit or
Unresolved; correct diagnoses and unsuitable remedies receive separate treatment.

The independent reviewer formed its initial findings without reading either external assessment.
It then compared their alternatives. The coordinator reconciled its judgment with decisive source,
the independent trace parse and pinned library contracts. The result remains one principal review.

**Evidence strength:** executing code and pinned interfaces were inspected; retained observations
were reparsed, not rerun. Corrections and target contracts below are **Proposed**, dated2026-10-18,
unless explicitly described as existing Implemented behavior or attributed historical measurement.
There were no new product tests, builds, database queries or service operations. Focused probes
were permitted, but the source and retained records settled the structural and interpretation
questions reported here. Runtime uncertainties are retained explicitly, not converted to passes.

Coverage includes acquisition, Ruff/ty/Pyrefly integration, normalization, selected graph access,
Catalog/Behavioral routing, scheduler/ingress, assurance, retention, native installation, publication
and finalization. Serving is considered as a consumer/preservation obligation. Embedding inference,
submitted MCP queries, real-library qualification, full benchmark campaigns and production fixes
are outside this execution. No operator/default schema cutover or paused-plan completion is implied.

The workload premise is a single-operator, machine-local compiler over pinned code, dependencies,
documentation and examples. It must accommodate many small/empty semantic families, overlapping
public roots, larger unrelated private scope, high degree/cycles, optional techniques, concurrent
independent producers, bounded memory/spill and interruption. A one-function fixture is revealing
for fixed work but does not substitute for FastMCP scale or establish its timing.

### What the retained trace actually establishes

The [evidence notes and reproducible method](../evidence/2026-10-18_catalog-execution-speed-comparative/README.md)
identify captures, historical source and limitations. Key corrections are:

| External claim | Independent result and consequence |
|---|---|
| Fixture timings used to forecast FastMCP performance | The selected helper compiled a tiny one-function fixture in306.683s, then timed out at360.837s during raw export. No semantic admission or publication occurred in that helper. |
| About95% native-store time; only16.9s analyzer/kernel work | Arithmetic reproduces as296.017s of312.954s **summed instrumented stage duration** overlapping owned-native-task/gate waits. The stages' wall union is165.766s. A native acknowledgement includes arbitrary client/validation/scheduling work; residual16.937s is not measured CPU or an end-to-end wall partition. |
| 2,481 effects and settlements characterize every compile write | Those are whole-helper **legacy authorization** phase counts, including post-compile work.2,280 of each finish before compile ends. Ordinary contribution writes also use operation pages, whose successful path is not counted by those legacy phases. |
| 1,785 tiny logical-row windows | The count/median reproduce, but window rows are **physical records**:15,498 total, median1;1,376 windows have≤4.5,986 rows are logical batch submissions over289 names, not demonstrated distinct final catalog cardinality. |
| 117.5s lost to conflicts; a34s mutual livelock |91 retried calls eventually committed, with446 retries;117.464s is full call elapsed summed across overlapping attempts, execution and backoff. Wall union65.698s is still not incremental penalty. Their interval union spans34.278s; hundreds of other operations progressed. Conflict keys and mutual livelock are unestablished. |
| 80.5s global-gate savings |47 waits sum80.528s but wall union28.636s. Neither is removable latency. Real exclusion over sequential network effects remains a source-established issue. |
| 150s optional retention blocks every next stage |15 retentions sum149.878s. Retrieval retention overlaps the Catalog-frontier stage and outlives compile by7.039s. Already-owned asynchronous retention exists. Foreground pure-product retention and shared-resource interference remain separate issues. |
| Native write concurrency1.34 | Reproduces only for whole-helper legacy-effect intervals:111.147s/82.643s union. It omits successful operation pages and does not measure all writes or CPU utilization. |
| About3ms per future FastMCP row |12.697s/4,096 is a descriptive ratio for the older synthetic write probe, with lifecycle effects and8,192 physical rows. Most windows had102 physical rows, not128. It is not a current marginal-cost coefficient. |

The source-context receipt says invocation/configuration comparison only and source-to-binary
**unknown**. Historical helper source is consistent with `f1b83562`; the current direct-admission
fixture was restored afterward. Do not turn either review's `b0dd321` baseline into an immutable
executed-source/build-closure claim. Likewise, STATUS records schema9 default/schema10 source;
that alone does not identify the installation used by an arbitrary run.

## 2. Responsibilities, semantic authority and preserved strengths

| Owner | Contract and adjacent consumers |
|---|---|
| `lctx-model::domain` | Fact identity/fidelity, graph projections, demand, coverage, transformations, invariants, result/value dependencies and published meaning. Physical execution consumes these definitions. |
| `cpg-extract` / `cpg-flow` | Actual pinned provider observations, source/role correspondence, capture and explicit provider completion. Ordinary diagnostic Pyrefly checking is not equivalent rich-fact export. |
| `cpg-core` | Requested-frontier schedule, exact immutable input selection, typed pure work, resource/cancellation ownership and completed artifact handoff. |
| `lctx-compiler-store` / `lctx-native` | Current native contributions/views, selected providers, ingress, installed physical content, operation pages, protection and checked recovery. These mechanisms do not independently define graph meaning. |
| `lctx-publisher` | Required definitions/roles/search, fresh actual seal, immutable manifest, reader pin and committed identity through fallible acknowledgement/finalization. |
| Existing run/evidence owners | Invocation/process identity, raw capture, observations, cleanup and retention. Diagnostic observations do not grant product authority. |

The fact/fidelity contract remains unchanged: Ruff canonical syntax, Pyrefly attributed typing
and protocol observations, and ty's explicitly selected flow view have distinct meanings and
revisions. Derived catalog/structural/behavioral outputs keep their stated model, coverage and
same-generation evidence. A missing/unrequested result never establishes absence. Relationship
identity, isolates, parallel sites, unresolved targets and projection universes survive every
physical alternative. Behavioral methods, optional analytics and embedding are not silently
removed from supported configurations because the default fixture does not request them.

Preserve these existing Implemented strengths:

- Dependency-ready scheduling and owned concurrent tasks (`compilation.rs:1305–1619`), canonical
  result ordering, explicit bounded-prefix/SCC semantics and real cancellation/drainage.
- Compact whole-owner NotRequested results, selected rich versus metadata input contracts,
  exact role/epoch/context binding and existing scoped/durable admission reuse.
- Indexed ordinary native references, shared contextual adjacency and already shared hydrated
  graph/SCC preparation. Neither "all stages serial" nor "whole graph rebuilt per root" is accurate.
- Operation-root recovery for contribution pages, exact replay/fence outcomes, immutable blocks,
  current-reader protection, independent incoming admission and committed-effect finality.

These strengths constrain replacements, but their current physical implementation is not itself
a requirement. Relevant principles are FP-01–07, DP-03/09/10/13/19–23 and CI-01–13; H1–4,
H5–20 and H21–28 explain the material tradeoffs without creating a checklist or cost model.

## 3. Findings and corrective contracts

Current disposition for every F item is **Open / Proposed; not scheduled**, owned by this review
until transferred to an implementation plan. The trigger is plan creation selecting this target;
closure requires each item's stated evidence. Existing publication-plan findings retain their
own current status; this review does not close or supersede them.

<a id="F01"></a>
### F01 — Private stage handoff purchases remote durability without a necessary consumer

**FP-01/03/07 · A3/A4; DP-10/19; H2/15/16/24/25/28.** `ProducerOutput` hands typed batches to
native ingress; `compiler.rs:5780–5830,5920–6221` lowers them into native graph/body/block forms.
`compiler_provider.rs:842–925` then scans native Values back into Arrow. Completion earns
membership inventory at`compiler.rs:6647–6681`; consumers and artifact construction acquire their
own protected views. This is coherent but imposes storage/lifecycle work between pure owners.

**Correction:** select the hybrid target. A private completed source owns immutable typed batches
or indexed spill, exact producer/profile/model/configuration, ordered roles and dependency views,
complete/partial/failed outcomes and relevant earned validity. Consumers select that same source;
source liveness and charge ownership survive the last borrower. Retain prepared materialized
projection graphs from normalization for compatible consumers instead of encode→persist→read→
hydrate (`projection/normalization.rs:934–960`, `analysis_graphs.rs:90–175`). Persist canonical
projection bytes when the requested artifact needs them, without retaining every representation.

Finalize and durably materialize each **requested** frontier, not only Catalog. Optional reusable
products have explicit independent consumers and costs; ordinary ephemeral results need no
per-family durable resumability. Cancellation joins accepted effects, drops or retires owned spill
and leaves no partial publication. External effects and declared nondeterminism retain their actual
outcomes; "pinned input" does not automatically make every configured compile deterministic.

Compare improved persistence and all-in-process alternatives. Improved persistence is a viable
fallback but keeps unnecessary remote handoffs; all-RAM is inadequate for bounded growth. Do not
introduce a multi-backend framework or retain both old authorities as compatibility code. No
existing arbitrary-stage crash-resume consumer was found in the inspected CLI/driver; if one is
claimed, identify its actual contract before it blocks the pivot.

**Closure:** exact outputs/provenance/coverage at all four requested frontiers, large/spilled
input, shared-buffer lifetime, source/context changes, cancellation, independent receiving and
failed/committed publication. Native output still contains all canonical facts/originals needed
by consumers, not merely query indexes. RC01/RC02 apply. Expected benefit is removed work, unmeasured.

<a id="F02"></a>
### F02 — The retained native boundary still expands semantic kinds into too many physical operations

**FP-07 · A4; DP-10/13/19/20; H2/4/8/9/23/24.** Nongraph packing already exists; graph ingress
still orders payloads, aliases and locators through bounded effects (`compiler.rs:6150–6252`).
`schema.surql:238–305` repeats scalar reads and `array::find` correspondence inside target loops.
Predicate/guard incidence and completion parents create real potential conflict surfaces. The
trace proves substantial contention, not its exact keys or removable penalty.

**Correction:** construct one validated, byte-bounded installation packet for compatible
payload/endpoint/alias/membership/protection effects. Prepare exact correspondence once, use
set reads and set mutations where the locked engine preserves the contract, and eliminate
unnecessary per-kind tails and empty physical containers. Empty declared contributions, failed
versus unrequested outcomes, logical rows and exact completed-view identities remain explicit.

Use current operation pages; do not rebuild their already implemented consolidation. Trace the
remaining generic effects to their actual owners before choosing more operation-level grouping.
Remove shared counters only where their protected failure/hazard has a cheaper valid replacement:
immutable content and reader pins alone do not exclude new holds, collector races, late writes or
changed definitions. Keep exact read/write conflict coverage and first-creation races. A short
manifest commit is not one giant data transaction, and no columnar graph packing may erase native
relationship identity, evidence or required query access.

A specific additional crossing is`complete_block_original_holds` (`compiler.rs:6400–6423`): a
32-ID page loops through singleton `inspect_expected_window_read` calls although `held.rs:480–495`
already supports point sets. Batch by admitted bytes/working set, preserving every block/guard/
original association; blindly decoding32 large blocks can multiply deep-read charges.

**Closure:** equal/unequal duplicates, missing endpoints, empty outputs, mixed existing/new
predicates, foreign/new holds, concurrent collection, ordered page refusal/unknown and whole-target
cleanup. Inspect actual access paths; a bulk-looking query can still hide scans. RC03 applies.

<a id="F03"></a>
### F03 — Assurance preparation can be shared more deeply without claiming that hashes prove semantics

**FP-04/07 · A2/A4; DP-03/09/23; H12/18–20.** `admission.rs:1412–1628,2173` already has scoped
and durable whole-input reuse. It does not indiscriminately rerun every kernel. Nevertheless,
reference acquisition, invariant preparation and physical completion can traverse equivalent
protected data repeatedly (`artifact.rs:1618–1745`, `compiler.rs:6661–6677`).

**Correction:** name each required property, exact premises and invalidation boundary. Reuse
source-owned semantic validity and prepared indexed inputs within that scope; fuse compatible
checks over a protected stream when each retains its distinct result. Accumulate completion
content/count evidence during **acknowledged canonical ingress**, not merely submissions, only
if duplicate/coalescing/order and partial-failure contracts match the completed descriptor.
Retain actual persisted reconciliation where storage or trust can differ. Independent receiving,
foreign source, changed model/config/epoch or tampered native state earns fresh assurance.

Do not introduce a second certificate system if existing evidence suffices. Digests establish
agreed content, not validity; construction typing does not prove every cross-row relation. A
checkpoint-specific checker cannot borrow full-factory evidence from a different definition.

**Closure:** scoped/full/cold receiving, independent equal-view owners, added/deleted/duplicate
rows, failed/unknown producers, changed input roles/definitions and unchanged standalone outputs.
Time preparation/read/compute/reuse separately only where unresolved priority needs it. RC04.

<a id="F04"></a>
### F04 — Unrequested work is narrowed after expensive prerequisites have already been earned

**FP-04/07 · A2/A4; H5–7/12/18.** Behavioral Catalog owners have compact declarations, but optional
Analytic/Embedding/Text contracts and launch preparation remain broader than their disabled
kernels (`analysis/owner.rs:179–310`, `embedding/text.rs:179–220`, `compilation.rs:1511–1535`).
Uniform fragments also stream inputs to exact EOF (`compilation/uniform.rs:126–191`).

**Correction:** let selected profile, techniques and configuration define the smallest valid
owner/declaration/representation closure **before** binding, checked-input preparation and output
setup. Preserve authored configuration, exact domain, NotRequested coverage and evidence closure.
Reuse earned compact properties where authorized; a bare descriptor/count does not grant trust.
Preserve currently supported captured fact families; any deliberate fact-family reduction is a separate product decision. Optional or catalog consumers must remain supported.
Derive provider availability from the model and authentically completed provider descriptors,
without rescanning richer data merely to restate established properties.

**Closure:** each optional configuration on/off, zero and unknown domains, alternate suppliers,
required protocol consumers and genuine input refusal. RC05 covers meaning-changing contract choices.

<a id="F05"></a>
### F05 — Visibility exclusion still includes sequential network work per logical relation

**FP-03/05/07 · A3/A4; H2/8/22/23.** `workspace.rs:1626–1655` holds the completion gate while
binding selections. Producer composition preparation correctly moved outside it, but the final
gate remains held across`publish_prepared_production_view` for each relation (`3786–3889`).
Coordinator launch also awaits one stage's preparation before preparing the next; it does poll
pending work (`compilation.rs:1511–1619`), so this is not a wholly synchronous scheduler.

**Correction:** prepare exact output sets outside exclusion, install/bind compatible sets with
one bounded operation or predecessor-checked commit, then expose coherent host visibility.
Disjoint preparations may overlap; overlapping predecessors must rebase or refuse honestly.
Do not replace the gate with per-relation locks that expose half a producer. Narrow synchronization
after reducing I/O grain; use existing owned-work composition for independent launches/provider
completion. Preserve genuine C0→C1→C2→S0→Retrieval value dependencies.

**Closure:** disjoint/overlapping producer completion, stale predecessors, mid-commit failure,
missing declared output, canonical ordering and cancellation. Gate wait sums are not speedup budgets.

<a id="F06"></a>
### F06 — Optional retention and late cache lookup still consume required foreground work

**FP-07 · A4; DP-09/19; H12/14/21/24/26.** Whole-producer retention already runs as an owned child
(`workspace.rs:273–288,3915–3921`). Distinct pure-product paths await retention before mandatory
emission (`catalog_core.rs:296–303`, `retrieval.rs:365–371`). Retained-product installation
rediscovers transitive mappings (`compiler/production.rs:303–428`). C0 lookup follows rich
hydration and dependency construction, so a hit can leave much of its cost intact.

**Correction:** distinguish cache lookup, population, required output readiness and enclosing
completion. Select useful products and compact exact keys early, perform window multi-lookup,
hydrate misses once, and batch compatible retention. Decline optional population before effects
when it buys no useful reuse or compromises required work. Once accepted, retention effects remain
owned and joined; background placement alone does not shorten command completion.

Read-disabled/read-only/read-write/warm policies are useful comparison dimensions only when a
real consumer needs them, not four mandatory runs or a permanent flag family. Shared registry
names do not prove identical locks: root installation hashes roots while request gates key on
requests. Exact contention edges remain I02 below.

**Closure:** useful hits, neutral fresh provenance, additions/deletions, read-only/no-population,
accepted failure/cancel and zero abandoned work. RC06 applies to foreground/population policy.

<a id="F07"></a>
### F07 — Analyzer fact export still contains repeated correspondence and serial CPU work

**FP-07 · A4; H6/9/12/22.** Normal Pyrefly `Errors`/`Exports` checking is not the integration's
`Require::Everything` plus rich fact export (`pyrefly_stage.rs:99–124,549–715`). Protocol queries
in`protocol_records.rs:176–303` reach binding searches in the pinned fork. Other candidates are
serial root export, repeated type presentation, linear writer dispatch and serial C2/S0 kernels.

**Correction:** module-local or native bulk correspondence, direct typed writer selection and
independent module/root work through existing ownership mechanisms. Select demand before native
export only while preserving currently supported captured fact families and every configured
consumer. Any deliberate fact-family reduction is a separate product decision. Memoize type work only under
an explicit encounter/depth/truncation contract (`type_records.rs:1034–1069`); changing encounter
accounting is a semantic decision. Provider completion may overlap when domains/effects are
independent; bounded queues and accepted writes still drain in their required order.

**Closure:** identical attributed facts/coverage, difficult protocol/with sites, lazy solving,
import promotions, depth/encounter exhaustion, shuffled order and cancellation. Neither the tiny
fixture nor absent slow-phase events justify declaring analyzer/planner/hash work universally cheap.

<a id="F08"></a>
### F08 — Contextual closure has an eager full-pair preparation route despite selected-root demand

**Additional independent finding · FP-07/A4; H5/7–10/15.** Ordinary native references already
use indexed frontiers. For contextual rules, however,`consumed_rows.rs:718–745,825–948` executes
the source-kind pair universe to EOF, writes ordering IPC, sorts it and writes compact adjacency
before selected traversal. The catalog contains a nine-input contextual optional join
(`catalog_scope_program.rs:374–414`). It is shared per edge source, not rebuilt for every root.

**Correction:** allow model-owned contextual rules to accept an exact current frontier and emit
all qualifying pairs for it, preserving coalesce/optional/nearest/negative semantics and intermediate
nodes. Compare keyed relational execution with the existing shared bulk adjacency; dense/all-root
work can legitimately favor the latter. Avoid a universal planner: introduce a selected strategy
only for demonstrated sparse-versus-dense consumer variation.

**Closure:** sparse roots plus unrelated private modules, reconvergence/cycles/high degree,
negative/missing bindings and dense all-root equivalence. Inspect examined scope, not only returned
rows. This is a rigid access-path finding, not a measured claim that keyed joins always win. RC07.

<a id="F09"></a>
### F09 — Overlapping catalog roots deep-copy and rehash already shared premises before cache hits

**Additional independent finding · FP-07/A4; H12/14/16/17.** `catalog_core.rs:231–289` hydrates a
union then creates rich owned partitions for all roots before dropping it. `CatalogData::selected_copy`
(`catalog/build.rs:19–23`) clones selected rows. `catalog_products.rs:124–179` then computes
`value_dependencies` over rich rows before lookup; shared class/signature/alias premises multiply
copies and hashes across roots.

**Correction:** one immutable charged union with borrowed exact `RowsView` partitions (existing
`normalized/rows.rs:51–149`); kernels consume views without independent copies. Derive compact
semantically qualified dependency tokens and multi-lookup before rich hydration where possible.
Keep neutral value identity separate from fresh invocation/source receipts. Native coverage's
intentional scope/context/family/status projection (`catalog/build.rs:1432–1475`) must not be
replaced by a physical full-row hash that destroys valid reuse. Explicit empty and absent domains
remain distinct. Disabling an uneconomic cache is a valid simpler alternative.

**Closure:** overlapping inherited/aliased roots, same values with fresh provenance, changed
coverage/context, hit/miss and resource refusal; backing retained once through the last borrower.

<a id="F10"></a>
### F10 — Byte integrity is re-established against the whole capture at each provider boundary

**FP-07/A4; H5/12/18–20.** `bundle.rs:588–595` calls `verify_closed` after every provider except
acquisition/assembly. `capture.rs:473–525` traverses membership and rereads every captured byte.
A whole-tree scan therefore scales with provider boundaries as well as corpus size.

**Correction:** a capture-owned immutable or read-verified source capability protects exact
membership/bytes through all consuming providers. Reuse that established property under its real
lifetime; restrict unavoidable verification to affected consumed inputs. Keep intermediate work
provisional until required final integrity is established. A read-only permission bit under the
same user is not sufficient immutability, and deleting checks on mutable paths is unsound.

**Closure:** concurrent/late additions, deletions, byte changes, dependency reads and cancelled
providers; no mixed-source result becomes admitted. E1 already notices repeated capture scans;
this review supplies the contract its optimization requires. RC08 applies.

<a id="F11"></a>
### F11 — A shared metadata registry serializes independent stream acquisition

**Additional independent finding · FP-07/A4; H12/21–23.** `compilation/uniform.rs:49–85` holds
one `Preparation.state` mutex across `load_fragment`, which opens sessions and streams required
relations (`126–191`). It coalesces equal keys but also blocks distinct exact-source fragments.

**Correction:** register per-exact-key owned pending preparation under a short registry lock,
then load outside that lock. Same-key callers share one protected result; unrelated keys progress
independently. Match full source/capability/policy identity, not a digest alone. A failed/Unknown
native acquisition remains its original failure, not a retryable empty `OnceCell` initializer.

**Closure:** same/distinct keys, initializer cancellation, EOF/owner failure, resource refusal,
last-borrower release and no partial memo publication. No critical-path magnitude is established.

<a id="F12"></a>
### F12 — Observations omit current successful writes and lose useful terminal timing

**FP-06/07 · A1/A4; DP-21/22; H26.** Successful operation pages have no equivalent of legacy
effect timing (`operation_pages.rs:83–248`); retry identity extraction recognizes only older
`native_effect` (`retry.rs:169–179`). Eager/streaming native readers drop statement statistics.
The SDK maps present valid durations, but missing/invalid durations become zero
(`third_party/surrealdb/src/engine/remote/grpc.rs:1702–1707`). JSON already preserves structured
span context; stable activity/handoff identity is a separate missing capability.

**Correction:** extend existing run-owned observations with typed phase/read/page/retry/statistic
forms, actual operation-root/page ordinal/query/statement identity, optional timing presence and
true terminal outcomes. Observe preparation, readiness, dispatch/start, kernel completion, required
output and enclosing cleanup distinctly. Preserve received timing **before** lossy defaulting.
Do not infer CPU from native acknowledgement, or network from client-minus-server remainder.

A small optional exporter can pair existing events and emit Perfetto tracks/counters/flows. A
subscriber alone does not convert Phase EVENT pairs into durations or causal edges. Async overlap
needs async tracks/separate lanes, and missing terminals remain incomplete. CPU sampling is
separate from entered/poll elapsed time. Remote cancellation/drainage is never inferred from
observer drop. Diagnostics must not gain semantic authority or require a telemetry service.

**Closure:** successful/failed/unknown pages, reconciliation, missing statistics, interruptions,
context propagation, ambiguity/capture loss and one actual executed DataFusion plan. RC09.

<a id="F13"></a>
### F13 — Diagnostic work itself lacks the smallest sufficient execution path

**Additional independent finding · FP-07/A4; H6/26/27.** `Phase::begin` (`phase.rs:139–155`) takes
an atomic ID, clock and captured dispatcher before interest is checked. `NativeReadObservation`
(`reader.rs:3464–3476`) demonstrates an enabled fast path. Repeated text parsing also discards
structure already supported by the JSON evidence adapter.

**Correction:** a dispatcher-correct inactive Phase representation can avoid disabled work,
without incorrectly consulting another task's subscriber. Prefer structured observations and
aggregate counters at module/window/query grain; add fine detail only for a concrete investigation.
Use existing capture/storage/finality owners and library exporters before bespoke instrumentation.
A direct append recorder is a candidate, not proof of crash durability or negligible overhead.

**Closure:** scoped/global disabled and enabled subscribers, cross-thread/async context,
normal/interrupt/abort/write failure and truthful loss markers. No exact zero-overhead or3% gate
is adopted; neither disabled cost nor whole-run speed effect was measured here.

## 4. Library fit and complete alternatives

Pinned interfaces: Arrow59.3.0, DataFusion55.1.0, tracing0.1.44/subscriber0.3.23, exact Pyrefly
fork`b8a668da`, SurrealDB3.3.0 vendored SDK/core/protocol family; actual workspace patches matter.
Selected library skills and exact sources qualify these claims; Context7 resolve→query and
primary documentation supplied current documentation leads. The diagnostics crosswalk records
version-specific qualifications.

| Capability | Qualified fit and remaining burden |
|---|---|
| Arrow and spillable typed compute | Immutable arrays/RecordBatch and IPC support the representation. Slices share whole buffers; charging slice length alone loses backing lifetime. Streaming does not bound every operator/consumer, and DataFusion spilling is operator-specific. Reuse the existing custom memory pool/owned disk directory; IPC supplies bytes, not source/admission authority. |
| Native set-shaped effects | Existing point-set readers and operation pages are stronger starting points than new per-row wrappers. Set reads are not automatically coherent snapshots; native schema/predicate/refusal/reconciliation semantics still govern publication and collection. |
| DataFusion metrics and shared preparation | Retain and execute the **same** physical plan, inspect its metrics afterward and label partial/cancelled results. Logical/provider clones do not imply one scan or a shared materialization. `EXPLAIN ANALYZE` repeats work. The selected datafusion-tracing bridge is an option, not required to expose native plan metrics. |
| Pyrefly telemetry | Existing subscriber start/finish/computed-step callbacks and subtask counters are usable; a read-only counter snapshot/delta may close the private accessor gap. Accumulated worker elapsed is not wall/CPU. CLI report-timings changes module execution and is not an equivalent throughput baseline. |
| Perfetto/tracing exporters | Existing libraries can supply timeline format/viewing and SQL analysis. Pair the actual Phase protocol and explicit handoffs; threaded span nesting is not an application dependency DAG. Avoid a bespoke viewer and a mandatory collector. |
| SDK/server diagnostics | Preserve optional received QueryStats before conversion. Optional traceparent propagation must cover ordinary and custom admitted gRPC routes. Old memory-engine OTLP/metrics probes do not qualify current RocksDB correlation or production overhead. Slow logs may expose query/parameters; select them under existing owned diagnostic boundaries. |

Primary references: [DataFusion metrics](https://datafusion.apache.org/user-guide/metrics.html),
[Perfetto conversion](https://perfetto.dev/docs/getting-started/converting),
[Perfetto imported formats](https://perfetto.dev/docs/getting-started/other-formats),
[tracing async instrumentation](https://docs.rs/tracing/latest/tracing/trait.Instrument.html),
[tracing-chrome0.7.2](https://docs.rs/tracing-chrome/0.7.2/tracing_chrome/).
Current docs are not substitutes for the locked implementation contracts above.

| Complete alternative | Judgment for this workload |
|---|---|
| Improve persistence everywhere | Viable and preserves current recovery locality; packet/visibility/assurance fixes help. Still pays remote representation and lifecycle on private pure hops; retain as comparison/fallback, not as an immutable architectural constraint. |
| Private typed/spillable compilation, durable requested-frontier outputs, no intermediate retained products | Credible simpler alternative: removes private persistence and cache maintenance but forgoes genuinely useful cross-attempt reuse. Choose it where cache economics do not warrant retained products; preserve bounded staging and all requested frontiers. |
| Private typed/spillable execution + selected durable products + requested-frontier native publication | Recommended Proposed target. Removes private transport work while preserving all actual consumers. Explicitly changes intermediate authority/restart scope; requires RC01/02 and full owner/cancellation/evidence contracts. |
| New database, generic scheduler, universal certificate or causal-simulation platform | No demonstrated need. Existing libraries and domain owners provide sufficient building blocks; adopt additional machinery only for an identified remaining gap. |

Unbounded RAM-only execution and one giant final data transaction are rejected implementation
variants, not assumptions inherent in E2's spillable proposal or in-process execution.

## 5. Change scenarios, gates and remaining investigations

| Scenario | Meaning and boundary to preserve; settling evidence |
|---|---|
| Add a fact family or optional analytic | Model declaration and actual new semantics extend the demand/codec/validation contract; no automatic remote transaction/lifecycle object per empty kind. Required consumers and unknown coverage remain complete. |
| Replace persisted hot handoff | Consumers retain immutable exact source/roles and typed meaning; requested-frontier artifacts, standalone import and pinned native serving remain valid. No second editable source or implicit adoption. |
| Grow unrelated private corpus / overlapping public roots | Contextual access and rich ownership follow required scope; dense bulk preparation remains available where justified. Compare semantic membership, not just counts. |
| Cold/warm cache and source change | Reuse avoids actual preparation when eligible; correct invalidation includes additions/deletions/negative domains, policy and context. Fresh provenance is reconstructed. |
| Concurrent producers / shared keys | Narrow visibility commit, atomic usable producer sets, exact predecessor checks and genuine contention/refusal. More tasks must not amplify retries or resident work. |
| Cancellation/crash during staging or final acknowledgement | Private work can rerun; accepted native effects keep their original recovery owner/outcome. A committed publication survives a later acknowledgement/finalizer failure. Failed seal admissions are never reset for another test. |
| Diagnostic interruption or absent engine stats | Partial trace remains partial; observer termination proves neither cleanup nor native finality. Actual errors retain their classification and evidence. |

| Judgment/gate | Verdict and inspected scope |
|---|---|
| A1 Localize change | Satisfied for inspected semantic/provider/native ownership; diagnostics need F12. Not whole-repository certification. |
| A2 Explicit domain meaning | Satisfied for inspected attributed facts, exact roles, coverage, model-owned operations and existing assurance; target replacements remain Proposed and must preserve those authorities. |
| A3 Composition | Satisfied in existing scheduler/library composition; current physical handoff constrains substitutes, addressed by F01/F05. No new generic framework is required. |
| A4 Execution fit | **Violated** by demonstrated unnecessary handoff, repeated preparation/copying and broad synchronization/acquisition. **Revise**, regardless of whether eventual FastMCP latency meets a numerical target. |
| G1–G7 | No new demonstrated semantic/publication failure in inspected existing paths; passes are scoped source judgments, not runtime qualification. Performance forecasts and trace interpretations require the corrections above to satisfy truthful claims. |
| G8 Library leverage | Unresolved for the complete replacement until scoped compositions are qualified. Existing set readers, typed views and plan metrics already support several local corrections. |
| CI-G1 / CI-G2 | No newly demonstrated fidelity/evidence-closure failure in inspected source; all alternatives must preserve provider meaning, unknowns and same-generation closure. Serving/runtime journeys were not rerun. |
| CI-G3 | Not applicable to the inspected optimization work: no gold/heldout evaluation or tuning was performed; protected reference data remain outside production inputs. |

The following uncertainties do not justify postponing established work, but constrain stronger claims:

- **I01 — FastMCP distribution and benefit:** a later equivalent genuine run must identify input
  scope, requested frontier/profile, optional methods, cache dimensions, executable/schema and full
  completion. No30s forecast or necessity claim is accepted before that evidence.
- **I02 — Conflict keys and realized blocking:** use actual operation/page/key categories and
  ownership edges to distinguish retries, gate/resource delay and required value dependencies.
  Cancellation bounds issuance; a retry-count ceiling requires a defined no-op/refusal/Unknown
  contract rather than dropping an issued query. No shared-capacity cap or deadline increase is a fix.
- **I03 — Selective versus bulk contextual strategy:** compare exact sparse/frontier and dense
  universes with negative/optional joins, degree and shared reuse. This decides the physical strategy,
  not whether code may silently narrow the graph universe.
- **I04 — Reusable products and private materialization granularity:** choose retained products by
  concrete consumers and exact keys; compare full-buffer ownership/spill/reconstruction against
  recomputation. Stage-by-stage crash-resume must have a named product requirement to justify it.
- **I05 — Diagnostic depth:** actual successful page stats and selected handoffs precede optional
  server tracing/CPU sampling. Counterfactual DAG scaling is only a sensitivity estimate under fixed
  scheduling; add a simulator/harness only if a concrete decision needs it.

## 6. Priorities, prerequisite order and rule impacts

Consequence priority is F01/F02/F04/F05 (systemic work), then F03/F06/F08/F09/F11 (repeated
assurance/preparation and selected execution), F07/F10 (provider work), and F12/F13 (truthful,
proportionate diagnostics). This is a qualitative recommendation, not a measured savings ranking.
Useful local diagnostics can proceed alongside the highest-value source-established corrections.

Prerequisite order differs: settle the F01 completed-source and requested-frontier contract;
carry its exact validity/resource/ownership into F03; define F02 native packet/visibility and F05
atomic host handoff; migrate actual consumers; remove obsolete intermediate protocols. F04 early
demand, F08 contextual selection, F09 borrowed partitions, F10 capture and F11 registry scope can
progress independently where their contracts do not depend on the pivot. Avoid deeply tuning an
intermediate protocol that the selected target will remove. Preserve existing qualified behavior
until replacement controls establish the selected contracts, then regenerate design-phase data;
no historical runtime compatibility or benchmark archive is an implementation prerequisite.

The review does **not** change these rules. Plan creation must present the following stable
impacts for operator disposition, including a meaningful alternative if a rule is retained:

| ID | Current rule / owner | Proposed change and dependent findings | If current rule is retained |
|---|---|---|---|
| <a id="RC01"></a>RC01 | DESIGN §B2/§B7/§B12, semantic-model §15, ADR-0138: native completed contributions are compiler authority; Arrow is bounded compute/transfer | Permit compiler-private typed/spillable completed sources under one semantic owner; durable authority at requested artifact/publication boundary. F01. | Keep persisted authority; pursue packet, scoped preparation and selected graph handoff only, without claiming those remove the whole transport tax. |
| <a id="RC02"></a>RC02 | ADR-0138/0178/0190, storage §6.1–6.2: persisted intermediate completion and native recovery obligations | Scope durable recovery to actual accepted native effects and independent consumers; private uncommitted compute may rerun. Preserve all requested frontiers and actual committed/Unknown outcomes. F01/F02. | Keep genuine current recovery boundaries and account their remaining work; no silent loss of restart guarantees. |
| <a id="RC03"></a>RC03 | ADR-0190 native layout/page/incidence realization and model-derived native schema | Select bounded whole-operation packets, exact prepared correspondence and grouped control effects; change schema/generation only through its owner. F02/F05. | Use current operation pages and set readers; optimize within their established hazard coverage. |
| <a id="RC04"></a>RC04 | ADR-0186/0187 and validation owner: scoped properties and full-factory durable reuse eligibility | Extend property transport and acknowledged-ingress evidence only with exact preserved premises; fresh receiving/changed-boundary checks stay independent. F03/F04. | Reuse current qualified routes and optimize preparation without granting new evidence authority. |
| <a id="RC05"></a>RC05 | Model owner/profile declarations, ADR-0160/0186 compact-unrequested contracts | Define early demand-specific owner/input/representation contracts, retaining all required domains/configuration/coverage. F04. | Existing declared inputs remain mandatory; optimize their acquisition rather than silently skip them. |
| <a id="RC06"></a>RC06 | Existing retained-product policy and ADR-0190 accepted-child joins | Select justified cache products and population timing; optional decline precedes effects, accepted work still joins. F06/F09. | Preserve current population/outcome obligations and improve set lookup/retention without detaching work. |
| <a id="RC07"></a>RC07 | Model catalog scope program / exact nominal closure contract | Allow equivalent frontier-bound contextual execution alongside justified bulk preparation; explicit strategy selection preserves universe/negative semantics. F08. | Retain complete shared adjacency; improve its set execution and amortization. |
| <a id="RC08"></a>RC08 | Acquisition/capture integrity boundary, ADR-0182/0184 and extraction owner | Carry protected capture validity rather than automatically reread whole bytes at every provider completion. F10. | Preserve checks on mutable paths; scope them only where actual protection evidence permits. |
| <a id="RC09"></a>RC09 | ADR-0147/0148, native Phase and evidence observation contracts | Extend typed observation/terminal/statistic vocabulary without conflating diagnostics with native success or cleanup. F12/F13. | Preserve existing authoritative raw observations; label missing causality/stats unknown. |

Pure local algorithms, borrowed partitions and registry-lock improvements need no new ADR merely
because code moves. Meaningful alternatives, new authority or assurance changes use the existing
ADR/design-owner route after operator disposition. No original review, active implementation plan
or accepted ADR was edited by this assessment.

**Completion boundary:** all external recommendations have dispositions in the crosswalk, additional
source findings have corrective contracts and settling scenarios, and the target/uncertainties/rule
impacts are explicit. Subsequent plan creation owns scheduling and migration. Existing PP90/PP99,
schema10 cutover and all previous runtime/qualification gaps remain open under their current owners.
