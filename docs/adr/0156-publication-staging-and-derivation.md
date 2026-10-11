---
id: ADR-0156
title: Publish one held staging root under an exact derivation identity
status: accepted
date: 2026-10-11
supersedes: []
superseded-by: null
design: ["storage-and-publication §6.1", "storage-and-publication §6.2"]
evidence: Proposed
---

## Context

The reconciled efficiency track schedules N05/N08: publication currently transfers each retained
attempt child, while retained manifests need an explicit current derivation identity. A handle hash
alone cannot prevent two lowering implementations from sharing derived rows or selection predicates.
[Native N05/N08](../plans/efficiency-alignment-native-plan_2026-10-11.md#N05) owns the contracts and
[coordinator §4](../plans/efficiency-alignment-plan_2026-10-11.md#4-package-sequence-and-sole-execution-checkpoint)
owns implementation and evidence. ADR-0152 owns the independent completed-content capability;
ADR-0155 owns exact native predicates and bounded ordinary write coordination.

## Options

1. Continue flat child-hold transfer. It uses the existing primitives, but repeats publication work
   proportional to every child and makes interruption progress depend on that fanout.
2. Add only a derivation hash to a publication handle. This is small, but leaves derived addresses,
   unique indexes and eligibility queries able to mix answer-affecting implementations.
3. Retain one durable private staging root, qualify all derived physical meaning by an explicit
   derivation, and atomically retain the winning root at marker publication. This reuses the existing
   bounded native hold/retirement machinery and keeps completed semantic content independent.

FP-07/A4 and H13/H19/H22/H23 favor complete change dependencies and owner-scoped lifecycle progress.
The extra root has a real consumer: the marker can retain the prepared closure through one incoming
edge. It does not replace independent ingress validation, native finality or paged retirement.

## Decision

A native `DerivedStaging` factory consumes an independently earned held frozen completed state.
It checks the exact persisted completion, attempt state/epoch/revision and current physical validity,
then establishes attempt→staging-root→completed-state and staging-root→semantic-admission
reachability before any child effect. Physical completion and semantic admission remain distinct
installation-local witnesses, with their exact incarnations retained by the root.
Its constructor is private; a root ID or deserialized DTO grants no mutation authority. Only derived
families can be written under this capability. Canonical content remains durably frozen. Existing
external endpoints are compared and retained, not created through a post-freeze canonical backdoor.

`DerivationIdentity/v1` binds responsible native/publication implementation, model, physical codecs,
trusted executable definitions, lexical policy and selected vector policy. Production-source closure
capture supplies conservative implementation identities without adding analyzer orchestration to
publisher's normal dependencies. Every derived address, index/uniqueness scope, eligibility query,
reader/resource/continuation, staging root and marker uses the handle's named derivation. Same-format
named realizations coexist only for actual live pinned consumers; no implicit latest selection exists.

Marker publication compares complete semantic/binding/derivation/definition meaning and atomically
retains one complete staging root under the exact publication/retirement guards. Before this
transition, independent reconciliation enumerates the complete expected derived child inventory,
including explicit empty families, and compares it with the root's actual direct holds at checked
EOF. The root predicate revision fences concurrent child incidence through publication. The marker
retains the exact root, completion and admission incarnations; matching semantic hashes alone
cannot substitute another physical realization. Equal publishers
converge on the winning root and its selected physical inventory. A losing private root is durably
nominated before its final attempt hold is released. Terminal cleanup nominates private roots even
for admitted attempts; a published root remains protected by its incoming publication hold. Existing
bounded high-degree retirement and explicit maintenance successors remain authoritative.

The unchanged-publication shortcut independently compares the complete stored marker and current
premises, checks exact reachable installed completions, and acquires an expected-incarnation hold
in the guarded transition before returning. It applies only to ordinary already-admitted completed
content. Explicit export/import and restore retain independent ingress admission and derived
construction. A successful hit performs no derived construction.
A marker committed before a later cleanup/readback failure remains reported as committed.

## Consequences

Publication can transfer one already-owned closure instead of recreating every child hold. This
requires coordinated Loader/reference/search/statistics, selection, backup/restore and retirement
changes. Authentic adjacent format conversion reconstructs destination ownership and earns fresh
admission/completion; it never imports a live source attempt as executable authority.

Source-only implementation on 2026-10-11 includes implementation identities, derived namespace
addressing and selection, private staging, exact derived child reconciliation, marker convergence
and the ordinary shortcut. Exact installed-view/admission factories and backup/restore consumers
remain under integration. These changes are not compiled or activated: the focused Rust compile
encountered ENOSPC, and additional build capacity is pending. Native migration and functional
controls remain open. No speed or whole-package acceptance claim follows from this decision. Actual crash
prefixes, equal-publisher/retirement races, shared-child preservation and two-derivation isolation
remain required by the coordinator. Revisit if an actual supported derived consumer cannot express
its answer-affecting dependencies or retain its exact named realization under this boundary.
