---
id: ADR-0106
title: Build analysis and catalog results from finite typed evidence
status: accepted
date: 2026-09-30
supersedes: [ADR-0005]
superseded-by: null
design: [§B10, §B11, §9, §9.9, §10, §11.1, §15.6, §15.9, §15.10]
evidence: Proposed
---

## Context

Phase 3 supplies normalized semantic ownership, privately admitted calls/bindings and persisted
native graph materializations. Retained analysis, catalog and synthesis still depend on dormant
`cpg-schema` tables and duplicate classifications. Reconnecting them mechanically would restore
those authorities and leave fresh recursive invocation guards and cyclic summary proofs unresolved.

The [Phase 4 detailed plan](../plans/semantic-model-phase4-detailed-plan_2026-09-30.md)
specifies the retained scope, exact seams, library fit, migration inventory and verification.
Phase 5 owns serving and product qualification. Phase 4 does not activate the old research
Stage 3–5 backlog, PR6, full-library comparisons or performance campaigns.

## Options

1. **Port dormant tables and consumers unchanged.** Lowest local edit count, but recreates
   divergent callable/catalog classification, provider-like derived evidence, untyped obligations
   and summary-proof identity problems. Maintenance and testing remain distributed.
2. **Design a general rule engine and solve unbounded recursive conditions.** Ascent/datafrog
   can compute finite relation fixpoints, but neither supplies the required typed witnesses,
   invocation-sensitive BDD conditions, partial coverage or current semantic budgets. Adding them
   would not eliminate the domain work; unbounded opaque guards are not a finite exact domain.
3. **Restore retained capability contracts under existing typed owners.** Preserve independent
   expectations, use the stored graph and pinned libraries, and explicitly bound recursive evidence
   and unsupported cases. This requires targeted model changes but localizes future changes.

## Decision

**Accepted target, 2026-09-30:** choose option 3.

- `lctx-model` owns analysis invocation/coverage, qualification, transfer/composition,
  obligation/discharge, catalog/selection, synthesis and retrieval meaning. `lctx-analytics`
  owns pure kernels; `cpg-core` orchestrates; `lctx-postgres` owns effects and storage.
  Derived support identifies analysis invocations and exact premises; native provider attribution
  is never fabricated.
- Correct derived override membership and symbolic class-of receivers before building the
  invocation graph on each new full collection. Hydrate the persisted native petgraph graph for
  algorithms through a borrowed, read-only view; private runtime indices never become domain IDs.
- One entry-value operation validates exact owner/read/formal, parameter-only reaches and complete
  provider/context coverage. Producers and stored validators share it. Composition requires
  private admission and records every source contribution, alternative and binding variant.
  Missing captured-state semantics produce an explicit obligation.
- Schedule SCCs callee-first. Semantic equality excludes accumulating proof identity and cost.
  Preserve exact invocation-distinct guards within the retained depth-8/proof-step-64 envelope.
  Keep nondominated bounded witness representatives; residuals use stable finite boundary keys.
  Never erase invocation distinctions and call the result Exact. Residuals cannot establish,
  refute or discharge a universal claim.
- Separate finite evidence-occurrence DAGs from final aggregate summary publication. A composition
  witness cites earlier witnesses; an aggregate cites witnesses only after closure. It is never
  its own proof premise. Coverage membership remains complete independently of proof deduplication.
- Retain five behavioral verdicts and separate four-state requirement selection. Negative and
  universal claims require closure over the exact relevant domain; absence and empty observations
  are not refutations. Programmatic synthesis obeys one per-value status policy and grounding
  contract. Optional briefs require an admissible Outcome; mandatory public catalog membership
  does not depend on a brief, behavior, ranking or embeddings.
- Catalog consumes canonical public exposures, callable/signature variants, types and binding
  semantics. Original source/document artifacts remain canonical evidence. Association basis,
  scenario context, execution intent and actual check receipts stay distinct.
- Use existing petgraph, bounded BDD, weighted ranking, Leiden and bounded FCA/RCA mechanisms.
  Optional techniques stay off by default. Their outputs retain universe, parameters, diagnostics
  and heuristic meaning. No statistical result becomes execution evidence.
- Shared embedding spec/codec/admission/cache policy has one owner. Analytic and retrieval
  consumption records have distinct subjects and one writer each, storing exact consumed bytes
  for service-free replay. Early analytic inputs do not depend on later briefs/retrieval.
  Vector failure can leave explicit lexical-only availability when optional.
- Keep programmatic synthesis, grounded evidence, no generative model in compile/query paths,
  and heuristic exclusion from behavioral proof. Use conditional operator review when the
  forward plan's named operator-workflow trigger is activated. Unreviewed briefs remain explicitly
  unreviewed; Phase 4 adds no mandatory manual queue.

