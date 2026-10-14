# Catalog creation: finite analyses and graph preparation

**Proposed, 2026-10-14.** Companion to the [coordinator](catalog-creation-plan_2026-10-14.md),
whose §6 owns all dispositions and evidence. Develops creation F01/F06 and D03–D06. The target is
the functional API/evidence product with optional Behavioral/analytic enrichment, not general Python
semantic completion or an automatic graph-engine replacement.

## 1. Meaning, workload and reusable foundations

Public exposures/access paths, implementation identities, source/effective defaults, usage roles,
invocation contexts, qualified alternatives and coverage are consequential distinctions. Their
model operations remain authoritative. New physical indexes or algorithms must preserve competing
evidence rather than select an arbitrary first row. Alias-rich classes, inherited members, corpus
artifacts, shared suffixes and cycles are ordinary supported shapes.

Current graph preparation already hydrates selected projection chunks once, retains exact topology
and prepares Behavioral invocation SCCs. Directed petgraph edges retain ArcIds and parallel evidence;
the vertex universe retains isolates. Traversal operates over the declared universe before output
selection. PreparedDelegation caches checked ordered adjacency, including failures. Delegation is
bounded BFS/canonical-parent traversal with capped shortest-final witnesses, not unrestricted path
enumeration. Summary already has callee-first SCC scheduling, endpoint/support indexes, semantic
delta pairs and Pareto representatives with independent proof evidence. Prepared BDD roots and
bounded apply already exist. Preserve these; "prepare graphs once" and "use SCCs" are not missing fixes.

Source routes: [C0](../../crates/lctx-model/src/domain/catalog/build.rs),
[public paths](../../crates/lctx-model/src/domain/catalog/paths.rs),
[C1](../../crates/lctx-model/src/domain/catalog/evidence/build.rs),
[Structural](../../crates/lctx-model/src/domain/structural/build.rs),
[controls](../../crates/lctx-model/src/domain/structural/controls.rs),
[delegation](../../crates/lctx-model/src/domain/analysis/delegation.rs),
[Summary](../../crates/lctx-model/src/domain/execution/summary_production.rs) and
[analytics](../../crates/lctx-model/src/domain/analytics/build.rs).
The [algorithm inquiry](../design_review/evidence/2026-10-14_catalog-creation-performance-target/algorithms-and-library-fit.md)
supplies pinned comparisons and exact additional source locations.

## 2. CA01 — one correspondence preparation per useful grain

Use charged local indexes when rich rows are already required. Where input remains columnar and
many rows can be rejected before decoding, compare one bulk DataFusion join/semi-join/grouping at
the actual scope loader. Do not repeatedly build a plan for each row or move prepared adjacency
back into tables for BFS. A filter expression does not establish provider pushdown or an indexed
native access path; inspect the selected plan when that alternative is used.

| Kernel | Prepared correspondence and consumed behavior |
|---|---|
| C0 exposure/callable/class/field | Exposure→all candidates; symbol/context/resolution→all matching class observations; class→fields, field→declarations/links, declaration→default assessments. Reuse the existing resolution→entity-candidate precedent; preserve all default/evidence states |
| C0 public paths | Class/context→ancestry/MRO assessment and ordered ancestors; declaration parent→children; event-site/context→all bindings; reusable member/shadowing correspondence where identity permits. Use deterministic ready work instead of restarting a pending-map scan |
| C1 evidence | Member→invocation frames; coverage-scope→artifacts; artifact/input→roles; attempt→bindings and member/parameter→options; artifact→deployment/ownership. Preserve binding authority/check status and exact release/source/context association |
| Structural | Callable→module/use/subsystem roles; member→core links; configured public-slot counts in one pass; occurrence→handoff alternatives. Do not reinterpret invocation as definition or lose unresolved events |
| Requested attributes | Symbol/context→all resolutions; sequence/list→members; scope membership set; stable native correspondence shared across namespaces while each object universe/provenance remains distinct |

