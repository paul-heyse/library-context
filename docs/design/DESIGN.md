# library-context — design

**This file is the current truth.** It says what the system *is*. `docs/adr/` says *why*, and what
was rejected. Change a governed section only in the same commit as the ADR that decides it, and
end the section with `> Decision: ADR-NNNN`. Sections are never renumbered: insert `§3.2.1` rather
than shifting `§3.3`. Budget: about 1,200 lines (ADR-0004). Column-level contracts live in
`cpg-schema` and are snapshot-tested; they are not repeated here.

**Labels.** Every claim carries a charter §D label (`Proposed`, `Interface-checked`,
`Implemented`, `Tested`, `Measured`, `Formally established`). A section's label applies unless a
line says otherwise. `Interface-checked` means the named library surface was read at the pinned
version (skill brief, probe or pinned source), not that our code uses it yet.

**Sources.** `Source: IP Lxxxx–Lyyyy` cites `docs/initial_plan/Initial_plan.md`, the research
input, by line, because its top-level section numbers repeat. `docs/initial_plan/DISPOSITION.md`
maps every one of its sections to where it landed here, or why it did not.

---

## §1 Scope

### §1.1 Objective and the v1 promise

**Proposed.** Source: IP L1650–1692, L2135–2143.

The long-term aim is deep, evidence-backed insight into Python libraries for coding agents. Stage
1 delivers a **capability compiler**. For one bounded library subsystem, it compiles current-release
code facts and selected official evidence (docs, examples, tests) into a small set of searchable
**capability briefs**. Each brief states one outcome anchored to a public operation (§10.3).
Agents reach them through two operations:

| Operation | Returns | Does not promise |
|---|---|---|
| `search_capabilities(library, query, limit)` | Published briefs relevant to a task, including ones whose API names differ from its wording | Exhaustive discovery of everything the library can do |
| `get_capability(snapshot_id, capability_id)` | The full brief: public entry point, controls, applicable cases, usage pattern, limits, evidence | A newly synthesized solution for arbitrary combinations of requirements |

All interpretation is precomputed at compile time. Nothing is synthesized, traversed or
generated during a request. The calling agent adapts the brief to its own task.

> Decision: ADR-0004

### §1.2 Increments

**Proposed.** Source: IP L3073–3085, reshaped by ADR-0004.

Each increment is a working vertical slice and ends with the review shown (ADR-0001 mechanics).

| # | Deliverable | Review |
|---|---|---|
| 1 | **One complete path.** Real FastMCP 4.0.3, with one hand-registered seed, `fastmcp.FastMCP.tool` (analytics config, §1.4), plus 2–4 distractor briefs for other public entry points. Families provenance, exports, signatures, calls, coverage, findings, embedding_cache. Pass A → increment-1 assertion kinds (§10.2) → brief → bundle → embeddings → exact-cosine search → hydration → both MCP tools | deep |
| 2 | **Analytics families on synthetic fixtures.** Passes B and C with the `syntax` and `lexical` families; single-layer community detection with seed-consensus stability; `page_rank`; FCA within one community | compact |
| 3 | **Pilot corpus, ~15–25 reviewed briefs** (§10.4 manual review). Docs family; compile-time embeddings for doc links and labels; RCA and extra community layers, each kept only if the §9.8 ablation shows it changes published output; gold scoring (§12) | deep |
| 4 | **Reliable serving.** Generations, lexical + RRF fusion, exact-symbol promotion, degraded lexical-only mode when the embedding service is down | compact |
| 5 | **Agent evaluation.** Held-out tasks, raw-vs-compiled comparison, and the decision on the LLM trigger (§B11) | deep |

> Decision: ADR-0004

### §1.3 Non-goals for stage 1

**Proposed.** Source: IP L2073–2087, L1878–1882.

- Analysis of the caller's own codebase.
- Comparison across library versions.
- Arbitrary composition planning.
- Constraint solving over requirements such as "under 500 MB".
- Query-time graph traversal.
- Native-extension bodies.
- A domain-capability ontology beyond the briefs.
- Python CFG, dataflow and alias analysis (§13).

### §1.4 Pilot, subsystem and gold reference

**Proposed.** Source: IP L1685–1691 (pilot sizing); pilot choice by operator decision, ADR-0004.

- **Library.** FastMCP **4.0.3**, the release the `fastmcp` capability skill captured. Its code
  lives in `fastmcp-slim`; `fastmcp` is a facade distribution, and `fastmcp-tasks` is
  first-party. `mcp` and `mcp-types` are dependencies: they are resolved, but they sit behind
  the analyzed boundary.
- **Subsystem.** The server-components surface. The compiler gets it only from the
  pre-registered **analytics config**, which declares the library's in-scope module prefixes and
  public roots, hand-written from the library's own public API. The config's digest is part of
  `content_digest`. For evaluation, this surface is expected to correspond to gold families
  `fm.register`, `fm.inputs`, `fm.outputs`, `fm.errors`, `fm.resources` and `fm.middleware`
  (about 30 operations). That correspondence is measured (§12), never used as input.
- **Gold reference.** The skill's reviewed capability families are used **only to evaluate**
  (§12). Nothing under `.claude/skills/` is ever a compiler input: acquisition fetches its own
  pinned artifacts, even when identical bytes exist in a skill's cache. Analytics parameters (§9)
  are pre-registered. The gold serves only as the development metric for keeping or removing
  techniques (§9.8), never for tuning parameters.
- **Serving.** The agent interface itself runs on FastMCP 4.0.5, the installed version. The skill
  diff between 4.0.3 and 4.0.5 shows only validation-logging changes (Interface-checked,
  2026-09-22).

> Decision: ADR-0004

### §1.5 Definition of done

**Proposed.** Source: IP L2088–2131.

Stage 1 is done when all three derivation families (delegation, configuration/restriction,
handoff) each have an end-to-end example. Each example must show:
- input facts;
- the named method;
- a structured finding;
- an assertion;
- evidence links;
- a published brief;
- successful retrieval by task wording. The target brief must rank first against the distractor
  briefs for the gold family's `task_aliases`.

In addition:
- the §12 evaluation must have run;
- no brief may reach publication by bypassing the analytics. A brief that is honestly explained
  by documentation alone is allowed, and is labelled that way.

> Decision: ADR-0004

---

## §2 Binding decisions

These are the load-bearing choices. `ADDENDUM.md` maps each one to charter principles and gates.
Changing one needs an ADR and a `standard` review.

### §B1 Ruff and Pyrefly are the only semantic front ends

**Interface-checked.** Source: IP L1–L425, L886–L897.

- **Ruff library crates** (in-process) supply source, tokens and syntax.
- **Pyrefly** (a pinned CLI run as a subprocess, §B8) supplies definitions, signatures, call
  resolution, class order and types, through its reports.
- **Binding history** comes from our own scope-aware recognizer over the Ruff AST (§4.2). Ruff's
  semantic model has no public driver.
- **No second parser or type checker.** Gaps are closed with adapters, normalization and our own
  analyses.

> Decision: ADR-0006

