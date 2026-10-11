# Efficiency alignment: model, analysis and synthesis

**Proposed implementation plan, 2026-10-11.** This is a supporting plan in the new
[efficiency-alignment track](efficiency-alignment-plan_2026-10-11.md). The coordinator owns the
baseline, operator decisions, execution status, source-finding disposition and combined exit.
Existing plans were not consulted and are not superseded. This document specifies future work;
no product implementation or tests were performed while authoring it.

Sources are original O-F31–O-F34, audit A-F07/A-F14/A-F15, and the audit's
[fresh-coverage inquiry](../design_review/evidence/2026-10-11_efficiency-alignment-audit/fresh-coverage.md).
`O-` means the [original review](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md);
`A-` means the [audit](../design_review/reviews/design_review_efficiency-alignment-audit_2026-10-11.md).
The executable model remains semantic authority. Improvements change preparation and physical work,
not the meaning of proof, coverage, a neighbor, a community, or a published synthesis decision.

## 1. Target and foundation assessment

The target is immutable, invocation-owned preparation feeding bounded operations. Prepared state
is keyed by the inputs it actually indexes, admitted before allocation, shared across the invocation,
and released with that invocation. It does not become another persistent cache or a new global
analysis service. Exact source receipts, semantic progress, independent validation and explicit
partial/refusal outcomes remain separate responsibilities.

Current strengths are useful foundations: petgraph supplies deterministic SCC scheduling;
`summary_worklist::Frontier` separates semantic progress from Pareto proof cost; checked model
transfers retain actual call/Entry evidence; synthesis retains complete decisions before selecting
briefs; embedding preparation already deduplicates texts and records refusals. Reuse these contracts.

Current amplification is concrete: repeated condition lowering, per-seed graph maps and adjacency
sorting, global Summary discovery scans, repeated synthesis joins and partition-local realization.
The optional analytics contract also requires real pair work; replacing it with a different method
is not a transparent optimization. Relevant H5/H7/H9/H12–H14/H17–H20 require selective preparation,
complete semantic inputs and honest live-state accounting, not a prescribed data structure.

| Package | Delivered behavior | Prerequisite |
|---|---|---|
| A01 | Shared checked condition preparation across actual consumers | Existing condition/model contracts; no native redesign |
| A02 | Prepared Structural frame and bounded deterministic adjacency consumption | A01 only where the consumer actually uses its condition preparation |
| A03 | Indexed Summary candidate discovery preserving proof alternatives | Existing SCC/worklist contracts; A01 for shared lowering integration |
| A04 | Exact analytics with prepared windows, bounded top-k and explicit pair-work policy | Existing method contract; resolve the bounded library/equivalence comparison below |
| A05 | Prepared automatic synthesis joins with complete decision population | Existing admitted input/frame contracts |
| A06 | Cross-root batching of distinct ready embedding work | Existing realization/dedup interface; serving cancellation contract for in-flight shutdown |
| A07 | Integrated semantic and refusal regression boundary | Working A01–A06 and compiler coverage/reuse consumers when changed |

Package boundaries are not synchronization barriers. A05 and A06 can proceed while A01–A04 are
being designed. Compiler contribution/value identity must not be used to broaden these caches
until the compiler plan has established the exact dependency and receipt contracts.

## 2. Implementation packages

<a id="A01"></a>
### A01 — Prepare conditions once at their real lifetime

**Owner:** `lctx-model::domain::conditions`, with execution/analysis consumers. Source entrypoints:
`conditions/graph.rs::CondGraph::join`, `Diagram::from_graph`, and `kernel.rs::union/effective`.
Constant/equality shortcuts already exist and must remain. Effective support is determined after
reduction; a full-graph variable limit can wrongly reject a cancelling expression.

Create an invocation-owned preparation object for validated condition-node inventories, normalized
variable correspondence and reusable lowering. Borrow indexed source rows rather than copying the
entire inventory at every call. Cache entries must distinguish graph/root, effective support and
relevant model/kernel version; enforce budget admission before retained state grows. Keep malformed
root/closure rejection and bounded pair work. Do not substitute syntactic equality for semantic
condition identity.

