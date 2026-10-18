# Catalog publication performance: compiler execution

**Approved implementation plan, 2026-10-17.** Companion to the [coordinator](catalog-publication-performance-plan_2026-10-17.md).
The coordinator owns shared contracts, dependencies, approved rule directions and all current
dispositions. This document develops F01/F04/F08, I01 and the compiler integration of F03/F05/F06.
It is a standalone new track; existing plans are unchanged. Authoring does not implement these changes.

## 1. Baseline, responsibilities and target

At baseline `789223017833c364085ef62f9333df082d88444a`, the review's source mechanisms remain
unchanged. `cpg-core` binds immutable completed inputs and performs selected computation;
`lctx-model` owns semantic requirements; compiler-store/native own actual persisted content,
admission, accepted work and finality. Keep those responsibilities. Pure domain work must not
acquire a second persistence authority or invent its own recovery protocol.

The [source inquiry](../design_review/evidence/2026-10-17_catalog-publication-performance/analysis-and-emission.md)
establishes four immediate opportunities: rich input scans whose visitors discard payload,
repeated capture preparation across overlapping roots, clone-and-buffer output publication,
and waiting for complete windows before integrating an available prefix. The
[publication/admission inquiry](../design_review/evidence/2026-10-17_catalog-publication-performance/publication-and-admission.md)
adds optional retention before readiness and the unresolved distinction between computation
reuse and authentically executed admission evidence.

The target carries three distinct things: exact semantic dependency identity, sufficient
admission properties, and the rich selected rows the operation actually consumes. Declaring
a dependency does not automatically request full decoding. Rich input needed by a global
analysis or negative premise remains complete; an unused payload prepass need not run merely
to prove that an adapter was dispatched.

Protected preparation is invocation-owned. Reuse follows exact source views, selected roles,
query shape, context, pin and resource ownership. No global rich cache, new task pool or
alternate canonical store is introduced. Improvements are expected from removing work and
moving ownership; no numerical benefit is established.

Support ownership remains model-owned. Compiler physical lowering uses typed DataFusion
frames for selected/full inputs, null-rejecting left semi membership, non-null projections and
distinct unions. It does not recursively interpolate ownership programs into SQL text. This
preserves each grain's complete universe, duplicate/null semantics, literal aliases and declared
ordering without introducing another materialization or assurance owner. Focused controls
exercise the actual distribution/corpus ownership program and independently expected scope,
NULL and duplicate results; the coordinator owns their outcomes and Catalog integration.

## 2. PC00 — Separate dependency admission from rich read demand

**Consumes:** PP00's contract/decision slice for RC01. **Delivers:** actual Base, Completion,
SourceCalls and Enriched adapters with explicit demand and no discarded full-payload prepass.
Primary sources are `crates/cpg-core/src/semantic_execution.rs:30–100` and
`crates/cpg-core/src/consumed_rows.rs:37–126`; model expected-domain admission is in
`crates/lctx-model/src/domain/analysis/expected.rs`.

For each currently declared input, identify whether it supplies dependency identity, expected
coverage metadata, selected rich computation, or a genuine complete universe. Express that
distinction through existing model declarations and completed-input owners. `ValidationInput`
ordering fields are not a projected-column contract. Keep exact relation/epoch/role identity
even when no payload is read.

Dispatch records fulfillment of the declared demand. Metadata paths acquire only model-owned
sufficient properties, following the existing SourceArtifact property path and Local's narrow
metadata inputs where suitable. Selected kernels use the existing protected rich acquisition.
Remove the `load_ignored` full stream for relations with no unique coverage or validity premise;
do not just delete model dependencies or suppress errors after beginning a required stream.

Migrate all four adapters and their typed dispatch inventories in this package. Preserve
NotRequested versus complete-empty, unsupported domains, genuine lower-owner tokens and
complete-input semantic admission. A later selective computation cannot manufacture absence
over an incomplete universe.

**Focused acceptance:** a declared but unused rich relation is not scanned; required coverage
still refuses missing ownership/metadata; selected rich reads return the same answers; a late
failure in a genuinely consumed stream fails the enclosing attempt. Include empty and
unsupported inputs. Inspect all four migrated callers, rather than demonstrating one helper.

## 3. PC01 — Prepare root membership once and share exact capture backing

**Consumes:** existing admitted root/capture interfaces and PP00's SC01; PC00 is not a blanket
prerequisite. **Delivers:** first-use preparation and invocation-scoped reuse in the real Base
capture path. Source: `crates/cpg-core/src/base_scope/capture.rs` and compiler-store's actual
selected frontier/adjacency owners.

