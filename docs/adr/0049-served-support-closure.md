---
id: ADR-0049
title: Serve cited finding and witness closure in each generation
status: accepted
date: 2026-09-25
supersedes: []
superseded-by: null
design: [§6.4, §10, §11.3]
evidence: Tested
revisit: A current generation exceeds the bounded cited-support projection, or a served finding needs source resolution beyond the declared witness and fact identities.
---

## Context

The [W3 finding](../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)
shows a `coordinates` assertion whose only support is a finding. The FORMAT 7 generation kept its
finding id and kind but omitted the finding's invocation, model, witness and source span. The
FastMCP structured result could name the finding but not resolve it; Markdown dropped even that
edge. Canonical Delta already holds the chain. §B12 makes the immutable generation the serving
projection, and §B13 keeps Delta and compiler code out of the server.

## Options

1. **Show only the existing support ids and report them unavailable.** This repairs Markdown's
   immediate omission but leaves the finding-only claim untraceable to its model or source.
2. **Query Delta or bundle every finding at serve time.** A Delta reader would break the one pinned
   generation boundary; exporting the full findings catalog would enlarge every generation even
   though only cited findings have a serving consumer.
3. **Export the closure of cited findings.** Three bounded, sorted IPC projections retain each
   cited finding and its analytic invocation/model, ordered witnesses with source spans, and
   members with cited fact identity. The server hydrates one canonical support object and both
   renderings consume it. This preserves the current authority boundary and makes support
   resolution independently testable. Choose this option.

## Decision

`cpg-schema::bundle::files` owns the FORMAT 8 serving schemas. `cpg-core::bundle` selects only
findings cited by served brief assertions from the published snapshot, with their invocation,
witness and member rows. It rejects a projection over its declared row cap rather than silently
truncating it. Witness source spans resolve through call, syntax or declaration sites; no
temporary graph index becomes a semantic id. `lctx_mcp.generation` checks the edges on load and
refuses absent cited findings, direct evidence, cited facts or witness source spans. The
structured `Support.finding` and Markdown assertion lines render the same hydrated closure;
fact-only and unavailable source resolution are explicit. The canonical Delta tables remain the
authority. An older FORMAT 7 generation is rebuilt from pinned inputs, not read historically.

## Consequences

The coordinates fixture now follows a finding-only claim to its invocation model and source
span in a real generated bundle; a removed finding row is refused, and a rebuild is byte-identical
(focused tests, 2026-09-25). The projection adds three small serving files and a FORMAT bump;
it does not add a second evidence framework or a server-side Delta dependency. Some finding
members name only a fact identity without a generic source span, so the response says `fact_only`
or `unavailable` where no witness span exists. Integrated Stage 3 qualification and the fresh
pilot remain pending under the plan; this ADR alone does not close W3.