### §B2 Arrow schemas are the authoritative data contract

**Proposed.**

- **What `cpg-schema` holds:**
  - the Arrow `Schema` definitions;
  - append-only category codebooks;
  - logical-ID newtypes;
  - key and reference declarations;
  - the family → node/edge view mapping (§3.2);
  - physical storage mappings;
  - Arrow-only batch builders and local validators.
- **Dependencies:** `arrow-*` only. DataFusion validators live in a core-only crate (§8).
- **No inferred schemas.** No schema is inferred from JSON or from a first batch. That includes
  the serving bundle's schema (§6.4) and the embedding exchange (§11.1).

### §B3 DataFusion constructs and validates relations

**Proposed.**

- Most nodes and edges are joins, projections and unions over extracted facts.
- Cross-table invariants are DataFusion queries, one per rule (§8).
- The same validators run in tests and before publication.

### §B4 Graph algorithms have named owners

**Interface-checked.** Source: IP L1335–1504, L2233–2362.

| Owner | Algorithms |
|---|---|
| petgraph 0.8.3 | Traversal, SCCs, `page_rank`, over immutable, explicitly declared projections (§5) |
| leiden-rs | Community detection (§9.4) |
| Our own code | Formal and relational concept analysis (§9.6) |

- **Each algorithm has a named consumer in the brief** (§9).
- **A relationship does not need a graph algorithm** just because it has two endpoints.

> Decision: ADR-0011

### §B5 Python semantics are custom Rust passes

**Proposed.**

- Python-specific semantics are our own Rust code with stated abstractions: v1 recognizers
  (§9.1–§9.3), and later CFG and dataflow (§13).
- Pyrefly's inference graph is never relabelled as runtime dataflow.

### §B6 Facts are first-class assertions with provenance

**Proposed.**

- Every assertion is a `facts` row carrying its run, `origin`, `extraction_mode`, `modality`,
  `fidelity` and model.
- Independent assertions are kept, including disagreement. They are never collapsed into mutable
  node properties.

### §B7 Delta canonical store, published by a `snapshots` append

**Interface-checked.**

- Fact tables are append-only Delta tables, one per fact family.
- A snapshot becomes visible only through one append to the `snapshots` table, made after
  validation passes.
- Readers resolve table versions through that row and filter by `snapshot_id` (§6).

> Decision: ADR-0009

### §B8 Pyrefly runs as a pinned subprocess; everything else is one workspace

**Interface-checked.**

- **Pyrefly** runs as a pinned CLI with a generated, explicit configuration. Its reports are
  decoded in Rust.
- **Ruff** library crates link in-process.
- **One core Rust workspace** holds extraction, construction, analytics and publication.
  Isolation from Pyrefly's internals comes from the process boundary.

> Decision: ADR-0006

### §B9 One pinned Rust dependency family

**Tested** (§7).

DataFusion, Arrow/Parquet, object_store and delta-rs each resolve to exactly one version, from
the set in §7.

> Decision: ADR-0002

### §B10 Exclusions

**Proposed.**

Excluded in stage 1:
- a graph database;
- a generic workflow engine;
- JSON-inferred canonical schemas;
- a whole-ontology petgraph instance;
- a general composition planner or constraint solver;
- neural reranking;
- graph embeddings.

**Allowed:** text embeddings, and a *rebuildable* search projection of published briefs (§B12).

> Decision: ADR-0005

### §B11 Insight synthesis is programmatic; no LLM in the query path

**Proposed.**

- Assertions come from typed findings through deterministic templates and extractive text
  selection (§10).
- There is no generative model in v1.
- **Adding a generative model** (a local vLLM model) needs an ADR, triggered by the §12 gap
  metric. Even then it runs only at compile time, under mechanical grounding.
- **No generative model ever runs in the query path.**

> Decision: ADR-0005

### §B12 Canonical store vs serving projections

**Proposed.**

- **Authority:**
  - the Delta fact store is authoritative for facts, findings and assertions;
  - the serving bundle (§6.4) is a derived, immutable projection, built only from a published
    snapshot and rebuildable byte-for-byte from Delta;
  - any search index built from the bundle is derived from it in turn.
- **No cross-store transactions:** the bundle manifest names the snapshot it came from.

> Decision: ADR-0009

### §B13 The agent interface is a FastMCP server over file-based generations

**Interface-checked.**

- The Python package `lctx_mcp` serves the two operations (§11.3) from one pinned serving
  generation per process.
- It reads files only: no Delta, no DataFusion, and no compiler code.

> Decision: ADR-0010

### §B14 One embedding spec, cached vectors

**Interface-checked** (the vLLM and Qwen behaviour); **Proposed** (our spec).

- One hashed embedding spec (§11.1) governs every vector.
- Compile-time vectors (Rust client) and query-time vectors (Python client) must both match it,
  checked against shared conformance vectors.
- Vectors are cached by `spec_hash + input_hash` in the canonical `embedding_cache` Delta table.
  Snapshots record the cache version they read, and bundles copy vectors from it. A cached vector
  is a run input.

> Decision: ADR-0010

---

## §3 Fact model

**Proposed** unless marked. Source: IP L427–L746 (ontology and identity), L975–L1143 (Arrow
contract), L1969–L2036 (v1 slice).

### §3.1 Layers and node kinds

The full ontology below is the **target model**. Increments introduce kinds only as their
consumers appear (§3.2).

| Layer | Node kinds |
|---|---|
| Source | `Artifact`, `SourceFile`, `Module`, `SyntaxNode`, `Token`, `Trivia` |
| Lexical semantics | `Scope`, `Symbol`, `Binding`, `Reference`, `Import`, `Export` |
| API / object model | `Function`, `Class`, `TypeAlias`, `Member`, `Signature`, `Parameter`, `TypeParameter` |
| Types | `Type` |
| Resolution | `CallSite`, `Argument`, `ResolutionSet`, `DispatchGroup`, `SyntheticCallable`, `ExternalSymbol` |
| Documentation | `Document`, `Passage`, `Example`, `Docstring` |
| Findings and briefs | `Finding`, `Assertion`, `Brief`, `UsagePattern` |
| Execution overlay (deferred) | `ControlGraph`, `ControlPoint`, `MemoryLocation`, `Access` |

A variable name is not a binding event. A binding event is not a reference. A type is not a
declaration, and an AST node is not an execution point.

### §3.2 Fact families and authority

- **Family tables are authoritative.** Each fact family is a set of typed Arrow tables. The
  family is also:
  - the Delta table group;
  - the coverage unit;
  - the unit an extractor declares it produces.

  "Fact family" and "relation family" are the same term.
- **The family → node/edge mapping.** `cpg-schema` declares how each family maps to node and edge
  kinds: kinds, endpoint kinds, role and ordinal columns. Endpoint-kind validation (§8) consumes
  this mapping.
- **Deferred views.** Generic `nodes`/`edges` views are **not materialized** until a pass reads
  them. Projections (§5) are built directly from family joins.
- **One producer per table.** A table is written by one producer: one extractor surface, or one
  Stage-D relational derivation (§4.1).
