---
id: ADR-0036
title: Discharge a sole literal pass finalizer on a pending return
status: superseded
date: 2026-09-25
supersedes: []
superseded-by: ADR-0037
design: [§9.9]
evidence: Tested
revisit: A CPython 3.14.7 counterexample shows a literal pass suite changes a pending return, or the pinned Ruff syntax no longer identifies its sole finalbody child.
---

## Context

`return_exit_statuses` withheld every return under a `finally` (§9.9), even a suite containing
only the literal `pass` statement. Python executes the suite on the way out of a pending
return, and a literal `pass` cannot replace it. The first Stage 3 L2 completion case can
admit this shape, but it needs a cited finalizer fact and must not treat a second frame or a
`with` exit as equally harmless (CI-05, CI-07, CI-11).

## Options

1. **Keep every finalizer unknown.** Simpler and sound, but blocks a direct-return identity
   path despite a source-proven no-op exit.
2. **Assume a finalizer without `return` preserves a pending return.** Unsound: it can raise,
   loop, or call user code, and an outer frame can still alter completion.
3. **Admit exactly one literal `finally: pass` frame.** Chosen. Every other pending frame
   stays unknown until it has its own ordered exit proof.

## Decision

The DataFusion return-ancestry relation recognizes a `try` whose entire direct `finalbody`
is one Ruff `StmtPass`. If that is the only pending `with`/`finally` frame and the ancestry
walk is not capped, the status has no control boundary and cites both the frame and pass
syntax facts. Two nested pass finalizers, any other finalizer suite, a `with`, and a capped
ancestry remain `unknown`. The ordinary expression-evaluation and path-condition premises
of each summary producer still apply.

The [Python 3.14 compound-statement reference](https://docs.python.org/3/reference/compound_stmts.html#the-finally-clause)
states that a pending return executes the `finally` clause before leaving. The analyzed
fixture admits the single pass frame, withholds nested pass and nontrivial frames, and the
shared validator rejects a forged pass fact. The two added nullable evidence columns are
a schema migration at compiler output version 66. This narrow decision is **Tested** on
2026-09-25; integrated Stage 3 acceptance remains `not_run`.

## Consequences

Finite direct and modeled-return paths can cite a no-op finalizer without conflating it
with user-defined cleanup. The restriction leaves common assignments and multiple nested
finalizers unknown; expanding them requires an ordered exit witness for each frame, not
just a wider syntax allowlist.
