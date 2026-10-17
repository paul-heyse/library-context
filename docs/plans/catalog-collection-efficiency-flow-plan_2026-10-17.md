# Catalog collection efficiency: canonical preparation and admitted flow

**Approved implementation plan · 2026-10-17.** Companion to the
[collection coordinator](catalog-collection-efficiency-plan_2026-10-17.md), whose §6 alone owns
current disposition and execution state. This document develops **TF00–TF03**, consuming source
review **P:F02, P:F03, P:F05, P:F06 and P:I04**. `P` means the
[Python fact-collection target review](../design_review/reviews/design_review_python-fact-collection-efficiency-target_2026-10-17.md),
not the runtime review's similarly numbered findings. Package descriptions below are requirements,
not a second status ledger. TF00/TF01/TF02 execution is authorized; TF03 remains later scope.
Actual evidence and operation limits are recorded by the coordinator.

## 1. Scope and assessed foundations

The intended result is the same attributed native flow facts with one canonical occurrence read,
indexed native correspondence, cancellation that reaches actual work, and admission before
adapter-owned raw results grow. Catalog keeps Flow `NotRequested`; Behavioral remains the explicit
consumer of ty flow. Ruff cancellation also applies to contextual collection used by Catalog.
The workload is complete pinned releases and corpora, including many sources, many uses in one
module, many reaching candidates per use, loops, captures and wider predicates. A small outgoing
batch and honest resource refusal do not alone establish a usable route for this workload.

