---
id: ADR-0073
title: Own catalog wire contracts in Rust and isolate catalog derivations
status: accepted
date: 2026-09-28
supersedes: []
superseded-by: null
design: [§B2, §B13, §3.4, §4.1, §6.5, §8, §11.3, §13, §14.3, §14.7, §14.9, §14.10, §14.11, §14.13]
evidence: Proposed
revisit: A concrete Python consumer requires generated domain models, the schema-backed Tool cannot preserve a required transport contract, or measured catalog work justifies an incremental or recursive engine behind the selected pure boundary.
---

## Context

PR1 supplies a qualified mandatory catalog with optional behavioral enrichment. PR2–PR5 add
options, scenarios, typed selection and agent packets. The
[catalog library-fit review](../design_review/reviews/design_review_catalog-library-fit_2026-09-28.md)
finds independently authored Rust/Python wire constraints and packet shapes (CLF/F01), catalog
loading entangled with derivation (CLF/F02), and empty scanned-dependency declarations for table
reads (CLF/F03). Expanding those boundaries would repeat decisions and impede isolated testing.

The operator accepts the review's design direction. This record refines ADR-0071/0072 without
replacing their product, identity, preservation or publication decisions. Canonical Arrow/Delta,
SQLx effect ownership, exact retrieval and existing semantic queries remain.

## Options

1. **Retain duplicate declarations and add parity fixtures.** Small interim repair, but each new
   requirement/packet still needs independent semantic edits in Rust and Python.
2. **Rust-owned wire contracts, generated schemas and a thin FastMCP Tool adapter (selected).**
   One definition drives decoding, structured output and advertised schemas; Python retains
   transport/rendering. Costs an explicit schema/version policy and bounded adapter qualification.
3. **Generate Python domain models from the same schema.** Credible when Python manipulates such
   records directly; unnecessary generator/coercion surface for the current transport consumer.
4. **Adopt Salsa/Ascent or replace SQLx now.** Those solve distinct execution/query problems and
   do not resolve wire authority. A pure catalog boundary and existing SQLx features meet the
   immediate scenarios with fewer lifecycles. Engines remain conditional on named consumers.

## Decision

`cpg-schema` owns finite request, requirement, witness and response contracts in a focused Rust
module. Canonical fact schemas remain Arrow-owned; matching flat wire components derive from
existing declarations, while nested envelopes have their own typed composition. Ordinary private
newtypes distinguish public-member, binding, signature, evidence, type, snapshot, generation and
retrieval-unit identities at domain boundaries without changing canonical hashes or stored bytes.
Syntax validation does not replace generation membership or relational validation.

Select Schemars for wire-schema generation, starting from the already locked 1.2.2 candidate and
qualifying its direct derive use. Generate separate input/output schemas with explicit
draft2020-12 settings, tagged variants, closed requests and actual string-ID encodings. Bounded
types share constraint policy with decoding. Rust owns semantic validation; JSON Schema does not
establish evidence closure, completeness or applicability. Select Rust `jsonschema` initially as
a dev dependency for independent conformance, with offline reference resolution and explicit
format policy. Exact new dependency pins/features follow implementation qualification.

The MCP integration uses a small schema-backed FastMCP `Tool`: generated parameters/output schema,
bounded `run`, native Rust decoding and existing async lifetime/error/cancellation contracts.
Custom Tool validation is explicit. Migrate tools in bounded slices, deleting superseded semantic
Pydantic definitions and string-key packet assembly after their consumers move. Keep genuinely
presentation-specific Python types and all compatible existing/spec'd query capabilities.
The new text contract uses a 500-Unicode-scalar limit for bounded selector/filter text, with a
separate encoded-byte budget; migration must explicitly reconcile the current native byte limit.
Other field-specific limits, missing/null and numeric/coercion policies are declared by their types.

`cpg-core::catalog` separates exact fact loading, reusable indexes, pure derivations and subsequent
materialization/embedding/publication. Keep suitable bulk joins in DataFusion and simple finite
closures as indexed worklists. Inputs declare relation membership, missing lookups, coverage,
roots/profile, provider and policy versions. Evidence-bearing outputs retain current snapshot,
source and citation identity; semantic-shape reuse never silently reuses obsolete attribution.

Keep SQLx and extend checked static/file queries as PR4 touches stable reads. Preserve the existing
inventory-driven dynamic hydration adapter and shared Arrow codecs, value binding, budgets and
leases. SQLx codecs stay in the PG adapter. Runtime `query_as` is not compile-time SQL checking.
Do not add Cornucopia, an ORM or `postgres-types` to obtain these capabilities.

Coarse immutable rebuilds remain the first reuse mechanism. Salsa, Ascent, Moka, dense collections,
interning and bitmaps require the named triggers and controls in
[§14.11](../design/sections/api-and-evidence-product.md#section-14-11) and forward-plan §7. Salsa
0.28.2 persistence is a possible disposable-cache experiment, not selected infrastructure or a
reason to upgrade the provider family. Ascent needs a genuine recursive multi-relation consumer.
Use `trybuild` for meaningful cross-module ID/state compile-fail controls; a private wrapper alone
does not justify a new harness. Other validation/builders remain consumer-triggered.

## Consequences

The selected target localizes predicate/packet changes and permits pure catalog fixtures without
acquisition, embeddings or a database. Schema generation still needs independent conformance;
domain validity and source fidelity remain runtime/publication obligations. Optional engines must
show cold/reused equivalence and value over the simpler selected mechanism before adoption.

Implementation is **Proposed**: this acceptance changes design and planning only, installs no
dependency and closes no finding. PR2 establishes the contract/derivation foundation; PR3 consumes
it; PR4 completes selection and migrated packets; PR5 completes tool/rebuild qualification.
[Forward-plan §3.0 and §6.2](../plans/behavioral-model-forward-plan_2026-09-24.md#30-consolidated-execution)
own execution and CLF/F01–F03 status. PR1 qualification, PR0 comparison blocks and retained semantic
obligations remain unchanged. Work uses editable fastdev environments, focused functional checks
during implementation, and formatting/integrated gates only after the authorized functional scope.
