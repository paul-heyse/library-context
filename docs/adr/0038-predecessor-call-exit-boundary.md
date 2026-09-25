---
id: ADR-0038
title: Withhold direct returns after unproved preceding calls
status: accepted
date: 2026-09-25
supersedes: []
superseded-by: null
design: [§9.9]
evidence: Tested
revisit: A path-sensitive predecessor normal-outcome relation can replace lexical call-order withholding with a cited finite execution proof.
---

## Context

The finite direct-value producer used a reachable return region and an identity value fact
as a positive path. Those facts do not prove that a source call executed before the return
completed normally. A self-call followed by `return value` exposed the gap; the prior SCC-wide
guard withheld it, but also withheld a terminating base return before recursion. DESIGN §9.9
requires a finite path, not merely a syntactically present return (CI-06, CI-07, DP-08).

## Options

1. **Keep all recursive functions unknown.** This is simple and was the preceding safeguard.
   It loses the finite base branch even when that return precedes the recursive call.
2. **Admit every true-region direct return.** This retains more results but the unconditional
   self-call example has no finite completed path. Rejected.
3. **Withhold a direct return with any earlier same-function source call.** Chosen as a
   conservative interim rule. It is source-backed, cheap and retains a return before the
   recursive call. It can also withhold a call in an alternate branch or a prior call that
   has a proved normal outcome.
4. **Full path-sensitive predecessor completion.** Target design: use control-flow and model
   witnesses for every predecessor. It is required to recover the conservative losses and
   settle non-call effects, but is not implemented by this slice.

## Decision

`summary_flow_seeds` admits a direct identity return only when no attributed source call in
the same callable occurs at an earlier byte position outside annotations. The cited
`call_syntax` and return syntax are part of the same pinned source snapshot. This is a
conservative lexical screen, not a full path-sensitive normal-completion proof. Modeled
assignment and local-call producers retain their own explicit call witnesses; recursive
modeled paths still await SCC composition. Withheld raw value paths retain an explicit
`summary_boundaries` unknown row. The shared publication validator regenerates the finite
summaries and boundaries.

Focused analyzed fixtures passed on 2026-09-25 for an unconditional self-call before return,
a nonrecursive earlier call, and a terminating base return before recursion. Integrated
Stage 3 acceptance is `not_run`.

## Consequences

The first finite recursive base path can now be cited without letting a preceding call
silently supply normal completion. Lexical order is intentionally overconservative and does
not settle user-defined operators, attribute access, loops or exceptions. The Stage 3
predecessor-execution relation must replace this screen, not accumulate independent
allowlists beside it.
