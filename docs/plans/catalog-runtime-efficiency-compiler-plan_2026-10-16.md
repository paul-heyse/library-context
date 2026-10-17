# Catalog runtime efficiency: demand and joined compiler execution

**Accepted target / implemented source, integrated acceptance pending, 2026-10-16.** Companion to the
[coordinator](catalog-runtime-efficiency-plan_2026-10-16.md), which alone owns rule decisions,
dependencies and disposition. This document develops runtime F01/F02. It consumes existing exact
completed inputs and finality rather than redefining semantic admission or provider meaning.

### Implementation checkpoint

CX00 and CX01 are integrated on `main`. Demand-shaped provider acquisition now earns reached
namespaces and contextual preparation through exact dependency-owned slots. Upper producers run
as independently polled joined tasks; one coordinator retains readiness, ordered integration and
prefix publication. Stage joins precede provider-descendant discovery and native finality, so a
late producer cannot register work after its owner has declared drainage.
[ADR-0179](../adr/0179-joined-owned-producer-execution.md) and the semantic owner govern the
changed placement; native capabilities retain [ADR-0178](../adr/0178-atomic-bounded-native-effects.md).
The [integrated review](../design_review/reviews/design_review_catalog-runtime-efficiency-integrated_2026-10-16.md)
accepts the inspected composition at its declared source boundary. Focused controls do not close
RE90's integrated producer, provider, CLI and reconstruction acceptance. The requirements below
remain the acceptance contract, rather than a second current-disposition ledger.

## 1. Foundations, responsibility and intended changes

