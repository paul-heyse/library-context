---
id: ADR-0005
title: Insight synthesis is programmatic; LLM interpretation deferred behind a measured gap
status: accepted
date: 2026-09-22
supersedes: []
superseded-by: null
design: [§B10, §B11, §9, §10]
evidence: Proposed
revisit: After increment 3, the unresolved-slot counts by brief section (DESIGN §10.3), confirmed by the increment-5 evaluation, show a gap that templates and extractive selection cannot fill.
---

## Context

IP L1886–L1966 and L2580–L2637 put an LLM interpretation pass between the findings and the
published briefs. They also defer community detection, FCA/RCA and multiple embedding views
(IP L2073–L2087).

The operator directed the reverse. v1 should get as far as it can **programmatically**, using
graph analytics (community detection), relational concept analysis and embeddings. A local vLLM
generation model is added only if a real gap remains.

## Options

1. **An LLM interpreter as in the research input.** Rejected for v1:
   - it adds a model service and non-determinism to compilation;
   - generated text needs its own verification;
   - nobody has measured whether it is needed.
2. **The simpler alternative: structural passes A–C plus templates, and nothing else.**
   Rejected. It cannot group APIs into capabilities, find applicable cases, rank entry points
   or link docs to APIs, which is where the operator expects analytic value.
3. **Programmatic synthesis over analytics** (Passes A–C, community detection, centrality,
   FCA → RCA, embeddings). Every technique needs a named consumer and a mechanical ablation.
   Chosen.

## Decision

- **Synthesis without a model.** Assertions are produced by deterministic templates from typed
  findings, plus verbatim sentences selected from docstrings and docs (DESIGN §10). There is no
  generative model in v1, and **never one in the query path** (§B11).
- **Outcome order.** Docstring summary → explicit doc mention → `unresolved`. The
  embedding-nearest passage is published only as a doc link. Otherwise statistical text would
  fill exactly the slots the gap metric counts.
- **Status is derived, not chosen.** `unresolved` dominates. Any statistical supporting or
  scope-defining finding makes the assertion `statistically_derived`. A per-kind policy in
  `cpg-schema` states each kind's brief section and permitted statuses, and a §8 validator
  enforces it.
- **Statistical output is limited.** It may set titles, grouping, seeds, ordering and doc links.
  It may never state a control, a limit or a behavioral claim.
- **Exclusions revised (§B10).** Text embeddings and a rebuildable search projection are
  allowed. A graph database, workflow engine, composition planner, constraint solver, neural
  reranking and graph embeddings stay excluded.
- **Techniques must earn their place.** Community detection, centrality, FCA/RCA and analytic
  embeddings each follow §9. The keep rule (§9.8):
  - a technique stays if it changes published output **and** improves the pre-registered
    development metric (§12(b)) without lowering §12(a) or (c);
  - otherwise it is removed by ADR.
- **No manual truth review dropped.** From increment 3, every brief gets one manual review pass
  before publication (§10.4). That is the one check the mechanical grounding cannot do.
- **LLM trigger.** An ADR adds a local vLLM generation model, compile-time only and under §10.4
  grounding, when the revisit condition holds.

## Consequences

- **What we gain.** Compilation stays deterministic and reproducible from `content_digest`.
  Every sentence in a brief is traceable to a template field or a verbatim source sentence.
- **What we give up.** Some Outcome and usage text will be terse or `unresolved`. That is
  deliberate: those counts are the measurement that justifies, or rules out, adding a model.
- **More analytic machinery to own.** leiden-rs, our own FCA/RCA and embedding kNN (ADR-0011).
