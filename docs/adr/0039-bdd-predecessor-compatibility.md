---
id: ADR-0039
title: Ignore an earlier call only when its region is BDD-incompatible with the return path
status: superseded
date: 2026-09-25
supersedes: [ADR-0038]
superseded-by: [ADR-0045]
design: [§9.9]
evidence: Tested
revisit: A path-sensitive predecessor normal-outcome relation can replace region compatibility plus lexical order with a cited execution proof.
---

## Context

ADR-0038 withheld every direct return after any earlier same-function source call. This
prevented a false positive after an unconditional recursive call, but a call in the `if`
branch cannot block a return in its incompatible `else` branch. Ty already supplies each
statement's region condition and the Stage 3 bounded BDD kernel owns compatibility. Using
only byte order would discard a demonstrably finite path (DESIGN §9.9; DP-08, CI-06, CI-07).

## Options

1. **Keep the ADR-0038 lexical screen.** Simpler, but permanently withholds a return whose
   condition is disjoint from the earlier call's execution region.
2. **Treat all earlier calls as compatible.** Retains more paths but promotes a return after
   an unconditional nonterminating self-call. Rejected.
3. **Use the existing bounded BDD kernel on attributed call and return conditions.** Chosen.
   The call matters unless its source region and return-value condition have a proved false
   conjunction. Missing, approximate or over-budget condition work withholds the return.

## Decision

DataFusion chooses the narrowest same-function ty statement region containing each Ruff
source call, preserving calls with no region as an unknown candidate. The direct finite
summary producer examines calls at earlier byte positions. It ignores one only when both
regions are present, the call region is not approximated, and their bounded BDD conjunction
is false. Every other earlier call blocks this direct proof, leaving `summary_boundaries`.
This is path incompatibility, not a proof that a compatible call returns normally. Modeled,
assignment and local-call summary producers retain their separate completion premises.

Focused analyzed controls passed on 2026-09-25: an unconditional self-call and a
nonrecursive earlier call withhold a positive flow, while a return before recursion and an
`else` return incompatible with an earlier `if` call are admitted. The shared validator
rebuilds the summary and boundary rows. Integrated Stage 3 acceptance is `not_run`.

## Consequences

The direct producer recovers a finite disjoint branch without a second expression
evaluator. Hydrating call regions and checking bounded BDD conjunction adds finite work to
each summary rebuild. A compatible prior call still needs its own normal-completion proof;
operators, descriptors, exceptional exits and loops remain separate open cases.
