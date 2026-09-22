# library-context — design

**This file is the current truth.** It says what the system *is*; `docs/adr/` says *why*, and
what was rejected. Change a governed section only in the same commit as the ADR that decides it,
and end the section with `> Decision: ADR-NNNN`. Sections are never renumbered: insert `§3.2.1`
rather than shifting `§3.3`. Budget: about 600 lines. Detail beyond that belongs in an ADR, a
table contract in code, or a test.

Every design claim carries a charter §D label (`Proposed`, `Interface-checked`, `Implemented`,
`Tested`, `Measured`, `Formally established`). Unless a section says otherwise, it is **Proposed**:
seeded 2026-09-22 from `docs/initial_plan/Initial_plan.md`, which is source-audited research, not
an implementation. The research input stays unedited; where this file departs from it, an ADR
says so.

---

## §1 Scope

### §1.1 Objective

Stage 1 is the first step towards delivering deep, evidence-backed insight on Python libraries to
coding agents: **a source-preserving, environment-qualified Python semantic graph with structural
types, explicit resolution sets, and analyzer-explanation relationships**, stored as typed fact
tables. Execution dependence (CFG, dataflow) is a separately identified overlay.

### §1.2 Increments

1. **Fact substrate.** Arrow schemas and codebooks, Ruff/Pyrefly adapters, provider-to-canonical
   identity mapping, DataFusion construction plans, validation, Delta publication.
2. **Projections and topology.** Module-dependency and source-call projections, SCCs,
   condensation, bounded reachability, typed result normalization.
3. **Execution semantics.** Python CFG lowering, dominance/postdominance under explicit policies,
   reaching definitions, aliasing, interprocedural summaries.

Each increment ends with a `deep` design review (ADR-0001).

### §1.3 Non-goals for stage 1

A domain-capability ontology ("columnar filtering", "schema evolution"); agent-facing query
surfaces (MCP tools, CLI); runtime probing of libraries; native-extension bodies. Each needs its
own evidence model and is a later stage.

---

## §2 Binding decisions

These are the load-bearing choices. `ADDENDUM.md` maps each to charter principles and gates;
changing one needs an ADR and a `standard` review.

### §B1 Ruff and Pyrefly are the only semantic front ends

Ruff supplies source, tokens, syntax and lexical semantics; Pyrefly supplies types, call
resolution and analyzer explanations. No second parser or type checker. Gaps are closed with
adapters, normalization and new analyses (Initial_plan §6.1).

### §B2 Arrow schemas are the authoritative data contract

One `cpg-schema` crate holds the Arrow `Schema` definitions, append-only category codebooks,
logical-ID newtypes, key and reference declarations, permitted edge endpoint kinds, physical
storage mappings, and Arrow-only batch builders and local validators. It depends on `arrow-*`
only, so adapters can link it; DataFusion validators live in a core-only crate (§8). No schema
is inferred from JSON or from a first batch.

### §B3 DataFusion constructs and validates relations

Most nodes and edges are joins, projections and unions over adapter output. Cross-table
invariants (key uniqueness, foreign references, endpoint kinds) are DataFusion validators, and
the same validators run in tests and before publication (§8).

### §B4 petgraph is used only for topology

SCCs, topological order, reachability and dominators over small, explicitly defined projections
(§5). A relationship does not need a graph algorithm just because it has two endpoints.

### §B5 Python semantics are custom Rust passes

CFG lowering, reaching definitions, alias and effect analysis are custom Rust with stated
abstractions. They are not provided by Arrow, DataFusion or petgraph, and Pyrefly's inference
graph is not relabelled as runtime dataflow.

### §B6 Facts are first-class assertions with provenance

Every assertion is a `facts` row carrying its run, `origin`, `extraction_mode`, `modality`,
`fidelity` and model. Independent assertions from different providers are kept, including
disagreement; they are never collapsed into mutable node properties.

### §B7 Delta persistence behind an immutable snapshot manifest

Fact tables persist as Delta tables grouped by relation family. A snapshot is published by a
manifest naming the exact Delta version of every table, written only after validation passes.
Readers use the manifest and never pick "latest" per table.

### §B8 Pinned adapters run as separate processes and emit Arrow IPC (provisional: ADR-0003 is proposed)

The Ruff and Pyrefly adapters are separate Cargo workspaces and processes, each pinned to one
analyzer revision, and emit Arrow IPC. This isolates fast-moving analyzer internals from the core
dependency family.

> Decision: ADR-0003

### §B9 One pinned Rust dependency family

DataFusion, Arrow/Parquet, object_store and delta-rs resolve to exactly one version each, from
the set in §7.

> Decision: ADR-0002

### §B10 Exclusions

