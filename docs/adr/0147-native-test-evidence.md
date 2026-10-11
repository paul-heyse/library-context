---
id: ADR-0147
title: Native test evidence remains authoritative behind selective verification access
status: accepted
date: 2026-10-11
supersedes: []
superseded-by: null
design: ["§1.2"]
evidence: Tested
revisit: A qualified native selective reader removes more ownership or decoding machinery than this file-backed projection, or a real producer upgrade changes its guarantees.
---

## Context

Thousands of native Rust/Python observations, interrupted runs and repeated agent investigation
need more precise access than boundary logs alone. The operator authorized the standalone
[test-evidence plan](../plans/agent-test-evidence-and-causal-analysis-plan_2026-10-11.md), based on
source-review F01–F04. Verification already owns selection/outcomes; run, environment, fixture
and storage owners govern supervision and lifetimes. These boundaries remain in force.

## Options

1. **Boundary logs and JUnit alone.** Simplest ownership, but final-session XML loses live phase
   coverage and repeatedly decoding whole outputs makes selected investigation unnecessarily broad.
2. **Native originals with a file-backed selective projection (selected).** Native tools retain
   their meanings. One streaming preparation per generation buys seekable output and stable
   citations with compact metadata. More files and duplicated normalized text require explicit
   lifetimes, but no database/service or new test catalog is necessary.
3. **Native replay alone or a private recording decoder.** Replay retains rich native detail but
   lacks composable selective cross-tool reads. Private decoding couples us to unsupported formats.
4. **A general reporter/telemetry store.** Duplicates producer semantics and adds persistent
   orchestration without a current consumer that justifies it.

## Decision

Native records own captured test results; full logs own their human channel; derived indexes own
access only. Verification sidecars separately describe invocation, source/context observations,
native outcome, capture health, postprocessing and final fixture outcome. Existing run records
and boundary-rerun summaries retain their contracts. No inferred binary-source closure or common
cause follows from matching timestamps, text or revision.

Managed capture composes invocation-owned effective configurations while preserving execution
policy and ordinary available parallelism. Unsupported capture runs the original command and
reports a separate gap. Native recording retention hands off to existing run/storage ownership.
Direct files avoid pipe backpressure; completed prefixes survive interruption without claiming
unfinished output or power-loss durability.

Selective readers hold storage admission, use compact metadata and seek selected payloads.
References bind original generation, invocation and native semantic identity; projection offsets
are rebuildable physical access. Portable readers validate originals and rebuild access products.
Cross-session citations require named existing consumer holds. Export never retires originals.

Exact-case planning uses the current verification dependency declarations, preserves original
selection policy and prints only. Existing `--rerun` remains the boundary rerun operation.

## Consequences

Agents can inspect phases and outputs without rerunning unrelated work, and can distinguish
native failures from incomplete evidence. Producer upgrades require adapter qualification.
Native truncation/normalization, unsupported identities and source-to-binary uncertainty remain
explicit. Reader caches and projection checks detect replaced inputs and accidental corruption;
exported outputs are not advertised as sanitizing arbitrary test text.

This complements ADR-0142 and applies ADR-0144; it replaces neither supervision nor storage
authority. Implementation and focused tooling acceptance belong to the plan's §8. Product
qualification, xdist, power-loss durability and measured agent effectiveness remain separate.