The targeted breadth pass adds actual consumers in `execution/model_context_transfer.rs::derive`
(currently clones all condition nodes and lowers qualification conditions) and
`execution/model_transfer.rs::derive`. Migrate these with the existing condition consumers rather
than optimizing only the review's initial examples. Preserve checked WithTarget reads, exact Entry
origins, context/scope compatibility and actual-call evidence. Produced-state reuse and independent
cold replay remain distinct; the latter must not secretly consume a producer's unchecked result.

**Closure:** cancelling expressions, dead atoms, shared subgraphs, invalid roots, budget refusal
and context-incompatible transfers retain their identities/outcomes. Repeated consumers of the same
inventory use the prepared representation. Static call-path evidence plus minimal pure controls
establish this; a timing campaign is not required.

<a id="A02"></a>
### A02 — Prepare Structural frames and bound adjacency before materialization

**Owner:** `domain/analysis/delegation.rs` and Structural build. Build entity/node maps, event
assessment indexes and ordered adjacency once per immutable frame. The current `traverse` builds
maps repeatedly and collects/sorts every outgoing edge before testing `Bounds::arcs`.

The prepared graph exposes deterministic `(target, site, arc)` traversal. Admit its retained size
before building it. If full ordered adjacency is unsuitable, use a charged ordered/selection route
that produces the same next edges without accumulating an unbounded temporary vector. Merely
truncating the library iterator changes witnesses and is forbidden. Selected-frame preparation
and per-seed frontier/parent/depth state remain separate, with independent charges and lifetimes.

**Closure:** isolated seeds do not rebuild the whole frame; a high-degree node with a small arc
budget does not allocate/sort its whole adjacency per seed. Preserve first witnesses, parallel edge
multiplicity, depth and vertex stops, `Stop::Arcs`, unresolved assessments and deterministic output.
Use low-bound/high-skew cases as functional controls of work shape, not latency benchmarks.

<a id="A03"></a>
### A03 — Index Summary discovery without pruning valid proofs

**Owner:** `execution/summary_production.rs`, consuming `summary_schedule` and `summary_worklist`.
The source paths at original B lines 1185–1234, 1310–1334 and 1942–1968 join Entry/support relations,
scan calls per proof and scan seeds/branches per SCC. Prepare seeds/branches by owner, calls by
caller/callee, and Entry/support adjacency once per immutable invocation.

Candidate generation follows those relations, not all calls or every component. Preserve all
semantically required support alternatives: indexing is not license to choose one representative
before the worklist's Pareto/semantic rules. Keep queue admission, stale-proof filtering, callee-first
SCC order, deterministic processing and residual construction at their existing owners.

The breadth pass through `summary_consequences.rs` confirms that finite alternatives, complete
call-member claims, open/excluded/proved member standing, and refutation coverage are separate
contracts. Migrate preparation consumers without collapsing these distinctions or equating an
exhausted queue with complete call closure. The shared obligation owner still computes verdicts.

**Closure:** many independent singleton SCCs no longer multiply unrelated discovery; recursive
SCCs, alias fan-out, multiple support proofs, dominating/non-dominating proof costs and bounded
residuals retain canonical results. Compare complete proof/assessment populations, not just the
first positive answer.

<a id="A04"></a>
### A04 — Repair optional analytics while retaining the declared exact methods

**Owner:** `domain/analytics::{neighbours,build}`. Preserve exact best-window target scoring and
its tie order. Preserve the declared community graph/method. The audit corrected the original:
co-use refusal is frame-local, not automatically global across all frames.

Prepare admitted vectors/windows and reusable correspondence once per selected analytic frame.
Use a bounded streaming top-k representation so candidate retention follows requested output while
score evaluation remains exact. Co-use construction streams deterministic exact pairs under the
existing charged work/result contract; avoid retained all-pairs intermediates and duplicate frame
preparation. Keep frame-local partial/stop information explicit.

Before selecting a replacement scoring primitive, compare the pinned library's composed route with
the existing exact kernel: required F32/F64 conversions, dimensionality, normalization, best-window
aggregation, deterministic tie order, error exposure and charged buffering must agree. Source and
small independent numeric cases can settle equivalence. If no library route preserves them, retain
the exact kernel and apply the preparation/retention repairs. This bounded decision blocks only
primitive replacement, not those repairs.

Do not adopt approximate shortlists, per-owner caps or bipartite weighting as transparent fixes.
They can change candidate winners or community meaning. Such a change requires a separately stated
method identity and operator-confirmed contract decision; it is not authorized by serving RC10.
Necessary exact pair work remains an explicit method/support limit. Optional large-scale method
redesign and quantitative thresholds are follow-ups, not falsely reported as eliminated work.