No graph database, embeddings or vector store, generic workflow engine, JSON-inferred canonical
schemas, or whole-ontology petgraph instance. Adding one is an ADR, not an implementation detail.

---

## §3 Fact model

### §3.1 Layers and node kinds

| Layer | Node kinds |
|---|---|
| Source | `Artifact`, `SourceFile`, `Module`, `SyntaxNode`, `Token`, `Trivia` |
| Lexical semantics | `Scope`, `Symbol`, `Binding`, `Reference`, `Import`, `Export` |
| API / object model | `Function`, `Class`, `TypeAlias`, `Member`, `Signature`, `Parameter`, `TypeParameter` |
| Types | `Type` |
| Resolution | `CallSite`, `Argument`, `ResolutionSet`, `DispatchGroup`, `SyntheticCallable`, `ExternalSymbol` |
| Analyzer explanation | `AnalysisBinding` |
| Execution overlay | `ControlGraph`, `ControlPoint`, `MemoryLocation`, `Access` |
| Documentation / diagnostics | `Docstring`, `Diagnostic`, `Fix`, `Edit` |

A variable name is not a binding event, a binding event is not a reference, a type is not a
declaration, and an AST node is not an execution point.

### §3.2 Core tables

`nodes`, `facts`, `edges`, `type_observations`, `resolutions` and `spans`, all keyed by
`snapshot_id`, with dedicated tables for bindings and references, signatures and parameters,
types, classes and members, calls, arguments and resolutions. `record_fields` preserves native
provider detail that has no dedicated column yet, loss-free (missing vs empty, exact numbers,
variant tags, ordered children, back-references). Column-level contracts live in `cpg-schema`,
not here.

### §3.3 Physical profiles

| Logical value | Computation (Arrow) | Delta input | Invariant |
|---|---|---|---|
| Node, fact, run or snapshot ID | `FixedSizeBinary(16)` | `Binary` | exactly 16 bytes |
| Content or schema digest | `FixedSizeBinary(32)` | `Binary` | exactly 32 bytes |
| Closed category | `Int16` | `Int16` | code present in the versioned codebook |
| Offset, ordinal, count | `Int64` | `Int64` | range checks |
| Projection-local dense index | `UInt32` | `Int64` | checked conversion |
| Ordinary text | `Utf8` | `Utf8` | field-specific validation |
| Optional factual flag | nullable `Boolean` | nullable `Boolean` | null (unknown) is distinct from `false` |
| Timestamp | `Timestamp(µs, UTC)` | same | UTC |

Conversion happens only at the declared Delta boundary, with checks in both directions.

### §3.4 Identity rules

- A qualified name is a label, not an identity. Package versions, roots, stubs and
  redeclarations stay distinct; source and stub are linked by `STUB_FOR`.
- A span alone is not a node ID: identity includes the syntax kind and the structural occurrence
  path.
