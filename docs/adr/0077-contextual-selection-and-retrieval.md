---
id: ADR-0077
title: Classify contextual requirements and rank immutable addressable evidence
status: accepted
date: 2026-09-28
supersedes: []
superseded-by: null
design: [§B12, §6.4, §11.2, §14.7, §14.8, §14.10]
evidence: Implemented
---

## Context

PR4 consumes the PR2/PR3 catalog but existing facet selection loses signature context and
operation ranks lose the passage that matched. The product owner requires scoped negative
claims, separate discovery groups and independently addressable evidence. Changing retrieval
rendering or embedding instructions must not require new canonical facts.

## Options

1. **Extend facet SQL and existing operation documents.** Small initially, but duplicates
   quantification between selection and retrieval, loses public aliases and ties retrieval
   changes to canonical publication. Rejected.
2. **A general solver, query language or new cache service.** More expressive but introduces
   unnecessary semantic and lifecycle owners without a selected task. Rejected.
3. **Pure typed classifier and immutable retrieval artifacts.** Selected. Existing catalog,
   generation publication, SQLx, Arrow, bounded BDD and BM25 services retain their responsibilities.

## Decision

`cpg-schema::selection` owns finite requirement evaluation, quantifiers, comparable-claim
conflicts and whole-conjunction contextual compatibility. The pure catalog records typed scoped
domains and closure evidence. PostgreSQL loads validated typed inputs; it does not implement a
second classifier. An immutable request-local prepared selection is shared by retrieval channels
and final assembly, without holding a database lease across embedding.

Typed selection defaults to discovery: supported, unresolved and conflicting groups page
independently. Strict admits only supported candidates. Contradictions are excluded. A clean
universal counterexample is decisive; otherwise relevant comparable-context conflict precedes
support. Complete empty domains are non-vacuously contradicted. Incomplete absence is unresolved.
Behavioral facet semantics remain available as typed predicates; callers migrate to the new selection contract. Current PR3
task receipts cannot establish a demonstrated combination or executed call-site values.

The schema-owned pure retrieval renderer consumes canonical catalog records. Publication and import reconstruct its units to validate text, ownership and anchors; core retains loading, embedding and artifact storage effects. Stable logical units,
rendered fragments and exact-spec vector receipts form immutable generation artifacts. Units
have many-to-many member/release associations and may have no API association. A finite shared
view catalog defines API/options, source, scenario and documentation/deployment families.
Deduplicate identical family/rendered inputs before lexical corpus statistics, retaining all
original occurrences. Collapse each member/family/channel to its best contribution, fuse channels
with RRF K60 inside families, then fuse equal-weight family ranks. Exact-symbol priority is an
explicit primary ordering; deterministic identity ties and both winning channels survive IPC.

Retain Qwen3 standard1024, exact PostgreSQL vectors and BM25S. Change the hashed query instruction
for the enlarged corpus with full-spec invalidation, including briefs. Cross-spec reuse is refused. Remove the obsolete fixed-view ANN implementation; exact is the only current route, and a future ANN engine requires current-content qualification. Use checked SQLx macros for stable reads and retain the validated dynamic
inventory adapter. Existing deployed migrations remain immutable.

## Consequences

Pure known-answer tests can establish classifier and ranking laws independently of effects.
Generation receipts bind canonical inputs, memberships, render/view versions and exact embedding
spec; changing retrieval can produce a new generation over the same snapshot. The larger contract
requires explicit budgets and evidence closure across native/MCP and current-format reconstruction.

This decision is accepted; implementation and qualification remain open in the
[forward plan PR4](../plans/behavioral-model-forward-plan_2026-09-24.md#pr4-execution).
PR5 retains broad rebuild reuse qualification, new browse/journey tools and evidence search.
Revisit if an exposed requirement needs a semantic model beyond the finite vocabulary or if
independent evaluation establishes a benefit from a different retrieval policy.
