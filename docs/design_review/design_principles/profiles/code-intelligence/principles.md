# Code-intelligence design principles

**Version 1.0 · 2026-09-24** · Domain profile for code-intelligence systems: static analysis
of source code into fact graphs, graph and program analyses over those facts, and
evidence-backed answers served to people and coding agents. Refines the
[core design principles](../../core/design-principles.md) under their layering rules (§B). It
adds and tightens; it never relaxes a core principle. It names no specific analyzer or library —
the repository binding does that.

## Functional target

A best-in-class code-intelligence system turns a pinned body of code (and its documentation,
examples and tests) into provider-attributed facts. It builds explicit graph projections and
program analyses over them, and answers questions — what exists, what calls what, what a
function does with its inputs, what is safe to use together — with claims an agent can trust and
trace to evidence. The workloads a design must serve:

- **extraction** from several analyzers whose coverage and fidelity differ and sometimes disagree;
- **graph and program analysis**: traversal, cycles, dominance, dataflow, summaries and ranking;
- **answering**: search, retrieval and synthesized briefs whose every claim cites evidence;
- **change**: a new library release, analyzer revision or fact family, re-analysed without
  silently changing what earlier answers meant;
- **evaluation** of answer quality against references that must not leak into the inputs.

For this profile, *correctness* in core principles §1 explicitly includes fidelity (a fact means
what its provider established, no more) and evidence closure (every served claim is backed by
facts that exist).

## Principle index

| ID | Principle | Level | Refines | Gate |
|---|---|---|---|---|
| CI-01 | Facts are attributed assertions | MUST | DP-01, DP-05, DP-21 | CI-G1 |
| CI-02 | Fidelity is typed and never relabelled | MUST | DP-02, DP-07 | CI-G1 |
| CI-03 | Relationships have their own identity | MUST | DP-04, DP-07 | G1 |
| CI-04 | Unknown is not absent | MUST | DP-02, DP-12 | CI-G1 |
| CI-05 | Every graph is a declared projection | MUST | DP-07, DP-08 | G6 |
| CI-06 | Claims are relative to a stated model | MUST | DP-08, DP-11, DP-22 | CI-G1 |
| CI-07 | Route analyses by their semantics | SHOULD | DP-13 | G8 |
| CI-08 | Cardinality is bounded; partial is not complete | MUST | DP-12, DP-20 | G5 |
| CI-09 | Heuristic analytics are governed | MUST | DP-07, DP-11, DP-21 | CI-G1 |
| CI-10 | Analysis inputs are pinned and hermetic | MUST | DP-18, DP-21 | G4 |
| CI-11 | Served claims close over their evidence | MUST | DP-21, DP-22 | CI-G2 |
| CI-12 | Evaluation is non-circular | MUST | DP-18, DP-22 | CI-G3 |
| CI-13 | Serving is a pinned, rebuildable projection | MUST | DP-01, DP-09, DP-19 | G5, G6 |

## Principles

### CI-01 — Facts are attributed assertions

**MUST · CI-G1 · refines DP-01, DP-05, DP-21.** A fact is an assertion by a provider, not a
property of the code. Each carries its provider, analyzer revision, configuration, run and
source span. When providers disagree, both assertions are kept and the disagreement is visible;
neither silently overwrites the other. Analysis results and served claims carry the same
provenance and cite the facts they rest on.

**Audit.** Can any fact, finding or served claim be traced to the provider, revision, run and
span behind it? Where two providers disagree, is the disagreement kept?

### CI-02 — Fidelity is typed and never relabelled

**MUST · CI-G1 · refines DP-02, DP-07.** Extracted, resolved, derived, observed, and inferred or
heuristic information stay distinguishable, and so do the kinds of relation each analyzer
produces. One relation is never presented as another: a type-inference dependency is not runtime
dataflow, a possible or conditional call target is not a call, a call edge is not "always
reached", and a structural neighbour is not a recommendation. Lossy representations (a type kept
only as display text, a collapsed candidate set) say so.

**Audit.** Can a consumer tell how each fact was established? Is any relation consumed under a
meaning its provider did not assert?

### CI-03 — Relationships have their own identity

**MUST · G1 · refines DP-04, DP-07.** A relationship is a first-class fact with a persistent
identity, typed endpoints, a kind, evidence and scope. A `(source, target)` pair is not an
identity: separate call sites, arguments or conditions can share endpoints. Nodes exist through
their own relation, so isolates survive, and unresolved or external endpoints are explicit typed
nodes rather than dropped rows.

**Audit.** Do parallel relationships between the same endpoints stay distinct? Does a node with
no relationships still exist? Where does an unresolved target go?

### CI-04 — Unknown is not absent

**MUST · CI-G1 · refines DP-02, DP-12.** Unresolved references, unanalysed modules, analyzer
boundaries and excluded scope are recorded explicitly, with reasons. Coverage is stated per fact
family and scope. An empty result over incomplete coverage is never read or served as absence,
and a partial result never carries the same label as a complete one.

**Audit.** For a query that returns nothing, can the system say whether the answer is "none" or
"not known here"? Is coverage recorded wherever an extractor could stop short?

### CI-05 — Every graph is a declared projection

**MUST · G6 · refines DP-07, DP-08.** A graph is a derived projection with a declared
specification: source snapshot, relation kinds, direction, multiplicity and simplification
policy, weight meaning, scope and completeness. The universe needed to compute an answer is
separate from the selector that chooses what to return — filtering requested symbols must not
remove the intermediate nodes a traversal or global metric needs. Different relations
(containment, calls, dataflow, imports, types, similarity) get separate projections, never an
untyped union. Graph-local indices never become identities.

