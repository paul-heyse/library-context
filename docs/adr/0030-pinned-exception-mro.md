---
id: ADR-0030
title: Use pinned Pyrefly MRO for modeled exception class relationships
status: superseded
date: 2026-09-25
supersedes: []
superseded-by: [ADR-0045]
design: [§B5, §3.2, §9.9]
evidence: Tested
revisit: A required exception match uses a class absent from the pinned context, Pyrefly reports a cyclic MRO, or a negative match is needed for a model class that denotes a family of possible subclasses.
---

## Context

ADR-0029 binds a modeled potential exception and a handler type to pinned class definitions.
Class identity proves a same-class or bare-handler candidate, but `except Exception` should
also match the model's `OSError` family. Comparing dotted strings or writing our own exception
hierarchy would introduce a second source of Python class meaning (§B5, §3.2, §9.9; DP-01,
CI-02). The pinned Pyrefly Pysa collector already exposes each retained class's resolved MRO or
a cyclic marker. We need its relationship with source provenance and a visible unknown where it
cannot resolve.

## Options

1. **Keep every different-class handler unknown.** This is sound and simplest, but leaves a
   pinned `OSError` → `Exception` match unanswerable even though the provider resolved it.
2. **Author a builtins exception hierarchy in the model catalog.** This could answer a few
   cases, but duplicates the pinned dependency class authority and would require maintaining
   a parallel identity/migration contract.
3. **Persist Pyrefly's pinned MRO per retained context class.** Chosen. Its ordered ancestor
   pairs are provider facts, with a marker for resolved-empty or cyclic MRO. The handler
   candidate joins the ancestor's `(module, ClassId)` to its pinned `context_definitions` row
   and cites the exact MRO fact. A missing/cyclic/unbound relationship remains unknown.

## Decision

`context_class_mro` is emitted by the pinned Pyrefly collector for each retained dependency
class, including an explicit empty/cyclic marker. The extractor version moves, and a shared
publication rule checks class coverage, dense order, marker shape and child identity. The
compiler may mark a modeled raise's handler as a `pinned_ancestor` candidate only when one
resolved MRO row names that handler's pinned class and carries its fact id. A different class
without such a row stays `class_relation_unknown`, not a nonmatch: an authored model exception
class may denote a family of subclasses. A positive class relation is still conditional on the
modeled target raising and does not prove handler selection, completion or a whole-operation
exception fate.

## Consequences

Source-backed subclass catch candidates become possible without a custom hierarchy. Additional
raw context facts and a versioned schema increase extraction and pilot cost; the Stage 3 pilot
must measure them. Cross-module ancestors that are not retained in `context_definitions` remain
unbound. Pyrefly's class MRO is a static model, not an observation that runtime `__bases__` was
unchanged. Negative class-match claims need a separate exact raised-class contract and cannot
be inferred merely from MRO nonmembership.