- Provider-local IDs are mapped through `(run, module, provider kind, local key)`.
- Deterministic IDs use a documented, versioned, length-delimited encoding with collision checks.
- Codebooks are append-only. Codes are never regenerated or reordered.
- Source coordinates are byte offsets into the exact UTF-8 parser input, with explicit source
  maps (notebooks, other providers' coordinate systems). Provider spans are converted, never
  assumed to share a coordinate system.
- Type variables keep binder identity: two unrelated parameters named `T` are distinct.
- Deduplicate repeated ingestion of the *same* assertion. Assertions from independent providers
  are separate facts, even when they agree (§B6).

### §3.5 Vocabularies

| Vocabulary | Values |
|---|---|
| `origin` | `input_context`, `source_observation`, `analyzer_assertion`, `derived_analysis`, `synthetic_model` |
| `fidelity` | `raw`, `native_structural`, `normalized_structural`, `report_projection`, `display_only` |
| coverage status | `complete_under_stated_model`, `partial`, `not_requested`, `unavailable`, `failed` |
| type role | `annotation`, `computed`, `expected`, `expected_or_computed`, `narrowed`, `unnarrowed`, `contextual`, `exported`, `decorated_callable`, `undecorated_callable`, `parameter`, `return`, `yield`, `send`, `native_answer` |

Missing output is not negative evidence: unavailable, unrequested, failed and unresolved are
recorded separately.

### §3.5.1 Type observations and class order

`HAS_TYPE` is a derived view over `type_observations` that keeps the role; it is never a
separately editable copy. Annotation-role types come from syntax or native annotation data,
never from TSP `getDeclaredType` (which returns the computed type). Pyrefly's reported MRO order
is kept as reported, never re-derived by topologically sorting base edges.

### §3.6 Resolution is a set

A call's targets form a `ResolutionSet` with a status, `has_unresolved_remainder` and
`candidate_set_complete_under_model`. Pysa `ifCalled` targets become `POTENTIAL_CALL_TARGET` on a
`Reference`, not `CallSite`s, and synthetic shims carry `synthetic_model` origin.

---

## §4 Pipeline

### §4.1 Stages

| Stage | Owner | Output |
|---|---|---|
| A. Source and analysis universe | Rust + Arrow, then DataFusion checks | `source_files`, `source_maps`, `modules`, `contexts`, `producers`, `runs`, `raw_records` |
| B. Typed provider facts | adapters (§B8) + Arrow builders | provider fact batches as Arrow IPC |
| C. Provider-local identity | Rust identity logic + DataFusion joins | `provider_node_map` (unique keys, checked before use) |
| D. Semantic relationships | DataFusion | canonical edges (`AST_CHILD`, `READS_BINDING`, `CALL_TARGET`, `TYPE_COMPONENT`, …) |
| E. Projections and analyses | DataFusion → petgraph → DataFusion (§5) | derived facts carrying lineage |

Unmapped or ambiguous rows become resolution or coverage issues. They are never dropped by an
inner join.

### §4.2 Adapter inputs

| Provider surface | Ingestion |
|---|---|
| Ruff AST + semantic model | native traversal into Arrow builders, after deferred semantic work has completed |
| Pyrefly Glean / Pysa / CinderX reports | decoded once into versioned provider structs, then Arrow |
| Pyrefly native types and answers | native adapter with `Require::Everything` retention |

A report field omitted because it equals the report's own default takes that default; it is
not "unknown". Ruff's `Checker` (which builds the semantic model) is crate-private, so the
Ruff adapter needs a narrow hook at the pinned revision; that assumption belongs to the
analyzer-revision ADR.

The Ruff and Pyrefly revisions are still open; they are the first ADR of increment 1.

---

## §5 Projections and graph analyses

A projection declares its snapshot, node kinds, edge kinds, accepted origins and fidelities,
candidate-target and unknown-target policies, entry/exit policy, and algorithm and model version.
The vertex universe is selected separately from the edges, so isolated vertices survive. Dense
indices are assigned deterministically (sorted canonical IDs) and are temporary coordinates.

The graph is `DiGraph<(), i64>`, holding topology plus an arc index; properties and provenance
stay in Arrow (`projection_nodes`, `projection_arcs`, `projection_arc_facts`). Results such as
`scc_results` and `dominance_results` (with an explicit `reachable` flag) go straight back to
Arrow and are joined to canonical IDs.

Projections are kept separate: module dependency, source invocation, potential call, type
structure, type-inference dependency, execution CFG, value dependence.

---

## §6 Persistence

### §6.1 Tables and manifest

Facts are batched across modules into relation-family Delta tables, never one table per module.
Reads go through Delta's log (table provider or scan), never a raw Parquet directory scan. The
snapshot manifest (§B7) is the only publication boundary.

---

## §7 Pinned dependency family

| Component | Pin | Source of truth |
|---|---|---|
| Rust toolchain | 1.98.1 | `rust-toolchain.toml` |
| DataFusion | =55.1.0 (`sql`, `parquet`) | `Cargo.toml` |
| Arrow / Parquet | =59.3.0 | `Cargo.toml` |
| object_store | =0.13.2 | `Cargo.toml` |
| delta-rs | git `58f07cd62bfbce3649a7e1c87c696288068ae184` (`datafusion`, `rustls`) | `Cargo.toml` |
| delta kernel | `buoyant_kernel` 0.25.1, git `8ba063f8`, branch `buoyant/main` | `Cargo.lock` |
| petgraph | =0.8.3 | `Cargo.toml` |

Verification history is in `docs/pins.md`. The family is **Tested**: the
`cpg-schema::family_smoke` tests (2026-09-22) write Delta and query it through DataFusion, and
`just deps` checks for single versions.

> Decision: ADR-0002

---

## §8 Validation

Two levels. **Local** (Arrow/Rust): exact physical types, nullability, widths, codebook
membership and numeric bounds, checked at every materialization boundary. **Cross-table**
(DataFusion): composite-key uniqueness, foreign references (anti-joins returning zero rows),
endpoint kinds, resolution completeness fields, source anchors and projection integrity. A
snapshot publishes only when both pass. Validators are library code shared by tests and
publication (§B3). They only read and reject; they never repair data.
They live in a core-only crate, so `cpg-schema` stays free of DataFusion.

---

## Revision history

| Date | Change | ADR |
|---|---|---|
| 2026-09-22 | Seeded from Initial_plan.md; §7 family verified by smoke build | ADR-0001, ADR-0002, ADR-0003 |
| 2026-09-22 | Restored increment-1 rules dropped in condensation; placed validators (baseline review F2, F6, O1) | — |
