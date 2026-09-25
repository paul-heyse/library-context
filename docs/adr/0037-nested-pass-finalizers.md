---
id: ADR-0037
title: Discharge an ordered chain of literal pass finalizers on a pending return
status: accepted
date: 2026-09-25
supersedes: [ADR-0036]
superseded-by: null
design: [§9.9]
evidence: Tested
revisit: A pinned CPython counterexample changes a pending return through pass-only finalizers, or a composed proof needs a non-linear exit witness.
---

## Context

ADR-0036 admitted exactly one `finally: pass` because its nullable status field could cite
only one pass. A return crossing two nested pass-only frames has the same normal completion,
but promoting it without recording both actions would lose the exit proof and path identity.
The Stage 3 finite summary already has ordered typed witness steps (ADR-0034; DESIGN §9.9).
CI-03 and CI-11 require that each source frame be attributed and reconstructible.

## Options

1. **Keep nested frames unknown.** This retains ADR-0036 and needs no code. It loses a
   source-provable normal exit and leaves an unnecessary Stage 3 boundary.
2. **Treat any number of pass-only frames as a boolean safe status.** The row becomes
   positive but cannot explain which frames ran, nor distinguish paths if source frames
   change. Rejected.
3. **Reconstruct ordered pass steps from source syntax.** Chosen. Keep the status row's
   nullable pass field for its original single-frame meaning and place all pass citations in
   ordered `summary_flow_steps` for each admitted finite path.

## Decision

The bounded return-ancestry walk admits a return only if every pending control frame is a
`try` whose entire direct `finalbody` is one literal Ruff `StmtPass`. `with`, a nontrivial
finalizer, or capped ancestry retains an explicit boundary. A source query reconstructs
the pass facts in inner-to-outer execution order and each finite direct or modeled flow
includes every one as a typed proof step. The shared validator regenerates status and
summary rows from source facts, rejecting missing or reordered steps. The one-frame status
fields remain nullable and mean exactly one frame; multi-frame status leaves them null.

This is a local normal-exit proof, not a guarantee that evaluating the return expression
or an earlier call completes. Focused direct and modeled fixtures, an effectful outer
finalizer control, and publication equality passed on 2026-09-25. Integrated Stage 3
acceptance remains `not_run`.

## Consequences

Nested no-op cleanup can now participate in finite value summaries without a new Arrow
schema or proof-step kind. The proof query is another source reconstruction at write and
validation time. Any finalizer that can execute user code still needs an independent
normal-exit witness; this decision does not classify it as safe.