- **Merged tables are derivations.** Where two providers contribute to one logical record, each
  writes its own raw table. The merged table is a DataFusion derivation that carries both
  `fact_id`s (`extraction_mode = relational_derivation`).
- **Disagreement is recorded.** When the raw tables disagree, the derivation records a
  `boundaries` row with `provider_disagreement`.

| Family | Tables | First increment |
|---|---|---|
| `provenance` | `releases`, `distributions`, `source_files`, `contexts`, `producers`, `runs`, `facts` | 1 |
| `exports` | raw: `declarations` (Ruff: qualified name, kind, parent, span, docstring text and span, `is_overload`), `export_syntax` (Ruff: `__all__`, import aliases), `public_names` (Pyrefly `coverage report --public-only`). Derived: `exports` (public access path → declaration) | 1 |
| `signatures` | raw: `parameter_syntax` (Ruff: ordinal, name, default text and span, annotation text), `parameter_semantics` (Pysa: kind, required, resolved annotation). Derived: `signatures`, `parameters` | 1 |
| `calls` | raw: `call_syntax` and `arguments` (Ruff: span, owner, ordinal, keyword, starred, expression span), `pysa_calls` (Pysa: candidates, receiver, phase, unresolved reasons). Derived: `call_sites`, `resolutions` (§3.6), `call_targets` (joined on span) | 1 |
| `embedding_cache` | `embedding_cache` (spec_hash, input_hash, vector as `List<Float32>`, model identity). Global and append-only; not snapshot-qualified; the key is unique | 1 |
| `coverage` | `coverage`, `boundaries` (§3.7) | 1 |
| `syntax` | `syntax_nodes` needed by recognizers: branches, raises, assignments, with their structural occurrence paths | 2 |
| `lexical` | `scopes`, `bindings` (with shadowed history), `references` | 2 |
| `types` | `type_observations` (§3.5.1) | 3 |
| `docs` | `documents`, `passages`, `mentions`, `examples` (examples and tests as sources) | 3 |
| `findings` | `findings`, `witnesses`, `evidence` (evidence_id → one of: fact, span, passage, example, fixture run), `assertions`, `assertion_support` (assertion → finding / evidence), `briefs` (with `review_state`), `brief_members`, `usage_patterns` | 1 |

Deferred until a consumer exists: `record_fields`, full type structure, CFG and dataflow tables.

> Decision: ADR-0008

### §3.3 Physical profiles

| Logical value | Computation (Arrow) | Delta | Invariant |
|---|---|---|---|
| Node, fact, run, snapshot ID | `FixedSizeBinary(16)` | `Binary` | exactly 16 bytes |
| Content, schema or spec digest | `FixedSizeBinary(32)` | `Binary` | exactly 32 bytes |
| Closed category | `Int16` | `Int16` | code present in the versioned codebook |
| Offset, ordinal, count | `Int64` | `Int64` | range checks; no unsigned types anywhere |
| Projection-local dense index | `UInt32` | never persisted | temporary coordinate |
| Ordinary text | `Utf8` | `Utf8` (read back as `Utf8View`) | field-specific validation |
| Optional factual flag | nullable `Boolean` | nullable `Boolean` | null (unknown) is distinct from `false` |
| Score or weight | `Float64` | `Float64` | finite |
| Timestamp | `Timestamp(µs, "UTC")` | same | timezone string exactly `"UTC"` |
| Embedding vector | `FixedSizeList<Float32, 2560>` | `List<Float32>` in `embedding_cache` (child renamed `element`) | length 2560, finite, unit norm, checked on read |

**Conversion happens only at the Delta boundary**, checked in both directions.
- **Interface-checked:** FixedSizeBinary is written as generic BINARY and comes back as `Binary`
  (likely as `BinaryView`, because DataFusion forces view types).
- **Read path:** BinaryView → Binary → FixedSizeBinary(16), with `CastOptions { safe: false }`.
- **Timestamps:** µs normalization is lossy for ns, so only µs is ever written.

### §3.4 Identity rules

- **A qualified name is a label, not an identity.** Package versions, roots, stubs and
  redeclarations stay distinct; source and stub are linked by `STUB_FOR`.
- **A span alone is not a node ID.** Identity includes the syntax kind and the structural
  occurrence path.
- **Provider-local IDs** are mapped through `(run, module, provider kind, local key)`.
- **Codebooks are append-only.** Codes are never regenerated or reordered.
- **Source coordinates** are byte offsets into the exact UTF-8 parser input, with explicit source
  maps. Other providers' coordinates are converted, never assumed to match. Pysa's `line:col` is
  converted through a per-file line table.
- **Type variables keep binder identity:** two unrelated parameters named `T` are distinct.
- **Deduplicate repeated ingestion of the *same* assertion only.** Assertions from independent
  providers are separate facts, even when they agree.

### §3.4.1 ID derivation

**Proposed.**

**Encoding.** `BLAKE3("lctx-id/v1" ‖ kind_tag ‖ len‖field ‖ len‖field …)`. Lengths are u64
little-endian. IDs are the first 16 bytes; digests are all 32. A version bump in the tag is a
migration (DM-51).

| ID | Derived from | Scope |
|---|---|---|
| `release_id` | distribution names + versions + artifact sha256s, sorted | global |
| `node_id` | `release_id`, node kind, path within the release, structural occurrence path, span (syntax nodes) or declaration occurrence (semantic entities) | stable across snapshots and runs |
| `context_id` | Python version, platform, ordered search paths, lock digest, config digests | global |
| `producer_id` | tool, tool revision, adapter build digest | global |
| `run_id` | `release_id`, `context_id`, `producer_id`, sorted enabled families, the producer's own config digest | global |
| `fact_id` | `run_id`, record kind, subject id(s), canonical payload bytes | per run |
| `finding_id`, `assertion_id`, `brief_id`, `evidence_id` | kind, subject `node_id`(s), canonical payload. **No config digest**, so an unchanged finding keeps its ID when parameters change; ablation diffs are joins. `capability_id` = `brief_id` | content |
| `snapshot_id` | a fresh random 128-bit value per compile attempt | execution identity (DM-12) |
| `content_digest` | sorted `run_id`s (each carrying its `release_id`), acquisition-manifest digest, compiler build digest, analytics-config digest, embedding spec hash, `embedding_cache` version used | compares reruns |

- **Keys are snapshot-qualified.** Uniqueness is checked on `(snapshot_id, key)`, so an identical
  rerun re-emits the same `node_id` and `fact_id` in a new snapshot without conflict.
- **Collisions are validator failures** (§8), never silently merged.
- **The compiler has its own run.** Analytics, synthesis and publication are a run of producer
  `lctx-compiler`, whose config digest is the analytics config's.
- **Overloads.** A public callable with `@overload` stubs is **one** declaration node (the
  implementation), with one `signatures` row per overload (`is_overload`) plus the implementation
  signature. Seeds and briefs attach to the declaration.

> Decision: ADR-0007

### §3.5 Vocabularies and codebooks

**Codebooks** are append-only `Int16`, versioned in `cpg-schema`.

