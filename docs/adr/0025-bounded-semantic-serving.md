---
id: ADR-0025
title: Serve bounded semantic queries through an in-process Rust extension
status: proposed
date: 2026-09-24
supersedes: [ADR-0010]
superseded-by: null
design: [§B13, §B14, §11.3]
evidence: Proposed
revisit: Measured pilot query latency or memory exceeds the interactive budget, or the pinned server cannot load a valid native extension.
---

## Context

ADR-0010 and DESIGN §B13 restrict FastMCP to lookups over materialized rows. The operator
rejected that restriction on 2026-09-24: Stage 3 Q09 asks whether a fate is compatible with a
user-supplied condition, and later definitions need implication. Precomputing every possible
query would be unbounded. Python must not independently reimplement the Rust condition semantics.
The file-based generation and one embedding spec remain useful.

## Options

1. **Precompute more combinations.** This is the simplest server change, but it cannot cover an
   open condition parameter. Rejected.
2. **Evaluate conditions in Python.** Packaging is simpler, but this duplicates identity, typed
   theory and budget behavior. Rejected.
3. **Launch a Rust query worker over IPC.** It reuses the kernel but adds a process and protocol
   to a local, single-operator product. Deferred.
4. **Use a pinned in-process Rust/PyO3 extension, chosen.** FastMCP keeps transport, typed inputs
   and retrieval; Rust executes bounded semantic filters over one immutable generation.

## Decision

- A server process pins one generation and validates its manifest, schema digests, native format
  and embedding spec at startup. The extension receives an immutable semantic projection from
  that generation; it has no writes, Delta, compiler or network access. Python may retain pyarrow
  and indexed dictionaries for direct lookups and lexical/vector retrieval.
- A user condition such as `transport == 'sse'` is an **entry-value constraint** anchored by
  operation id and formal parameter id, not a free source-predicate atom. An attributed bridge
  from entry value to each source test site must prove the place/definition mapping and
  effect-stability along the intervening path. Without it, the filter returns `unknown`; it
  never merges atoms by spelling. `compatible` means the condition is **not refuted by the
  stated may-model**. It does not assert an executable path. `find_operations` partitions its
  universe into matched, proven-excluded, source-open and unexamined candidates. An unvisited or
  undecided operation remains in `unknown`, and `complete` is true only when both source-open
  and unexamined sets are empty. An early budget stop returns a generation/query-bound resumable
  cursor and its unexamined count; it never presents a partial scan as exhaustive.
- The `flow_test_value_links` proof relation records operation/formal id, source test/use id
  and span, resolved value place, scope/path, proof origin (`same_evaluation`,
  `direct_parameter_reach_no_effect`, or `modeled_identity_transfer`), cited flow/summary fact
  ids and effect-model digest. Its producer is the Rust flow/summary pass; initially it can
  certify only direct paths without calls or writes, then summaries can add modeled
  identity-preserving transfers. A merely pure transform does not establish value identity.
  The shared publication validator recomputes referenced identities and rejects
  missing/ambiguous bindings, alias uncertainty, an unmodeled call/write, mixed snapshots or a
  changed effect-model digest. Absence or rejection of a link yields `unknown` at serving.
- Rust answers condition compatibility and implication, effect and role filtering, and witness
  traversal as their stages land. Inputs are typed, never arbitrary SQL or code. Every traversal
  has row, node, pair-work and depth budgets. A hit returns explicit `unknown` and `truncated`, never
  a negative or `complete` claim. Cursors bind generation, query and deterministic order.
  Compile and serve use the same Rust condition kernel; Python has no twin evaluator.
- The native package is a separate `python/lctx_semantics` uv workspace member, built by
  `maturin==1.15.0` from a Cargo workspace crate with `pyo3 = 0.29.2`. It exposes
  `lctx_semantics._native` for pinned CPython 3.14.7. The pure-Python `lctx_mcp` package
  depends on that member. Both an editable `uv run` and a built wheel must load it; startup
  rejects extension/kernel format mismatches against the generation manifest.
- Results cite row/node ids and verdicts from the pinned generation. The interface has no hard
  wall-clock guarantee; latency is measured and a sustained breach triggers an architectural
  revisit. The FastMCP layer maps
  domain errors to tool errors. The extension is built and pinned with the uv workspace and
  Cargo lock, with a same-generation integration test.
- ADR-0010's embedding model/spec, cached vectors, view policy, conformance rule, lexical
  degradation, BM25/fusion and exact-symbol promotion continue as DESIGN §B14 and §11.1–§11.2
  state. This supersession changes the executor boundary only.

DESIGN §B13, §B14 and §11.3 are amended in the same commit. A standard design review precedes
acceptance.

## Consequences

Users can ask bounded semantic questions without rebuilding a generation for each predicate.
The native module adds packaging and ABI checks, and a query can legitimately return
`unknown` at a budget boundary. No worker lifecycle or cross-process synchronization is
added. Later semantic features must expose their data through the generation and respect the
same budget and provenance contract.