Compute first consuming root once as `(kind,id) -> minimum canonical root ordinal`. Retain
all exact root memberships separately, then generate acquisition and local assembly from the
same prepared correspondence. Replace the per-active-root distinct-ID/earlier-root anti-query
pattern. This preparation is local relational work; use existing Arrow/DataFusion or typed
indexes according to the backing already present, without converting merely to use a join.

Extend successful supporting-row acquisition across successive outer admitted root windows
through one charged, spillable invocation backing. Keys include exact source view/role,
selection/query shape, context and live ownership. Pin the acquired scope while borrowed;
retain EOF/drainage and cancellation outcomes. Never publish an incomplete acquisition as a
successful reusable entry. Drop/spill backing when its last actual consumer ends.

Native capture already occurs once per outer window; local resource-driven shrinking already
reuses it. Preserve that behavior. Computed adjacency already has shared OnceCells; extend
the missing lifetime instead of layering an overlapping cache. Preserve logical root isolation.
A union native point acquisition is eligible only if exact local partitioning, independent
admission and error ownership establish the same supported behavior; it is not needed to
complete the first-use reduction.

**Focused acceptance:** overlapping/disjoint roots, common native forward dependencies,
repeated supporting rows in a later outer window, complete-empty acquisition, resource refusal,
failed EOF and cancellation. Verify local retry does not replay native acquisition, and that
cross-window borrowers hold the actual source and charges until released.

## 4. PC02 — Transfer canonical output ownership into existing batching

**Consumes:** RC03's governed decision slice and existing canonical row/batch contracts.
**Delivers:** a consuming internal output path, migrated producer callers and no duplicate
long-lived ownership of already-consumed rows. It can initially use the current native sink;
it does not wait for the new physical representation or journal to be implemented.

Current `producer_operations::emit_results` clones borrowed rows; `ProducerOutput::push`
validates, locks and buffers them before batch construction validates/orders them again.
Introduce a consuming canonical batch handoff into the existing writer, transferring the
real backing/reservations. Preserve cross-flush duplicate-ID equality and changed-payload
refusal. Keep independently supplied unchecked rows on their required validating boundary.
Do not add a second deduplication policy or an uncharged conversion buffer.

Retain each result family until its actual final consequence/discharge consumer, then drain
and release it. Summary's 44 record and 14 vocabulary families remain accounted for; the
fixed-width PairOutcome clone is not a rich-heap explanation of its physical write cost.
Rich clone-before-reservation is eliminated by ownership transfer rather than covered by
an after-the-fact reservation.

Migration includes Summary, normalization, Local, execution, Models, Structural, Analytic,
catalog core/evidence, synthesis preparation, retrieval preparation and final coverage wherever
the surveyed borrowed emitter owns canonical results. Preserve already-batched provider output
as a useful existing path. Each producer migration moves its necessary consumers and removes
its obsolete emitter path; a representative migration alone does not close F04.

**Focused acceptance:** a rich heap-owning record; empty declared families; identical IDs across
flushes; conflicting same-ID rows; failure/cancellation during transfer; reservation return
after success and refusal. Validate actual migrated callers, ordering and last-use release.

## 5. PC03 — Integrate ready canonical prefixes while independent work continues

**Consumes:** PC02's working transfer for the migrated scope and an owned-work lifecycle slice
from PP00. **Delivers:** next-ordinal private integration without waiting for every unrelated
job in the current window. Source: `crates/cpg-core/src/owned_work.rs` and its actual callers.

Retain existing task registration and accepted-work joins. Make the next canonical result
available while later independent jobs compute; keep out-of-order results charged and bounded
by actual resource admission. Transfer the result into the existing canonical writer, overlap
its native persistence with useful independent preparation where ownership permits, and free
consumed state. Preserve the distinction between private persisted prefixes and completed
producer visibility.

A failed later job poisons completion and drains accepted peers. Resource refusal, cancellation,
worker panic and native Unknown preserve their actual owners; they cannot be converted into
successful EOF. Avoid locks across unrelated compute/persistence. Do not add worker/thread caps
or change arrival-order semantics to make the path pass.

Migrate the applicable existing whole-window callers in Summary, C0/C1, Completion, Models,
Structural, Analytic and retrieval preparation. Classify any remaining whole-window barrier
by its concrete semantic dependency. Ordered bodies and premise-growing rounds remain ordered.
One-frame Summary gets no kernel parallelism from this change; PA04 owns that separate question.