| Codebook | Values |
|---|---|
| `origin` | input_context, source_observation, analyzer_assertion, derived_analysis, synthetic_model |
| `fidelity` | raw, native_structural, normalized_structural, report_projection, display_only |
| `extraction_mode` | native_traversal, report_decode, relational_derivation, graph_analysis, recognizer, statistical_analysis, template_synthesis, fixture_execution, manual_review |
| `modality` | definite (holds whenever the subject exists, under the model), candidate (one of a set), potential (holds if some condition occurs, e.g. "if called") |
| `coverage_status` | complete_under_stated_model, partial, not_requested, unavailable, failed |
| `resolution_status` | resolved, partial, unresolved, not_attempted |
| `resolution_domain` | call, attribute, import, name, type |
| `invocation_phase` | call, new, init, decorator, property_get, property_set |
| `boundary_reason` | native_unavailable, unresolved_target, unsupported_unpacking, ambiguous_binding, unsupported_control_flow, scope_boundary, budget_reached, missing_evidence, not_requested, provider_disagreement |
| `pysa_unresolved_reason` | the 14 variants of Pysa's unresolved-call reason at the pinned pyrefly, spelled as Pysa spells them; appended when the pin moves |
| `evidence_status` | structurally_observed, documented, statistically_derived, fixture_checked, unresolved |
| `finding_kind`, `assertion_kind`, `analytic_method`, `fact_family`, `type_role` | Defined in `cpg-schema` as their consumers land (§9, §10) |

- **Missing output is not negative evidence.** "Unavailable", "not requested", "failed" and
  "unresolved" are recorded separately.
- **`model_id` is not a codebook.** It is `<producer_id>/<surface>` (e.g. `pysa-json`,
  `coverage-public`, `ruff-ast`, `lctx-compiler/pass-a`), validated against `producers`.
- **`type_role` target values:** annotation, computed, expected, narrowed, unnarrowed, contextual,
  parameter, return, and further values from IP L576–623.

> Decision: ADR-0008

### §3.5.1 Type observations and class order

- `HAS_TYPE` is a derived view over `type_observations` that keeps the role. It is never an
  editable copy.
- **Annotation-role types** come from syntax or native annotation data, never from TSP
  `getDeclaredType` (which returns the computed type).
- **Pyrefly's MRO** is kept exactly as reported. It excludes the class itself and `object`, so it
  is labelled as ancestors, not as a complete runtime MRO. It is never re-derived by
  topologically sorting base edges.

### §3.6 Resolution is a set

- **A call's targets form a `ResolutionSet`,** carrying:
  - `status`;
  - `domain`;
  - `has_unresolved_remainder`;
  - `candidate_set_complete_under_model`;
  - Pysa's unresolved reasons, which have 14 variants (Interface-checked).
- **Targets record their invocation phase.** Candidate targets are `modality = candidate`.
- **Callable values that may be invoked** (Pysa `ifCalled`) become `potential` targets on a
  `Reference`, not `CallSite`s.
- **Synthetic sites** (Pysa `artificial-call`) carry `origin = synthetic_model`.

### §3.7 Coverage and boundaries

- **`coverage(snapshot_id, run_id, scope_kind, scope_node_id, fact_family, status, reason)`**
  - `scope_kind` is release, module or callable.
  - Every family an extractor declares gets a row for every module in scope.
  - A module an extractor never reached is `failed` or `unavailable`, never absent.
- **`boundaries(snapshot_id, fact_id, subject_node_id, fact_family, boundary_reason, detail)`**
  - Resolution issues and analysis stops, with one row per stop.
  - Analytics and briefs read these rows instead of treating "the analysis stopped" as "the
    library has nothing more".

> Decision: ADR-0008

---

## §4 Pipeline

### §4.0 Library acquisition and the run contract

**Proposed.** Source: IP L1146–L1170 (Stage A), L536 (run supporting tables).

**Release.** A release is a set of distributions, each identified by artifact sha256.
- Library code comes from the wheel bytes.
- Docs, examples and tests come from the upstream tag tarball (sha256), with an explicit path map
  from tarball paths to package modules.
- Inputs are listed in a locked acquisition manifest, verified before use. The pattern is the
  one the fastmcp skill uses: `build/acquire.py`, `build/manifests/*.json`,
  `build/acquired/*/ACQUISITION.json`.

**Context.** Analysis runs in a uv venv built from a lock, never an ambient environment. The
context records:
- the Python version and platform;
- the **ordered** search paths;
- the lock digest;
- the digests of every configuration file an analyzer receives.

