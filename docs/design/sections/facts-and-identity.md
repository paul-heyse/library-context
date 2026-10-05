# Facts and identity

**Native facts implemented; graph-native compiler acceptance in progress, 2026-10-05.**
`lctx-model::domain` owns nominal identities, provider-qualified assertions, supports and coverage.
`cpg-core` compiles completed private streams without a store. [§15](semantic-model.md) owns the
shared graph contract; the [graph-native coordinator](../../plans/graph-native-pivot-plan_2026-10-05.md)
owns acceptance. Provider objects are transient. Native publication/serving and real-library/operator
acceptance are subsequent work. Earlier PostgreSQL receipts do not establish replacement acceptance.

## §3 Fact model

Column-level contracts, references and invariants derive from executable declarations. Fact
production and semantic interpretation are separate owners: normalized, analytic and product
relations cannot become native facts by relabeling their support.

### §3.1 Layers and node kinds

The model distinguishes captured artifacts/modules/occurrences; lexical scopes, bindings and
references; provider symbols/signatures/parameters/types; call sites/arguments/target alternatives;
original documents/deployment evidence; and native Flow observations. Normalization introduces
semantic entities, places and projections without replacing the provider's assertions.

A variable name is not a binding event; a binding event is not a reference. A type is not a
declaration. A declaration's evaluation owner can differ from its callable body owner. [§15.1](semantic-model.md#section-15-1)
and [§15.4](semantic-model.md#section-15-4) define these distinctions. Execution overlays require a
named consumer and remain Proposed unless implemented by a declared owner.

> Decision: ADR-0085, ADR-0086

### §3.2 Fact families and authority

Native acquisition/extraction supplies Artifacts, Exports, Signatures, Calls, Syntax, Lexical,
Types, Docs, Deployment and, when requested, Flow. `cpg-extract` owns captured pinned input and
provider adaptation; `cpg-flow` reads ty's semantic index over the independent Ruff parse.
Pyrefly retains its embedded Ruff line; the latest independent Ruff owns canonical structural
syntax and native lexical context. Exact byte range, syntax kind and structural parent
correspondence join those observations; provider-local IDs never cross invocations. Migration
is Implemented with scoped tests; the current coordinator records its qualification limits.

Assertions state their original context, scope, condition, modality and approximation. Separate
supports identify provider/run/surface, evidence, origin, extraction mode and fidelity. A precise
source identity does not strengthen a candidate runtime target or a report projection. Policies
admit only the provenance they require; display-only data cannot become behavioral premises.
Provider disagreement and incomplete resolution remain explicit.

Signatures retain ordered variants and complete enumeration membership. Symbol and parameter
links point to their native declaration occurrences; synthesized callables remain distinct from
source bodies. Documentation and extracted examples retain original byte anchors and parent
context. Materialized code is linked to its original block, never substituted for its evidence.

> Decision: ADR-0117, ADR-0086, ADR-0089, ADR-0092, ADR-0015, ADR-0045

### §3.3 Physical profiles

**Implemented / Tested for facts, 2026-09-30.** Catalog is the default profile and does not request
Flow. Behavioral explicitly requests it. `NotRequested`, unavailable, partial and complete-empty
are different outcomes. Facts publication does not select the generation and does not establish
normalized, analysis, catalog or serving readiness merely because those relations exist elsewhere.

Model declarations lower mechanically to internal Arrow and selected graph codecs. Snapshot changes are schema
migrations; no second hand-written column contract or old-format reader is maintained.

> Decision: ADR-0078, ADR-0086

### §3.4 Identity rules

Typed `Id<T>` values distinguish semantic questions and owners. Captured input, context,
provider identity, original source range and the record's declared key fields govern identity.
Content digests also cover payload. Two rows with one identity and different payload refuse;
identical duplicates deduplicate. A generation's lifecycle identifier is not a semantic entity ID.

> Decision: ADR-0117, ADR-0073, ADR-0086

### §3.4.1 ID derivation

Model derives generate key encoding, references, lowerings and codecs. Native symbols/modules
remain provider-qualified; unresolved spelling never equals a resolved definition. Acquired
modules, provider-bundled stubs and namespace packages have distinct identities. Conditions use
canonical Merkle identities rather than library-local BDD indexes. Later derivations name the
exact original premises and invocation; rendering never supplies semantic identity.

> Decision: ADR-0085, ADR-0086, ADR-0089, ADR-0045, ADR-0117

### §3.5 Vocabularies and codebooks

**Implemented, 2026-09-30; Phase 4 epoch qualification in progress.** Shared vocabulary prefixes
are immutable publication authorities. Facts closes the initial prefix. Later declared groups
publish private contributions atomically with their dependent ordinary results; older receipts and
payload meanings remain unchanged. Normalized replay reads the Facts prefix even after analysis
adds qualification/condition vocabulary. [§15.11](semantic-model.md#section-15-11) owns that protocol.

Codes are append-only: retired allocations remain reserved. Producers and validators use the same
model declarations, never test-only replicas. No legacy ID bridge or compatibility vocabulary exists.

> Decision: ADR-0086, ADR-0105, ADR-0108, ADR-0015, ADR-0117

### §3.5.1 Type observations and class order

Native type terms, observations, class traits/ancestry and record-field observations retain their
source/support. Normalized dispatch uses complete declared class membership and MRO evidence;
unknown subclasses stay open. Object receivers and symbolic class-of receivers remain distinct.
A source field or constructor association does not establish allocation, alias or mutation state.
Source/effective default uncertainty is explicit; missing evidence never means an absent default.

### §3.6 Resolution is a set

Targets, signature variants, dispatch alternatives and lexical bindings retain their full admitted
membership. Completeness requires its own evidence. One supported alternative does not discharge
an unresolved sibling, and an empty observed set is not a negative. Normalized policy and call-shape
binding are owned by [§15.5](semantic-model.md#section-15-5); consumers do not infer a second policy.

> Decision: ADR-0117, ADR-0121

### §3.7 Coverage and boundaries

Coverage is scoped to provider/run/context/family and the exact captured input. An independent
expected-domain selector distinguishes absence of observations from absence of an expected
capability. Refusal reasons preserve unsupported syntax, undecodable input, recovered parse,
missing declaration, dynamic access, scope boundary and deterministic resource limits.

Complete-negative claims require the stated complete membership and admitted body/domain. Positive
premises do not silently erase an open set question. The shared verdict and discharge owner is
[§15.8](semantic-model.md#section-15-8), not a presentation layer.

> Decision: ADR-0086, ADR-0089, ADR-0094

### §3.8 Graph catalog and edge registry

**Implemented / focused-Tested for normalized projections, 2026-09-30.** Model-owned projection
declarations derive exact source inventory and stored vertex/arc lineage. Candidate invocation,
source/effective body and boundary decisions are made before graph construction. A selector never
shrinks the graph universe. [§15.10](semantic-model.md#section-15-10) and [§5](storage-and-publication.md#section-5)
own graph hydration and borrowed algorithm access. Published exports and native serving remain subsequent graph-native packages
reconstruction work; retained `cpg-schema` contracts are not current fact authority.

> Decision: ADR-0086, ADR-0100, ADR-0103, ADR-0085
