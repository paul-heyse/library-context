---
id: ADR-0092
title: Own facts roots, supporting scope and native callable identity
status: accepted
date: 2026-09-30
supersedes: []
superseded-by: null
design: [§15]
evidence: Proposed
revisit: A new provider needs a different requested root universe or native callable identity.
---

## Context

The operator accepted the remaining phase 0–2 execution plan on 2026-09-30. Completing call,
type and coverage producers exposes distinctions identified by the A6–A8 review F05/F06 and
P0 exit F05. [DESIGN §15](../design/sections/semantic-model.md) owns these contracts. Capturing a
dependency for import resolution does not request whole-dependency analysis. Native implicit
callables are not ordinary symbols, and named callable types are not identified by bare names.

## Options

1. Let producers choose roots and callables independently. This duplicates meaning and prevents
   exact coverage admission from proving completeness.
2. Analyze the complete captured dependency closure. This changes the product scope, cost and
   claim beyond the requested library and its selected corpus.
3. Use typed acquisition uses and provider-qualified nominal references (chosen). Existing
   model operations and shared validators suffice; no new registry or compatibility layer is needed.

## Decision

- `ArtifactUse` determines requested Python roots (Release, Example, Test, DocBlock) and
  documents (Document). A shared model operation governs producers and coverage admission.
- Signatures have both artifact coverage and Input coverage. Input coverage describes exactly
  the supporting definitions referenced by requested roots: imports/re-exports, call targets and
  nominal type references. It never claims all definitions of captured dependencies.
- Only NotRequested coverage has no provider or invocation. Requested scopes have real providers.
- `ProviderCallable` distinguishes ordinary symbols, module bodies, class bodies and decorator
  applications. Module bodies carry provider/context and module identity. Callers participate in
  shared assertion subject authorization. Legacy SymbolKind codes 6–8 are reserved and rejected.
- Named Callable and Overload type terms reference native Function/Method symbols. Anonymous
  callables remain structural. The transitive type closure validates owner, context and subtype.
  Type-variable values use VariableForm; typing forms are a closed codebook.
- Assembly owns shared vocabulary. Build identity covers the normalized production source
  closure and pinned manifests rather than a producer-maintained list of selected files.
- CLI task receipts are frozen before provider execution and require a declared corpus;
  malformed bytes remain evidence with a failed interpretation and Partial deployment coverage.
  Generation control owns new compile outcomes; retained operation attempts are historical only.

## Consequences

These accepted refinements are Proposed until implemented and focused-tested. The
[cutover plan](../plans/semantic-model-cutover-plan_2026-09-29.md) owns completion, qualification
and finding disposition. They preserve clean reconstruction and do not restore phases 3–5.