**Implemented / Interface-checked baseline, inspected 2026-10-17:** main
`ffc4dc7d7c6feb87766e8bd9b64e2997c19a4dae`. The source review examined `842e1f4e`; the
relevant current adapters and their owners were read again for this plan. The existing runtime
foundations are implemented, with enclosing acceptance transferred to the
[collection coordinator](catalog-collection-efficiency-plan_2026-10-17.md#6-sole-disposition-and-current-checkpoint).
They are prerequisites to reuse, not proposed replacements or claims of whole-product qualification.

Architectural authorities are [extraction §4](../design/sections/acquisition-and-extraction.md),
[semantic §15](../design/sections/semantic-model.md), especially §15.1/§15.11, and
[DESIGN §B1/§B5/§B8](../design/DESIGN.md). The foundation assessment uses core/template 3.3,
heuristics 1.0, code-intelligence profile 1.5 and the repository binding selected by
[standard.toml](../design_review/design_principles/standard.toml). The
[review evidence](../design_review/evidence/2026-10-17_python-fact-collection-efficiency/README.md)
and [library comparison](../design_review/evidence/2026-10-17_python-fact-collection-efficiency/library-comparison.md)
remain dated source evidence, not new executed results.

| Foundation and current source | Assessment and target consequence |
|---|---|
| `cpg-extract/src/bundle.rs`: `StageContext`, `register_cancellation`, guarded input/output, provider join/drain | Retain one producer authority. Expose an internal cloneable check/registration handle from that owner so native work and callbacks need not compete for a mutable `StageContext` borrow. Do not introduce another cancellation registry or detach work from its join. |
| `lctx-model/src/domain/resources.rs`, `charged.rs`: attempt pool, `StateCharge`, admitted mutation/transfer | Reuse these implemented capabilities for raw adapters and new indexes. Reservations cover owned capacity and live scratch before allocation, and follow actual owners. They are not host RSS accounting. |
| `domain/attachment.rs`: `OccurrenceIndex::new/from_parts`, charged alternatives, exact/ambiguous/budget outcomes; `cpg-extract/src/assembly.rs`: `Attacher` | Keep this model operation authoritative. A constructor over already owned occurrence rows is a physical preparation seam, not a new attachment policy or inherited admission result. |
| `cpg-extract/src/ty_flow.rs`: `Index::new`, `Writer::attach`, `Writer::write` | Existing charged lexical partitions are useful. The full occurrence read is repeated by lazy attachment, and native shapes are scanned for every attachment. Correct these before changing source partitioning. |
| `cpg-flow/src/lib.rs`: `visit_modules`, `module`, `Walk`, `Places`, `Visitor`; `predicate.rs`, `narrowing.rs` | Full virtual preparation and per-module ordered consumption preserve import context and stopping behavior. Whole raw vectors, binding temporaries, formula clones and correspondence caches grow before downstream admission. Add cooperative checks and a synchronous admitted consumption seam here. |
| `cpg-extract/src/ruff_context.rs`: production `Collect`; `pyrefly_stage.rs`: `context_rows` caller | The real collector must implement the fork's existing checkpoint using provider cancellation. A test-only cancelling observer does not establish production connection. Keep diagnostic channels and contextual completeness separate. |

This is a focused assessment of the foundations needed by these packages, not another review of
the complete compiler. FP-01/02/04/06 favor the existing analyzer/model boundary and DB-free local
tests. FP-07, H9/H12/H16/H17 favor one preparation, candidate buckets and releasing completed raw
groups. DP-20/CI-08 require admission before owned expansion; H11 requires the consumption route
as well as refusal. None of those goals needs a new scheduler, general cache, queue framework,
concurrency cap, allocator framework or persistent token dataset.

## 2. Shared contracts and dependencies

| Boundary | Contract retained or delivered |
|---|---|
| Captured source → analyzers | Exact captured membership/bytes, explicit interpreter/platform and source roles; independent Ruff original-byte syntax and ty's separately characterized runtime view remain distinct nominal families. |
| Canonical preparation → flow lookup and attachment | One declared exact `Occurrence` input stream supplies owned rows and both physical views. Model attachment independently validates its required premises and retains its own reservations. Sharing input does not confer independently earned admission. |
| Native shapes → canonical queries | Span buckets retain every supported kind/role alternative; the current optional-kind filter, scalar-wrapper exclusion, sorting/deduplication and uniqueness refusal remain exact. Native indices never become canonical identity. |
| Native execution → typed writer | Synchronous delivery of completed groups, charged retained correspondence and explicit module terminal outcome. Consumer failure stops immediately, and no later top-level module collection query starts. Required cross-module native dependency queries within the current collection remain available. No success terminal follows cancellation, resource refusal or consumer error. |
| Provider → compiler completion | Pending output may have been handed off before failure; provider failure, actual native join and ingress drainage still prevent a partial contribution becoming a completed view. A delivered group is not contribution completion. |

TF00 needs the existing guarded stream, attempt budget and model attacher. TF01 can begin alongside
TF00 after their shared cancellation-check and lifetime signatures are settled. TF02 consumes
**working, migrated** canonical preparation and cancellation, not merely proposed interfaces.
TF03 begins only after TF00/TF01/TF02 have implemented and locally verified those contracts.
Investigation can inspect later routes earlier, but cannot select a dependent execution design
on imaginary ownership guarantees. Runtime/native finality changes remain with the coordinator.

The implementation surfaces overlap: `bundle.rs`, `ty_flow.rs`, `cpg-flow/src/lib.rs` and its
predicate/narrowing helpers. Coordinate edits by these surfaces even when packages are logically
independent. Ruff's caller/checkpoint edit also overlaps analyzer companion work on shared
context preparation. Native node enumeration overlaps any separately selected node-kind conversion;
neither author should introduce a competing correspondence table.

## 3. TF00 — One canonical preparation and exact native-shape lookup

**Sources:** [P:F02](../design_review/reviews/design_review_python-fact-collection-efficiency-target_2026-10-17.md#F02),
[P:F03](../design_review/reviews/design_review_python-fact-collection-efficiency-target_2026-10-17.md#F03).

Prepare the occurrence inventory once during `TyFlow::Index::new`, before native module queries.
Use one charged row vector with an identity-to-position view for existing flow lookups; do not
clone that inventory into a second full map. Retain source/identity membership needed by imports,
lexical ownership and cross-source target closure. Build the existing model `OccurrenceIndex`
from the vector's borrowed slice through an `Attacher` constructor and install it into the
stage's existing attacher slot under `StageContext::guard`. `Writer` continues using that attacher.
The internal installation operation checks failed/cancelled stage state, uses the same attempt
pool, and refuses an inconsistent already prepared replacement; it does not expose caller-built
attachment policy. The ordinary lazy attacher remains for stages that have no prepared inventory.

Read batches incrementally, admitting row clones and lookup capacity before retaining them.
Release each input batch after its useful fields have been copied. Charge any temporary slice
or position storage before constructing the model index; avoid collecting a second batch list.
The flow row inventory and compact model index may coexist because they serve different access
patterns. Both remain admitted until their actual last consumer; do not release the row charge
when an attacher still borrows its backing. The existing model constructor owns its copied paths
and reservations, allowing the row inventory to be released independently when flow lookup ends.
Preserve duplicate/conflict detection; map insertion must not hide a malformed duplicate input.

For native correspondence, prepare charged ordered span buckets once per module during native
node enumeration. Preserve raw-node encounter multiplicity for region uniqueness, then derive
ordered distinct supported `(SyntaxKind, OccurrenceRole)` candidates for attachment at lookup.
Deduplicating the raw bucket would change regions for duplicate statement ranges. At lookup, examine only the span's candidate
bucket, then apply exactly the existing optional-kind and scalar-wrapper predicates. Keep unsupported
native kinds distinguishable from a unique supported shape; never take the first span or convenient
kind. The model attacher still decides exact versus innermost/ambiguous/unmatched/budget-exceeded.
This index is not a semantic equivalence between Ruff families.

One-read preparation is selected over source-at-a-time scalar queries because the current input
contract is a typed relation stream and lexical targets can cross sources. Preparing both views
from one inventory is simpler than a new shared catalog/cache. Native selective or source-partitioned
reading remains TF03's question, after this duplicate read is removed. Native shape buckets replace
the repeated complete-node scan; they add bounded-by-inventory preparation and lifetime obligations
instead of multiplying that scan by flow events. Speed and memory improvements remain unmeasured.

Complete predicate operand correspondence precedes condition emission: retain stable native test/leaf
order and the first full native Atom match, including a first match with no operand. Shared
immutable admitted condition backing removes raw graph clones without changing model algebra or
native cache encounter limits. These are implementation refinements under ADR-0182.

**Migration/removal.** Move flow's occurrence consumers to the shared vector/position access and
its writer to the installed attacher. Remove the flow-specific duplicate `Occurrence` read and
per-attachment `flow.nodes.iter().filter(...)` scan. Keep the model validator and attachment
budgets; do not replace them with test-only matching. Retire raw-node retention when its last
production consumer has moved to the prepared shape view.

**Revealing controls.** A DB-free counted `ProviderSink::read` boundary must observe one occurrence
read for real flow lookup plus attachment. Compare exact attachment outcomes, candidate ordering
and full emitted attachment/boundary facts on equal spans with different kinds/roles, duplicate
native shapes, scalar wrappers, annotations, unsupported shapes and many events. Use an independent
small expected candidate set. Cancel/refuse during preparation and retain/drop the last borrower
to show charges remain until actual backing ends. Source inspection establishes that lookup is
bucket-local; no timing threshold is required.

## 4. TF01 — Cancellation reaches production Ruff and ty work

**Source:** [P:F05](../design_review/reviews/design_review_python-fact-collection-efficiency-target_2026-10-17.md#F05).

Carry a cloneable view of the existing provider cancellation owner into canonical contextual
collection and `cpg-flow` preparation/traversal. The view can check cancellation and register a
native callback without holding a mutable stage borrow. It neither owns a second terminal state
nor joins workers. Registration retains the existing synchronized cancel/register race behavior.

In `ruff_context.rs`, pass this view from the actual `pyrefly_stage.rs::context_rows` call into
production `Collect`. Implement `Sink::checkpoint` using it; preserve the actual operational
error for propagation through `context_rows`, rather than reducing provider cancellation to a
usable partial result. The upstream observer's `StopReason::Cancelled` is the stop signal, while
the provider owner decides attempt failure. Existing observational row/work limits and unsupported
syntax continue to expose their contextual incompleteness. Requested diagnostics retain their
own completeness and two suppression channels; this package does not fuse the Checker runs.

`cpg-flow::visit_modules` receives the same operation check, attempt budget and a native-token
registration closure. It owns `FlowDb`, obtains `salsa::Database::cancellation_token()` for the
actual query handle, and retains the returned provider registration guard through preparation,
every native/lazy query and final traversal cleanup. The callback calls token `cancel()` only;
no native joining or blocking database mutation runs inside provider cancellation.

**Exact library boundary, Interface-checked 2026-10-17:** workspace and lockfile select Salsa
0.28.5; its `database.rs`, `zalsa_local.rs` and `cancelled.rs` were inspected. Token cancellation
sets the local handle's atomic flag without waiting. `Database::trigger_cancellation` instead
can block awaiting other handles and is unsuitable here. The shared Salsa skill is indexed at
0.28.4, so its cancellation leads must not be transferred without this 0.28.5 check. Catch native
query cancellation at the query boundary with the library's `Cancelled::catch`; ordinary panics
continue unwinding. Local cancellation, pending-write cancellation and propagated native panic
do not become module model limitations or empty flow. Preserve their operational cause through
the provider error route. Do not blindly retry a cancelled handle or clear its flag as success.

Check before and after each source rename/installation, before the first callback, around native
queries, at source-order visitor checkpoints, and inside candidate, loop-header, predicate,
snapshot and formula-lowering iterations. Check again before group delivery and provider close.
Retain the full virtual import universe before semantic queries; cancellation interrupts its
preparation rather than permitting a smaller universe to run. Native registrations survive all
work that can still query that handle. If TF03 later uses clones, each actual query handle gets
its own token registration; cancelling one clone does not establish cancellation of its peers.

**Support limit.** Initial parser calls are indivisible upstream work. Salsa cancellation is
cooperative; fixpoint-cycle regions can defer cancellation while completing their native cycle.
Checks around these calls reduce avoidable subsequent work but promise no immediate interruption
or fixed latency. A cancellation request retains the actual worker, source buffers, database and
reservations until native return/unwind and join. Timeout or request acknowledgement is not drain
evidence. These limits must remain explicit in the extraction owner.

**Migration/removal and evidence.** Replace the real collector's default checkpoint, and callback-only
flow checks, with the connected checks above; remove no join/drain protection. DB-free controls
cancel the production collector mid-traversal, virtual preparation before any module callback,
an actual query handle, and a large adapter traversal. Assert no later preparation unit or top-level
module collection starts,
operational failure remains failure, and charges/registration survive to actual return. Exercise
the existing provider owner with an in-memory sink and real worker join; a custom observer alone
does not close P:F05. Retain tests for cancel-before-register and unregister/replacement races.

## 5. TF02 — Admitted raw growth and synchronous complete-group consumption

**Source:** [P:F06](../design_review/reviews/design_review_python-fact-collection-efficiency-target_2026-10-17.md#F06).

### Selected physical route

Replace production's complete expanded `ModuleFlow` handoff with synchronous completed-group
delivery inside the existing one-module-at-a-time visitor. A module receiver in `ty_flow.rs`
lowers groups through the current typed writers, using TF00 preparation. `cpg-flow` owns native
enumeration and raw correspondence; the receiver owns canonical identities, qualification,
support, coverage and model emission. This is a specialized fallible callback contract, not a
general streaming framework or asynchronous queue. Local type/function names remain implementation
choices, but the contract must make operation control, ownership and module terminal state explicit.

Prepare source-view identity and native shape/place correspondence first. Announce definitions
with stable module-local ordinals before groups that reference them; any definition discovered
lazily is announced before its first dependent group. Preserve use ordinals and definition
correspondence in charged compact maps. A completed use group includes its complete native
candidate inventory, original ordinals, expanded reaching relationships and capture snapshot
information. Lower all candidates needed for the existing false-only decision before selecting
that decision: incremental delivery must not choose a candidate while a competitor is unseen.
Keep raw-to-typed reaching correspondence until every referring candidate is consumed, then
release it. Global raw indices may become compact counters plus local group offsets internally;
their association and model ordinals must remain identical.

Value-source/call-path, region, predicate/test-leaf and attribute-load groups follow their native
dependencies. Preserve source-order call steps, match-arm/evaluation identity, current stable
sorting where required, and per-family meaningful encounter order. Necessary sorting scratch is
charged; physical grouping must not silently redefine an observable ordering contract. Emit final
skip counts, syntax recovery and module coverage only at the module terminal event after all
required groups have succeeded. Consumer failure stops traversal and later module queries
immediately. Required lazy native dependency queries inside the current module remain available.
Failed module-model preparation retains its original `input_index`, contributes the
existing failed-module boundary and can continue its peers, as today.

Completed groups are consumed before the producer advances. This releases large candidate/formula
payloads as uses are completed while retaining only genuinely necessary module correspondence,
native cached premises and current group scratch. A module with many independent uses therefore
has a route whose raw expanded live set does not include every already consumed use. A single
large candidate group is admitted as a complete group; it cannot be truncated to fit. Module-local
definition/use correspondence and any indispensable diagram cache remain charged until last use.
This is a credible growing-module route alongside admission, not a claim that every input fits
every budget or that the opaque native database is constant-size.

### Admission before allocation

Use the existing attempt `ResourceBudget` and `StateCharge` for **all adapter-owned** source
copies/renamed buffers, prepared metadata, nodes/shapes/places, raw groups, use/definition maps,
candidate and snapshot lists, loop pending/seen/output buffers, predicate/region/diagram caches,
visitor stacks/sets, formula graphs and atom/string clones. Audit `predicate.rs` and `narrowing.rs`
as well as the top-level vectors. Temporary `bindings`/`lowered`/`live` collections are owned
growth too. Borrow native iterators where possible; where a borrow forces collection, reserve its
capacity and lowered payload before collecting. Avoid retaining both complete temporary lanes.

Compute checked growth allowances from borrowed native data and already admitted inputs before
`reserve`, `collect`, formatting, cloning or mutation. `StateCharge::admit` on an already allocated
value, or charging a `ChargedVec::push` after constructing its heap-owning argument, does not by
itself cover that construction. Admit the argument's strings/formula backing first. Use existing
charged mutation/transfer helpers where suitable, and small specialized fallible builders for the
remaining adapter allocations. Reconcile excess reservations after construction without releasing
capacity that remains allocated. Charge reservation metadata and preserve same-pool transfer;
do not release a producer's charge before a receiver owns the moved backing.

`Condition` is the model's `CondGraph<native::Atom>`. Its current clone, `not`, `and`, `or` and
`map` allocate. The adapter must pre-admit complete output/scratch bounds from node counts and
leaf heap sizes, or construct their admitted equivalent through the same model operations.
Retain `MAX_GRAPH_NODES`, simplification, refusal nodes, approximation flags and encounter behavior;
resource admission is not permission to simplify a formula differently. Source text used in
opaque atoms and repeated condition clones is included. Shared immutable backing may remove a
copy only when complete graph/approximation identity and lifetimes are preserved; no new global
formula interner is needed. Check operational cancellation inside controllable lowering loops.

The attempt pool governs these owned representations, not every opaque Ruff/ty/Salsa allocation
or process RSS. Native source/AST/index/query retention is disclosed separately. Do not infer an
RSS guarantee from admitted-byte peaks, a source-size limit, or per-condition node limits.
No thread, worker, source or candidate cap is introduced to make the route pass.

### Failures and completion

Give the flow runner an explicit operational result distinct from its module-model outcome and
consumer result. For example, the outer result can distinguish native cancellation, attempt
resource/model-operation error and the callback's error; the module event separately carries a
typed model limitation with explanatory detail. Preserve `ModelError::Resource` and the provider's
actual cancellation/error cause across the boundary. Do not stringify them into `ModuleFlow::error`
and subsequently report `OutsideProviderModel`. Do not catch an arbitrary panic as a model limit. ADR-0182 selects private typed operational
unwinding across infallible native visitor helpers, caught only at the public runner boundary.
A separate private consumer-stop marker immediately leaves those helpers; the original callback
error remains outside its payload, preserving borrowed/non-Send errors and preventing continued
sibling preparation. Unrelated panics are rethrown.

Existing source admission/undecodable-source and genuine supported-model limitations keep their
current explicit coverage rules. New attempt-budget refusal in raw construction is an attempt
failure. The receiver cannot emit a successful terminal coverage event after an operational
failure. Earlier rows remain private pending output and may drain, but no partial contribution
earns completed visibility; existing provider/sink completion and join/ingress owners enforce this.

### Alternatives, migration and revealing controls

Charging the existing full `ModuleFlow` is useful transitional wiring but cannot complete TF02:
it leaves whole-module expansion ahead of consumption. Smaller downstream batches have the same
gap. Asynchronous queues add overlapping state and another failure lifetime without a need here.
Disk spooling is a possible later alternative for indispensable large correspondence, but requires
an actual existing workspace/run/storage owner, named consumer and drainage-protected admission;
this plan selects synchronous groups and charged compact retention first. No new spill framework
or unowned temporary files are authorized. If exact grouping proves impossible for a required
dependency, establish a concrete charged/spillable replacement route before claiming closure.

Migrate `TyFlow::run` and `Writer::write` together with the native visitor. Remove the old production
whole-module raw-vector path, duplicate binding temporaries and unconditional formula copies where
the admitted group route replaces them. Standalone raw characterization tests may collect the same
groups through a deliberately charged test receiver; that receiver is not a second production
implementation or compatibility reader. Preserve every existing raw family and support relation.

DB-free controls must compare complete facts and required ordering for many uses/candidates,
false-only/unbound/deleted competitors, nested/loop headers, unattached Bound candidates, eager/lazy
captures, compound formulas and recovered source. Explicit expected inventories challenge lost
candidates or ordinals; comparing two receivers that share a faulty grouping decision is insufficient.
Include a growing module under a budget that admits necessary correspondence and one group while
not admitting all expanded groups together. Show successful complete consumption and release of
earlier payloads. Add resource refusal before an owned allocation, cancellation during expansion,
consumer failure, and last-owner release controls using the actual attempt pool. These establish
behavior/resource lifetime, not measured speed or general capacity.

## 6. TF03 — Qualify source partitioning, retention and module-query placement

**Source:** [P:I04](../design_review/reviews/design_review_python-fact-collection-efficiency-target_2026-10-17.md#I04).
This package consumes working TF00/TF01/TF02. It must investigate and then either implement the
supported selected route or record a justified retained design and revisit trigger at coordinator §6;
merely leaving an unexamined optimization list is not its completion outcome.

Compare native selective source reads with one streaming partition pass over the exact declared
canonical input. Include lexical target/reference closure, source/context distinctions, attachment
candidate domains and genuinely cross-source correspondence. Reject scalar reads per occurrence.
Select source-scoped preparation only when its contract expresses the complete sufficient universe;
otherwise retain the one-read shared preparation with the reason. Selection does not earn another
semantic admission or permit missing referenced targets.

Inspect original captured text, rename token parse, transformed buffers, virtual filesystem and
`parsed_module`/semantic-index retention through their last consumers. Original text still supplies
opaque atoms, unsentinelling, runtime-view identity and source checks; it cannot be dropped merely
because ty has transformed text. Parser-mediated recovery is required for malformed interpolations;
bare lexing is not equivalent. Reusing canonical tokens requires exact source kind, target version,
recovery and source identity correspondence. Renamed bytes still need their own semantic parse.
Qualify ty's existing AST-clearing capability against all remaining borrowers and reparsing; clearing
state that the next lazy query reconstructs may increase total work. Implement only releases that
remove a real overlapping lifetime without losing those consumers.

Assess same-revision parallel native module queries last. `FlowDb` is cloneable, but that fact alone
does not establish equivalent query/cancellation behavior. Every clone must see the full prepared
virtual universe and explicit Program settings, retain actual query ownership, and register its
own cancellation token. Exported-name/wildcard queries and cycles still need cross-file context.
Current callbacks execute in input order, finish before the next top-level module collection query,
and a callback error prevents later top-level collection. This preserves required lazy dependency
queries within the current module. Eager speculative querying with an ordered result buffer preserves
output order but violates the latter guarantee and overlaps raw state; it is not an equivalent
replacement. Retain serial query placement unless a qualified capability preserves both ordering
and stopping. A proposal to relax that guarantee goes to the coordinator for rule-impact/decision
handling before dependent implementation; this plan does not select such relaxation or add caps.

Settling evidence includes package/submodule and wildcard cycles, exported-name lazy queries,
malformed f-strings, annotations, source-view hashes/byte geometry, failure at an early input,
query traces showing no later top-level collection after refusal, and token cancellation of each actual handle.
Use source inspection plus the smallest meaningful DB-free native controls. Record the selected
route, concrete benefit/tradeoff and any unsupported choice at the coordinator, then migrate its
consumers and remove redundant copies/preparation. No benchmark campaign is a completion gate.

## 7. Verification, rule changes and handoff

All implementation controls below are **planned / not_run** by this plan author. During TF00–TF02,
compile touched crates and run narrow revealing controls, using existing release defaults until
the current profile owner changes them. Suitable command shapes are `cargo check -p cpg-flow
-p cpg-extract`, `cargo nextest run --release -p cpg-flow --lib settings_tests`, and selected
`-p cpg-flow --test flow_shapes`, `--test capture_timing` or `--test call_paths` filters. Ruff's
production checkpoint controls belong in `cpg-extract --test ruff_context` and provider-owner
cancellation controls; add/select the precise new cases that exercise the changed seam. These
local native-analyzer tests need no database. Do not launch the entire extraction family merely
because a package has ended.

The typed flow cases in `crates/cpg-extract/tests/cases/typed_flow.rs` and their
`extraction_types` target are later affected controls, not proof of being DB-free. Any actual
compiler/provider/native-ingress completion control runs only after the coordinator's safety
prerequisites through existing `just verify`/`just fixture` owners. No shared/operator store inspection,
scratch service, recovery, installer mutation or native campaign is implicit in this companion.
Protected Unknowns, outstanding attachment ownership and preserved captures stay with their
current owner. Full Catalog and assembled validation await separate planning after this phase's
implementation and focused controls; Catalog alone cannot establish Behavioral flow equivalence. Real-library
qualification, publication/selection/adoption and measured performance require their separate scope.

**Rule changes, 2026-10-17:** the source review lists none for F02/F03/F05/F06, and this design
retains those contracts. No operator-confirmed rule change is needed for the proposed corrections.
TF03 cannot silently change callback stop behavior, parser recovery, source universe, model limits
or exact-fact requirements. Any newly exposed change is presented through the coordinator before
fixing the dependent choice. An implementation selecting a meaningful architectural alternative
updates its architectural owner and uses the existing ADR route; accepting an ADR does not close
its finding.

At completion, update the extraction/model operating owners for enduring preparation, cancellation,
failure and raw ownership contracts, and return source locations, removed paths and exact check
outcomes to the coordinator. The root owns integration, applicable non-functional leaves, document
publication checks and final `just turn-end`. This companion introduces no generated output or new
storage lifetime. It remains a current consumer of P and its evidence while these obligations are
open; once enduring contracts and any retained investigation trigger live at their normal owners,
the coordinator retires obsolete planning material under the current-working-set policy.
