# Selective graph-compilation reuse, hashing and invalidation

**Proposed target, 2026-10-09; production implementation not_run.** Companion to
[model-owned graph compilation](graph-compilation-kernels-and-hashing-plan_2026-10-09.md), based on
the [graph/hash review](../design_review/reviews/design_review_graph-compilation-kernels-and-hashing_2026-10-09.md).
The [persisted coordinator](persisted-graph-execution-plan_2026-10-07.md) owns all scheduled
source finding dispositions and combined acceptance. This document owns reuse contracts and
GR0–GR6 packages, not another findings/status ledger.

## 1. Selected direction and baseline

Use a model-owned content-addressed cache of pure compiled-operation products, persisted through
the existing native infrastructure, with an explicit dependency graph. BLAKE3 identifies canonical
programs, dependency manifests and result content. XXH3-128 supports in-memory operation
hash-consing and bucket lookup with full equality. Reuse can skip pure computation; current
attempt ownership, completion, provenance and admission are still established independently.

Graph/hash RC02 was **accepted on 2026-10-09**. Cross-run reuse is now scheduled, superseding the
coordinator's earlier dependency-foundations-only horizon. RC01 permits the companion's graph,
native and relational lowerings. GR0/GK0 schedule complementary ADR and architectural-owner
updates before dependent production. This authoring does not amend accepted ADRs or install
runtime policy. Broader native persistence/direct sealing and captured supplier contracts remain.

Baseline and standard are the compiler companion's `8f11d9a0`/core3.3/CI1.5. Source inspection
confirms [ContributionSpec/CompletedView](../../crates/lctx-model/src/domain/completed.rs),
[SourceSnapshot](../../crates/lctx-model/src/domain/analysis/sources.rs) and canonical output
folds identify logical data independently of physical database names. Current prepared inputs,
providers, aliases, checked properties and read handles nevertheless belong to their Workspace
or pinned viewer; equal digests do not transfer those capabilities across attempts.

Equal spec, exact inputs and canonical outputs can retain completed identity across physical
attempts. Equal output rows after changed input provenance need not retain contribution/view
identity. Consequently, pure product value equality, computational preparation and contribution
provenance are distinct. Existing checked-input validation remains attempt-local.

## 2. Library choice and full-operation fit

| Candidate | Decision and reason | What reopens it |
|---|---|---|
| Existing native product persistence + exact manifests + petgraph dependency operations | **Selected.** Fits immutable completed inputs and pure model products without introducing a second tracked database authority. Domain dependency/substitutability and admission contracts are the needed bespoke portion; existing codecs, store, graph/SCC/set and transport libraries do the mechanics. | An operation cannot specify its complete dependencies or its pure result; it is ineligible until that contract is fixed, not silently cached. |
| Salsa0.28.5 tracked queries/backdating/persistence | **Alternative, not selected for this target.** Persistence exists but is not currently enabled. It requires modelling native reads/absence, exclusive serialization and cancellation/clone lifetimes, and another retained ingredient format beside canonical products. | A long-lived fine-grained mutable-query workload can naturally replace the explicit product graph and justify its retained state. Compare complete lifecycle, not only query speed. |
| Ascent0.8.1/Datafrog2.0.1 | **Possible kernel lowerings, not the cross-run store.** Snapshot fixpoints may fit relation operations; Datafrog monotonic variables/fixed-negative antijoin and unqualified Ascent retractions do not establish deletion support. Neither is currently locked. | A named multi-relation fixpoint cannot be adequately expressed with existing graph/set kernels. Qualify deletion/negation, convergence and generated Rust cost before adopting. |
| Differential/DBSP | **Not selected.** Signed-delta propagation is eligible, but the review supplies no qualified project integration; exact immutable-product reuse serves the current batch compiler. | Repeated insertions/deletions dominate an actual incremental workload and snapshot recomputation is inadequate. A subsequent selected engine must subsume rather than duplicate the dependency owner. |
| Moka0.12.16 future cache | **Selected for GR5 replaceable prepared values.** Coalesced fallible initialization and weighted eviction replace bespoke concurrent-init/eviction machinery; exact dependencies, resource ownership and pins stay with the viewer. It is not the persistent product store or canonical program interner. | A concrete primitive limitation prevents preserving the viewer's preparation/cancellation contract; retain equivalent typed owner semantics when replacing it. |