**Focused acceptance:** deliberately reversed completion order, a slow later job, failure after
an earlier persisted prefix, cancelled acknowledgement, queued-result resource refusal and
zero remaining reservations after actual joins. Completed visibility must remain unavailable
until all required work and native terminality are established.

## 6. PC04 — Remove optional retention from required producer readiness

**Consumes:** RC07's decision slice and actual completed-input/retention ownership.
**Delivers:** downstream readiness after required completion, with optional retained reuse
continuing under its own bounded child owner and joined enclosing lifetime.

`workspace.rs:3470–3474` awaits retention before handoff. Separate the completed artifact/view
capability needed by current consumers from creating an independent retained product for later
reuse. Prepare effect-free decline before submission. For accepted retention, retain the exact
completed source, operation identity, budget and native recovery owner until its real terminal
join. The enclosing operation reports accepted retention failure or Unknown according to that
contract; downstream readiness is not a claim that the whole operation already succeeded.

Keep retention where it purchases useful avoided computation. Reuse exact selection/inventory
preparation from PN03/PC05 when available; those enhancements do not block the lifecycle split
if existing acquisition is sufficient. Never swallow an accepted ambiguous effect as a cache
miss, detach work when its source drops, or preserve a second global cache without a consumer.

**Focused acceptance:** a consumer progresses while retention is delayed; independent retention
survives source handoff; effect-free decline creates no native obligation; accepted failure,
cancellation and source-drop paths join and retain unresolved state correctly. Historical
118.866s overlapping retention phases are not a promised amount of saved wall time.

## 7. PC05 — Resolve I01 and implement sufficient admission preparation

**Consumes:** current model invariants, executed scoped evidence and exact installed-view
capabilities. **Delivers:** operation-owned selected-capability preparation, plus a reasoned
decision and implementation route for whole-input executed-property reuse.

The inspected `admission/scoped.rs:309–321` acquires capabilities before each reuse lookup,
then separately revalidates original retained pins. Prepare distinct compatible acquisitions
once per actual admission operation. Keep each invariant's ordered requested/resolved roles,
model/profile/factory/revision, complete dependency mappings, physical validity context and
uniform-owner premises separate. Share native acquisition where these premises allow it;
do not equate roles merely because they point to the same physical view. Original pins still
need current validity, even when a new final capability has an equal selector.

**I01 decision:** first identify the independent failure class of acquisition, revalidation
and execution. The ordinary `!earn` memo and `Execution::WholeInput` cannot simply be admitted
as scoped evidence. If a private executed whole-input property can carry all required actual
execution, ledger/frontier, role, owner/pin and cancellation premises, specify that authentic
carrier and implement reuse through the model/admission owners. Publish reusable successes
only after the complete validation attempt succeeds; failure invalidates them atomically.
If the required whole-input pass detects a distinct actual-state failure not discharged by
such a carrier, retain that necessary pass and share only its stable preparation. Record the
counterexample and decision; do not leave I01 as an indefinite future optimization.

Changing what qualifies as executed admission evidence is an additional rule impact, beyond
blanket approval of existing memo use. Expose its exact proposed meaning for operator decision
before dependent implementation. Operation-local acquisition sharing that preserves the current
evidence contract may proceed independently. No digest, nominal descriptor or prior success
grants receiving authority. Cold/incoming/installed receiving remains independent.

**Focused acceptance:** multiple invariants sharing a view with distinct roles; requested and
resolved epoch differences; changed model/factory/ledger/frontier; original pin invalidation;
complete-empty applicability; cancelled/failed validation prefixes publishing no reusable
success. Assert reuse with an independently specified invariant failure, not only a cache counter.

## 8. Integration boundary

PC00/PC01 and model preparation work are independent of native representation replacement.
PC02/PC03 must continue using the actual store executor, including when PN02/PN04 replace
its installation/recovery internals. PC04 and PC05 share ownership surfaces in workspace and
admission; coordinate edits, not artificial whole-document barriers. Model-side F03 corrections
are designed in the [analysis companion](catalog-publication-performance-analysis-plan_2026-10-17.md),
including the real Completion caller migration.

At focused completion, inspect every migrated production route and run minimal revealing
controls through existing verification/fixture owners. Compilation and applicable hygiene
belong to PP90. Full recursive/real-library/assembled acceptance and benefit measurement remain
the coordinator's later-validation handoff; no compiler package may claim those from a focused pass.
