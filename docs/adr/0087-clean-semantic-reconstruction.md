---
id: ADR-0087
title: Reconstruct semantic layers without compatibility paths
status: accepted
date: 2026-09-29
supersedes: [ADR-0084]
superseded-by: null
design: [§15, §1.2]
evidence: Proposed
---

## Context

The operator rejected compatibility machinery on 2026-09-29 and chose a clean staged rebuild.
The old plan's adapters, legacy IDs, schema shim and dual-run parity risk maintaining competing
architectures. Analysis and MCP may remain unavailable until their new layers are implemented.

## Options

1. Compatibility-backed layered migration: keeps the product running but preserves competing models.
2. Rewrite all layers before any bounded exit: delays feedback and obscures fault attribution.
3. Clean reconstruction with independently qualified layers (chosen).

## Decision

The existing cutover plan remains the single execution owner. Phase 0 is reopened for typed domain
contracts; phase 1 installs the PostgreSQL-only runtime; phase 2 publishes typed facts; phases 3–5
reconstruct normalization, analysis/catalog and serving. Product features remain paused until phase 5.

Do not implement adapters, legacy-ID mappings, schema shims, dual writers, fallback readers, bundle
continuity or a deployed legacy binary. Remove obsolete runnable paths; Git retains their source.
Preserve independent fixtures, expected semantics, protected benchmark artifacts and relevant dated
evidence. Old-format equality is not an acceptance condition. New semantics use independent controls.

Code deletion does not retire a capability. The plan's §6 records retained behavioral obligations and
P3–P5 owners. An actual capability-retirement proposal requires explicit disposition and suitable
evidence; an inconclusive historical ablation is not proof of dispensability. No mandatory upfront
ablation campaign is required for P0–P2.

Facts-only generations explicitly record their capability frontier and per-provider coverage.
Successful publication or a behavioral profile never advertises unavailable analyses. Unsupported
higher-layer requests refuse; they do not return empty supported answers.

P0–P2 are one implementation scope for formatting and integrated-gate timing. Use focused compile
checks/tests during execution, a fresh design/target review at P0 exit, then formatting and integrated
gates once all functional scope is complete. Facts-only qualification does not close suspended
analysis/MCP obligations. Fix forward; no rollback architecture is maintained.

## Consequences

Intermediate product availability is intentionally sacrificed for one active architecture. This does
not waive behavioral acceptance or permission to delete unrelated data. Quiesce project readers before
operator state replacement. Implementation and verification remain the active plan's responsibility.
