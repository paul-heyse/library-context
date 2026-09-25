---
id: ADR-0031
title: Prove only bounded modeled-exception paths through handlers
status: superseded
date: 2026-09-25
supersedes: []
superseded-by: [ADR-0045]
design: [§9.9]
evidence: Tested
revisit: A nested handler, context manager, or finalizer can be composed with equally cited exit evidence under a bounded path budget, or the Stage 3 pilot shows the direct-only rule misses a registered handler question.
---

## Context

DESIGN §9.9 needs L2 exception fates before a summary can claim that a modeled
raise is converted by an authored handler. A candidate clause, even with a
pinned class match and clause-order proof, does not establish that the modeled
call raises, that an inner frame propagates, or that a handler body completes.
The first source-local completion witness is a sole direct `return None` with
Ruff and ty citations; ty marks its handler region approximate. Promoting
syntactic containment to an operation-level catch would violate DP-08 and
CI-06, while never composing any handler would leave the registered Stage 3
questions entirely unknown.

## Options

1. **Keep candidate handlers and body actions separate.** This is sound and
   simpler, but cannot establish even a conditional path from a modeled raise
   to a simple direct handler return.
2. **Treat a positive class match as a completed catch.** This fills more rows
   but is unsound around preceding clauses, inner `try`/`with`, `finally`,
   computed handler actions and open target dispatch.
3. **Compose only a bounded direct path.** Chosen. Require a complete syntax
   ancestry walk, first positive clause after no prior possible clause, one
   `return None` body action, a direct function-body `try`, no inner
   `try`/`with`, and no `finally` on that frame. Retain source/target model
   modalities, unresolved dispatch and the handler region's approximation.

## Decision

`modeled_exception_return_none_paths` records a **candidate-local conditional
path** for one modeled exception action, not an operation-level catch or
normal-return verdict. It cites the modeled call, raised class, handler class
relationship, sole `return None` action and source/region facts. Absence from
this positive relation is unknown unless the independent walk and candidate
coverage establish a particular negative claim; no consumer may treat it as
proof that the exception escapes. Any inner frame, `with`, finalizer, open
class relation or multi-action/computed handler withholds this path. L3 must
compose target selection, source condition and other exits before serving a
verdict.

## Consequences

The simple `open(path)`/`except OSError: return None` shape has a cited
conditional path while difficult control shapes stay explicit boundaries.
This adds one analysis table, shared reconstruction and focused positive and
withholding cases. It intentionally leaves nested propagation and general
handler completion to a later bounded composition. Revisit at the trigger in
the header, with the final pilot's row counts and timing before widening the
path grammar.
