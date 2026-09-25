---
id: ADR-0027
title: An unresolved try or context manager withholds a definite raise escape
status: superseded
date: 2026-09-25
supersedes: [ADR-0022]
superseded-by: [ADR-0045]
design: [§3.9, §9.9]
evidence: Tested
revisit: Resolved L2 handler and resource fates can prove a specific raise escapes across an enclosing frame.
---

## Context

ADR-0022 defined a raise guard using handler names parsed from source text and assumed a
context manager other than recognized `suppress(...)` cannot suppress. Stage 3 now records
handler class identity separately from handler matching, and its L2 contract requires unknown
to remain unknown. A shadowed handler name may refer to an arbitrary exception class; an opaque
context manager can suppress in `__exit__`; a `finally` return can replace an exception. The
old shortcut could make such a raise a definite `raises_when` fate or a normal-path guard.

## Options

1. **Keep the text heuristic until all L2 fates are built.** Simpler in code churn, but a
   `with manager:` enclosing `raise ValueError()` can yield a false definite escape. The
   existing handler-name and hardcoded built-in-ancestor lookup also lack lexical identity.
2. **Withhold escape inside any unresolved `try` or `with` body; retain explicit unframed
   raises.** Chosen. This loses some positive guards temporarily but cannot promote an
   unresolved frame into a definite escape.
3. **Implement full exception matching and context-manager effects first.** This is the Stage 3
   destination, but requires source class, path, handler action and model evidence not yet
   present. No text fallback may fill those gaps.

## Decision

The producer sets `raise_sites.escapes` only for an explicit raise outside a `try` or `with`
body in its owning function. Its false value means **unknown**, never caught. The old
source-text parser for handler and raised class names, hardcoded built-in exception ancestry,
and `suppress(` substring exception are removed from this decision. Resolved L2 fates may later
admit narrower positive escape witnesses, with cited class, frame and exit evidence.

All other ADR-0022 decisions remain in force: the stated runtime flow abstraction, places,
closed condition language, verdicts, model catalog, provider boundaries and serving
separation. This ADR supersedes the one old raise-escape clause; it does not revise the other
parts of the behavior model.

## Consequences

Opaque context managers, mismatched handler names and `finally` bodies no longer establish a
guard from their enclosed raise. Existing `raises_when` answers can become unknown until L2
proves the frame's action. A focused fixture checks opaque `with`, `finally`, an unrelated
handler and an unframed raise. The integrated repository gate and pilot remain for Stage 3's
assembled end.
