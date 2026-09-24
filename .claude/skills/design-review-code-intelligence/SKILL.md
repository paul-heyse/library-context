---
name: design-review-code-intelligence
description: Code-intelligence profile for the design-review skill — applies CI-01–CI-13 and gates CI-G1–CI-G3 (attributed facts, typed fidelity with no relabelled relations, relationship identity, unknown-is-not-absent, declared graph projections, claims relative to a stated model, routing analyses by semantics, bounded cardinality, governed heuristics, hermetic analysis inputs, evidence closure for served claims, non-circular evaluation, pinned serving). Use together with design-review whenever the subject is static analysis, fact extraction, code graphs, program or graph analyses, retrieval, or evidence-backed answers served to agents.
allowed-tools: Read, Glob, Grep, Bash, Write, Edit, Agent
---

# Design review — code-intelligence profile

This skill **layers onto `design-review`**; it never replaces it. Load the core skill first.
The core fixes the review's shape (slots 1–12), gates G1–G8, the finding standard and the
decision rules. This profile adds principles, gates, slot content and lenses for
code-intelligence systems.

## The standard

Find the profile through the repository's `standard.toml` (`[[profiles]]` entry
`code-intelligence`). It points to two documents:

| Document | Role |
|---|---|
| Profile principles | CI-01–CI-13 with the core principles each refines, gates CI-G1–CI-G3, false positives, and the functional target the design must serve |
| Profile review additions | What the profile adds to each core slot: fact and fidelity table, analysis record columns, journeys, known-answer shapes, and calibrated finding shapes |

Profile principles tighten core principles; a profile MUST is never waived by an exception
record. The repository binding says which analyzers, stores and libraries fill each role. This
profile names none.

## What the profile adds to the review

1. **Profile gates CI-G1–CI-G3 in slot 6**, settled independently like the core gates.
2. **The fact and fidelity table** (slot 2), reconstructed from the subject. Cells you had to
   invent are unresolved decisions.
3. **Analysis record columns in slot 4** for every projection, analysis or synthesis stage.
4. **Code-intelligence journeys in slot 5**, chosen by relevance. In a target-purpose review the
   workloads in the profile's functional target are in scope even when current plans exclude them.

## Lenses

Fidelity and evidence claims are easy to assert in prose. Trace them through the design or code,
using judgment about where doubt is material:

- **Follow a served claim backwards** (CI-11): answer → synthesis → finding → fact → span, all in
  one snapshot. The step where a citation could resolve elsewhere, or where "mentions" stands in
  for "establishes", is usually the finding.
- **Follow an empty answer** (CI-04): what coverage sits under it, and does the served form say
  "none" or "not known"?
- **Read each relation at its source** (CI-02): what did the provider actually assert, and is it
  consumed under that meaning downstream?
- **Check each projection's universe** (CI-05): is anything filtered before a traversal or a
  global metric that the answer depends on?
- **Find the model** (CI-06): for each behavioural claim, which runtime model and assumptions,
  and are they visible where the claim is served?
- **Trace heuristics to their consumers** (CI-09): may a ranking, community or similarity output
  become a control, limit or behaviour in a served answer?
- **Look for ambient inputs** (CI-10, CI-12): analyzer configuration discovered at run time, and
  any path from evaluation references into inputs or parameters.

A known-answer shape (profile review additions, slot 10) settles a doubtful graph or analysis
claim cheaply; use one where reasoning leaves real doubt.

## Failure modes specific to this profile

- An empty result accepted as evidence that a caller, use or capability does not exist.
- A relation's meaning taken from its name rather than from what its provider asserts.
- A heuristic score or community accepted as a structural or behavioural fact.
- A behavioural claim judged sound without asking which model it assumes.
- Evidence closure judged from the synthesis code alone, without following a claim to its facts.