`cpg-core` coordinates producers, private results, model-owned readiness, integration and prefix
publication. `lctx-compiler-store` supplies exact native-backed inputs, scope preparation and
independent admission. `cpg-extract`/`cpg-flow` retain independent captured observations. Read
[semantic §15.1](../design/sections/semantic-model.md#section-15-1),
[acquisition/extraction](../design/sections/acquisition-and-extraction.md) and
[physical compilation](../design/sections/semantic-model.md#section-15-11).

The baseline already has demand-shaped DataFusion providers, private per-relation preparation,
checked predecessor rebase/handoff, complete-key optional Catalog requests and independently earned
reference admission. Mutable captured inputs still require full opening/post-provider checks.
Facts providers already run on explicit owned native threads with joins. Retain those mechanisms.

At the RE00 baseline, scope preparation eagerly resolved every declared table provider and
constructed contextual adjacency before root demand. The upper producer scheduler stored ordinary
futures inside one parent task; synchronous `borrowed_cpu` prevented polling siblings in that task.
The integrated corrections retain scope universes, coordinator integration and exact lifetime.

## 2. CX00 — Demand-shaped scope acquisition

### Contract and physical alternatives

Separate stable compiled scope-program metadata from earned provider ports and contextual
preparation. Inputs retain exact model/program, selected completed dependencies, provider roles,
physical copies, native owner/pins, client/session/pool and resource lifetime. A reached rule earns
the relation it needs; declaring an unused family must not automatically acquire its native port.

Keep native declared-reference frontier acquisition. For contextual pairs, use a frontier-constrained
query when its full semantic candidate universe can be represented. Reuse charged exact-dependency
adjacency for a repeated/dense consumer when that route removes repeated work. Start from existing
scope lowering and DataFusion/native selection; do not introduce a universal cost planner or a
new global adjacency cache. Select routes by declared rule semantics and actual consumers, with
the reason recorded in the implementation's owning contract.

Nearest/first rules must rank their complete partition before relevant postconditions; reverse
traversal, virtual/physical roots, absent references and unknown coverage retain their exact meaning.
A selected output region is not necessarily the sufficient input universe for a negative answer.
Do not push predicates earlier when they change selection. Whole-universe work that is necessary
remains explicit; remove only incidental eager acquisition.

Preparation owns resources until the last actual consumer/borrower. Reuse keys include exact
dependency views, vocabulary/model/program/context, physical owner/client/pool and relevant epoch.
Do not substitute library handle equality, nominal IDs or source hashes for those premises.
Failed preparation is not published into a reusable success slot; partial native work retains
its actual finality/recovery obligations. RN03 is consumed only if its new protected scope is used;
CX00 can otherwise retain existing valid native capability ownership.

### Error visibility and consumer migration

An input never consumed by production need not fail that operation solely because eager setup
previously inspected it. This is an intentional timing/visibility change, not identical eager
failure ordering. Independent admission still examines its complete declared domain and rejects
malformed content; selective production cannot certify an unexamined input.

Migrate callable normalization, Base, SourceCall, Summary and other actual scope-program consumers
to the common demand contract. Each retains its proper root universe and negative premises.
Remove eager provider/adjacency setup from callers, rather than leave both paths active. Reuse the
existing compiled scope and batch discovery contracts so RA00 does not acquire a second engine.

### Revealing evidence

Use service-free exact finite controls for empty/sparse/dense demand, unused malformed relation,
cycles, nearest/first partitions, reverse edges, absent physical roots and unknown coverage.
Compare root membership, proof/evidence distinctions and coverage against independent finite
expectations. Separately show whole-domain admission still rejects the malformed unused input.
Native controls establish actual acquisition and finality where necessary. Inspection must show
unused providers are not earned; a faster timeout receipt alone is insufficient.

## 3. CX01 — Joined independent producer execution

### Execution ownership

Keep one existing coordinator for readiness decisions, private terminal-result buffering,
predecessor acceptance, binding and ordered prefix publication. Place independently executable
producer work in independently polled joined tasks under the existing runtime. Preserve actual
Workspace/source/native capabilities, exact tracing context, cancellation and charges through
the final join. Semantic integration order follows model prerequisites, not task finish order.

Use task ownership to remove the same-parent polling dependency; it is not necessary to replace
every borrowed synchronous CPU kernel with detached work. Move a CPU closure to an owned joined
blocking task only where ownership can be carried completely and it permits needed progress.
The pinned Tokio `block_in_place` lets independent tasks run but suspends siblings within its own
task. A started `spawn_blocking` closure cannot be aborted; shutdown timeout stops waiting, not
work. Preserve cooperative cancellation at existing bounded kernel boundaries and actual joining.
If a borrowed kernel cannot safely transfer ownership, retain it inside its independently scheduled
producer task; do not manufacture `'static` ownership or release borrowed inputs early.

Verify the actual compiler/CLI/Python runtime entrypoints during CX01. The retained borrowed-kernel
route promises sibling progress on a multi-thread runtime, where `block_in_place` permits other
tasks to progress. On a current-thread runtime, `borrowed_cpu` executes inline: preserve correctness
and drainage, but do not claim sibling progress during that call. If a supported entrypoint needs
such progress, give its CPU work fully owned joined blocking execution before making the claim.
Do not silently replace the caller's runtime or narrow existing correctness support.

This is a change in placement under the existing scheduler, not a new executor framework or a
worker/thread cap. Facts OS-thread ownership already supplies independent execution and remains.
Facts/Normalized admission barriers remain until a substantive legal earlier consumer warrants a
separate decision. Producer completion does not automatically establish admission or readiness.

### Failure, publication and migration

First observed failure cancels new useful work while the coordinator drives already issued
operations to their actual terminal. Drain/join every started producer before retiring shared
preparation, inputs, native sessions, spill or signal ownership. Buffered private results cannot
escape cancellation. Panic/join failure, operation refusal, acknowledged commit, lost response and
failed finalization retain their distinct structured observations and actual recovery recipient.
Dropping or aborting a handle is not drainage and must not become a successful cleanup report.

Migrate all ordinary upper/normalization producer loops that share this scheduler, together with
buffer acceptance, rebase/freeze waits and error paths. Preserve the checked handoff and once-only
result integration. Reuse existing finite buffering and real readiness dependencies; do not create
a second terminal queue or binding owner. Apply RC04's ADR/semantic-owner route before relying on
the changed placement, then remove obsolete unspawned-producer assumptions and controls.

### Revealing evidence

A service-free two-ready-producer case gates synchronous CPU work in one producer and shows the
other producer progresses. Check deterministic integration despite opposite finish order, delayed
predecessor acceptance, cancellation before first poll, first failure with another task running,
buffer discard, panic/join failure and input/charge retention until actual joins. Use explicit
gates/handshakes rather than wall-clock performance assertions.

Run the progress control under the supported multi-thread realization and a separate current-thread
correctness/drainage control. Where fully owned blocking execution is selected for a current-thread
consumer, that consumer also needs the gated sibling-progress control.

Retain inherited producer/CLI controls and actual provider Resource-negative finality. Compound
paired-layout and Analysis journeys validate assembly later; they cannot replace local scheduling
or scope-universe cases, nor diagnose a named downstream kernel before it is reached.

## 4. Integration and completion

CX00 and CX01 can proceed independently after their contracts are clear. Coordinate common
compiler/normalization edits and RA00 discovery APIs under one integration owner. No full native
protocol replacement is a prerequisite for either correction. Preserve capture integrity, private
composition/rebase, old-port stability, reference schema/subtype/empty/EOF checks and failure-atomic
memoization from the earlier efficiency delivery.

During implementation, run touched-crate compile checks and revealing controls. RE90 owns final
model/compiler/CLI/provider/flow/reference-admission acceptance and the six reconstruction cases;
ordinary Catalog remains independent of optional Behavioral work. New source/identity changes
may require fresh generated outputs, but historical receipt hashing does not establish a current
binary and old failures are never rewritten as passes.