Context7 resolve/query and exact pinned source were refreshed for Salsa/xxhash-rust during
authoring. Salsa's [persistence API](https://docs.rs/crate/salsa/0.28.5/source/src/database.rs) and
[backdating algorithm](https://salsa-rs.github.io/salsa/reference/algorithm.html) are acknowledged;
the shared skill's0.28.4 receipts do not qualify0.28.5 integration. The review's DataFusion55.1.0,
SurrealDB3.3.0, petgraph0.8.3 and fixedbitset0.5.7 evidence supplies current mechanics. No library
version or feature was changed while writing this plan.

GR1 adds one exact direct dependency when production execution is authorized:

```toml
xxhash-rust = { version = "=0.8.19", default-features = false, features = ["xxh3"] }
```

Choose `xxh3_128` for already canonical compact program bytes and `Xxh3Default` for incremental
fragments. The pinned implementation has inline state and a default secret; no boxed secret
allocation is required. Its compile-time SIMD selection needs no new CPU-target flags. Twox-hash
2.1.5 remains a viable alternative: runtime long-input dispatch and raw storage are real
capabilities, but convenience XXH3 streaming allocates boxed secret storage and offers no needed
advantage for these compact internal keys. This is an interface/integration choice, not measured
throughput. Current transitive XXH64/XXH32 features do not already enable XXH3.

GR5 adds `moka = { version = "=0.12.16", default-features = false, features = ["future"] }`
to its serving owner when implementation is authorized. It is absent from the current lockfile;
the [tagged manifest](https://github.com/moka-rs/moka/blob/v0.12.16/Cargo.toml),
[pinned future-cache contract](https://docs.rs/moka/0.12.16/moka/future/struct.Cache.html) and
[initializer source](https://github.com/moka-rs/moka/blob/v0.12.16/src/future/value_initializer.rs)
were checked after Context7 discovery. Errors are uncached and same-key fallible initializers
coalesce when they use the same error type. Generic cache weights/maintenance are best effort;
they do not certify exact resource ownership or snapshot semantics.

## 3. Products and exact dependency contracts

### 3.1 Separate three reusable products

| Product | Inputs affecting its meaning | Portable representation and reuse lifetime |
|---|---|---|
| Canonical operation program | Model/vocabulary, operation revision, typed fields/operators, parameters that change shape and owned policies | Exact framed immutable intent; process/model interning. Physical SQL/table aliases, dense indices and code pointers are not portable identity. |
| Prepared shape/topology | Program, selected vertex/typed directional arc membership, multiplicity/order policy, layout/lowering revision | Compact canonical nominal inventory; attempt/collection or pinned-view preparation. A portable shape can be rehydrated into fresh indices/providers; a live provider is never persisted. |
| Pure result value | Program/kernel contract/code/settings and complete role-associated value/membership/absence premises | Canonical typed output inventory, explicit eligible outcome, count/content and dependency manifest; persisted across runs. Fresh contribution/source descriptors retain current provenance. |

Operation contracts declare which values may substitute and what must be refreshed. Default
result equality is full canonical typed payload equality including identity, attribution and
evidence when those are output fields. A special computation-only shape/value may exclude source
coordinates only when its owner regenerates every affected occurrence/support/output identity
from current inputs; it cannot relabel old source links as current. No general purity inference
from a function signature, relation name or deterministic hash is used.

### 3.2 Complete manifests and keys

The model defines versioned canonical encoding using existing tagged/length-framed `KeySink`
meaning. Rust `Hash`, native JSON order, platform `TypeId`, arena IDs and pointer addresses are
not persistent encodings. The cache request key includes product kind, model/vocabulary identity,
operation/kernel and relevant implementation revision, policy/profile/settings, result contract,
parameters, and role-associated dependency tokens. Store full BLAKE3 keys; never truncate them
to semantic Id or treat XXH equality as a hit.

Dependencies distinguish:

- Exact whole-view source descriptors: conservative default, including absent target namespace
  markers, complete membership and coverage. Changed view identity invalidates the computation
  unless a finer equivalence contract applies.
- Positive selected values: typed nominal key plus canonical payload content, role/association
  and applicable context. These alone never qualify an absence/completeness claim.
- Complete selection domains: program/predicate/root/context plus the authoritative selected
  membership/content and missing/empty/coverage outcome. Enumerate or maintain the full matching
  domain under exact completed input ownership. A new matching row must change the token even
  when every prior returned row is unchanged.
- Topology: full independent vertex inventory and typed/directional/identity-bearing arc
  membership, including internal SCC edges and external associations. Member lists or child
  hash multisets do not encode topology.
- Binding/provenance/effects: capture/provider/model/policy inputs a result actually observes;
  keep full contribution provenance separate from computational equivalence. Effects and
  mutable capabilities are not reusable values.

GR2 first supports exact whole-view tokens; GR4 implements finer domains for the compiled
callable/class/root membership and selected topology operations, with clean-recomputation cases.
These are delivered slices of one target, not a standing coarse-only design. Existing selectors
can compute a compact domain token before rich hydration. If a domain remains expensive to
enumerate, retain its whole-view dependency rather than certify an incomplete positive set.

Native counts/digests from a cache entry are not authoritative evidence that a current domain
is complete. The current bound source computes or independently validates each token. A cached
selector result is valid only after its full declared input tokens match; changed inputs require
fresh selection before a result-value hit can be considered.

### 3.3 Propagation and equality cutoff

The dependency graph links declared source/domain/program tokens to pure products and their
consumers, separate from semantic call/evidence edges. The cumulative driver processes requested
products in the existing producer-readiness order. It recomputes changed dependency domains,
invalidates affected products and either finds a qualified exact entry or runs the pure operation.
Unaffected independent products can be reused.

After recomputation, compare canonical product values. Equal values stop downstream *value*
propagation only where the consumer declares that value as its complete computational premise.
Current contribution/source provenance and all consumers that observe it still refresh. An
ordinary `SourceSnapshot` consumer remains exact-view-dependent; no automatic weakening occurs.
Global admission checks retain all required premises even when no pure kernel ran.

Positive closure cycles use existing SCC/worklist kernels inside one pure product. Changes that
merge/split SCCs recompute the affected topology and schedule; dependency scheduling does not
invoke Salsa-style recursive queries or assume a cyclic transfer is a DAG. Negative domains and
deletions trigger fresh affected snapshot computation. This design does not claim a general
incremental edge-deletion algorithm or introduce background automatic recompilation.

## 4. Persisted store, concurrency and fresh admission

`lctx-surrealdb` owns one disposable, version-qualified compiler-reuse database on the configured
native server, separate from attempt-private and selected operator databases. An explicit reuse
store configuration/capability supplies its namespace/database; fixture launchers own disposable
instances. Its narrow operations are lookup, checked complete insertion, lease/pin and retirement.
Core receives no raw admin/publication client or permission to select/publish this database.
One server/store implementation is reused; no daemon or second primary backend is introduced.

Cache tables contain canonical request/product manifests, typed output bodies/content and
dependency edges. They are derived acceleration state, never completed compiler-view authority
or served snapshots. A version header covers cache encoding/schema and program/result contracts;
incompatible entries are discarded/rebuilt. Private cache metadata alone does not change artifact
format3/completed-state format2 or existing nominal hash domains. Any required published-format
change is a separately explicit GK0 decision, not a silent cache side effect.

Only terminally successful, fully accounted products may enter the cache. Store header/output
manifest/payload as one complete immutable entry or leave it unreachable; interrupted/partial,
failed, pending, timed-out and resource-refused operations never produce reusable success.
Late transport errors keep the entry invisible. Valid semantic complete-empty/NotRequested
outcomes are eligible only with their owner's complete domains and exact requested settings.
Concurrent writers can compute the same key; compare complete canonical entries on conflict and
retain one equal product. A well-formed same-key/different-value conflict is a nondeterminism or
contract failure, not success or blanket `IGNORE`.

Lookup checks version/model/contract, current dependency tokens, inventory/outcomes, actual
canonical payload digests and native stored-body equivalence. Invalid/missing/corrupt derived
entries are evicted and fresh computation supplies the result; diagnostics retain the cause.
A determinism conflict fails the owning operation for correction. No arbitrary cache envelope
mints a checked-input, admitted-facts, publisher handle, reader credential or completion receipt.

Cached rows enter the current attempt through ordinary typed ingress. Register the current
ContributionSpec and exact dependency descriptors, complete its write/binding visibility handoff,
then perform the required semantic/global/final admission and freeze/drain sequence. Cached
canonical rows can match an old value; its completed owner/provenance is never blindly copied.
Independent cold import/restore and actual-state reconciliation remain separate trust boundaries.
Originals, contextual evidence and full model predicates still govern validity. The cache does
not replay providers as an admission oracle or skip finalization.

Retained state uses the existing runtime budget and explicit configured cache byte allowance;
choose the allowance in runtime configuration, not another hardcoded1GiB cap or test-thread cap.
Estimate/charge entry growth before allocation, spill bounded canonical transfers and preserve
live consumer leases during retirement. Evict oldest unleased derived entries to respect the
configured allowance; drop releases attempt-local charges only after actual terminality. Cache
unavailability/capacity exhaustion falls back to fresh computation without changing product
semantics. Native operation failure still follows the existing acknowledged drain/outcome rules.

Pinned serving preparation uses a separate viewer-owned in-memory cache of program/layout or
qualified exact closure results. Its key includes immutable realization, program/root/policy and
binding; leases cannot outlive or cross their pinned snapshot. Cached answers retain current
unknown/absence and evidence/cursor contracts. Snapshot replacement constructs a new owner;
old readers keep their old cache and pin until quiescent. Embedding values keep their existing
cache and spec identity; no duplicate vector cache is introduced.

GR5 uses `moka::future::Cache<ExactPreparedKey, Arc<ChargedPreparedValue>>` and `try_get_with`.
Prepare with viewer-owned cancellation; each request can stop waiting without making its token
the lifetime of shared preparation. Reserve payload and owned-metadata charge inside the
initializer before allocation, and retain it with the value's immutable pin. Eviction removes
the cache reference; active borrowers retain value, pin and charge to their last `Arc`.
Use existing resource admission for distinct-key preparation. Moka's approximate cache weight
does not bound total loading/native/library-internal memory. A 1KiB-ceiling weight unit avoids
truncating byte counts into its `u32` weigher; bypass caching if the checked scaled value cannot
fit, while the actual value remains charged. Viewer retirement first fences new request and
preparation admission, then cancels and drains all initializer/insertion owners and their owned
submitted work. Only after no owner can insert another value does it finally invalidate and
drain applicable cache maintenance, or drop all cache-owned references, before waiting for
external reader/value leases. An early invalidation alone is insufficient: an initializer can
finish and insert afterward. Aborted initializers can be retried by remaining admitted waiters
while the viewer is open; the retirement fence also rejects retries by existing waiters.
Externally spawned work still needs its ordinary cancellation/drain owner. Tests exercise
initializer cancellation/retry and successful insertion racing retirement; the latter must leave
no cache-owned pin or charge after the final release.
The canonical program interner remains non-evicting within its model/program lifetime, because
cache eviction cannot guarantee pointer/dense-ID uniqueness while old values remain alive.

## 5. Packages and complete opportunity coverage

| Package | Working prerequisite | Result and actual consumers |
|---|---|---|
| **GR0 — reuse contracts/decisions** | RC02; resolved target-plan review; GK1 contract design | Model-owned purity/substitutability, canonical program/product/dependency schema, ADR/owner changes, store capability/lifecycle and exact/finer dependency scopes. |
| **GR1 — program identity and interning** | GK1 definitions and GR0 encoding | Direct pinned XXH3 feature, charged exact-equality operation interner, BLAKE3 durable program identity. First scope compiler consumers reuse shape without global hasher changes. |
| **GR2 — portable exact products** | GR0; working GK2 access; existing typed ingress/admission | Native disposable cache and conservative exact-view product reuse. Normalize/class-scope and topology products work across two physical attempts; current contribution ownership/admission is reconstructed. |
| **GR3 — driver and complete dependency graph** | GR2; current completed-source contracts | Cumulative compiler binds products, requested dependencies and reverse edges; changed domain/program inputs select recomputation, pure value equality is distinct from provenance refresh. |
| **GR4 — selected domains and propagation** | GK3/GK4 actual owned operations; GR3 | Finer callable/class/root membership and selected topology tokens, negative/missing coverage, SCC updates and clean-rebuild equality. Adopt reusable upper pure products through the family map below. |
| **GR5 — pinned request preparation** | GK5 binding; GR1 layouts and GR0 leases | Moka-backed viewer-owned inspection/browse preparation with coalesced fallible initialization, independent request outcomes, retained charges/pins after eviction and cancellation/retry controls. |
| **GR6 — integration, retirement and qualification** | GK6, GR2–GR5 consumers | Cache-off/cold/hit/reload/changed-input equivalence, corruption/finality/concurrency controls, surviving PC/CU/BC and native/MCP acceptance; obsolete competing preparation removed. |

GR0 and GK1 design contracts together; actual GR1 depends on the implemented declarations.
GK2's ordinary execution works before persisted hits. GR2 delivers exact reuse before GR4's
selected-domain propagation, so no incomplete domain manifest gates usable fresh computation.
GK5 can precede GR5. Shared schema/manifest/identity edits have one integration owner.

| Review opportunity | Scheduled route and applicability |
|---|---|
| Dense indices/bitsets/reusable visits | GK2/GK4; choose dense state for reusable selected universes, retain native/spill access for sparse/skew. Nominal mapping and universe length/generation are required. |
| Interned selectors/fields/programs | GR1 plus GK1/GK2; exact immutable program structure reused across parameterized roots. Field/relation meaning stays model-owned. |
| Hash-consed expressions/subprograms | GR1 interns pure operation subprograms; GK4 inspects repeated transfer/term structures and reuses existing BDD canonicalization. A second expression interner is added only for a named unequal existing boundary; absence of such a consumer is a recorded decision, not overlap deferral. |
| Topology/program fingerprints | GR1/GR2/GK4; role-associated canonical program and vertex/arc inventories, including internal SCC topology. |
| Selected results/SCC propagation | GR3/GR4; complete selection domains, value/provenance separation and fresh computation after deletion/negative updates. |
| Cross-run persistence | GR2–GR4; portable canonical products, current ownership/admission, explicit capacity and retirement. This is active scope. |
| Same-pin request reuse | GR5/GK5; reusable metadata/layout first, exact closure values only under complete request dependency and evidence contracts. |

Facts-provider outputs remain eligible only under exact captured contract/settings/source and
complete coverage; an external acquisition/runtime effect is never memoized as pure. GK3/GR4
migrate Normalized and execution/support products; GK4/GR4 cover Structural/Local/Model/Summary,
selected Analytic and C0–C2/S0/E0 pure preparation/results. Existing embedding inference/value
reuse remains its owned subsystem. Each actual operation gets an eligible product contract or
an explicit effect/semantic reason it must execute afresh; no blanket caching of every stage.

## 6. Acceptance, investigations and cutover

**Planned / not_run.** Independent small expected graphs/rows challenge production declarations;
clean-rebuild equivalence is necessary for finer incremental claims but is not the only oracle.
Use normal available parallelism and current release-profile controls, disposable native fixtures
and exact module/target selection. GK7/GR6 share one final-source affected assembled acceptance
with PC6/CU6/BC3/BC5 obligations; do not rerun broad journeys after each package.

| Case | Required distinction |
|---|---|
| Same input, new physical attempt; cache reload | Equal pure values, reconstructed new native providers/ownership and complete current admission; no old checked capability survives. |
| Unrelated domain edit; matching-row insertion/deletion | Independently unaffected products reuse; matching membership/absence invalidates, including a previously empty lookup. Compare clean rows/evidence. |
| New target for unresolved lookup; coverage/context change | Unknown/absent/resolved and foreign-context outcomes remain correct; whole-view tokens protect unqualified finer cases. |
| Same pure values, changed provenance/span/provider/policy/code | Only explicitly value-only consumers may cut off; contribution/input/source/support identities refresh or computation invalidates as declared. |
| SCC merge/split/internal-edge/parallel-arc change | Topology key and required schedule change even when members or node payloads are unchanged. Preserve isolates and exact witnesses. |
| Forced XXH collision; reordered canonical fragments | Different programs remain different under full equality; equivalent canonical values/order are stable, full BLAKE3 identities unchanged. |
| Corrupt/stale/partial/native-tampered cache; same-key different valid result | Invalid derived entry falls back fresh; terminality and actual-state rejection remain. Nondeterministic result conflict is a failure. |
| Concurrent readers/writers; cancellation, eviction and retirement | Live leases/state remain charged through terminality, no incomplete entry becomes visible, independent consumer/pin survives another cancellation. Race successful initialization against retirement: new admission is fenced, all insertion owners drain before final cache release, and no cache-owned pin/charge survives that release. |
| Cache-off/cold/hit versus full compiler/serving outputs | Exact both-profile/frontier/catalog/evidence/unknown outcomes and original bytes; final compile/admit/seal/backup/restore/native MCP boundaries remain independent. |

GR2/GK2 settle whether manifest/key construction repeats the avoided rich scan; GR4 settles
value-only cutoff and selected-domain completeness; GR5 settles request lifetime/retained state.
Use static inspection or small controls where sufficient. Actual quantitative benefit requires
cold/no-hit/warm/reload measurements of the complete chosen operation, including encoding,
construction, transfer, admission and retained memory; no benchmark campaign is required to
publish the structural correction or this plan.

Rebuild incompatible disposable cache products from pinned inputs, remove old cache formats and
competing implementations after integration, and retain no historical runtime generation without
a current consumer. Runtime/operator adoption is separately authorized; these documents schedule
fixture-backed implementation and do not activate an operator cache/database. No protected
evaluation, real-library/Qwen run, compatibility reader, floating toolchain or wholesale lock
update is introduced. Authoring validation is docs-check, scoped turn-end and independent target
review, distinct from implementation acceptance and measured reuse benefit.

**Target reviewed, 2026-10-09:** the [independent plan-target review](../design_review/reviews/design_review_graph-compilation-and-reuse-plan_2026-10-09.md)
accepts the combined target at Proposed/interface strength. Its separate F01 corrected the GR5
retirement order and retry fence in §4; the coordinator §8 owns current disposition and the
required runtime race control. Source graph/hash findings remain open; reuse is not implemented.