Preparation belongs to the invocation/window/frame that consumes it. Bind complete input selection,
context/model/profile and resource ownership. Charge map buckets, nested values, keys and live backing;
release after actual last consumer. Sharing complete immutable MRO/member preparation can avoid
repeating work across public paths, but each path still emits its own public answer. No persisted
index cache, generic index framework or new semantic schema is selected.

Maintain MRO precedence/shadowing, rebound/incomplete/open ancestry, recursive/depth boundaries,
ambiguous/missing correspondence, distinct source/effective options and Absent/Unknown/Unavailable/
Literal/Expression/Factory defaults. A join that multiplies rows accidentally or drops an unmatched
assessment is not equivalent. Required combinations must remain; remove incidental searches, not
semantically necessary output.

**Acceptance:** independently specified two aliases to one class, competing candidates, crossed
contexts, incomplete MRO and all default states; C1 binding/role/deployment distinctions; Structural
duplicate module candidates and unresolved targets. Inspect one preparation per stable grain and no
identified repeated full-relation lookup. Current compact C1 admitted-frame routing is preserved,
not misreported as whole-rich-frame rehydration. No elapsed-time threshold is needed.

## 3. CA02 — allocation-matched scratch and compact controls

`catalog/paths.rs` charges per-expansion `chain`/`nearest` maps to a function-wide StateCharge.
`structural/controls.rs` similarly charges each root's visited/pending state to one outer charge.
Charged containers do not own that external reservation; dropping them does not release it. Separate
retained roots/pending/output state from per-expansion/per-root scratch and give each reservation
the same lifetime as the actual allocation. Existing per-domain check charges in Selection provide
a local precedent. Do not release live queue capacity, nested paths, output or borrowed backing.

For Behavioral controls prepare caller/source-formal→outgoing argument flows and corresponding
unfollowed arguments. Replace copied full pending prefixes with predecessor/witness arena handles;
consume processed queue entries and release/retain their charges according to actual allocation.
The visited key remains `(target, formal, suppressed)`; reconverging different formals or suppression
states are not duplicates. Admit the canonical state/witness consistently with existing BFS ordering,
depth/vertex/arc limits and partial reasons. Do not count released scratch as retained memory, or
silently undercharge an arena that remains live for reconstruction.

Internal arenas can materialize the current bounded Path/Step output at the real publication consumer.
This does not require RC03. Published prefix duplication remains until D04 selects a different actual
consumer contract. No universal resource framework or unbounded queue is selected.

**Acceptance:** a small-budget many-class case fits current scratch plus retained output without
cumulative dead charges; a two-root control releases first-root scratch before starting the second;
diamond/reconvergence, self/two-node cycles, parallel flows, distinct formals and suppressed/unsuppressed
states retain canonical witness and partial/refusal semantics. Check charging through failure,
cancellation, final borrower and arena materialization. Control costs are Behavioral; C0 charge
lifetime affects mandatory Catalog work.

## 4. CA03 — bounded graph and analytical decisions

All four decisions require a supported complete-route comparison, not speculative library adoption.
Use the selected rust-graphs/reasoning/DataFusion/fixedbitset skills and exact resolved versions for
material APIs. Current direct pins include petgraph0.8.3, fixedbitset0.5.7, DataFusion55.1.0,
biodivine-lib-bdd0.6.3, leiden-rs0.8.1 and fcars0.2.2. Recheck manifests/skills before transferring a
claim. Additional documentation lookup follows Context7/primary-source routes; empty search is not
absence evidence. No dependency move is selected by these plans alone.

### D03 — repeated public seeds and shared suffixes

Compare independent bounded BFS with frame-local boundary/role classification reuse first, then
source-labelled frontier or shared suffix work if actual public seeds repeat useful work. Complete
keys include exact materialization, frame/boundaries, direction/arc roles, bounds and selected source.
Per-seed shortest witnesses, omitted flags, examined-work/partial reasons and unresolved events remain.
SCC condensation can share Boolean reachability but is not itself shortest path, transfer or control
evidence. All-pairs closure can create dense state never requested by this consumer.

Settle with alias-chain/diamond/cycle shapes, parallel alternatives and external/subsystem boundaries.
Retain independent BFS when sharing adds more state/semantic complexity than the repeated work it
removes. Reopen for a concrete many-wrapper suffix workload, not graph cyclicity alone.

