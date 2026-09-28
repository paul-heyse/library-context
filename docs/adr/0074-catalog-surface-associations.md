---
id: ADR-0074
title: Normalize public surfaces and preserve exact configuration associations
status: accepted
date: 2026-09-28
supersedes: []
superseded-by: null
design: [§3.9, §14.3, §14.4, §14.11]
evidence: Proposed
revisit: A selected product task needs an unsupported decorator or recursive association that cannot be expressed by the bounded normalization contract.
---

## Context

PR2 extends the catalog to selected decorators, accessors and configuration fields. Existing
behavioral admission and synthesis classify decorators independently, and declaration metadata
cannot establish runtime value identity. The forward plan owns AP/F02 and CLF/F01–F03.

## Options

1. **Extend each consumer independently.** Small edits, but binding, display and admission drift.
2. **One aspect-specific ordered normalizer and exact association kernel (selected).** Explicit
   inputs permit isolated known answers and retain useful source information under uncertainty.
3. **General decorator execution, heap analysis or a new rule engine.** Much broader semantics
   and lifecycle than the selected product tasks require; retained research remains available.

## Decision

The catalog derives source and effective aspects separately from resolved identities and ordered
observations. Shared normalization feeds catalog packets, behavioral admission and synthesis.
A recognized metadata decorator does not establish body behavior. Unknown wrappers retain source
contracts and named uncertainty. Binding candidates precede preferred/effective selection; ordered
MRO/shadowing never silently collapses conflicting attributed alternatives.

Configuration contracts preserve original syntax, literal defaults and unevaluated factories.
Provider-declared fields and constructor associations are distinct from proven runtime identity.
Exact storage requires a supported constructor, receiver, field and local flow without unknown
conversion, mutation, descriptor effects or escape. A reader link identifies a direct access to
that receiver field; it does not prove the stored value survives intervening mutation. Cross-method
composition therefore remains unknown without an additional temporal witness. Pydantic/attrs
declarations alone do not prove identity; TypedDict fields describe keys. Unsupported cases remain
unknown.

ADR-0073 governs Rust wire contracts and isolated catalog derivations. All current MCP tools and
the capability resource migrate in PR2; future classifier and new-tool semantics remain PR4/PR5.
Schemars also describes existing publication/configuration contracts in their owning modules.
Offline conformance does not replace semantic validity or change canonical bytes. Indexed pure
functions remain production computation; Salsa receives only an isolated evidence experiment,
and Ascent retains the genuine recursive-summary comparison under S4.

## Consequences

A supported decorator or field policy changes one semantic owner. Explicit aspect and provenance
contracts cost more than a single classifier flag, but prevent source usefulness from weakening
behavioral admission. No analyzed source is executed. There is no new cache or inference authority.
Implementation and qualification remain Proposed until the
[forward plan](../plans/behavioral-model-forward-plan_2026-09-24.md) records their actual evidence.
