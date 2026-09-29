---
id: ADR-0085
title: Typed domain definitions own the semantic model
status: accepted
date: 2026-09-29
supersedes: [ADR-0082]
superseded-by: null
design: [§15, §B2, §B6, §B10, §3.8, §3.9, §5, §9.9]
evidence: Proposed
---

## Context

The operator accepted a clean redesign on 2026-09-29 after the phase-0 review returned Revise.
The prior declaration machinery leaves nominal IDs interchangeable, references stringly typed,
identity mappings repeated, and producer ownership duplicated. An empty production registry did
not establish a working model. Source findings are C03–C13 in the cutover-core review and F01–F13
in the semantic-data-model review. Their disposition belongs to the cutover plan §8.

## Options

1. Repair the relation metadata machinery. Smallest edit, but preserves independently authored
   keys, row types, references and semantic operations.
2. SeaORM entity-first authoring. Generates tables and foreign keys, but leaves evidence, coverage,
   identity and operation semantics in a separate contract mechanism.
3. Typed Rust domain definitions with mechanical lowerings (chosen). A bounded derive owns the
   repetitive representations; SQL, serialization and compute remain library capabilities.

## Decision

Ordinary Rust structs/enums in `lctx-model` own units, attributes, relationships, keys and invariants.
A small `lctx-model-macros` derive produces nominal identifiers, typed field references, keys and
metadata. One typed root manifest defines membership. Only a privately constructed `ValidatedModel`
can create a storage or execution specification. Unsupported shapes fail explicitly.

Entity identity, qualified assertion identity and supporting-evidence identity are distinct.
One declared key defines hashing and equality; payload digests still detect attribute changes.
A mergeable conclusion retains its ID when conditions widen. Conflicting same-key payloads fail
unless an explicit owned merge rule applies. Support multiplicity never implies stronger evidence.

Reference targets, subtype requirements, provenance and key participation are orthogonal properties.
Tagged alternatives require valid discriminants, active payloads and absent inactive payloads.
Relationships are first-class records. Derivation premise targets follow typed references.

Generated forms include Arrow schemas, storage-row codecs, PostgreSQL constraints, inventories,
model descriptions and typed operation ports. `serde_arrow` 0.15.1 uses explicit Arrow-59 schemas;
SeaQuery 1.0.2 renders SQL in the PostgreSQL owner. SQLx/pgpq retain effects; DataFusion retains
relational compute. No ORM, application Salsa database or universal inference engine is introduced.
Salsa remains pinned inside ty. BDD conditions and bounded operations retain their existing owner.

A stage's typed outputs alone declare producer ownership. Provider assembly preserves observations;
normalization alone owns agreement, uniqueness and completeness. Coverage and capability availability
are explicit and generation-specific. Meaningful absence requires the appropriate coverage.

All unaffected ADR-0082 clauses carry forward: evidence fidelity, occurrence-based atom identity,
one binder and call-policy owner, bounded conditions, explicit obligations, pure analytics,
role-generated navigation, and programmatic synthesis. The corrected contracts in plan P0.4 govern
transfer keys/composition, receivers, control influences and verdicts. Later layers remain unimplemented.

## Consequences

The same model change reaches every representation mechanically. Building a correct bounded derive
and qualifying its lowerings is real work; names or metadata alone are not acceptance. The plan
requires production declarations, negative controls and a real PostgreSQL round trip in phase 0.
Acceptance is a decision, not closure evidence. See the active cutover plan for implementation status.