### D04 — published witness representation

Inventory actual Path/PathStep, ControlPath/ControlStep, synthesis, original-evidence and native
hydration/rendering consumers. Compare internal predecessor arenas with current output against a
shared immutable witness DAG preserving every required path/ArcId/qualification identity and bounded
explanation. State which consumers require flat enumeration, canonical path selection or independent
alternatives; no arbitrary deduplication of proofs is permitted.

RC03 conditionally permits choosing a changed published representation. If chosen, specify identity,
codecs, complete consumers and source-independent known answers together; record the ADR/owner change,
version deliberately and regenerate current data with no old-format reader or dual runtime path.
Do not change semantic v4 into a canonical-state DAG under this permission. If flat output is the
simpler adequate consumer contract, retain it and complete internal CA02 independently. Acceptance
covers ambiguous/parallel/reconvergent witnesses, missing/corrupt nodes, deterministic ordering and
end-to-end evidence rendering, not just graph compression.

### D05 — optional analytic growth and demand

Analytics are selected independently; unrequested results remain explicit without computation.
Co-use/type/mention clique contribution can grow quadratically with owner degree; full required pair
evidence cannot be removed by a hash map. Compare owner→targets incidence and on-demand weighted
aggregation against pair materialization while preserving selected universe, exact weights and
evidence lookup. Leiden's repeated resolutions/seeds serve stability diagnostics; reducing them
changes its recipe and is not merely an execution refactor.

FCA contexts retain exact ordinal row/column bitsets; many independent attributes can produce many
concepts/pseudo-intents. Current RCA performs one direct relational attribute step, not an unbounded
fixed point. Compare direct intent/extent/implication queries for actual consumers against complete
frequent concept enumeration; specify any changed output contract explicitly. fcars is an existing
independent concept-set oracle, not a drop-in bounded frequent-concept/implication implementation.
Qualify support pruning, work/refusal, implication output and deterministic meaning before adoption.

Keep the sparse custom weighted PageRank kernel: the inspected petgraph serial built-in does not
meet its weighted/residual/work contract. Preserve full computation universe, dangling nodes and
qualified heuristic output. Leiden communities never become correctness partitions for Summary or
controls. Bitset ordinals and lengths bind one exact universe; equal-looking bits across frames do
not establish equal sets.

Settle selected changes with high-degree incidence/weighted pairs, isolates/parallel arcs, shuffled
inputs and independent-attribute concept cases. Compare partitions by membership, numeric results
under stated tolerances, and incomplete enumeration honestly. Keep current optional recipes with
explicit growth/support triggers if no simpler equivalent or consumer-approved narrower result wins.

### D06 — Summary compatibility candidates

Preserve existing callee-first SCC and delta pair queues, support indexes, stale-representative
rejection, Pareto incomparable costs, separate proof evidence and explicit work/depth/proof residuals.
Assess whether descriptor/port compatibility indexes can reject impossible compositions before
combining endpoint/support buckets. Do not drop genuinely qualifying independent proof combinations,
deduplicate only by endpoints, or equate SCC order with semantic convergence.

Use a recursive SCC, two incomparable costs, equal-cost independent evidence and a late improved
callee proof. The states, qualified proofs and residual outcomes must agree with independently
specified expectations. Retain current indexed worklist if new machinery does not simplify the
actual candidate route. Salsa/Datalog/another solver is not justified merely by recursive topology.

## 5. Integration and completion

CA01/CA02 can proceed before broader native/scheduling changes, coordinating shared model files.
CA03 selected changes integrate real producers, admission, synthesis/retrieval and affected native
consumers before their obsolete representation is removed. Algorithms consume exact completed input
authority from compiler/native owners; they do not own those admission policies.

Every package reports its migrated consumers, deleted repeated work, revealing controls and support
limits to coordinator §6. Pure graph/model controls establish finite contracts, not persistence,
serving, whole FastMCP completion or measured performance. CX01/CX02 and regeneration retain those
separate exits. No benchmark corpus, all-analytic campaign or Behavioral research backlog is imported.