**Explicit inputs only.** Extractors receive their environment and configuration as arguments.
- **A separate analysis venv.** Each release gets its own venv, built from its own acquisition
  lock (for FastMCP 4.0.3, not the project's `uv.lock`, which serves 4.0.5).
- **Pyrefly's generated config fixes everything ambient:**
  - the interpreter path points into that venv;
  - search-path heuristics and walk-up fallback are disabled;
  - ignore-file handling is fixed;
  - config discovery never runs.
- **Recorded.** The `pyrefly dump-config` output digest is stored in `contexts`.

So nothing ambient (`PATH`, `VIRTUAL_ENV`, an upward `pyproject.toml`) can change an answer
without changing `context_id` (G4).

**Run.** A run is one producer applied to one context for a declared set of families under one
analysis configuration. `runs` records that; `producers` records tool, revision and adapter build
digest.

> Decision: ADR-0007

### §4.1 Stages

| Stage | Owner | Output |
|---|---|---|
| A. Source and analysis universe | Rust + Arrow, then DataFusion checks | `provenance` family |
| B. Typed provider facts | extractors (§4.2) + Arrow builders | family batches |
| C. Provider-local identity | Rust identity logic + DataFusion joins | `provider_node_map`, with keys checked unique before use |
| D. Semantic relationships | DataFusion | family tables and derived views |
| E. Projections and analytics | DataFusion → petgraph / leiden-rs / FCA → DataFusion (§5, §9) | `findings` family, with lineage |
| F. Synthesis | Rust templates + extractive selection (§10) | assertions, briefs |
| G. Publication | Rust (§6) | `snapshots` row; serving bundle |

Unmapped or ambiguous rows become `boundaries` rows. They are never dropped by an inner join.

### §4.2 Extraction

**Interface-checked** (pyrefly-ruff skill; installed pyrefly 1.3.1; 2026-09-22). Pending the ADR-0006 spikes.

| Provider surface | Mode | Families (v1) |
|---|---|---|
| Ruff library crates `=0.0.13`: parser, AST, tokens | in-process native traversal | raw `declarations` (with docstrings), `export_syntax`, `parameter_syntax`, `call_syntax`, `arguments`; `syntax` (increment 2) |
| Our local-binding recognizer over the Ruff AST | in-process `recognizer` | `lexical` (bindings and shadowed history within a scope) |
| Pyrefly `check --report-pysa DIR --report-pysa-format json` | subprocess + `report_decode` | raw `parameter_semantics`, `pysa_calls` (candidates, receivers, overrides, unresolved reasons), class ancestry; `types` (increment 3) |
| Pyrefly `coverage report --public-only` | subprocess + `report_decode` | raw `public_names`. **This defines "public"**; Ruff's `__all__` corroborates, and a disagreement becomes a `provider_disagreement` boundary |
| Pyrefly `check --report-glean DIR` | subprocess + `report_decode` | cross-references and docstrings. **Added only when a consumer needs references**, not in increment 1 |

**Decoder rules**
- Output order varies between runs, so decoders **sort** every record set by canonical key.
- Absolute paths are normalized to release-relative ones.
- Glean offsets are UTF-8 bytes, matching Ruff's `TextRange`. Pysa positions are `line:col` and
  are converted (§3.4).
- A report field omitted because it equals the report's documented default takes that default.
  It is not "unknown".
- **Pysa is authoritative for call targets.** Glean's caller→callee pairs are per function and
  silently drop unresolved calls, so they are never used for `calls`.

**Binding rule (conservative)**
- Any binding of a parameter's name that appears lexically before a guard or forwarding site in
  the same scope marks that site `ambiguous_binding` (a boundary).
- The motivating case is pyarrow's `write_dataset`. Its `schema` guard only applies to
  caller-supplied scanners, because earlier branches rebind `schema`.

**Deferred:** Ruff's full semantic model. Its building blocks are public, but nothing public
drives them. It is deferred until binding kinds or typing-only contexts are needed.

> Decision: ADR-0006

---

## §5 Projections

**Interface-checked** (petgraph skill). Source: IP L1335–L1415, L2233–L2362.

**Declaring a projection.** A projection declares:
- the snapshot;
- node and edge kinds;
- the accepted origins and fidelities;
- candidate-target and unknown-target policies;
- the algorithm and parameter set.

The vertex universe is selected separately from the edges, so isolated public APIs survive.

**The invocation projection (v1)**
- It is built by joining call-site ownership with call targets in DataFusion.
- It keeps `call_site_id`, `resolution_id`, `invocation_phase` and `has_unresolved_remainder` on
  every arc.
- **Parallel call sites are preserved.**
- **Excluded:** `potential` targets and `synthetic_model` arcs.
- **Constructor phases stay tagged.**

**Three identities**

| Identity | Scope | Persisted? |
|---|---|---|
| Canonical `node_id` / `fact_id` | persistent | yes |
| Projection row | links an arc to its evidence rows | yes |
| petgraph `NodeIndex` / `EdgeIndex` | temporary coordinates | **never** |

**Container.** `petgraph::Graph<(), ArcRow, Directed, u32>`.
- It is immutable once built and allows parallel edges.
- Nodes are added in canonical-ID order. Nothing is ever removed, because removal silently
  re-points held indices.
- `try_add_*` is used at the u32 limit.

**Determinism rules**
- `edges_directed` returns the newest edge first, and petgraph's `Bfs` and `Dfs` visit sibling
  nodes in opposite orders.
- So traversals never rely on walker order. They collect a node's edges and **sort by
  (target canonical id, arc key)** before choosing.
- SCC members are sorted by canonical id.

> Decision: ADR-0011

---

## §6 Persistence and publication

**Interface-checked** (deltalake skill probes). Source: IP L1603–L1623, L2967–L3006. Pending the ADR-0009 Delta probe.

### §6.1 Canonical tables and publication

- **Family tables.** Each fact family is a set of append-only Delta tables. Every row carries
  `snapshot_id` as a plain column; tables are not partitioned.
- **A compile attempt:**
  1. writes its rows;
  2. runs local and cross-table validation (§8);
  3. then, **and only then**, appends one row per table to `snapshots`: (snapshot_id,
     content_digest, table, Delta version, schema digest, row count), in a single commit.
- **That commit is the publication act** (Delta commits are atomic per table; there is no
  multi-table commit). The row set also records the `embedding_cache` version the attempt read.
- **An error on the `snapshots` append itself** is ambiguous (`delta.commit.1`). Re-read
  `snapshots`, classify the attempt as published or unpublished, and only then retry or build
  the bundle.
- **Any write error aborts the attempt.** A retry is a new attempt with a new `snapshot_id`,
  because a Delta write error does not mean nothing committed (`delta.commit.1`). Rows from
  unpublished attempts are invisible to readers.
- **Adapter failure on a module** gives `coverage.status = failed` for that module and family. The
  snapshot can still publish, with that gap visible.
- **`SaveMode::Ignore` is never used.** It appends to existing tables (`delta.write.3`).
- **No vacuum or optimize on fact tables** in stage 1. Vacuum defaults to `dry_run=false` and Lite
  mode.

### §6.2 Readers

1. Resolve the snapshot's row set in `snapshots`.
2. Load each table **at its recorded version**, using an unloaded builder with a version or
   `load_version`, and assert `table.version() == Some(v)`.

   A provider built on an already-loaded handle ignores the requested version
   (`delta.open.2`, `delta.read.4`).
3. **Filter `snapshot_id`.** The version is an audit coordinate and a lower bound, not a row
   selector.
4. Project columns by name through the DataFusion provider. Never use `scan_table().with_columns`,
   which returned the wrong column for a partition-first schema (`delta.read.3`).
5. Never scan the Parquet directory directly (`delta.read.2`).
6. Register one object store per table root per session (`delta.storage.4`).

### §6.3 Schema evolution

- **Additive nullable columns only**, via `SchemaMode::Merge`. Anything else is a new table
  (`edges_v2`), with the move recorded in `snapshots`.
- Schema digests are computed from the Delta schema, not from Arrow read back, because read-back
  changes `Utf8` → `Utf8View` and renames list children to `element`.

### §6.4 Serving generations

- **Derivation.** The bundle is built **only after** the `snapshots` append succeeds, by reading
  the published snapshot at its recorded versions, including `embedding_cache`. That is its one
  derivation path. Rebuilding a generation from Delta gives byte-identical files.
- **Bundle.** It is a directory `generations/<key>/` of Arrow IPC files, with declared schemas
  from `cpg-schema`:
  - `briefs`, `brief_members`, `assertions`, `evidence`, `usage_patterns`;
  - `symbol_map` (exact qualified public symbol → brief);
  - `embedding_spec`, `vectors` (§11.1);
  - `MANIFEST.json`, listing the snapshot, content digest, per-file sha256, row counts and per-file
    **schema digests** in `cpg-schema`'s canonical schema form (field name, type, nullability,
    metadata; conformance-tested in Rust and Python).
- **Activation.**
  - The bundle is smoke-queried by the serving code's own test entry point.
  - Then the `generations/active` symlink is switched by atomic rename.
  - A running server keeps the generation it loaded; restarting it picks up the new one.

> Decision: ADR-0009

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

**Tested:** the `cpg-schema::family_smoke` tests (2026-09-22) write Delta and query it through
DataFusion, and `just deps` checks for single versions.

The analyzer, analytics-crate, Python-stack and model pins are in `docs/pins.md`, which is
authoritative for them (ADR-0006, ADR-0010, ADR-0011).

> Decision: ADR-0002

---

