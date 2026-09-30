---
id: ADR-0106
title: Build analysis and catalog results from finite typed evidence
status: proposed
date: 2026-09-30
supersedes: []
superseded-by: null
design: [§9, §9.9, §10, §11.1, §15.6, §15.9, §15.10]
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

**Proposed:** choose option 3.

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
  and heuristic exclusion from behavioral proof. Propose conditional operator review when the
  forward plan's named operator-workflow trigger is activated. Unreviewed briefs remain explicitly
  unreviewed; Phase 4 adds no mandatory manual queue.

## Consequences

The plan chooses finite exact witnesses plus honest residuals instead of an unbounded guarded
fixpoint. This addresses the open SCC decision in §15.6; it does not claim recursive completeness.
Detailed proof, residual and all-alternative controls are required before acceptance as implemented.

The manual-review policy is a proposed replacement for ADR-0005's unconditional review clause,
aligned with the forward plan's conditional workflow adoption. Acceptance must supersede that
record through the ADR lifecycle while preserving programmatic synthesis, evidence grounding,
statistical limitations and the remaining current Outcome policy at its owning sections.
Until that acceptance, this proposal does not change the accepted review requirement.

Implementation packages R1–E0 and their prerequisites own the migration. Independent dormant
expectations move before legacy producer deletion. P5 recovery assets remain explicitly inventoried;
no adapter or second semantic model becomes active. APIs and generated storage declarations are
validated together, with real-store cases for publication and independent controls for semantics.

Revisit when a concrete product claim requires recursion beyond the bounded evidence contract,
richer closure/effect stability, a new model channel, or a measured algorithm limitation. Such work
must identify its consumer and qualification, not silently weaken an existing verdict.

This is **Proposed**, supported by source/interface inspection on 2026-09-30. No new kernel,
publication path, embedding service execution or retrieval-quality result is claimed Tested or
Measured. The detailed plan and parent cutover findings own implementation status.
