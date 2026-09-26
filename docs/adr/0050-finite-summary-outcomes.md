---
id: ADR-0050
title: Compose finite summaries over explicit inputs and publish typed refusals
status: accepted
date: 2026-09-25
supersedes: []
superseded-by: null
design: [§9.9]
evidence: Tested
revisit: A second origin shares one raw return key but requires a distinct persisted refusal, or recursive composition needs a richer semantic state than finite inputs and outcomes provide.
---

## Context

The first finite summary rules lived in `cpg-core/src/summaries.rs` alongside DataFusion
acquisition. The producer skipped unsupported or capped paths, while a separate SQL anti-join
inferred `call_transfer` or `unsupported_control_flow` from missing positives. A depth cap thus
lost its actual cause, and testing a normal path required extraction and publication. W5
(ARC-02/03) in the [forward plan](../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)
requires a pure composition boundary and cause-preserving unknowns before SCC composition.
The governed owner is [§9.9](../design/sections/behavioral-analysis.md#section-9-9).

## Options

1. **Keep the SQL complement and add reason overrides afterward.** This is a small patch, but
   two owners would still decide whether a path is open and the analytics transformation could
   not be exercised without a session.
2. **Add a new summary service or provider abstraction.** This separates concerns but introduces
   a lifecycle and contract that no second consumer needs.
3. **Pass typed relations into `lctx-analytics` and return flows, proofs, refusals and coverage
   boundaries together.** Chosen. `cpg-core` acquires the relations and publishes the result;
   one pure producer decides finite outcomes. The existing petgraph SCC schedule remains an
   input, and the bounded BDD kernel remains the condition authority.

## Decision

`lctx-analytics::summaries::finite` composes finite summaries over `FiniteSummaryInputs` with
no session, extraction, embedding, Delta or server dependency. Its `FiniteSummaryOutcome`
contains admitted flows, ordered proof steps, typed refusals and `summary_boundaries` rows.
`summary_boundary_candidates` supplies each same-callable parameter-origin return contribution.
A unique candidate's specific producer cause replaces the generic open-call/control fallback;
a second origin sharing that persisted key remains open independently. A positive does not
erase an explicit refusal on a distinct candidate path. The publisher and shared validator use
the same outcome, with the validator reconstructing it once.

The append-only `boundary_reason` codebook gains `summary_depth_limit`,
`condition_work_limit`, `condition_node_limit` and `condition_atom_limit`. These are unknown
boundaries, never negative transfer claims. A dynamic call with no narrower proof retains
`call_transfer`; `missing_evidence` alone does not erase that more informative fallback.
The persisted boundary key still groups a raw fact and condition, so path-specific origin
identity remains the next semantic migration in the forward plan; this decision does not
pretend to distinguish such siblings in the serving response.

> Decision: ADR-0050

## Consequences

A pure test can challenge an admitted direct identity against a preceding-call control, and a
nine-hop chain reaches the actual depth cap. A focused 2026-09-25 compile/bundle/native fixture
confirmed the cap and control causes survive publication and the native response after rebuilding
the editable Rust extension. The modeled-identity fixture also preserved its dynamic sibling's
`call_transfer` boundary. Integrated Stage 3 tests, the pilot, and path-specific boundary identity
remain pending in the [forward plan](../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition).
