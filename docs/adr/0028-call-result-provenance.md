---
id: ADR-0028
title: Retain nested call provenance for value transfers
status: proposed
date: 2026-09-25
supersedes: []
superseded-by: null
design: [§B5, §3.9, §9.9]
evidence: Proposed
revisit: A focused nested-call fixture cannot map every emitted call step to one pinned source call and argument role, or the Stage 3 pilot exceeds its recorded flow/summary budget.
---

## Context

Stage 2's `flow_values.through_call` and `value_flows.through_call` say a use lies inside a call whose result *may* carry it. They name no producing call. The Stage 3 model compiler now binds `typing.cast` and other authored transfers to exact call sites and arguments, but a summary cannot join those model rows to a `return f(x)` flow using the boolean. `return g(f(x))` requires two ordered transfers, and a callee-name use is not an input argument. Rewalking Python text in the summary executor would create another source interpreter and discard the producer's condition/provenance. This is a DP-02, DP-08 and CI-02 fidelity boundary in DESIGN §B5, §3.9 and §9.9.

## Options

1. **Keep the boolean and decline call-transfer discharge.** The simplest implementation is soundly unknown, but it cannot meet Stage 3's registered `call_transfer` objective or compose nested modeled calls.
2. **Store one enclosing call per use.** This supports `f(x)` but drops the outer `g` in `g(f(x))`, and a call-site-only key cannot distinguish the callee expression from an argument.
3. **Emit an ordered call path with operand roles at the flow producer, then bind its spans to Ruff call/argument facts.** Chosen. The source AST already exists in `cpg-flow`; each path step retains outer-to-inner order, call span and whether the use enters by callee or argument. `arguments` retains the argument value span separately from its role span. Each source path is joined only by an exact unique byte-range match in the same module/snapshot. An absent or ambiguous match is a boundary, not a transfer.

## Decision

The flow provider keeps `through_call` as a derived compatibility flag but makes an ordered call path the authority for *which* calls a value crosses. A raw flow-call-step row cites its parent `flow_values` fact and records step ordinal, enclosing call span and operand role/value span. The semantic bridge resolves each step to one `call_syntax` and, for an argument role, one `arguments` fact using the persisted argument value span. It validates the complete ordered path before L3 can apply a model summary. A callee role, computed argument, starred shape, unresolved target, missing link or budget cut stays unknown unless a later model proves its semantics. No source-text parser runs in L3 or serving.

`flow_values` remains the provider's use-to-sink observation; model application, source argument binding and composed summary are separate relations. The `through_call` boolean alone never upgrades a verdict. The path's identity is its parent fact and ordinal, not a library-local AST index. A call span is a join coordinate, not a semantic call identity until the unique source fact is cited.

The analysis seam retains one `value_flow_contributions` row per raw fact and source
origin before `value_flows` merges paths. A local call path joins by that exact raw
fact id. A call inherited through an earlier reaching definition is marked
separately and must follow that definition's own raw fact; it cannot borrow the
current fact's call links. This preserves provenance without treating a merged
sink span as a unique transfer witness.
The row also separates the transfer accumulated before the fact's use from
the transfer in the local fact. Even if both cross calls, only the local
call steps belong to this fact; a direct one-call discharge requires an
identity upstream path or a proved predecessor summary.
The contribution also names the sink callable separately from the source
parameter's owner. A nested function can read a captured outer parameter;
summaries belong to the inner callable even though the source identity belongs
to the outer one.

The first L3 bridge admits only a direct return whose raw value fact has exactly
one ordered call step, an exact Ruff argument link, an unchanged upstream
parameter value owned by the same callable, and a pinned model transfer whose
input argument and output call expression are those same source nodes. It
retains flow approximation, condition and both modalities. An assigned
intermediate or nested call is withheld until its predecessor path can be
proved. This is a candidate transfer path, not a completed summary or verdict.

For an inherited call, the next bridge joins the successor raw fact's use to
each cited `flow_reaching` definition, that definition's value span to earlier
raw `flow_values` facts, and the same source parameter's unmerged contribution.
It retains the reaching fact, loop/approximation flags and all three condition
ids separately. These are **predecessor candidates**: the derivation does not
assert that the edge's conditions are compatible, that it is the only reaching
definition, or that the earlier call result reaches the final return. The
composed condition ids in `value_flow_contributions` are analysis identities,
not necessarily rows in the provider `conditions` table. L3 must persist and
hydrate their structural BDD closures before condition composition.

## Consequences

A nested modeled call can be composed in order with exact input/output citations; unsupported chains remain diagnosable. This adds a raw fact family and an exact-join validator, and requires an explicit extractor/schema migration. Focused direct, nested, callee, computed and keyword cases precede summary use; the Stage 3 pilot later measures row counts and work. Revisit if exact joins prove too incomplete or costly, but do not replace unknown with a textual heuristic.

The contribution relation and its full reconstruction validator are implemented
and focused-tested on 2026-09-25. The direct candidate and predecessor bridges
are likewise focused-tested. Completed transfer discharge and SCC composition are still
proposed; the integrated pilot has not run on this change.