**Closure:** exhaustive small independent references cover multiple windows, ties, self-exclusion,
empty/invalid vectors, high-degree co-use and frame-local refusal. The implementation removes
avoidable preparation/retention without claiming a subquadratic exact algorithm it does not provide.
The source-finding disposition must distinguish repaired amplification from necessary pair work.

<a id="A05"></a>
### A05 — Index synthesis joins and retain every decision

**Owner:** `domain/synthesis/automatic.rs`, with `cpg-core::synthesis` as composition consumer.
Prepare member frames by exact member/context, analytic result selections by frame/method,
rank by result/target, and community membership by selected result/entity. Use multi-valued buckets
where multiplicity matters; a single-value map must not hide duplicate-slot ambiguity.

Consume admitted inputs and actual configuration. Preserve absent analytics, configured selections,
deterministic rankings, community caps and all `DecisionReason` transitions. The current automatic synthesis implementation
publishes decisions for candidates that do not become briefs, so early termination at `brief_budget`
is incorrect. Keep the existing compact documentary path and hydrate rich selected members late.

**Closure:** complete automatic decision and selected-output populations match for absent methods,
ambiguous slots, tied ranks, configured communities, duplicate slots and exhausted brief budgets.
Candidate joins use keyed lookup. Retained preparation is charged and released at invocation end.

<a id="A06"></a>
### A06 — Batch distinct ready embedding work across the render window

**Owner:** `cpg-core::retrieval/analytic_embedding` composed with model embedding realization.
The current renderer already has bounded root windows; realization already deduplicates texts,
reuses values/refusals and flushes at a batch or budget boundary. Preserve those facts rather than
assuming one request per root.

Collect only distinct ready text/spec requests across the existing charged render window. Coalesce
compatible requests through the existing session preparation/realization boundary; do not retain
full rendered payloads just to fill a remote batch. Route each returned value/refusal back to exact
root/window/spec attribution. Keep per-root publication and verification. A root may require
multiple requests, and duplicate text may require none.

Analytic and retrieval consumers share an implementation only where their spec/result contracts
match. Cancellation retains ownership until actual remote/native completion and drains outstanding
work; use the serving plan's request-lifetime contract instead of a second scheduler. No new vLLM
service, worker cap or deployment change is introduced.

**Closure:** duplicated and distinct texts across roots yield the same attributed values/refusals;
small budgets and cancellation neither lose pending work nor grow live payload state without charge.
Observe bounded request composition in a deterministic fake transport control, then qualify actual
transport behavior within the functional integration boundary; a mock alone does not pass that leaf.

<a id="A07"></a>
### A07 — Integrate semantic preservation and independent observation

Run the affected pure model/analytics controls after each coherent implementation, then the affected
compiler/synthesis functional controls once their consumers are integrated. No database is needed
for finite pure kernels; persistence claims require actual native controls in the coordinator's
functional exit. Optional methods receive focused enabled-path controls even though default-off.

The breadth pass inspected `lctx-eval::observer::decode_for_call` and `mcp_observer`: final public
bytes, duplicate field/qualification checks and an independent decoder remain useful foundations.
Preserve them across output/coverage migrations. Do not generate expected answers with the same
new preparation routine or read protected evaluation populations. There is no basis in this pass
for an evaluation rewrite or a new global verdict authority.

## 3. Interfaces, transitions and completion

Compiler input binding owns exact source/value eligibility. This plan borrows its immutable inputs;
it cannot relax `ExactView` receipts or profile availability independently. Native admission owns
persisted integrity; a prepared in-memory kernel is not a durable validity witness. Serving consumes
published semantics and must see the same coverage, omission and method identity.

Prefer internal prepared views and adapters over new public schemas. If a change alters published
method/output identity, budget semantics or qualification meaning, state that change, revise the
owning contract and migrate all producers/consumers before publication. Old and new meaning must
not share an identity accidentally. Do not retain compatibility-only implementations after the
current design's coordinated cutover; preserve named live consumers until that cutover is complete.

The coordinator is the sole progress/disposition owner. This companion completes when A01–A07
have their functional evidence and migrated consumers; performance benefit and large real-library
scale remain unmeasured follow-ups. It cannot claim whole-product or evaluation-population qualification.
