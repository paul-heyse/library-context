# Execution foundations and preparation reuse

**Proposed implementation plan · 2026-10-02.** This companion develops F03–F05 and
opportunities §9.1/§9.2 of the [incremental review](../design_review/reviews/design_review_semantic-model-incremental-alignment_2026-10-02.md).
The [coordinator](semantic-model-incremental-alignment-plan_2026-10-02.md) owns current
disposition, shared editing scope and combined acceptance. No remediation has been implemented.

## 1. Baseline and design constraints

Static inspection used main `42551010` plus the preserved working tree on 2026-10-02.
The model already owns analytic methods, semantic definitions, typed outputs and replay.
The scheduler already owns dependencies, publication order, typed access and completion.
This plan consolidates repeated policy/closure/orchestration decisions and proven repeated
preparation; it does not replace these foundations with a workflow engine.

Authorities are [§9](../design/sections/analytics.md#section-9),
[behavioral analysis](../design/sections/behavioral-analysis.md),
[§15](../design/sections/semantic-model.md) and ADR-0085/0086/0106/0108.
The retained finite model, heuristic restrictions and hard frontiers remain unchanged.

Important foundation conclusions:

- [Schedule:630](../../crates/lctx-model/src/domain/stages.rs#L630) rejects duplicate
  input TypeIds. Epoch-specific requirements therefore cannot simply become duplicate grants.
- [read_at_epoch:1457](../../crates/lctx-model/src/domain/stages.rs#L1457) can narrow a
  vocabulary grant to an acknowledged earlier prefix. One sufficient grant can serve several
  exact consumed epochs without giving those epochs the same content.
- PublicationBoundary numeric codes are not chronological order. Schedule builds a checked
  order from publication groups before dependency construction and binds that same order
  to its final digest ([stages.rs:529](../../crates/lctx-model/src/domain/stages.rs#L529)).
- PreparedGraphs deliberately borrows StageAccess. Ordinary detached blocking work would
  require a new owned grant/lifetime/drain contract, not just a different function call.
- S0 already reads each nominal input once, but frames and automatic selection both decode
  and retain AnalyticFrame, analytic::Invocation and TechniqueResult. This is concrete reuse
  potential, rather than evidence that all synthesis inventories should be merged.

## 2. Retained analytic policy — F03

Introduce one model-owned fixed policy value. Its ordinary conversions supply persisted
MethodParameters, method-specific kernel bindings and the complete policy identity encoding.
Definition construction, production and replay consume this value. No new configuration UI,
policy table or generic algorithm provider is required.

| Method | Preserved policy and binding |
|---|---|
| PageRank | Damping 0.85, tolerance 1e-10, 100 iterations, current MAX_WORK, canonical weighted accumulation/dangling treatment |
| Communities | Resolutions 0.5/1/2/4, seeds 0–9 per resolution, RBER, 100 iterations, epsilon 1e-10, quality histories, hub percentile 0.95, equal retained layer weighting and current degeneracy/selection rules |
| Concepts/RCA | Support two, enumeration bound 20,000, current work policy and one RCA scaling step |
| Neighbors | Floor 0.5, retained top three and current work accounting |

Keep current MethodParameters and run/profile observations. Encode the complete structured
policy into AnalysisDefinition.semantic_version and generate any human-readable recipe from
it. Community scalar fields are representative/selected-profile settings, not a claim that
there is one run; retain all forty run observations and histories. Derive community preflight
work multipliers from the profile so the old 4000 multiplier cannot become a second authority.

Kernel mechanics remain where they are. Remove production reliance on independently authored
ranking defaults, community grids, concept/neighbor arguments and duplicated numeric recipe
text. A single policy edit must affect both actual bindings and definition identity. This
repairs change propagation; current source digests already prevent the identity collision
that a careless description of the review might imply.

Source: [analytics/build.rs:61](../../crates/lctx-model/src/domain/analytics/build.rs#L61),
[ranking.rs:25](../../crates/lctx-model/src/domain/analytics/ranking.rs#L25),
[communities.rs:72](../../crates/lctx-model/src/domain/analytics/communities.rs#L72),
[attributes.rs:692](../../crates/lctx-model/src/domain/analytics/attributes.rs#L692)
and [vectors.rs:174](../../crates/lctx-model/src/domain/analytics/vectors.rs#L174).

## 3. Finite upper-stage binding — F04

Use one closed upper-stage inventory in cpg-core, following the existing Normalization
declaration/runner pattern. Each variant supplies its owner-built declaration, Analysis or
Catalog phase, optional vocabulary publication boundary, graph needs and runner. Cover every
current route: configuration/native inventory, embedding preparation/text/effect, catalog
core/evidence/selection, Local, execution sub-stages, authored models, Summary, Structural,
Analytic, synthesis, retrieval and both frontier assessments.

Build declarations and publication groups from the same inventory. Facts group membership
continues to derive from provider declarations. Preserve the current ordered vocabulary
groups: Facts, Local, Model, Summary, Structural, Analytic and, for Catalog, Synthesis.
Do not sort boundary codes to produce this order or add unused nominal boundaries.

Execute Schedule order, resolving each upper declaration to exactly one binding. Preflight
rejects missing/duplicate routes and Analysis-phase dependencies available only after the
Catalog checkpoint. The binding does not replace model dependencies or infer relation meaning.
Facts/Normalized/Analysis checkpoints and one final publication stay explicit.

Graph needs are Summary→callable invocation, Structural→callable invocation plus definition
containment, Analytic→callable invocation. Share one preparation from completed sources using
the union of scheduled needs, but acquire it through an actual consuming stage’s permitted
inputs and prove that its grants cover that union. Retain per-consumer source/budget checks.
A new incompatible source set requires explicit separate preparation or preflight refusal;
never silently reuse another receipt’s graph.

Remove separate Catalog-name classification, graph-consumer names, string execution dispatch
and hand-maintained publication-group stage names as their finite replacements integrate.
Source: [compilation.rs:110](../../crates/cpg-core/src/compilation.rs#L110), lines 110–224
and 381–610; [pipeline.rs:18](../../crates/cpg-core/src/normalize/pipeline.rs#L18).

## 4. Exact dependency closure and sufficient grants — F05

The shared model operation takes explicit roots, ordinary owned outputs, lower-layer policy,
consumer vocabulary-epoch policy and the scheduler’s checked publication order. It returns
two products: exact read/validation requirements and one sufficient Stage grant per relation.
Do not conflate these products or replace owner input declarations with every transitive row.

Required behavior:

1. Traverse referenced targets and complete invariant inputs. Retain explicit invariant epochs
   and ordered validator contracts. Resolve unannotated vocabulary through owner policy:
   C1/C2 currently Local, Analytic Structural, S0 Analytic.
2. Distinguish direct roots, field references and invariant dependencies when resolving epochs.
   Do not implicitly inherit every target’s epoch from its referring row.
3. Directly consumed facts remain requirements/grants. The existing omission of inferred ordinary
   fact dependencies is an explicit lower-layer rule, never permission to erase invariant
   declarations or the operation’s actual consumed inputs.
4. A transitive ordinary dependency on the producer’s unfinished output refuses. Filtering roots
   used only for output validation is explicit; it cannot authorize an actual read. An earlier
   vocabulary prefix remains legitimate when this stage subsequently extends that vocabulary.
5. Preserve requirements at distinct epochs. Project them to one vocabulary grant at the widest
   required publication ordinal using the scheduler’s order, not enum codes or name-only dedup.
6. Preserve availability, transport and validator obligations. Incompatible policies/orders refuse
   or remain independently enforced. ValidationInput.order is a stream grouping/column-order
   contract, not a selective row projection; do not alphabetize/union it away.

Extract and reuse Schedule’s existing publication-group order construction for declaration
planning. Its planning order may compare epochs but grants no runtime prefix authority; the
final Schedule revalidates groups and binds ordinals to its actual digest. The finite binding
can provide groups before owner declarations need their closure. There is one order owner.

Actual consumption continues to originate from each operation’s declared consumed inputs.
Use existing [ConsumedInputs](../../crates/cpg-core/src/consumed_rows.rs) to resolve exact
source receipts and separate sessions for distinct acknowledged prefixes. StageSession’s
name-keyed table registration must not collapse those sessions. Coalesce only identical
acknowledged sources, while retaining every independent validator obligation.

Migrate all four closure builders: C1, C2, Analytic and S0. Migrate Analytic/S0 name-only runtime
loaders to their actual consumed declarations and epoch-aware sessions; Summary already
provides the pattern. Native preparation can reuse the conventions without making lctx-model
depend on cpg-core. Remove the four traversal loops and blanket epoch overwrite/dedup logic.

## 5. Borrowed CPU work and operational progress — opportunity §9.1

This opportunity is included: substantial synchronous kernels suspend the async stage, and
the existing sampler runs in that same task ([stage_runtime.rs:78](../../crates/cpg-core/src/stage_runtime.rs#L78)).
It cannot sample while the synchronous branch is executing.

Add a small borrowed CPU adapter at the stage boundary. On a multi-thread Tokio runtime use
block_in_place; on current-thread or no-runtime contexts run inline explicitly and report
the placement. Tokio 1.53.1 supports borrowed closures here; it does not cancel executing
blocking work. Production Runtime::new supplies multi-thread support. Inline fallback is
functional and panic-free but carries no responsiveness claim.

Move sampling to one separately spawned operational task per active stage. Its task owns only
measurement state, not semantic access. A scope guard stops it on success, error, cancellation
or unwinding; await its acknowledged exit on normal completion. Volatile stage/CPU-region and
frame start/end observations use existing tracing/measurement routes, with no database progress
registry and no operational timing included in semantic records or content digests.

Apply the adapter to Analytic, Structural and Summary borrowed kernels. Retain graph/access
borrows and reservations synchronously through return. Yield and observe cancellation between
existing frame/operation boundaries; a cancelled attempt is poisoned and cannot publish a
completion. An opaque executing kernel, including a Leiden run, drains before the next boundary.
Do not promise hard kernel cancellation, a new deadline contract or faster algorithms.

A detached spawn_blocking migration is rejected for this scope because the graph borrows
would require owned grants and a worker registry. If caller-return-before-kernel-completion
becomes a supported requirement, reopen that separate contract instead of silently changing it.

## 6. Shared analytic parents in S0 — opportunity §9.2

This opportunity is included with an exact three-relation scope. frames::Data and automatic::Data
both retain AnalyticFrame, analytic::Invocation and TechniqueResult; production visits both
for the same batch ([frames.rs:30](../../crates/lctx-model/src/domain/synthesis/frames.rs#L30),
[automatic.rs:47](../../crates/lctx-model/src/domain/synthesis/automatic.rs#L47),
[production.rs:33](../../crates/lctx-model/src/domain/synthesis/production.rs#L33)).

Introduce one narrow AnalyticParents inventory owned by frames::Data. Automatic seed selection
borrows it and retains only its unique usage/rank/community rows. Decode and retain each common
relation once in production. Preserve each operation’s complete input declarations and the
current context/configuration/definition checks; differing epoch universes cannot be shared.

Migrate production fan-out, S0 producer calls, frame parent verification, automatic selection,
seed replay checks and fixtures together. Standalone replay owns its own AnalyticParents from
its declared inputs; it does not borrow undeclared producer state. Remove duplicate three-row
stores/decoders. Do not merge documentary, control, Summary and pattern inventories into a
universal context. Reduced duplicate construction is source-substantiated; quantitative memory
or speed improvements remain unmeasured.

## 7. Packages, integration and acceptance

| Package | Prerequisite | Result and responsible boundary |
|---|---|---|
| E0 finite routes/order | Existing owner declarations | cpg-core owns one upper binding; model owns reused checked publication order; current profile/frontier routes preserved |
| E1 closure operation | E0 order construction | Model owns exact requirements→sufficient grants; meaningful negative and two-epoch controls |
| E2 closure/loader migration | Working E1 | C1/C2/Analytic/S0 consume helper; Analytic/S0 actual loaders preserve exact source universes; old loops removed |
| E3 retained policy | Existing analytic owner | One authored policy supplies records, identity and kernel arguments; existing defaults/results preserved |
| E4 borrowed CPU/progress | Settled E0 integration | Shared CPU placement and independent sampler; all three adapters migrated with explicit fallback/drain semantics |
| E5 S0 common parents | E2 consumed-input migration | One production inventory for the three common relations; replay remains independently scoped |

E3 is logically independent but shares analytics/build.rs with E2. One integrator owns that
file, accepting bounded patches serially. E4 shares stage/runtime/compilation surfaces with E0;
integrate after the binding. S0 reuse follows loader migration to avoid sharing wrong epochs.
Native plans may design against E1 while E2 proceeds, but an agreed interface is not tested
input availability. No separate worktree is needed for this read-only planning stage.

Meaningful controls to run during authorized implementation:

- Change one authored policy; assert captured kernel settings and definition identity change
  together while independent numerical answers, convergence/limits, shuffled inputs and
  membership-based partition expectations retain their stated tolerances.
- Compare both profiles and Analysis/Catalog stage sets, order, checkpoints and publication
  groups. Missing binding or cross-phase prerequisite refuses before effects; each route runs once.
- Use an early vocabulary prefix and a later prefix with an added row. One sufficient grant
  serves two consumed requirements with different receipts and row universes. Future prefix,
  own-output cycle, extra invariant dependency and conflicting validator order refuse as specified.
- Preserve direct fact inputs and compare unchanged existing declaration grants against
  source-written expectations, not output generated by the new closure helper.
- During a controlled borrowed CPU operation, an independent heartbeat/sampler advances on
  multi-thread runtime. Cancellation does not release borrow/reservation early; completion is
  suppressed at the next boundary. Current-thread fallback is explicit and panic-free.
- S0 common parents have one production decode/storage route; canonical parents/seed choices
  and crossed-context/wrong-definition refusals remain unchanged. Replay receives declared inputs.

Use existing model/stage/analytics/synthesis tests and targeted real-store controls. Add a small
deterministic heartbeat control rather than benchmarking the real library. During slices run
normalized release compile checks and targeted tests; combined gates run once after all functional
scope, as owned by the coordinator. No tests, builds, probes or new performance measurements
were run during authoring. Source/definition/model identity changes and final reconstruction
are shared coordinator obligations, not a compatibility migration.

## 8. Alternatives and completion

The selected design uses fixed policy constructors, ordinary typed closure functions and finite
bindings. A configurable workflow DSL, broad preparation cache or new policy tables would add
authority and lifecycle burden. Reuse is limited to proven common input and existing checked
source scopes. Input grant coalescing is mechanical capability lowering, not semantic epoch loss.

Independent assessment must challenge a legitimate two-epoch consumer, a new analytic policy,
a new stage route and a current-thread fixture. Closure of F03/F04/F05 requires integrated
consumer migration and removal; interface agreement or helper tests alone is insufficient.
Opportunities §9.1/§9.2 require the stated functional controls, not a claimed performance gain.
Record execution state only at the coordinator; wider Q0 and activation remain stopped.