## §8 Validation

**Proposed.** Source: IP L1581–L1601.

**Local** (Arrow/Rust), at every materialization boundary:
- exact physical types, nullability and widths;
- codebook membership;
- numeric bounds;
- finite floats and unit-norm vectors.

**Cross-table** (DataFusion), **one query per rule**:
- uniqueness of `(snapshot_id, key)` via `GROUP BY … HAVING count(*) > 1`;
- foreign references via `LEFT ANTI JOIN` returning zero rows;
- endpoint kinds;
- coverage completeness (a row for every declared family × module);
- every assertion cites existing findings and evidence;
- every public symbol in a brief exists in `exports`.

**Rules**
- **Read-only.** Validators only read and reject; they never repair.
- **Shared.** Validators are library code, used both by tests and before publication. There are
  no test-only copies.
- **Not delegated.** DataFusion `Constraints` are informational and are not enforced, so they are
  never relied on.

**Determinism**
- Every relation that is published, compared or snapshot-tested has a **total `ORDER BY`**.
- Every `row_number()` ends in a unique tie-break.
- Every cast in validation code uses `safe: false`.

> Decision: ADR-0008

---

## §9 Analytics

**Proposed** (methods); **Interface-checked** (the libraries named). Source: IP L1735–L1885,
L2233–L2578. Scope extended by ADR-0005 to cover community detection, concept analysis and
embeddings.

**Rules for every technique:**
- it must have a **named consumer** in the brief;
- it must be **deterministic** for fixed inputs and parameters;
- it must record its method and parameters in `analytic_method` / `findings`;
- its effect must be measurable by ablation (§9.8).

**Parameters.** Analysis parameters and the subsystem declaration (§1.4) live in one versioned,
pre-registered analytics config. Its digest is the `lctx-compiler` run's config digest and part of
`content_digest`. Defaults below are starting budgets, not measured optima.

### §9.1 Pass A — public entry point and delegation

- **Question.** Which public API exposes the mechanism, and what does it already coordinate?
- **Method.**
  1. Map public access paths → declarations (`exports`).
  2. From each seed (the declaration node, §3.4.1), run an **explicit BFS with parent pointers**
     over the invocation projection (§5).
     - **Witnesses.** The first witness to a target is the BFS path under sorted adjacency. Up to
       two alternatives are the next paths that differ in their final arc, taken in canonical
       arc order. Parallel call sites are distinct arcs, so each can be a witness.
     - **Truncation.** `omitted_paths = true` when more existed.
     - Default budgets: depth ≤ 2; ≤ 128 vertices and ≤ 512 edges per seed; ≤ 3 witness paths per
       target.
     - The traversal stays inside the subsystem and never crosses `potential`, `synthetic` or
       native boundaries.
- **Output.**
  - Findings: `public_alias`, `direct_delegation`, `bounded_delegation_path`,
    `implementation_boundary`, `incomplete_resolution`.
  - Each carries ordered witness call sites, depth, stop reason and an `omitted_paths` flag.
- **Interpretation boundary.** A call edge never means "always reached" or "recommended". That
  needs `documented` evidence.

### §9.2 Pass B — controls and local restrictions

- **Question.** Which controls expose the behavior, and which local conditions constrain them?
- **Recognizers.** Four:

| Recognizer | Accepted pattern | Output |
|---|---|---|
| direct forwarding | a call argument resolves to a source parameter binding, mapped per candidate signature (including implicit receivers) | `forwarding` |
| identity alias | one unambiguous local assignment in straight-line code | `forwarding` (via alias) |
| defaulted or transformed argument | a literal, default or expression supplied downstream | `transformed_argument` |
| local restriction | a supported predicate over known parameters leading to a local `raise` | `conditional_raise` |

- **Never guessed:**
  - `*args`/`**kwargs` passthrough;
  - ambiguous overloads;
  - multiple writes;
  - property or subscript access;
  - any §4.2 `ambiguous_binding` site.
- **Visited key.** `(callable, formal_parameter, mapping_context)`.
- **Promotion.** A `conditional_raise` is reported as "the implementation raises in this
  branch". It becomes a public precondition only with supporting evidence, and never when an
  enclosing handler may catch it.

### §9.3 Pass C — direct handoff

- **Question.** Which public APIs already connect without an adapter?
- **Method.** Over the CPGs of **official examples and tests**, recognize
  `x = producer(...); consumer(x, ...)` and direct nesting, in straight-line regions only.
- **Rejected as a handoff:**
  - reassignment;
  - additional consumers;
  - escapes;
  - resource boundaries.

  Type compatibility alone is a `candidate`, never a published pattern.
- **Output.** A `handoff` finding with example id, region, producer call, consumer call, binding,
  consumer argument and status.

### §9.4 Community detection

- **Label.** Pending the ADR-0011 spike.
- **Consumers:**
  - **seed selection**, which decides which entry points get briefs within the brief budget;
  - the brief's **Related** field (§10.3).

  A community is not a brief boundary: a brief is one outcome at one public operation
  (IP L1697).
- **Library.** leiden-rs 0.8.1 (`default-features = false, features = ["petgraph"]`, so it runs
  sequentially), with the **CPM** quality function.
- **Determinism.**
  - The seed is recorded.
  - `rand` is pinned, because rand does not promise reproducible sequences across versions.
  - Crate versions are recorded in the method parameters.
- **Graph.**
  - The vertex universe is public and private callables in the subsystem, with results projected
    onto public APIs.
  - Layers:
    - increment 2 uses calls and co-use in examples and tests;
    - increment 3 adds shared parameter types, doc co-mention and embedding kNN, but **only if the
      ablation shows gain**.
  - Each layer is normalized to unit total weight.
  - Hubs (degree above the configured percentile) are down-weighted.
- **Stability.** Consensus over N seeds (default 10), reported per community as a membership
  agreement score.
- **Outputs.** `statistically_derived` findings.

### §9.5 Centrality

- **Consumer.** The primary entry point within a community, which orders seeds for briefs.
- **Method.** petgraph `page_rank` over the usage graph, combined with usage counts from examples
  and tests.
- **Output.** A `statistically_derived` ranking finding.

### §9.6 Formal and relational concept analysis

- **Consumer.** Applicable cases and modes, shared controls, and implication-style assertions
  (e.g. "every writer accepting `filesystem` also accepts `format`").
- **FCA (increment 2).**
  - Our own NextClosure implementation; no maintained crate exists.
  - Objects: the public APIs of one **structurally defined scope**: the subsystem, one module, or
    one class hierarchy. Communities are never an FCA scope, because their membership is
    statistical.
  - Attributes: parameter names, parameter and return types, raised exception types, decorators.
  - A support threshold is applied. There is no stability index: it is #P-hard.
- **RCA (increment 3).** Adds one relational-scaling step (∃-scaling over calls and handoffs).
  It is kept only if the ablation shows it changes published output.
- **Output.**
  - Concepts become `applicable_case` findings.
  - Implications with confidence 1 over the support threshold become `implication` findings.
  - Both are `structurally_observed`, because they are exact over the extracted attributes, with
    the attribute scope stated.

