# Catalog execution speed: private compiler execution

**Implemented source / integrated functional acceptance pending,2026-10-18.** Companion to the
[coordinator](catalog-execution-speed-plan_2026-10-18.md). Its
[§6](catalog-execution-speed-plan_2026-10-18.md#6-sole-disposition-and-current-checkpoint)
alone owns current execution disposition. This document owns SC00–SC04 and develops F01/F03/F04/F05/F11,
with F06/F08/F09 consumers. RC01/02/04/05 directions were explicitly approved2026-10-18;
actual ADR/design-owner changes precede the dependent implementation. Existing plans are unchanged.

ADR-0191 now governs the selected cutover. Private immutable backing and semantic validation live
in`lctx-compiler-store::private_source`/`private_validation`; Workspace owns producer completion,
exact input sets, operation cancellation and the requested native boundary. Native construction is
fallible/absent for a private operation, rather than represented by a disconnected native facade.
The existing64GiB shared pool supplies actual grant identity and pressure relief; sealed IPC is a
contingency. The coordinator records the sole current checkpoint, review and grouped checks.
The selected SC02 correction additionally retains actual successful per-predicate properties across
private scope release. Native admission reconciles full stored typed values and EOF under held
current capabilities before adopting an exact context; whole-input, scoped and empty applicability
remain distinct. This source implementation is independently accepted; its new native transfer
control and the failed upper-frontier controls remain acceptance obligations in coordinator §6.

## 1. Baseline, foundations and target

Baseline`9c457a0c`: product source is the review's`b0dd321e`. The
[review](../design_review/reviews/design_review_catalog-execution-speed-comparative-target_2026-10-18.md)
and its crosswalk supply the diagnoses, preserved behavior and rejected forecasts. Source inspection
and focused independent contract advice add these implementation constraints:

- `CompletedRelation` (`cpg-core/src/workspace.rs:1881–1903`) owns native store/view/bridge and
  execution lifetime. `CompletedInputs` embeds native producing scope. These are migration
  surfaces, not neutral containers whose meaning remains correct with a new byte buffer.
- `workspace/input_catalog.rs:8–99` resolves ports through native table providers, which earn
  installed-view protection. Private selection must instead use the actual private source owner.
- `lctx-model::domain::checked_admission::CheckedValidationPremise` already represents actual
  pure semantic execution without persistence or selected-view authority. The surrounding native
  scoped memo owns additional installed views, allocation/currentness and pins. Separate those
  layers without weakening either eligibility contract.
- `SourceSnapshot` is semantic metadata; it need not imply a native CompletedView. Preserve
  result-affecting identity, ordered roles and context; do not equate physical identity with meaning.
- CLI setup creates a native allocation before acquisition (`lctx/src/compile.rs:48–93`,
  compiler-store`compiler.rs:4310–4352`). Deferring row writes alone does not remove that lifecycle.

**Target:** a compiler operation owns capture, CPU/provider tasks, private completed sources,
selected typed inputs, working state/spill and cleanup. Native sessions appear only for real
native input/cache/effect or requested artifact materialization. The model still owns semantics.
No universal backend adapter, second editable canonical store or indefinite dual runtime is needed.

Apply FP-01/03/04/05/07, DP-03/09/10/19/20/23 and H2/5/12/16–24. Remove unnecessary work before
adding concurrency. Many semantic kinds need not create separate remote objects or durable roots.
A source can be immutable while its physical representation changes under one owned contract.

## 2. SC00 — Private completed sources and bounded backing

**Consumes:** SP00's private/native authority, durability and exact-source decision slices.
**Delivers:** working private output/input ownership and lifetime primitives with a minimal actual
producer→completed source→consumer path, including spill. It does not yet establish whole-CLI migration.

Replace mandatory native store/view/bridge dependencies at the compiler-facing boundary with an
owned source of immutable typed rows/batches or indexed spill. Its authoritative construction binds
validated model/schema, producer/profile/configuration, exact ordered consumed roles/epochs,
complete contributor inventory, declared output domains and producer outcome. Content semantic
identity, operation identity, physical backing and full executable provenance remain distinct.
Pending output becomes usable only through successful complete-set handoff; absent declared output
refuses, including its empty case. A completed empty source is not missing or NotRequested.

The private operation owns one budget/cancellation/task registry and spill lifetime independent
of the mutable workspace registry. Sources, providers, plans, streams and async borrowers retain
that owner and actual reservations. RecordBatch slices can retain whole buffers; charge backing,
not merely selected lengths, and release only after the last borrower. Use existing Arrow/DataFusion
memory/disk integration and typed Rows/RowsView rather than another general memory manager.
IPC is an internal immutable backing format, not an admission certificate or required export step.
Select representation/indexing by real consumer access; stream/partition/spill instead of assuming
whole-compile RAM, and avoid retaining encoded plus hydrated graphs without a current borrower.

Private cancellation stops further ordinary work and joins accepted CPU/provider/stream operations
before releasing backing. It must not invent native recovery requests for pure abandoned computation.
Conversely real optional cache/native operations keep their separate authenticated owners and actual
uncertainty. Readiness may observe required final-target prerequisites early; it must not allocate
native compiler state or confer selection/publication authority on private output.

Preserve actual canonicalization, duplicate/content collision and ordering rules. If a logical or
wire identity must change, declare its revision and semantic mapping through the owner; do not
promise old native physical IDs survive in private memory or reinterpret an old receipt.

**Focused closure:** successful and empty completion; missing output; same-key unequal content;
wrong model/profile/role; pending/cancelled producer; shared-buffer slices; spill pressure and
last-borrower release; input provider outliving workspace registry; no native attempt created by a
pure private producer/consumer control. Existing native readers retain real protection separately.

## 3. SC01 — Migrate the real compilation chain and entry points

**Consumes:** implemented SC00 source/read/write portions, SN00 boundary contract, model dependency
semantics. **Delivers:** actual provider/normalization/upper compilation over private sources,
connected to working SN02/SN03 before the ordinary command switches. A memory-only test adapter
or one illustrative stage does not close this package.

Migrate provider output sinks, declaration/writer lookup, accepted-batch ownership, producer
completion/composition, exact input ports, lazy DataFusion registration/scans, consumed rows,
normalization/graph adapters, analysis/catalog owners and final frontier acquisition. Inventory
ordinary CLI, native bindings and supported library callers instead of treating one FastMCP command
as the full API. Preserve the signatures/observable behavior of requested facts/normalized/analysis/
catalog outputs and Catalog/Behavioral profile routing unless an explicit owned revision is selected.

Private DataFusion providers may project/filter exact private typed data but cannot call installed-view
selection, manufacture native ProducingScope, or claim native currentness. Existing native providers
remain for actual incoming/persisted products and independently admitted content. A genuine retained
native hit is either a pinned read-backed input or a fully checked private copy with owned charge;
a deserialized receipt never becomes a live private property. On cache miss, compute normally.
New attribution/invocation receipts remain honest even when value computation is reused.

Remove remote contribution/membership/view installation from ordinary hot producer handoff.
Preserve exact semantic snapshots across later contributions: a consumer's declared earlier role
cannot widen to latest shared vocabulary. Full union/coalescing checks still occur where independent
contributors can conflict. Complete-set output visibility, original evidence, unknown domains,
SCC/body/premise-growth ordering and current resource/refusal semantics remain.

Split the compiler-private operation from actual native target allocation. Allocate the target at
materialization (SN02), or acquire an independently owned native read/cache session when that real
consumer starts. Do not silently turn configured native failures into cache misses after effects
were accepted. Pure optional preparation can decline before its first native request under AA04.

The existing capture/full-compilation attestation binds actual captures/frontier/profile/settings
and private completed content. Final freeze joins required children before handing to SN00/SN02.
Native publication returns an unselected immutable handle; export remains explicitly requested
product transport, with independent receiving. There is no mandatory raw-export intermediary.
Finalization composes private drainage, accepted native effects, optional retention and committed
publication identity without replacing one with another's outcome.

**Migration/deletion:** private producers and their actual readers move together. Remove old private
native-selection/bridge adapters once their consumers migrate; keep genuine persisted-input and
publisher/receiving operations only for their named purposes. Do not install a permanent feature
flag or old-format runtime as a rollback mechanism. Switch each supported ordinary route only when
its private chain and required native boundary work end to end.

**Focused closure:** same facts/provenance/coverage across frontiers/profiles; isolated private
normalization and upper controls; actual private→native requested artifact; retained-product hit/miss,
foreign provider and closed source; caller/CLI cancellation; full dependent cleanup. Verify source
readiness, semantic admission, native commit and command end as distinct milestones.

## 4. SC02 — Demand before preparation and carry exact semantic validity

**Consumes:** SP00 RC04/05, SC00 exact-source selection and existing model validation owner; SN00
qualifies the actual native mapping. **Delivers:** smallest valid demanded declarations/preparation,
shared immutable semantic properties and authentic availability without repeated rich acquisition.

Derive owner/input/representation demand from profile, techniques and explicit configuration before
binding and checked-input setup. Compact disabled owners still establish exact domain, authored
configuration, NotRequested and required evidence; do not merely return early after rich setup.
Preserve currently supported captured fact families and every configured consumer. Any deliberate
fact-family or assurance reduction is a separate semantic/product decision, not an optimization.

Extend existing opaque fixed-driver semantic premises as appropriate. Actual visits, order,
completeness and successful EOF under exact source/model/definition/context earn a property.
Serializable eligibility describes an executable definition, not proof it ran. Separate validation
preparation from results; share indexed/compact immutable properties where the consumer's contract
permits. Group checks over one protected stream only when every distinct obligation/outcome remains.
Never pad missing roles, reconstruct producer evidence from raw observations, treat digest equality
as validity or borrow full-factory evidence from a checkpoint-specific definition.

Keep all original currentness checks for real native inputs. Where private predicate validity
maps into native materialization, SN00 must establish actual stored equality, definitions and exact
selected ownership before reusing it. Incoming standalone content always earns fresh admission.
Failed/cancelled/Unknown acquisition or incomplete validation publishes no reusable prefix.

Use authentic completed provider contracts for availability and compact outcome properties;
no name-only supplier discovery, first-role-only binding or raw count grants authority. Domain,
negative/unknown completeness and exact source role remain model-owned even when no payload scan
is necessary. Preserve current private memo failures and resource pool distinctions.

**Focused closure:** optional on/off and alternate provider routes; explicit empty/missing/unrequested;
reused versus independently executed required predicates; changed definition, order, role, context,
source or profile; equal-view independent/failed owners; late invalid row; no memo publication on
failed EOF/cleanup. Physical target/incoming controls belong to SN00/SN02, not mocked private grants.

## 5. SC03 — Coherent set visibility and dependency-ready preparation

**Consumes:** SC00 exact completion/predecessor contract; SN01 only for genuine native binding sets.
**Delivers:** complete producer visibility with necessary short exclusion and independently progressing
ready work. Existing dependency-ready scheduler and canonical ready-prefix behavior are retained.

Current `bind_scheduled_inputs`/prefix freezing holds completion_gate across per-selection native
I/O; final producer publication holds it across per-relation effects even though private composition
preparation already moved out. Remove the remote per-kind work from private handoff. Prepare and
validate a complete output set outside exclusion, check exact predecessors, then atomically publish
host metadata. A conflicting predecessor rebases the affected set or refuses according to the
existing actual failure contract; never replay uncertain effects as pure preparation.

For real native output sets, use SN01's bounded set-shaped operation and coherent host visibility.
Do not replace the gate with per-relation locks that expose half a producer. Keep required declared
outputs and semantic boundary snapshots atomic. Native durable page prefix and host visible producer
completion are different scopes, each with explicit failure/Unknown handling.

Prepare independent ready stages/provider completions through existing owned-work orchestration
instead of awaiting unrelated launch setup serially. Prerequisites C0→C1→C2→S0→Retrieval, body order,
premise-growing rounds and callee/SCC semantics remain genuine. More asynchronous jobs cannot replace
removing contention or unbounded memory. Preserve charged queues and useful downstream backpressure;
no worker/thread cap or larger timeout is an acceptance strategy.

**Focused closure:** disjoint completion; overlapping unions and unequal duplicates; stale predecessor;
missing output; cancellation before/after coherent handoff; later failed canonical prefix; no pending
input observed; required ordering/determinism and authentic task drainage. Compare actual access and
critical-section work structurally, not subtraction of historical gate wait sums.

## 6. SC04 — Exact-key owned preparation without global I/O exclusion

**Consumes:** current protected-source preparation contract; adapt to SC00 as consumers migrate.
**Delivers:** single-flight preparation for equal exact keys while distinct source keys load independently.

`compilation/uniform.rs:49–85` holds one registry mutex across session/stream acquisition. Register
an owned pending initializer under a short registry lock, then load outside it. A key must establish
full source/capability/definition/policy equality, not just hash/name equality. The initializer owns
actual protected inputs, reservations, cancellation and complete EOF. Same-key waiters borrow that
exact pending/result owner; distinct keys share no loading exclusion.

A terminal failed/Unknown initializer retains its original error and settled/uncertain work.
Do not use retryable OnceCell semantics to reissue accepted effects after cancellation/failure.
If purely effect-free construction may retry, distinguish it before native acquisition. Publish no
partial fragment. Drop registry references at last useful demand while actual borrowers retain backing.
Use an existing library initialization primitive only where its failure/ownership semantics fit.

**Focused closure:** equal/distinct source keys; initializer cancellation before/after acceptance;
failed source after apparent memo success; missing terminal EOF; reservation refusal and last borrower.
No new native resource is silently acquired during a private cache-hit interpretation.

## 7. Integration and completion boundaries

SC00 foundations and SN00 early contracts can develop concurrently. SC01 public switching requires
actual SN02/SN03 requested-frontier consumers, not their interface agreement alone. SC02/SC04 and
analysis/extraction local improvements may proceed on current exact sources while the private migration
is implemented; their old native assumptions must not survive unnoticed afterward.

The compiler boundary changes are exercised with focused functionality through SP90. Full compile,
strict keep-going Clippy and applicable leaves run at functional scope end; full-library/assembled
benefit evidence remains SP95's separate scope. SP99 must verify all affected constructors/providers,
source methods and caller paths migrated; preserve genuine native APIs for incoming/published uses,
not compatibility-only hot execution. Source-accepted design and implemented primitives alone do not
close F01/F03/F04/F05/F11. All live dispositions remain in the coordinator.