### Retained synthesis and technique policy

This record consolidates the surviving decisions and amendments of ADR-0005. Synthesis uses
deterministic templates from typed findings and verbatim source sentences, with reproducible
content identity. No generative model runs in compilation or the query path. A future compile-time
local generation model requires a new ADR and mechanical grounding, only after unresolved-slot
counts by brief section expose a gap templates/extractive selection cannot fill, confirmed by the
independent held-out evaluation. This remains a conditional research trigger; Phase 4 activates
neither that evaluation nor a generation service. The query-path prohibition survives permanently.

Outcome selection remains docstring summary, then an explicit document mention, then unresolved.
An embedding-nearest passage is a document link only and cannot fill the gap metric's Outcome
slot. No admissible Outcome means omit the optional brief with an explicit reason; it never
removes a mandatory public catalog member. Status derives from support: unresolved dominates;
statistical supporting or scope-defining evidence makes the value statistically derived where
its kind permits that status. One model-owned per-kind/section policy enforces the floor and ceiling.

Statistical output may nominate titles, grouping, seeds, ordering and document links; it never
states controls, limits or behavioral claims. Definitions decide semantic membership. Every
sentence traces to a template field or a verbatim source span. Grounding validates same-generation
symbols, parameters, findings, witnesses and exact source spans, snippet parsing/name binding and
status against evidence. Warnings, limits and unresolved conditions survive rendering budgets.
Repository text remains untrusted data, never compiler instructions. Mechanical grounding does
not establish that extracted prose is true of a particular entry point or imply manual approval.

Every technique requires a named consumer in the served model (a tool output or brief), fixed-input
reproducibility, declared method/parameters/universe/diagnostics and a mechanical ablation of
published output. The current product keep criterion in §9.8 supersedes the historical brief-only
metric: a new default needs independent product-task benefit against the existing default and
unstructured-evidence baseline, with no incorrect/misleading claim or displacement regression.
Parameter freezes and current off-by-default variants stay unchanged; no upfront ablation campaign
or automatic deletion is activated by this cutover. Retain optional kernels/tests until a selected
consumer or measured cost/removal case warrants an ADR. The historical brief-default ablation
and its held-out/second-library reconsideration triggers remain dated evidence in ADR-0020,
not a second current keep rule.

Text embeddings and rebuildable search projections remain allowed. Graph databases, generic
workflow engines, general composition planners/constraint solvers, neural reranking and graph
embeddings remain excluded under §B10; the bounded condition kernel is not such a solver. Existing
conditional solver adoption needs its separately registered consumer and decision.

Conditional operator review replaces only the unconditional manual pass before every brief.
Until the forward plan's [F1 workflow trigger](../plans/behavioral-model-forward-plan_2026-09-24.md#postgresql-adoption)
selects an operator-facing review consumer, grounding may publish explicitly unreviewed briefs.
When selected, append exact-subject-revision review events with provenance/idempotency and
optimistic conflicts, freeze the selected revision into canonical compiler inputs, and qualify
backup/restore and replay. Never patch a published brief, infer approval from absent events, or
claim unreviewed prose was checked. No mandatory manual queue is added by Phase 4.

## Consequences

The plan chooses finite exact witnesses plus honest residuals instead of an unbounded guarded
fixpoint. This addresses the open SCC decision in §15.6; it does not claim recursive completeness.
Detailed proof, residual and all-alternative controls are required before acceptance as implemented.

ADR-0005 is superseded and retired after carrying its surviving decisions into this record and
the current owners. Its unconditional per-brief manual-review target is rejected in favor of the
conditional operator workflow above; its no-model, grounding, Outcome and heuristic restrictions
survive. Adoption is authorized under the unchanged independently reviewed
[Phase 4 design](../design_review/reviews/design_review_phase4-plan_2026-09-30.md), and establishes
no implemented review workflow.

Implementation packages R1–E0 and their prerequisites own the migration. Independent dormant
expectations move before legacy producer deletion. P5 recovery assets remain explicitly inventoried;
no adapter or second semantic model becomes active. APIs and generated storage declarations are
validated together, with real-store cases for publication and independent controls for semantics.

Revisit when a concrete product claim requires recursion beyond the bounded evidence contract,
richer closure/effect stability, a new model channel, or a measured algorithm limitation. Such work
must identify its consumer and qualification, not silently weaken an existing verdict.

This is an **Accepted target** with evidence labeled **Proposed**, supported by source/interface inspection on 2026-09-30. No new kernel,
publication path, embedding service execution or retrieval-quality result is claimed Tested or
Measured. The detailed plan and parent cutover findings own implementation status.