### §9.7 Embeddings in analytics

- **Consumers:**
  - linking doc passages to APIs, supplementing explicit mentions;
  - labelling communities by their nearest doc heading;
  - kNN as an optional community layer.
- **Method.** Vectors come from the cache (§11.1). kNN runs in Rust with a fixed `k` and a
  similarity margin.
- **Output.** `statistically_derived` findings.

### §9.8 Determinism and ablation

- **Determinism oracles:**
  - shuffled input row order gives byte-identical findings, communities and concepts;
  - reruns with the same `content_digest` give identical output.
- **Ablation is mechanical.** Disable one technique and diff the published assertions, briefs
  and boundaries. This is a join on content IDs (§3.4.1).
- **Keep rule.** Keep a technique only if it **changes published output and** improves the
  pre-registered development metric (§12(b)) without lowering §12(a) or (c). Otherwise it is
  removed by ADR.
- **Unbiased check.** Using the gold for this choice makes it a development set. The unbiased
  check is the increment-5 held-out evaluation.
- **Agent-based evaluation** is reserved for the raw-vs-compiled comparison (§12).

> Decision: ADR-0011

---

## §10 Synthesis and briefs

**Proposed.** Source: IP L1695–L1731, L1886–L1966, L2580–L2637. Changed by ADR-0005: there is no
LLM interpreter.

### §10.1 Findings

**A finding is a typed record,** carrying:
- `finding_kind`;
- its subject and related nodes;
- ordered witnesses (fact ids, spans);
- conditions: source-linked text plus predicate node references;
- boundaries;
- method and parameters.

**It is never a sentence.** Text is produced only in §10.2.

### §10.2 Assertions

**Each assertion is atomic,** and carries:
- `assertion_kind`;
- the applicable case;
- supporting finding ids and evidence ids;
- conditions and limitations;
- `evidence_status`.

**Text** comes from a deterministic template per `assertion_kind`, filled from finding fields, or
from verbatim sentences selected from docstrings or docs.

**Evidence statuses:**

| Status | Meaning |
|---|---|
| `structurally_observed` | read directly from extracted facts |
| `documented` | stated in an official docstring, doc or example |
| `statistically_derived` | community, centrality, kNN; carries method, parameters and a stability score |
| `fixture_checked` | supported by an executed fixture for the stated inputs only |
| `unresolved` | a slot the evidence could not fill |

**Rule:** `statistically_derived` output may set titles, grouping, seeds, ordering, Related
entries and doc links. **It may never state a control, a limit or a behavioral claim.**

**Enforcement.**
- **Kind policy.** `cpg-schema` declares, for each `assertion_kind`, its brief section and its
  permitted statuses.
- **Status propagation.** An assertion's status is derived, never chosen. `unresolved` if any
  supporting finding is unresolved. Otherwise `statistically_derived` if any supporting finding,
  **or any finding that defined its scope**, is `statistically_derived`. Otherwise the
  strongest status its evidence supports.
- **Validator.** A §8 query (assertions ⋈ kind policy ⋈ supporting findings) must return zero
  violations.

**Increment-1 assertion kinds**

| Kind | Brief section | From | Permitted statuses |
|---|---|---|---|
| `outcome` | Outcome | docstring summary or explicit doc mention | documented, unresolved |
| `public_access` | Public access | Pass A `public_alias`, `exports` | structurally_observed |
| `coordinates` | Public access, "already coordinates" | Pass A `direct_delegation`, `bounded_delegation_path` | structurally_observed |
| `parameter` | Important controls | `parameters` (name, kind, default, required) | structurally_observed |
| `analysis_boundary` | Limits and prerequisites | `boundaries` on the seed's neighbourhood | structurally_observed |
| `related` | Related | community co-membership, page rank | statistically_derived |

### §10.3 Brief structure and the Outcome order

**One brief = one outcome, anchored to one public operation**, optionally with one direct handoff.

| Section | Filled from |
|---|---|
| Outcome | the order below |
| Public access | Pass A `public_alias` / exports, plus "already coordinates" from Pass A delegation findings (with the note that these are implementation details the caller does not need to rebuild) |
| Applicable input or mode | FCA cases; explicit branches |
| Important controls | Pass B forwarding + parameter docs |
| Usage pattern | Pass C handoff or official example, with setup preserved |
| Limits and prerequisites | Pass B restrictions, documented warnings, boundaries |
| Evidence | all cited findings and evidence ids |
| Related | other briefs in the same community, ordered by page rank (`statistically_derived`) |

**Outcome order.** Take the first source that applies:
1. the entry point's docstring summary line;
2. the lead sentence of a doc passage that **explicitly** mentions the entry point;
3. otherwise `unresolved`.

The nearest doc passage by embedding is never an Outcome. It is published as a doc link
(`statistically_derived`), which keeps the gap metric honest.

**The count of `unresolved` slots, by section, is the §B11 gap metric.**

### §10.4 Grounding checks

These are mechanical and run before publication.
- Every named public symbol and parameter exists in this snapshot.
- Every cited finding, witness path and evidence id exists in this snapshot.
- Every snippet parses and refers only to existing public APIs.
- No assertion's status exceeds its evidence (see the §10.2 rule).
- Warnings and unresolved conditions are never dropped for length. An over-long brief is split by
  applicable case instead.

**Manual review.** From increment 3, each brief gets one manual review pass before
publication, which checks that its extracted sentences are true of **this** entry point. The
review is recorded as `briefs.review_state` with extraction mode `manual_review`. Mechanical
checks cannot establish that (IP L1965, L2634).

Repository text is treated as untrusted data. It is never an instruction to the compiler.

### §10.5 Usage patterns

- **Contents.** One principal operation, or one direct handoff, plus the setup it needs
  (initialization, schema, resources, configuration).
- **Trimming.** Test-only details are removed only when that removes no precondition.
- **Publication.** A pattern is published only with an official example, a relevant test, or an
  executed fixture.
- **Fixtures** run offline, with no network or credentials. A pass supports only the tested case.

> Decision: ADR-0005

---

## §11 Serving and agent interface

**Interface-checked** (fastmcp skill 4.0.3 vs installed 4.0.5; vLLM 0.30.0 source; Qwen3 model
card). Source: IP L2037–L2071, L2640–L2965. Pending the ADR-0010 spikes.

### §11.1 Embedding spec and vectors

**Model.** Qwen3-Embedding-4B, served by a **separate vLLM 0.30.0 service**
(`vllm serve Qwen/Qwen3-Embedding-4B --runner pooling`).
- Output: 2,560 dimensions, `Float32`, cosine.
- vLLM L2-normalizes by default under pooling.
- Never send `dimensions`: the model is not Matryoshka-enabled without overrides.

**Spec.** The spec (model and revision, tokenizer revision, pooling, instruction template,
document template, dimensions, dtype, normalization) is hashed into `spec_hash`.
- **Query template (query only):** `Instruct: {task_description}\nQuery:{query}`. There is no
  space after `Query:`. Documents take no prefix.
