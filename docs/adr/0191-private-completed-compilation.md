---
id: ADR-0191
title: Private completed sources own computation and native admission owns requested outputs
status: accepted
date: 2026-10-18
supersedes: []
superseded-by: null
design: ["§B2", "§B3", "§B7", "§B12", "§6.1", "§15.1", "§15.11"]
evidence: Implemented
revisit: A private consumer requires durable intermediate restart, or a supported source cannot retain exact typed membership and validity without native authority.
---

## Context

The execution-speed review F01/F03/F04/F05/F11 identifies mandatory remote handoff, repeated
preparation and broad exclusion in ordinary compilation. The operator accepted RC01–RC09 on
2026-10-18 and authorized SC00–SC04 with the native slices needed for complete ordinary output.
The [coordinator §6](../plans/catalog-execution-speed-plan_2026-10-18.md#6-sole-disposition-and-current-checkpoint)
owns implementation and evidence. This decision changes the compiler-private realization;
existing native publication, receiving, protection and committed-effect contracts survive.

## Options

1. Improve persisted transport throughout. This preserves current intermediate recovery but
   retains native allocation, ingress and binding effects without an independent consumer.
2. Keep everything in RAM. This is simple for small sources but cannot honor explicit resource
   refusal and useful borrowers when the combined working set exceeds available capacity.
3. Own immutable typed sources privately, with contingency spill and native materialization at
   requested outputs and genuine retained-product effects (selected). This removes remote
   handoff while retaining exact source meaning and real native recovery where effects occur.

## Decision

The compiler owns private operation identity, cancellation, accepted work and immutable completed
sources independently of native target allocation. Model declarations own typed records, canonical
identity, producer/profile/configuration, ordered dependency roles and epochs, coverage and outcomes.
Logical completion metadata never grants installed-view, pin, persistence or publication authority.
Private computed content has one immutable owner and may be represented by shared resident Arrow
backing or sealed indexed IPC. A view or slice retains the full actual backing reservation.

Execution is memory-first using the existing shared64GiB default and normal available CPU parallelism.
Spill is a contingency, not a required stage handoff or target workload. Release dead backing before
adding copies; admit encoding/read/index workspace and preserve explicit refusal under genuine
pressure. Do not add stage memory pools, thread/job caps, all-RAM assumptions or a general backend
framework. Complete producer sets become visible atomically after pure preparation and exact
predecessor checks; private preparation may rebase, accepted uncertain native effects may not.

Pure semantic assurance uses the existing model-owned opaque fixed driver and exact protected
premises, never a deserialized eligibility definition or matching digest. Early model-owned demand
selects sufficient declarations before rich preparation without reducing supported fact families,
authored configurations, exact domains, unknown coverage or NotRequested evidence. Compatible
preparation may share exact retained source inputs; distinct preparations perform I/O independently.
Accepted initialization retains its terminal result and joins even if every waiter disappears.

Every requested Facts/Normalized/Analysis/Catalog artifact retains genuine native canonical facts,
originals, graph/evidence closure, actual ownership, physical reconciliation and admission. Allocate
its actual native attempt when materialization starts. Preserve each required contributor and exact
dependency identity independently of payload coalescing. Ordinary publication consumes the genuine
admitted artifact and commits an unselected immutable manifest; export remains explicit product
transport with fresh independent incoming admission. No mandatory self-export/import test setup.

Private uncommitted computation may rerun after interruption. Accepted native reads/cache/effects
retain their existing authenticated owners, recovery, pins, uncertainty and real drainage. Record
committed publication identity before later acknowledgement/finalization can fail. No private
metadata creates a native capability; no failed seal can be reset by row restoration.

Logical content is defined by model-owned canonical typed rows, independently of native physical
envelopes. Native completion and receiving reconstruct that logical content from actual stored rows.
The cutover uses completed-state format5, contribution identity`compiler-completed-contribution/v3`,
logical source identity`lctx-completed-logical-source/v2` and native schema11. Physical membership
hashes retain their storage role. Regenerate affected owned data; do not reinterpret old receipts
or add an old-format reader.

Private semantic bindings retain sealed immutable sources. Their executed property identities
include actual content/count as well as logical roles and declarations. Contributor-dependent
compact qualification compares actual singleton outputs and canonical unions before granting its
semantic result. Compiler-exposed providers/plans/streams retain the operation owner and honor
cancellation; underlying immutable backing may remain retained by its actual borrowers.

## Consequences

Required typed buffers move without a remote lifecycle at every private hop. Exact closure,
canonical duplicate checks, resources, spill and accepted-child drainage remain implementation
obligations, not assumed library properties. The earlier comparative target review supplies the
preimplementation architectural review; the integrated private-compiler review accepts the inspected
corrections at Implemented strength. Functional verification and required shared-contract qualification
remain with coordinator §6. This decision does not itself establish Tested or Measured acceptance.

Use one task-owned native campaign for affected native controls, preserving the shared default,
protected Unknowns and operator state. Broader native optimization/regeneration, full-library benefit
measurement and old-plan reconciliation remain separately scoped. No legacy reader, compatibility
runtime or historical data preservation is required solely for this design phase. Earlier native
rules remain in force for actual persisted/incoming/published consumers; this decision replaces only
the compiler-private authority, preparation and durability clauses identified above.
