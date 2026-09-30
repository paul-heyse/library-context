---
id: ADR-0096
title: Preserve unavailable native signatures without binding them
status: accepted
date: 2026-09-30
supersedes: []
superseded-by: null
design: [§15]
evidence: Implemented
revisit: A provider offers evidence that makes an unavailable signature bindable
---

## Context

The full pinned FastMCP facts pilot exposed an expanded native constructor signature with
repeated `action` and `content` slots. These are provider evidence, but cannot describe an
unambiguous parameter declaration. [DESIGN §15](../design/sections/semantic-model.md) owns
signature interpretation. ADR-0095 retains positional kinds and order for bindable lists;
its duplicate-name refusal remains in force for that form.

## Options

1. **Retain an explicitly unavailable native form.** Keep ordered shapes and displayed annotations,
report partial coverage, and refuse binding this variant.
2. Drop the signature. This discards supported native evidence and its annotations.
3. Deduplicate or rename slots. This invents a callable contract the provider did not state.
4. Refuse the entire facts generation. This prevents publication of independently supported facts
because a provider shape cannot support one downstream interpretation.

## Decision

Choose option 1. Append `NativeUnavailable` to the signature-form codebook. Its ordered members
retain every individually valid shape, including repeated names; normal list grammar still applies
to `List`. The existing parameter-count ceiling applies to both forms. Only invalid list shape
interpretation is converted to this form; resource, operational-limit and infrastructure failures
remain fail-stop errors. Producers emit subject boundaries and partial signature coverage, including
supporting dependency definitions. Unavailable variants never attach their slots as declaration
parameters. Binding returns `OutsideProviderModel` and makes no argument assignment.

## Consequences

Facts publication can retain native evidence without granting unsupported invocation meaning.
No codebook values are renumbered and no native slots are changed. Model roundtrip/binding controls
and the real pilot qualify the form; the cutover plan owns assembled qualification. A future
interpretation requires new evidence and a new explicit form or a justified list reconstruction.
