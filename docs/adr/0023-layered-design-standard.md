---
id: ADR-0023
title: Reviews use a layered design standard: a shared core, a code-intelligence profile and a library-context binding
status: superseded
date: 2026-09-24
supersedes: []
superseded-by: ADR-0040
design: [§2]
evidence: Implemented
revisit: A review finds a principle it cannot apply without a repository-specific reading, or the shared core changes and the copy here has not been refreshed.
---

## Context

ADR-0001 decided that reviews "use the charter verbatim, with a repository-specific
`ADDENDUM.md`", and that findings become an ADR, a test or ast-grep rule, or a Deferred row.
That standard had grown into six documents: the charter (60 principles), the directive, the
template, `REVIEW_REFERENCE.md`, `ADDENDUM.md` and the operator's graph guidelines, which mixed
code-intelligence rules with crate-specific notes. The operator replaced the same charter in
another repository with a condensed, repository-agnostic core that adds library-first principles
and relies on the reviewer's judgment rather than required probes, tests or records, and asked
for the same approach here. This record replaces only ADR-0001's review-standard bullet; the rest
of ADR-0001 stands, and ADR-0001 carries a pointer amendment (the ADR-0004 precedent).

## Options

1. **The simpler alternative: keep the charter and amend `ADDENDUM.md`.** It loses: the charter's
   60 principles stay unused in bulk, it has no library-first or bespoke-code principle, and the
   graph guidelines' domain rules stay mixed with crate notes.
2. **Supersede ADR-0001 with a restated process record.** It loses: five unchanged decisions would
   be restated, six live references retargeted, and ADR-0001's revisit trigger reset, for a change
   to one bullet.
3. **Amend ADR-0001 in place.** Not allowed: amendments are factual corrections, not new decisions.
4. **A focused record for the review standard, with a pointer on ADR-0001** (chosen).

## Decision

- Reviews use the layered standard declared in `docs/design_review/design_principles/standard.toml`:
  - **core** (`core/`): design principles DP-01–DP-24, gates G1–G8, evidence vocabulary §D and
    the review template (change and design tiers, conformance and target purposes, slots 1–12),
    carried **verbatim** from the shared text and never edited here;
  - **code-intelligence profile** (`profiles/code-intelligence/`): CI-01–CI-13 and gates
    CI-G1–CI-G3, naming no library;
  - **library-context binding** (`binding/library-context.md`): the successor to `ADDENDUM.md`,
    with the §B map, recurring questions, vocabularies, cadence, where findings land, the graph
    guideline mechanisms, defect shapes in this codebase, known conflicts and lineage.
- The `design-review` skill is the shared core skill; `design-review-code-intelligence` layers the
  profile onto it; the `design-reviewer` subagent loads both.
- Findings that are acted on still land as an ADR, a test or `rules/` entry, or a Deferred row.
  **Naming a check in every finding is no longer required**: reviewers and implementers use
  judgment about where a test, rule or probe pays for itself (binding §4).
- The review cadence is unchanged (ADR-0001, ADR-0021).
- The six superseded documents stay in place with a banner so earlier citations resolve; the
  binding's §8 and core principles §I map every old ID and section.

## Consequences

Reviews cite `DP-nn`, `CI-nn` and gates G1–G8 and CI-G1–CI-G3; the charter's weighted dimensions
are retired. Accepted ADRs and earlier reviews keep their `DM-nn` and `ADDENDUM §n` citations,
read through the lineage tables. The core is a copy: a change to the shared core is copied in
whole, and nothing checks that the copies match. No new check was added; `just lint-agents`
knows the new process skill.
