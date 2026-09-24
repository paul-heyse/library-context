---
name: design-review
description: Review a proposed design document or an existing code scope against a layered design standard — core design principles (DP-01–DP-24, gates G1–G8, covering authority, typing, transformations, reuse, library-first and bespoke-code economy, effects, publication and evidence) plus any domain profiles and repository binding the repository declares in a standard.toml — and produce one evidence-grounded review document. Use for design reviews, architecture or code-quality audits against the principles, library-leverage assessments, and "does this design hold up" questions.
allowed-tools: Read, Glob, Grep, Bash, Write, Edit, Agent
user-invocable: true
model-baseline: claude-5 (2026-08)
---

# Design review

Review a **proposed design** (document), an **existing implementation** (code), or both, and
produce one review document.

**How you conduct the analysis is yours to decide** — where you start, what you read, whether
you delegate breadth. What this skill fixes is the other end: what the review must establish,
what a claim must rest on, and how the result is organized. [REFERENCE.md](REFERENCE.md)
offers lenses and calibration examples that have proven useful; none is a required step.

## Find the standard

The standard is layered (core → domain profiles → repository binding). Locate the repository's
`standard.toml` — the repository's agent instructions name it; otherwise search for it. It
declares:

| Entry | Role |
|---|---|
| `[core]` | The design principles (`DP-nn`, gates `G1`–`G8`, evidence vocabulary §D, placement §C, library consideration §F, false positives §G, exceptions §H) and the review template: tiers, purposes, finding standard, decision rules, slots 1–12 |
| `[[profiles]]` | Domain principles and gates with their own prefix, the review additions they make to template slots, and a companion skill to load alongside this one |
| `[binding]` | Where reviews live, which authorities decide which roles, local policies, commands for evidence, routes for required changes, and known conflicts |

Read the core principles and template in full; read each declared profile and load its skill;
read the binding. If no `standard.toml` exists, apply the core alone and say so. If the core is
missing, stop and report it — there is nothing to review against.

**Layering.** Profiles and bindings add or tighten; they never relax a core principle
(principles §B). A MUST in any layer cannot be waived by an exception record. A binding's
*known conflicts* table says which authority to follow until a conflict is resolved; state in
the review which you followed and whether it affected the decision.

## Arguments

- **target** (required): a document path, a code scope, or both.
- **tier** (optional): `change` or `design` (template, *Tier*). Infer from scope if omitted.
- **purpose** (optional): `conformance` or `target` (template, *Purpose*). Default from the binding.
- **focus** (optional): pillars, principle IDs or gates to emphasize. Focus shifts depth; it
  never suppresses a MUST-level defect found elsewhere.
- **slug** (optional): file descriptor; infer it if omitted.

## Output

Write one review where the binding says reviews live, following the template's slots and the
profile additions for each slot. Close by telling the user: scope and coverage, gate verdicts,
the top findings in severity order, the decision, and the file path.

## What the review has to establish

1. **Every gate settled on its own evidence** — core G1–G8 and each profile's gates. Pass, fail,
   unresolved or not applicable with a scope reason. Never averaged, never softened by strengths
   elsewhere; unresolved is not a pass.
2. **A verdict per applicable principle** — satisfied, violated or unresolved (template,
   *Principle verdicts*). Resist upgrading an unresolved requirement because the surrounding
   design is good, or downgrading it to a violation for emphasis.
3. **Findings that meet the template's finding standard**, grouped by cause.
4. **Library leverage** (slot 8) for the bespoke generic capabilities that matter in scope,
   at the depth your judgment says they deserve. Delegating breadth to a library-leverage
   reviewer is optional.
5. **A decision that follows the template's decision rules.**
6. **Claims labelled at the strength the evidence supports** (principles §D; template, *What a
   claim can rest on*).

## Judgment calibration

- **Meaning and authority, not vocabulary.** Principles §G lists the hidden defect behind each
  attractive claim; rating a design highly for fluent use of the standard's own words is the
  most common way a review goes wrong.
- **Evidence in proportion to doubt.** Establish facts by the most efficient reliable means —
  reading documentation, source and types, and reasoning from experience. Run a probe or test
  only where material uncertainty remains, and never demand probes, tests or written records
  from the author as proof of diligence when the reasoning is sound.
- **Library first cuts one way for bespoke code and the other for libraries.** Bespoke generic
  machinery where a library clearly fits, with no stated reason, is a finding (DP-13, DP-16). Adopting a library capability
  is never over-construction merely because no consumer exists yet. A specialized domain
  algorithm behind a complete contract is aligned (principles §C).
- **Attack guarantees; don't just summarize them.** Try to break each claimed guarantee: two
  authorities disagreeing; an invalid value reaching an operation that assumes validity; a retry
  double-applying an effect; a partial output indistinguishable from a committed one; a cache
  hit or rewrite changing the answer; an addition, removal or failed lookup leaving a stale
  reused result; an output filter used as an input filter; a limit truncating silently. Record
  guarantees you did not attack as asserted, in the coverage note.
- **Construct the alternatives.** Slot 9's library-owned and simplest rows need real work. When
  one wins, it is usually the headline.
- **Target purpose widens the analysis, not the evidence labels.** In a target review, include
  the workloads the functional target needs even when current plans exclude them, and record
  blocking policy as a required change; keep evidence labels honest.

## Before writing the file

- Every finding has a concrete consequence; otherwise it is a preference.
- Every citation does work in its argument and was read at the grain it is cited at.
- Gate verdicts rest on their own evidence, not on the overall impression.
- No evidence label outruns what was inspected; *Tested* and *Measured* name the test or benchmark.
- The alternatives were constructed, and over-construction was examined as well as
  under-specification.
- The coverage note separates "examined and clean" from "not examined".

## Failure modes

A review with any of these is worse than none: principles enumerated regardless of scope;
alignment asserted from vocabulary; findings with no consequence; bespoke machinery recommended
where a library provides the capability; a library or mechanism recommended because a catalog
lists it rather than because an operation in scope needs it; unverified secondhand evidence;
file or line counts offered as evidence of duplicated meaning.

## Edge cases

| Situation | Handling |
|---|---|
| Target ambiguous (document or code) | Ask; what the review can establish depends on it |
| Scope larger than the tier supports | Narrow to the semantically load-bearing part; state the rule and what was excluded |
| Document describes code that does not exist yet | Document-stage claims only |
| Document and code disagree | A finding against the pair; name which is authoritative |
| A claim cannot be verified with what is available | Record it as asserted, name the settling check in slot 10 |
| Nothing wrong found | Say so, with the coverage statement; do not manufacture findings |
| Mechanical refactor or pure performance change | Say which burdens of proof apply and compress the semantic slots |
