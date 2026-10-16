# Catalog efficiency: compiler preparation and independent admission

**Accepted directions / implementation in progress, 2026-10-15.** Companion to the [coordinator](catalog-efficiency-plan_2026-10-15.md),
which alone owns disposition. This document carries I02, the capture-integrity half of I03,
I04 and I06 from the [source review](../design_review/reviews/design_review_catalog-completion-efficiency-target_2026-10-15.md).
[ADR-0174](../adr/0174-catalog-preparation-and-lexical-postings.md) owns the selected decisions.
The coordinator §6 alone owns implementation, focused receipts and remaining acceptance.

## 1. Functional responsibility and foundations

The compiler orchestrates exact captured providers, completed inputs, normalization, Catalog
products and independently earned receiving authority. Model declarations own identities,
roles, dependency keys, ambiguity, coverage and graph meaning. Preparation changes how an
operation obtains its inputs; it does not redefine those inputs or grant success to another owner.

Read [acquisition/extraction](../design/sections/acquisition-and-extraction.md),
[semantic compilation](../design/sections/semantic-model.md#section-15-11) and
[storage](../design/sections/storage-and-publication.md#section-6-1).

| Foundation | Reuse and assessed limit |
|---|---|
| [Workspace composition](../../crates/cpg-core/src/workspace.rs) and [production](../../crates/lctx-compiler-store/src/compiler/production.rs) | Private per-relation exact composition precedes gate-held predecessor recheck/bind/handoff; only changed outputs rebase |
| [Compiler ingredients](../../crates/lctx-compiler-store/src/compiler.rs) and [membership demand](../../crates/cpg-core/src/workspace/membership_demand.rs) | Binary partitions reuse unchanged leaves/subtrees and retain according to actual consumers; not repeated native rereading of every contributor |
| [Compilation driver](../../crates/cpg-core/src/compilation.rs) | Exact ready scheduling and within-phase overlap are implemented; broader phase barriers may protect shared epochs/admission |
| [Captured bundle](../../crates/cpg-extract/src/bundle.rs) and [capture](../../crates/cpg-extract/src/capture.rs) | Writable captured inputs justify opening/post-provider checks; immutable ownership is not already supplied by readonly permissions |
| [Catalog core](../../crates/cpg-core/src/catalog_core.rs) and [products](../../crates/cpg-core/src/catalog_products.rs) | Bounded public-name windows and whole-stage reuse; one admitted granular request survives lookup/build/retention with pure-only fallback |
| [Reference admission](../../crates/lctx-compiler-store/src/admission.rs) | Protected current-owner, schema/subtype/empty and failure-atomic premise obligations; invocation IPC shares original inputs and exact target-family projections without sharing validity |

Distinct Pyrefly resolver States, independent latest Ruff and ty role/source correspondence have
real consumers. Pyrefly pool/source-buffer and canonical span reuse are already useful. Do not
merge provider authority or analyzer working sets merely to make a preparation graph smaller.

## 2. Investigation and delivery contract

For each package, state the exact consumer and input premises, compare the existing route with
the named alternatives, and decide from removed work, retained working set and full lifecycle/
integration burden. Use source/types and revealing cases first; a numerical benchmark is not
needed to establish repeated preparation.

Each decision records what it enables, why a retained boundary is necessary, what would reopen
the choice and which consumers must migrate. A beneficial selected route is implemented in
this track. If evidence remains inadequate, retain the precise limitation and dependent work
open; do not schedule a speculative cache or weaker authority as an interim solution.

Changes to eager visibility or persisted selector semantics need additional explicit rule approval
and the existing ADR route. These choices are not covered by the four already-approved impacts.

## 3. CC00 — I02 composition, handoff and broader readiness

### Composition decision

**Selected, 2026-10-15:** private eager composition outside the gate, exact affected-predecessor
recheck and per-relation rebase, followed by gate-held binding/coherent visibility. Native failure and
Unknown remain terminal. Existing binary ingredients and actual ownership are reused. Demand-visible
assembly is not selected; exact selected consumers still require an earned view. ADR-0174 owns rationale.

Trace actual current/frozen/running/remaining consumers and the point at which they require an
exact cumulative union. Compare:

- current eager union using existing binary ingredients;
- private merge preparation with checked atomic handoff against the still-current inputs;
- incremental exact union properties;
- demand assembly that materializes only an actual selected consumer's required union.

Evaluate actual changed-aggregate merge work, handoff exclusion and retention. Do not assume
all prior contributors are reread or that cached counts prove nominal validity. Equal rows
coalesce; unequal full payloads refuse; exact rows, physical copies, vocabulary/input context
and count/conflict visibility must remain atomic for the consumer that relies on them.

A private merge route must recheck its relevant premises when publishing and handle concurrent
completion/cancellation without exposing a stale frontier. Demand assembly must preserve the
earlier selected-port contract rather than replace an exact view with pending metadata.
Retain optional retention's genuine pre-effect decline and release according to actual demand.

### Readiness decision

**Retain, 2026-10-15:** current Facts/Normalized barriers. Six input-free configuration producers
are legal early work, but moving them changes the admission inventory/frontier and overlaps lifecycles
without a substantive early consumer. Reopen for a material consumer with a complete earlier authority
slice; this is a supported retain decision, not an assertion that no early computation is legal.

Treat cross-phase readiness as a separate question, not an automatic consequence of changing unions.
Find a concrete upper consumer that could earn exact epoch/input authority before all facts or
normalization finishes. Compare its complete early route against the existing Facts-epoch and
normalized-admission barriers. A waiting stage alone is not evidence of unnecessary work.

If a legal earlier consumer exists, specify the exact completion/admission slice it consumes,
who earns it, how later failure/cancellation affects it, and how shared vocabulary/epochs remain
consistent. Then extend the existing readiness scheduling without relabeling partial inputs as
completed or reintroducing gate-wait starvation. If no such consumer exists, retain and explain
the barrier. Composition can improve even when broader phase fusion is rejected.

### Conditional delivery and controls

Deliver the selected composition and/or readiness route with all current/frozen/request/retention
consumers, release behavior and necessary decision changes. Preserve existing exact preparation
and optional cache semantics; retire only replaced paths.

Controls include shuffled contributor completion, reconvergent overlapping memberships, equal and
unequal copies, concurrent handoff/cancellation, old selected ports, dropped optional retention,
partial-provider failure, epoch disagreement and a real legally early consumer if selected.
Use actual native composition for claims about persisted exact views; pure scheduling tests do
not establish native admission. Coordinate CN00/CN05 only when the selected route consumes their
cut, and CN06 only if physical selector representation changes.

<a id="cc01"></a>
## 4. CC01 — I03 captured-input integrity at justified grain

**Retain, 2026-10-15:** full opening and post-provider byte/membership checks of the mutable captured
closure. Support/docs/examples/configuration remain revision inputs, and readonly permissions are not
an immutable owner. Provider close verifies before final flush/outcome; Acquisition/Assembly retain
explicit exclusions. Existing persistent-mutation/sticky-poison and interrupted-provider controls
provide the narrow functional route. Reopen for a complete immutable owner or an adopted narrower
provider contract, not an mtime/checksum cache. ADR-0174 owns the comparison.

This package owns actual analysis inputs. [CD02](catalog-efficiency-development-plan_2026-10-15.md#cd02)
owns auxiliary repository source observations; one cannot prove the other's guarantees.

Identify opening and each post-provider verification's promised captured membership/content,
mutation exposure and failure timing. Establish which files each provider actually consumes,
including non-Python documents/examples/configuration where relevant. A checksum result has
scope and time; readonly permissions do not make same-process storage immutable.

Compare retaining complete writable-closure checks with:

- an actual immutable capture owner covering membership, bytes and lifetime;
- equally complete consumed-scope verification for a provider whose contract permits it;
- sharing a check only while the identical protected premise remains valid.

Account for changes during opening, during provider work and between providers. Narrow scope
only if no admitted result or advertised capture guarantee depends on omitted inputs. Keep
acquisition projects, provider versions/configuration and original source roles exact. Integrity
is not replaced by mtime, a previous digest or a source-to-binary guess.

If selected, deliver the new owner or scoped verification contract through every affected provider
and finalization path, release protection at its actual terminal, and delete redundant checks only
after their guarantee is supplied elsewhere. If no simpler complete route exists, keep the checks
with a supported explanation and mutation-related revisit trigger.

**Controls:** bytes and membership changed before/during/after provider work; consumed and unconsumed
files; configuration/doc inputs; interrupted provider; concurrent borrowers; original attribution
and exact provider completion. Known-good provider outputs alone do not establish integrity.

**Implementation follow-up,2026-10-16:** immutable completed-input catalogs now capture charged
ports without eagerly earning every declared table. DataFusion resolves a requested port into the
unchanged earned NativeTable before logical planning/optimization, under its captured producing
owner. Qualified/plain aliases share that exact port; predicates have private charged candidate
overlays, and fixed-port templates retain exact provider identity checks. Captured aliases are
immutable. Unknown/empty views gain no authority from metadata or row counts. This CC00/CA00
preparation refinement changes no native schema, admission guard or deadline. The coordinator §6
owns review, focused failures/corrections and remaining consumer acceptance.

**Closeout implementation,2026-10-16:** ingress operation acknowledgements/refusals and actual
driver joins now have distinct results. Ordered related operation errors preserve diagnostics
without inventing finalizer failures. Lost acknowledgements, missing driver owners, typed join
errors and nested Unknown remain strict. Protocol-only controls construct their process signal
owner once; actual native controls retain receipt-backed factories and strict drainage. The moved
binding control now earns its real completed contribution. Source review accepts these corrections;
coordinator §6 owns current checks, original failures and pending native/provider acceptance.

## 5. CC02 — I04 useful Catalog requests and cross-window preparation

**Selected, 2026-10-15:** one charged complete-key request moved through lookup/computation/retention;
cache absence constructs none, while fresh-only eligibility can still retain. Optional Resource
pressure releases it and retries only the refused pure preparation once; prepared rows survive and
output headroom is checked before effects. Keep window-local borrowed indexes: rich global backing
extends lifetimes and signature-only sharing omits context/evidence distinctions. ADR-0174 owns rationale.

C0's bounded public-name windows are a strength. Alias scope already preselects module/name
dependencies, and whole-stage reuse precedes C0. The issue is overlapping useful ingredients
and request preparation within actual granular-cache demand, not a proven release-wide
exposure-by-binding cross-product.

Map complete granular dependency-key construction and the work needed before lookup. Compare
carrying the one admitted request through fresh retention, sharing exact immutable ingredients
across overlapping aliases/signatures/windows, and omitting a granular cache when lookup and
retention require nearly the same rich computation. An optional-cache-disabled execution must
not perform preparation solely for a cache it does not use.

Keep root/public exposure membership, source/effective option meaning, provider/profile/model/
configuration/input/vocabulary dependencies and current authority complete. Sharing a signature
ingredient must not transfer an alias's evidence or caller context. Retained keys and values
keep actual resource/backing owners; use live demand rather than speculative warm retention.

If selected, deliver one request interpretation consumed by lookup/computation/retention as
appropriate, or simplify away an uneconomic layer. Migrate every producer/consumer and update
retention/refusal behavior together. Do not cache partial computation as complete success or
let optional retention enlarge the mandatory live working set.

**Controls:** repeated aliases/signatures across windows; disjoint contexts; changed configuration
and exact dependency inputs; cache enabled/disabled; whole-stage hit; granular hit/miss; optional
pre-effect decline; stale keys; equal public names with different context; exact original witnesses.
Demonstrate reuse scope or removed construction directly; no arbitrary cache-size/thread policy.

## 6. CC03 — I06 independently earned reference-admission preparation

**Selected, 2026-10-15:** lazy invocation-owned, full-schema Arrow IPC captures demanded original
inputs once; exact target-set ID/subtype projections reuse local spillable preparation. Full-payload
null-sensitive pairwise SQL remains against local IPC because a grouped replacement has not qualified
all supported types. Original owner/current checks, empty/memo applicability, complete EOF and
failure-atomic memo commit remain. Local IPC is preparation, never completed authority (ADR-0174).

Independent receiving validates original protected sources in the receiving owner. Never replace
it with producer success, a portable memo, a reused digest or diagnostic replay.

Map same-family selection comparisons, reference-target filtering/unions and field-level join
planning. Determine which fields use the same exact target selections and column/schema contract.
Compare a charged per-call ordered correspondence inventory and shared exact target preparation
against current pairwise work. Suitable native set/joins and bounded DataFusion planning are
eligible; choose by the complete access/transfer/admission operation.

Grouped conflict validation still compares full fields, nulls and competing physical copies.
Preserve original owner/session context, current checks before/after use, known-empty obligations,
missing-target refusal, subtype/column schema and complete EOF. Empty data does not excuse
applicable owner/schema validation. Memo publication is failure-atomic: no field or later caller
may borrow success from a partially failed reference phase.

Keep preparation and validity separate in the interface. One target inventory may feed multiple
field checks, but each check retains its declared premises and any independently required
observation. Charge retained plans/correspondence/batches for the actual invocation lifetime;
release before unrelated later admission phases where feasible.

If beneficial, deliver the prepared route and all field/subtype consumers together, deleting
superseded broad preparation. If retaining the current route, explain the obligations making
its broader scope necessary and the workload that would reopen the choice. The historical
approximately 100-second admission intervals do not prove which scan dominated.

**Controls:** shared target sets across fields; same-family equal/conflicting copies; nulls and
missing references; empty source/target; subtype mismatches; changed original current owner;
late failure after an earlier field passed; cancellation and physical EOF; no leaked memo on
failure. Use independent receiving on actual native content for persistence claims. CN06 is a
prerequisite only if the selected route changes the consumed selector contract.

## 7. Integration and completion

These are independent investigations with interfaces, not a serial pipeline. Decide representation/
visibility before implementing incompatible consumers, but do not block CC01 or bounded CC02
work on a rejected phase-fusion proposal. Coordinate shared compiler/admission files with one
integrator and preserve concurrent source work.

Reassess local-spill preparation plus native ingestion if the combined route removes more work
without duplicating persistence/recovery authority; a complete alternative must include originals,
independent receiving, reuse and all lifecycle costs. Library-built relational or graph operations
can execute model-owned semantics, but moving a check does not remove its completeness obligations.

Functional completion requires the decision, selected delivery, actual consumer migration and
revealing controls. Compile affected crates, retain current pin/profile routes, and participate
in CE90. Changed project representations regenerate under CE80 after drainage; no compatibility-only
reader or historical runtime generation is retained without a named consumer. Existing plan
dispositions and real-library/operator qualification remain outside this document.
