# Catalog efficiency: Base, Summary and analytic preparation

**Proposed, 2026-10-15.** Companion to the [coordinator](catalog-efficiency-plan_2026-10-15.md).
It develops F05/F06 and I05 from the [source review](../design_review/reviews/design_review_catalog-completion-efficiency-target_2026-10-15.md).
Coordinator §6 owns disposition. These changes serve explicit Behavioral support and named
analytics; they do not make Behavioral execution an ordinary Catalog prerequisite.

## 1. Responsibility and foundations

The model owns root/frame meaning, provider qualifications, call binding, conditions, work,
proof identity, pair outcomes and partial results. cpg-core obtains exact completed inputs and
adapts execution; pure graph and finite kernels retain their own semantics.

Read [Behavioral analysis](../design/sections/behavioral-analysis.md),
[semantic transfers/coverage](../design/sections/semantic-model.md#section-15-6) and
[analytics](../design/sections/analytics.md). Existing prepared edges/frontier lookup, shared
frame metadata, lazy consumed-attempt preparation, Discovery indexes, SCC ordering, Pareto
frontiers and deduplicated queues are useful. F05/F06 remove remaining repeated input/correspondence
preparation, not replace those algorithms based on a timeout.

The workload is many expression roots with overlapping dependencies, many calls sharing a few
signatures, recursion/reconvergence and high-degree analytic scopes. Bounded output alone does
not establish a feasible route. Shared preparation must also avoid release-wide eager hydration,
unbounded retained indexes and prematurely exposing malformed unconsumed data.

## 2. CA00 — F05 bounded Base cohorts with independent root semantics

Current [Base execution](../../crates/cpg-core/src/semantic_execution.rs) selects, hydrates and
evaluates each expression root separately. [BaseScopes](../../crates/cpg-core/src/base_scope.rs)
walks declared inputs for each grain, while
[PreparedExecution](../../crates/lctx-model/src/domain/execution/evaluation.rs) reconstructs
syntax preparation despite shared frame metadata.

Consume bounded compatible root cohorts under exact frame/input selection. Share immutable
dependency acquisition, source/position correspondence and syntax indexes when roots overlap.
Use existing PreparedEdges batch/partitioned hydration/PreparedPositions foundations instead
of a general memoization framework. Known-empty selections can skip needless reader/query setup
only while retaining their owner/schema/emptiness obligations.

Keep a root membership partition through acquisition and evaluation. Each root retains its
context, proof/evaluation state, error exposure, work admission and conservative verdict.
Do not memoize result values solely because some dependencies overlap, or let one root's success
substitute for another's evidence. Frame/model/configuration changes are real compatibility
boundaries. Cohort size follows admitted useful work and actual retained bytes, not thread caps.

Specify the point at which shared preparation is charged and dropped. Avoid eager hydration of
unreachable malformed companions or a whole release just to amortize setup. Cancellation/refusal
must preserve accepted native batches through acknowledgement/drainage; sharing an Arc does not
share session/pin ownership.

**Delivery:** migrate cpg-core acquisition and the model's prepared syntax consumption together,
retain the same public root answers/proofs and remove redundant per-root whole-inventory setup.
Changes to native ingress are composed through CN03 where applicable; the logical cohort design
can proceed independently.

**Controls:** overlapping and disjoint roots; shared descendants; mixed frames/contexts; shuffled
root order; duplicate root requests where supported; unreachable malformed data; known-empty
inputs; per-root refusal/exhaustion; cancellation; independent known finite answers and proofs.
Inspect preparation reuse under exact inputs; historical phase counts are not RPC or speed claims.

## 3. CA01 — F06 charged ordered Summary correspondence

Current [Summary production](../../crates/lctx-model/src/domain/execution/summary_production.rs)
caches consumed attempts, but first-use preparation scans broad immutable headers, declarations,
parameters, links and arguments; residuals search events/seeds again.

Build charged borrowed correspondence buckets once per exact immutable Summary invocation.
Index by the actual attempt/symbol/declaration/signature/origin keys needed by consumers, preserving
input order and every competing row. Lazily realize an attempt only when consumed. Reuse
signature ingredients across attempts without sharing caller binding, admission or success.
A one-row map that silently chooses a declaration is invalid.

Preserve exact owner/frame/context and source identity, ambiguity/missing-premise timing,
caller/callee ports, dynamic conditions/guards, attempted-pair outcomes, fresh proofs and residual
coverage. Leave the existing graph-bound SCC schedule, nondominated depth/cost frontiers and
queue semantics intact unless an independently established defect requires another scoped decision.
No SCC-dominance inference follows from the recursive run.

Preparation must remain borrow-scoped and charged through its final consumer, then release before
unrelated later publication where possible. Do not construct rich unconsumed attempts or replace
pair-specific outcomes with a cached callee result.

**Delivery:** migrate call preparation, binding and residual consumers to the shared inventory;
delete superseded broad scans while keeping public and private paths on the same algorithm.
No new persistent cache or cross-invocation authority is implied.

**Controls:** shared signatures with distinct bindings; competing headers/declarations; mixed
contexts; unconsumed malformed attempts; recursive and reconvergent calls; equal-cost distinct
witnesses; exhausted pair work; residual identity and independent finite known answers.
Existing replay can supplement comparisons, but is not the sole oracle for changed correspondence.

## 4. CA02 — I05 analytic expansion and representation

Identify the actual consumers of co-use target pairs, weights/evidence, communities/centrality,
FCA/RCA concepts and lattice relationships. Current degree-squared admission and row/column
bitsets have real purposes. Exhausted layers correctly withhold a complete combined partition.
There is no established equivalent pair elision.

Compare sparse incidence, suitable bitmap/library representations and streamed aggregation with
current expansion at the complete operation: preparation, graph conversion, kernel support,
intermediate/retained state, evidence production, deterministic output and lifecycle. Use the
existing rust-graphs/fixedbitset/library routes at actual pins when choosing a mechanism.

Preserve the declared universe, isolates, multiplicity/weights, projection/method/settings/seeds,
pair evidence, lattice meaning and partial/exhaustion semantics. A global analysis may need
more than the output subset. If named products intrinsically require pair expansion, acknowledge
that necessary work and choose a credible admitted physical route rather than claiming a sparse
input automatically removes it.

**Decision:** choose a supported representation per consumer, retain current forms with a
supported rationale, or state a specific deferred capability and reopening trigger. A different
published product/semantic policy needs its own decision approval before delivery. Do not
silently omit pairs, isolates or exhausted evidence to satisfy limits.

**Conditional delivery:** implement the selected construction/kernel/consumer changes together,
remove redundant copies or conversions, and regenerate changed project-derived products. Keep
ordinary Catalog NotRequested domains and optional analytics settings meaningful.

**Controls:** high-degree shared utilities, sparse/dense attribute incidence, empty graphs,
isolates, duplicate weighted observations, disconnected components, deterministic ordering/seeds,
finite known concepts/lattice relations and exhausted layers. Expected outputs must come from
independent small known graphs/contexts or a justified independent oracle, not the same new transform.

## 5. Dependencies, acceptance and preserved boundaries

CA00 and CA01 can proceed independently after current exact input/borrow/resource contracts are
reconciled. CA02 first needs named consumer semantics, not completion of either Behavioral fix.
One integrator owns shared model/execution interfaces; separate implementation packages do not
create runtime stage barriers.

Pure kernels use database-independent controls. Persisted cohort acquisition and independently
completed outputs use affected native controls where those claims are made. Compile affected
crates and run scope-end applicable hygiene once, then participate in coordinator CE90.
No whole-family or full journey after every slice, no benchmark prerequisite, and no weakening
of proof, provider, work or coverage semantics. Old recursive/finite acceptance retains its
original outcome until its own owner reconciles it later.