- **Document text** is the deterministic brief projection (IP L2666–L2689): outcome, applicable
  case, public APIs, controls, usage description and limits, capped at 2,048 tokens. An over-long
  brief is split by applicable case, never truncated.
- **Rejected responses:** wrong count, wrong index mapping, wrong length, non-finite values,
  norm ≠ 1 ± ε, model mismatch.

**Clients.**
- Compile-time vectors come from Rust (`reqwest` + `tokio`).
- Query-time vectors come from Python (`httpx`).
- Both clients check the spec against the same **conformance vectors** (fixed inputs → expected
  vectors within tolerance).

**Cache.** Vectors are keyed by `spec_hash + input_hash` in the canonical `embedding_cache` Delta
table (§3.2), because vLLM numerics vary with batching. Snapshots record the cache version they
read, and bundles copy the vectors they need from it.
- A **deterministic fake embedder**, with its own spec hash, is used by tests and `just check`.
- Mixing spec hashes within one generation is rejected.

### §11.2 Retrieval

**In-process, over the pinned generation.**

1. **Lexical.** BM25 over `lexical_text`. That text is the brief's text plus split forms of
   public names (`write_dataset` → `write dataset`).
2. **Vector.** Exact cosine over the generation's vectors, using the instruction-prefixed query
   vector.
3. **Fusion.** Reciprocal-rank fusion with K = 60, 1-based ranks, and ties broken by `brief_id`.
4. **Exact symbols.** Matches in `symbol_map` are merged by brief id and **recorded as promoted**.
5. **Return.** At most `limit` results (default 5), with relevance and coverage information.
   **A score is never presented as proof of task fit.**
6. **Degraded mode.** Lexical + exact-symbol only when the embedding service is down, reported in
   the result.

**Hydration.** `(snapshot_id, brief_id)` → the full brief, all conditions, limits, evidence and
usage patterns, by deterministic lookup. It never depends on a second search.

**LanceDB.** LanceDB 0.39.0 (Python) is **deferred behind a trigger**: corpus above a few
thousand briefs, or ANN / managed FTS needed.
- Its hybrid, FTS and RRF call chain is Interface-checked.
- Its wheel isolates its own Arrow 58 / DataFusion 54, so it would not affect §B9.

### §11.3 FastMCP contract

- **Package.** `python/lctx_mcp`. It depends on `fastmcp` 4.0.x, `pyarrow` (pinned; the bundle
  reader), `numpy` and `httpx`, never on vLLM.
- **Startup checks.** The lifespan rejects a generation whose per-file schema digests differ from
  the canonical schemas it expects, or whose `embedding_spec` hash differs from its query
  client's spec.

**Tools**

| Tool | Parameters | Output |
|---|---|---|
| `search_capabilities` | library, query (1–4000 chars), limit (1–10) | Pydantic `SearchResult` object: snapshot, generation key, mode (hybrid or lexical-only), coverage summary, hits (brief id, title, outcome, outcome `evidence_status`, relevance score, rank source, promoted flag). An unknown library raises `ToolError` |
| `get_capability` | snapshot_id, capability_id | Pydantic `Capability` object: all §10.3 sections, evidence and statuses |

- **Annotations.** Both tools carry `ToolAnnotations(read_only_hint=True, idempotent_hint=True,
  open_world_hint=False)`.
- **Output shape.** Always object outputs, never bare lists.
- **Resource.** A `capability://{snapshot_id}/{capability_id}` template shares the hydration code.
  Resource reads return MIME text, not structured content.
- **State.** A lifespan loads the active generation once and exposes it through
  `ctx.lifespan_context`. One generation per process.
- **Errors.** `ToolError` for unknown ids or a snapshot mismatch; `mask_error_details=True`.
- **Transport.** stdio. Nothing may write to stdout.
- **Tests.** `fastmcp.Client(mcp)` in both the auto and legacy protocol modes, asserting
  `.structured_content`.

> Decision: ADR-0010

---

## §12 Evaluation

**Proposed.** Source: IP L2116–L2131, L3026–L3072.

**Fixtures.** The IP L3030 table, plus:
- guard after parameter rebinding;
- shuffled input gives identical communities and concepts;
- LFR planted-partition graphs for community detection;
- FCA over a hand-computed context.

**Gold scoring** (§1.4). A small committed extract of the gold families records each family's
`authoring_sha256`, `operations`, `task_aliases` and static-evidence spans. Scores:
- (a) best-match Jaccard between each gold family's `operations` and the public members of the
  compiled briefs;
- (b) hit@5 of `search_capabilities` on the gold `task_aliases`. This is the pre-registered
  **development metric** for the §9.8 keep rule, never for parameter tuning;
- (c) recall of gold static-evidence spans by compiled evidence.

**Ablation** (§9.8). Diff the published output with each technique disabled, apply the keep
rule, and record the result in the increment-3 review.

**Agent evaluation** (increment 5).
- About 20 held-out task prompts and 5–10 usage fixtures.
- Raw-evidence retrieval is compared against compiled briefs under similar context budgets.
- Questions: was the right built-in capability found; did the agent discover the important
  control; did it use a supported handoff; did it respect the limits; did it avoid rebuilding
  library behavior?

**LLM trigger (§B11).** After increment 3, `unresolved` slot counts by brief section are reported.
If the increment-5 evaluation attributes failures to those slots, an ADR adds a local generation
model under §10.4 grounding.

> Decision: ADR-0004

---

## §13 Deferred

Each item returns by ADR when a consumer needs it.

| Deferred | Where it is described | Trigger |
|---|---|---|
| Full ontology tables, `record_fields`, native type graph | IP L469–L717, L334–L409 | an analytic or brief needs the detail |
| Ruff semantic-model port | IP L99–L166 | binding kinds or typing-only context are needed |
| Glean decoding | IP L209–L231 | a consumer needs cross-references |
| CinderX, TSP query surfaces | IP L290–L332, L410–L425 | narrowing or contextual types are needed |
| Python CFG, dominance, dataflow, aliasing, summaries | IP L821–L884, L1505–L1563 | a pass needs path-sensitive facts |
| SCC condensation, dominators on projections | IP L1416–L1503 | a consumer beyond recursion labelling |
| LanceDB, ANN indexes | IP L2766–L2965 | corpus size or managed FTS (§11.2) |
| LLM interpretation | IP L1886–L1966, L2580–L2637 | §12 gap metric (§B11) |
| Graph embeddings, neural reranking, composition planning | IP L2073–L2087 | an ADR after increment 5 |

---

## Revision history

| Date | Change | ADR |
|---|---|---|
| 2026-09-22 | Seeded from Initial_plan.md; §7 family verified by smoke build | ADR-0001, ADR-0002, ADR-0003 |
| 2026-09-22 | Restored increment-1 rules dropped in condensation; placed validators (baseline review F2, F6, O1) | — |
| 2026-09-22 | Rewritten for the capability-compiler target: §1, §B1/§B4/§B8/§B10 revised, §B11–§B14 added, §3–§6 and §8 detailed, §9–§13 added | ADR-0004 … ADR-0011 |