**Audit.** Does each analysis name its projection and the policies it applies? Could an output
filter change a global result? Are traversal directions declared per relation?

### CI-06 — Claims are relative to a stated model

**MUST · CI-G1 · refines DP-08, DP-11, DP-22.** Exact results (exact over a stated projection),
conservative results (sound under a stated runtime model and its assumptions) and heuristic
results are kept apart and labelled. Behavioural answers carry a verdict that distinguishes at
least established, conditional, refuted under the stated model, unknown, and not analysed. The
model's assumptions — what dynamic behaviour it covers and what it ignores — are declared, so no
answer claims more than its model supports.

**Audit.** What model does each behavioural claim assume, and is it stated? Can a
refuted-under-model answer be read as proof of absence outside the model?

### CI-07 — Route analyses by their semantics

**SHOULD · G8 · refines DP-13.** Fact construction, joins, aggregation and validation are
relational work for a query engine. Reachability, cycles, dominance and ranking are topology for
graph libraries. Reaching definitions, liveness, taint and interprocedural summaries are
fixed-point program analyses with transfer and join semantics — bare reachability is not a sound
dataflow analysis. A bounded relationship pattern can stay a join; a graph is built when topology
is the question.

**Audit.** Is each analysis placed where its semantics fit? Is any program-analysis question
answered by plain reachability?

### CI-08 — Cardinality is bounded; partial is not complete

**MUST · G5 · refines DP-12, DP-20.** Store direct facts. Compute closures, paths and
neighbourhoods on demand and retain compact witnesses, not every path. Do not materialize
all-pairs results, all paths or dense similarity matrices unless an answer requires them and
their cost is bounded. A requested bound ("within three hops") can be a complete answer; an
operational limit that interrupts work produces a partial result, labelled as such, with what
was left uncovered where practical.

**Audit.** Is any closure or path set materialized eagerly? When a budget is hit, is the result
distinguishable from a complete one?

### CI-09 — Heuristic analytics are governed

**MUST · CI-G1 · refines DP-07, DP-11, DP-21.** Communities, centrality, similarity and embeddings
record their method, parameters, projection, seed, and convergence or quality diagnostics — or
record them as unavailable rather than claiming convergence. Their labels are run-local. They
rank, group and suggest; they never on their own establish a control, a limit, a dependency or a
behavioural claim served to an agent.

**Audit.** Can a heuristic output reach a served claim as though it were a fact? Are its
settings recorded so the run can be explained?

### CI-10 — Analysis inputs are pinned and hermetic

**MUST · G4 · refines DP-18, DP-21.** The analysed code, its dependencies and the analyzer
versions are pinned and acquired explicitly. Analyzer configuration is constructed and recorded,
never discovered from ambient files, environment or the analysing project's own setup. Anything
that can change a fact is part of the recorded run context.

**Audit.** Could an ambient file, environment variable or local installation change a fact
without changing the recorded context?

### CI-11 — Served claims close over their evidence

**MUST · CI-G2 · refines DP-21, DP-22.** Every served claim cites facts and findings that exist in
the same snapshot, every symbol or parameter it names exists there, and each claim carries its
evidence status. Synthesis — templated, extractive or generative — is traceable from claim to
evidence. A claim whose evidence cannot be cited is not served, or is served as unknown.

**Audit.** Can a served claim cite anything outside its snapshot, or name a symbol that does not
exist? Is each claim's evidence status visible to the consumer?

### CI-12 — Evaluation is non-circular

**MUST · CI-G3 · refines DP-18, DP-22.** Reference answers, gold catalogs and evaluation sets are
never inputs to extraction, analysis or synthesis, and parameters are not tuned against them.
Success criteria are written before the evaluation runs. An evaluation states what it covers.

**Audit.** Can any evaluation reference reach the system's inputs or influence its parameters?
Were the criteria fixed before the results were seen?

### CI-13 — Serving is a pinned, rebuildable projection

**MUST · G5, G6 · refines DP-01, DP-09, DP-19.** The serving layer (indexes, bundles, search
structures) is derived from one canonical snapshot and can be rebuilt from it; it never becomes
a second authority. A serving process holds one generation for its lifetime, and names the
snapshot it serves. Query-time and index-time representations — embeddings, tokenizers,
ranking features — come from one declared spec, and a cached representation is not reused after
its spec changes.

**Audit.** Can a server mix generations, or serve without naming its snapshot? Can a query vector
and an indexed vector come from different specs?

## Profile gates

These add to core gates G1–G8.

| Gate | Fails when… | Principles |
|---|---|---|
| CI-G1 — Fidelity | A relabelled relation, a heuristic stated as fact, an unknown read as absent, or a claim exceeding its stated model can reach a published result. | CI-01, CI-02, CI-04, CI-06, CI-09 |
| CI-G2 — Evidence closure | A served claim can cite evidence missing from its snapshot, name a symbol that does not exist, or hide its evidence status. | CI-11 |
| CI-G3 — Evaluation integrity | Evaluation references can reach the system's inputs or tune its parameters. | CI-12 |

## Common false positives in code intelligence

| Attractive claim | Hidden defect to check |
|---|---|
| "The call graph is complete." | Unresolved and dynamic calls were dropped instead of recorded. |
| "No caller uses this." | The empty result sits over incomplete coverage. |
| "This analysis is sound." | Sound only for a runtime model nobody stated, or bare reachability standing in for dataflow. |
| "The graph has one edge per relationship." | Parallel call sites were collapsed and their evidence lost. |
| "The ranking finds the important APIs." | A heuristic score is being served as a behavioural claim. |
| "Every claim is grounded." | Citations point to another snapshot, or to evidence that only mentions the symbol. |
| "Evaluation shows it works." | The reference set shaped the inputs or the parameters. |
