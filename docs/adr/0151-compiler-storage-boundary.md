---
id: ADR-0151
title: Separate validated compiler storage from analyzer orchestration and shared native mechanics
status: accepted
date: 2026-10-11
supersedes: []
superseded-by: null
design: [§6.1, §15.1]
evidence: Interface-checked
revisit: A new native consumer requires a responsibility outside these boundaries, or responsible implementation no longer enters its semantic fingerprint.
---

## Context

The efficiency track's C01/D02 contracts require publication and independent restored-content
admission without analyzer orchestration. The former native package also combined shared physical
installation/control with compiler-store providers. Keeping those dependencies together makes
unrelated presentation and analyzer changes affect consumers and their conservative fingerprints.
The [efficiency coordinator](../plans/efficiency-alignment-plan_2026-10-11.md) owns delivery and
acceptance; original plans retain their separate scope.

## Options

1. Keep the existing crates. This minimizes immediate edits but publication continues to link
   extraction and its provider implementations merely to obtain private admitted capabilities.
2. Feature-gate the existing package. Features can reduce compilation but do not narrow the
   declared normal/build dependency walk used by producer identities. Complicated feature combinations
   also expose an admission boundary that still depends on orchestration.
3. Separate existing responsibilities physically (selected). A lower validated compiler-storage
   owner serves compilation and restored admission; shared native mechanics remain independent.
   This requires migrating actual constructors, manifests and consumers together, but preserves
   one semantic admission implementation and lets publication avoid analyzer dependencies.

The design principles and efficient-architecture heuristics favor matching authority and physical
change propagation to the operation. This is a qualitative choice; no build-time saving is Measured.

## Decision

`lctx-model` owns semantic provider inventories, captures, checked canonical batches, embedding
contracts and resources. Executable providers remain in extraction. `lctx-native` owns installed
physical schema/codec, canonical lowering, control/lifecycle/recovery, transport and shared selected
access whose meaning affects admission or derivation. `lctx-compiler-store` owns native compiler
contributions, immutable completed views, provider adapters, independently validated admission and
private admitted/export/restored capabilities. It has no normal/build dependency on `cpg-core` or
analyzers. `cpg-core` retains acquisition, providers, scheduling and the public completed-input session.
Publisher consumes the lower validated capabilities. Serving-only preparation and presentation stay
in `lctx-serving`; embedding/tokenizer implementation stays in `lctx-embed` over model contracts.

Private construction remains mandatory: moving storage does not turn a caller descriptor into
semantic admission. Compilation and restore invoke the same actual validation rules. A borrowed
read-only planning adapter can preserve the compiler's invocation preparation; lower restored
admission owns its own spillable runtime. No unrestricted session is added to admitted artifacts.

Keep full source and runtime-asset bytes in responsible normal/build fingerprint closures. Native
codec, canonical ingestion, shared selected semantics and derivation remain responsible changes.
Only physically separated presentation changes may leave compiler identity unchanged. Features,
file-name exclusion lists and cached digest labels do not establish that boundary. Retain DataFusion's
Parquet capability exposed through the existing public completed-input session.

## Consequences

Consumers migrate namespaces and direct dependencies together; the old omnibus package has no
compatibility facade. Native unit fixtures may depend on compiler storage as a dev dependency,
which does not establish a normal/build dependency. The physical move does not itself change a
native schema, portable format, service generation or operator selection.

The decision is in force at Interface-checked strength. Semantic extraction, focused builds,
dependency and fingerprint mutation controls, restored admission and integrated consumer acceptance
remain recorded in the efficiency coordinator. No source finding closes merely because this record
is accepted. Future functional changes to the shared selected reader remain semantic dependencies;
this record does not promise that every reader edit is presentation-only.
