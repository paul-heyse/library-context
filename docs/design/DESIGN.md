# library-context — design

**This file is the current truth.** It says what the system *is*. `docs/adr/` says *why*, and what
was rejected. Change a governed section only in the same commit as the ADR that decides it, and
end the section with `> Decision: ADR-NNNN`. Sections are never renumbered: insert `§3.2.1` rather
than shifting `§3.3`. There is no line budget: the detail the design needs comes before length
(operator, 2026-09-22; ADR-0004 amendment). Column-level contracts live in
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
| 1 | **One complete path.** Real FastMCP 4.0.5 (`libraries/fastmcp`, §4.0), with one hand-registered seed, `fastmcp.FastMCP.tool` (analytics config, §1.4), plus 2–4 distractor briefs for other public entry points. Families provenance, exports, signatures, calls, coverage, findings, embedding_cache. Pass A → increment-1 assertion kinds (§10.2) → brief → bundle → embeddings → hybrid search (BM25, exact cosine, RRF, exact-symbol promotion, degraded lexical-only mode; moved up from increment 4, ADR-0004 amendment) → hydration → both MCP tools | deep |
| 2 | **Analytics families on synthetic fixtures.** Passes B and C with the `syntax` and `lexical` families; single-layer community detection with seed-consensus stability; `page_rank`; FCA within one structural scope (a community's membership is statistical, §9.6) | compact |
| 3 | **Pilot corpus, ~15–25 reviewed briefs** (§10.4 manual review). Docs family; compile-time embeddings for doc links and labels; RCA and extra community layers, each kept only if the §9.8 ablation shows it changes published output; gold scoring (§12) | deep |
| 4 | **Reliable serving.** The generation lifecycle (smoke query, atomic activation, byte-rebuild verify, manifest check at load) and serving reliability (degraded mode under embedder failure, measured latency). Hybrid retrieval itself is built in increment 1 (ADR-0004 amendment, 2026-09-23) | compact |
| 5 | **Agent evaluation.** Held-out tasks, raw-vs-compiled comparison, and the decision on the LLM trigger (§B11) | deep |

**CPG first** (operator decision, 2026-09-22; ADR-0004 amendment, ADR-0014). Increment 1's
slices 1–3 built the extraction, derivation and publication path. Before its analytic path
(analytics config, Pass A, briefs), the CPG is completed in slices C1–C6 (§3.8):

| Slice | Delivers | Review |
|---|---|---|
| C1 | The node and edge catalogs over the increment-1 families; typed external and synthetic endpoints; dependency definitions; retention; per-stage metrics | standard (ADR-0014) + compact |
| C2 | `syntax` | compact |
| C3 | `lexical`, with our recognizer's full name resolution | compact |
| C4 | `types`: observations, type structure, record fields | compact |
| C5 | The source corpus: `docs`, and examples and tests as a usage run | compact |
| C6 | The whole CPG on the pilot, measured | deep |

So `syntax` and `lexical` move up from increment 2, and `types` and `docs` from increment 3. The
analytics that read them stay in their increments, and every family names its consumer.

> Decision: ADR-0004, ADR-0014, ADR-0001, ADR-0013

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

- **Library.** FastMCP **4.0.5**, acquired as `libraries/fastmcp` (§4.0, ADR-0013) with the same
  install line the `fastmcp` skill uses:
  `fastmcp[anthropic,openai,gemini,azure,apps,code-mode,tasks]==4.0.5`. The release is three
  distributions: `fastmcp` is a facade, `fastmcp-slim` holds the code (257 modules) and
  `fastmcp-tasks` (18 modules) comes through the `tasks` extra. `mcp` and `mcp-types` 2.2.0 are
  dependencies: resolved, but behind the analyzed boundary. **Tested** (`just pilot`,
  2026-09-22; re-verified 2026-09-23 by the C6 review): 275 modules, 103 distributions, every
  validation rule passing.
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
- **One version.** The gold and the analysis name one FastMCP: `scripts/check_gold.py`
  (`just gold`, run by `just test-all`) fails when the skill's install line or its resolved release versions differ from
  `libraries/fastmcp`. The move from 4.0.3 is not cosmetic. The 4.0.3 → 4.0.5 source diff
  (2026-09-22) changes behaviour, not only logging: the no-context strictness helper returns
  `None` rather than `False`; `LocalProvider.get_tasks` applies transforms; client pagination
  tests `is None`; cursor offsets are validated; version sorting gains a tie-break; OpenAPI body
  naming and multipart encoding change; the OAuth proxy rejects `expires_in <= 0`. The skill's
  reviewed claims are re-reviewed against it.
- **Serving.** The agent interface runs on FastMCP from the project's own environment
  (ADR-0010), which today is also 4.0.5. That environment is never an analysis input.

> Decision: ADR-0004, ADR-0013

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

**Tested** (spike `spike/pyrefly-inproc`, 2026-09-22). Source: IP L1–L425, L886–L897.

- **Pyrefly**, linked in-process from a pinned, minimally patched fork (§B8, §4.2), supplies:
  - definitions and signatures;
  - call resolution, through its own Pysa collectors;
  - class order;
  - public names;
  - types, when a consumer needs them.
- **Ruff library crates** (`=0.0.11`, the line Pyrefly compiles against) supply syntax. They walk
  Pyrefly's own parse, so there is one parse and one byte coordinate system.
- **Binding history** comes from our own scope-aware recognizer over the Ruff AST (§4.2.4).
  Pyrefly's binding IR drops statically decided branches, so it is not conservative enough for
  this, and Ruff's semantic model has no public driver.
- **No second parser or type checker.** Gaps are closed with adapters, normalization and our own
  analyses.

> Decision: ADR-0012

### §B2 Arrow schemas are the authoritative data contract

**Implemented** and **Tested** for the fact, derived and catalog tables (`contracts_snapshot`,
`registry_snapshot`, `batches_type_check_and_sort_canonically_regardless_of_input_order`; C6
review, 2026-09-23); the serving and embedding clauses are **Proposed**.

- **What `cpg-schema` holds:**
  - the Arrow `Schema` definitions;
  - append-only category codebooks;
  - logical-ID newtypes;
  - key and reference declarations;
  - the family → node/edge view mapping (§3.2);
  - physical storage mappings;
  - Arrow-only batch builders and local validators.
- **Dependencies:** `arrow-*`, plus `blake3` for id derivation (§3.4.1). DataFusion validators
  live in a core-only crate (§8).
- **No inferred schemas.** No schema is inferred from JSON or from a first batch. That includes
  the serving bundle's schema (§6.4) and the embedding exchange (§11.1).

> Decision: ADR-0012

### §B3 DataFusion constructs and validates relations

**Implemented** and **Tested** (C6 review, 2026-09-23): the derivations and the 496 rules run in
DataFusion; `validate` is the one validator publication (`attempt.rs`) and the tests call; what a
rule can prove is stated in §8 (its edit guards counted apart).

- Most nodes and edges are joins, projections and unions over extracted facts.
- Cross-table invariants are DataFusion queries, one per rule (§8).
- The same validators run in tests and before publication.

### §B4 Graph algorithms have named owners

**Interface-checked.** Source: IP L1335–1504, L2233–2362.

| Owner | Algorithms |
|---|---|
| petgraph 0.8.3 | Traversal and SCCs, over immutable, explicitly declared projections (§5) |
| leiden-rs | Community detection (§9.4), fed a normalized, sorted edge list |
| Our own code | Weighted PageRank with convergence diagnostics (§9.5; petgraph's `page_rank` takes no weights, miscounts parallel arcs and reports no convergence: the library-leverage review, D1); formal and relational concept analysis (§9.6); condensation from `kosaraju_scc` membership (iterative; petgraph's `condensation` merges parallel edges) |

- **Each algorithm has a named consumer in the brief** (§9).
- **A relationship does not need a graph algorithm** just because it has two endpoints.

> Decision: ADR-0011

### §B5 Python semantics are custom Rust passes

**Proposed.**

- Python-specific semantics are our own Rust code with stated abstractions: v1 recognizers
  (§9.1–§9.3), and later CFG and dataflow (§13).
- Pyrefly's inference graph is never relabelled as runtime dataflow.

### §B6 Facts are first-class assertions with provenance

**Implemented** and **Tested** (the `fact:`/`fact-payload:` cases; `provider_disagreement` rows in
the derived snapshots; C6 review, 2026-09-23).

- Every extracted assertion is a `facts` row carrying its run, `origin`, `extraction_mode`,
  `modality`, `fidelity` and model.
- **Analysis results carry their provenance in-row** (ADR-0019; **Proposed**, Pass A's findings
  **Implemented** in slice 1.4): findings, assertions and briefs
  hold their run (`lctx-compiler`), model, extraction mode, method invocation and
  `evidence_status`, cite the `fact_id`s, `edge_id`s and node ids they rest on, and are
  rebuildable from the snapshot, the analytics config and the `compiler_digest`. Operator review
  verdicts (ADR-0020, to be written in slice 3.5; **Proposed**) are the one analysis-side input,
  so they are `facts` rows.
- **Scope** (ADR-0008, carried by ADR-0014, amended by ADR-0019): `facts` rows are the extracted
  assertions and operator review verdicts. A derived join row (Stage C/D, §4.1) is not a `facts` row: it is
  traced by the `fact_id`s it cites plus its snapshot's `compiler_digest` (§3.4.1, §6.1), and it
  is rebuildable from them. That includes the `nodes` and `edges` catalogs (§3.8). An edge is a
  first-class relationship through its persistent `edge_id` and its evidence `fact_id`s, not
  through a `facts` row of its own.
- Independent assertions are kept, including disagreement. They are never collapsed into mutable
  node properties.

> Decision: ADR-0014, ADR-0019

### §B7 Delta canonical store, published by a `snapshots` append

**Implemented** and **Tested** (`an_attempt_publishes_every_table_and_readers_see_only_published_rows`,
`reads_pin_the_version_and_filter_the_snapshot`, `a_failed_snapshots_append_is_classified_by_rereading`,
`retention_keeps_old_versions_loadable`; C6 review, 2026-09-23).

- Fact tables are append-only Delta tables, one per fact family.
- A snapshot becomes visible only through one append to the `snapshots` table, made after
  validation passes.
- Readers resolve table versions through that row and filter by `snapshot_id` (§6).

> Decision: ADR-0017 (superseding ADR-0009)

### §B8 Pyrefly and Ruff link in-process; one workspace, one process

**Tested** (spike S1–S4, 2026-09-22) for the in-process build, the fork dependency by git
revision, the driver and CLI parity. The panic policy is **Proposed**.

- **Pyrefly** is a git dependency on the fork `paul-heyse/pyrefly` (branch `lctx/1.3.1-r2` since
  C4; `lctx/1.3.1` keeps the first revision), pinned by revision. The fork is tag 1.3.1 plus
  `third_party/pyrefly-1.3.1.patch`, which changes visibility, adds a no-write reporter switch, a
  borrow of the reporter and an accessor composing existing upstream queries (a module's wildcard
  set, like the exposed `get_exports_data`), and changes no logic.
- **Ruff** library crates are pinned to the ruff line Pyrefly compiles against.
- **One workspace, one process.** Extraction, construction, analytics and publication live in
  one core Rust workspace and run in one process. There is no IPC.
- **Isolation is by contract, not by process:** an explicit configuration (§4.2.1), refusal of
  the ambient variables that cannot be cleared, and abort on any panic (§4.2.5).
- **The CLI** of the same revision is kept only as a parity-test oracle (§4.2.5).

> Decision: ADR-0012

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

> Decision: ADR-0017 (superseding ADR-0009)

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

**The v1 node kinds** (the `node_kind` codebook, grown by CPG slice; §3.8; ADR-0014):

| Slice | Node kinds |
|---|---|
| C1 | `module`, `class`, `function` (every `def`, overload stubs included), `parameter`, `call_site`, `argument`, `export` (a public access path), `external_module`, `external_symbol`, `synthetic_callable` |
| C2 | `syntax_node` |
| C3 | `scope`, `binding`, `reference` |
| C4 | `type`, `field` |
| C5 | `document`, `passage`, `code_block` |

- `ExternalSymbol` and `SyntheticCallable` above are these kinds.
- A `ResolutionSet` is keyed by its call site (§3.6), so it is not a separate node.
- One syntactic occurrence has one node id and one kind. An argument or a reference is a role,
  identified from its carrier (§3.4.1).

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
- **The node and edge catalogs** (ADR-0014; **Implemented** and **Tested** in C1, §3.8).
  - `nodes` and `edges` are Stage-D derived tables of family `graph`, generated from one registry
    (§3.8).
  - They carry identity, kind, endpoints and evidence, never a payload. So the family tables stay
    the only authority, as the typed extension tables of the catalogs.
  - Projections (§5) select from the catalogs by kind, derivation class and evidence, and join
    the extension tables for payload.
- **One producer per table.** A fact table is written by one producer: one extractor surface, or
  one Stage-C/D derivation (§4.1). `runs`, `contexts`, `producers` and `facts` are registries each
  producer appends its own rows to. `releases`, `distributions` and `source_files` are written
  **once per extractor run**, which carries Stage A's output; later producers reference
  `release_id` and never append (ADR-0013, amended for C5). An attempt holds several **extractor** runs only over
  distinct releases (the library and its corpus), since the family keys carry no run. The
  `lctx-compiler` run (analytics and synthesis, ADR-0019) and the `manual-review` run are over
  the library release too, but they declare no families and write no `releases`,
  `distributions` or `source_files`; the extractor-only rules are scoped to runs that declare a
  code family. The corpus run
  has its own context (its search path adds the tree ahead of site-packages) over the same
  environment, whose `distributions` it lists too. What both runs assert about one thing (a
  dependency module or definition, a type term and its structure) is one node and one edge,
  from its first fact; the other run's fact is a counted duplicate in the lineage. `distributions` is keyed by `context_id`:
  the environment belongs to the context.
- **Merged tables are derivations.** Where two providers contribute to one logical record, each
  writes its own raw table. The merged table is a DataFusion derivation
  (`relational_derivation`) that carries keys, both `fact_id`s and what the join decides: a
  mapping, a status, a reason, or an aggregate of raw values (`signatures.form` and
  `function_key`, `resolutions.unresolved_reason`). It never copies a raw payload column
  unchanged, so the raw table stays the authority (**Implemented**, `cpg_schema::derived`,
  slice 2; **Tested** by the derived-table snapshots).
- **Gaps are recorded in the derived row.** Where a null needs explaining, a `reason` column
  holds it: `provider_disagreement`, `missing_evidence`, `outside_provider_model`,
  `no_source_declaration` (a function Pysa describes without a `def` of its own) or
  `unreachable_in_context` (a `def` the context never binds). `boundaries` stays the extractor's.
  A null node with a null reason has one declared meaning per table: in `exports`, the origin is
  not a `def` or `class` of the release; in `call_targets`, the target is outside the release (a
  rule rejects a release target with neither).
  - **From C1 (ADR-0014; Implemented):** these null meanings are retired.
    - Every target is a typed node, with a target outside the release being an
      `external_symbol`. `exports.target_node_id` is the declaration, the dependency definition,
      or the module (of the release or a dependency) the path names.
    - A reason appears **only where a provider says why**: Stage C's own reason; for an export,
      Pyrefly's symbol kind of the origin, or `missing_evidence` when Pyrefly traces no origin or
      records no kind. Since C3 a variable-like origin in a release module (a variable, attribute,
      constant, parameter, type parameter or type alias) targets its module-scope `binding`: the
      last binding event of the name by ordinal, unbinding events excluded. `variable_origin`
      remains only for a variable-like origin in a dependency module, which has no binding node;
      a release-origin variable with no binding is our failure (C3 review F2).
    - Any other unmapped target is our own failure to find what the provider referenced. It keeps
      a null reason, and a `typed:*` rule rejects the snapshot. There is no catch-all reason
      (ADR-0014 review F1).

| Family | Tables | First increment |
|---|---|---|
| `provenance` | `releases` (library, requirement, lock digest, or a source tree's label), `distributions` (every installed distribution: version, artifact sha256s, `RECORD` digest, in the release or not), `source_files` (C1: with its release `distribution`; ADR-0015: its `text`, the bytes every span indexes, null only when not UTF-8, and its `role`: `release`, `example`, `test` or `doc_block`), `contexts`, `producers`, `runs`, `facts`. C1: `context_modules` (each dependency module a fact references: name, site-relative path or Pyrefly's bundled typeshed, distribution and version) and `context_definitions` (the Pysa definitions of those modules: the existence source of `external_symbol` nodes; §3.8) | 1 |
| `exports` | raw: `declarations` (Ruff: qualified name, kind, parent, span, docstring text and span, `is_overload`), `export_syntax` (Ruff: import aliases, `__all__` statement span; syntax evidence only), `public_names` (Pyrefly: access path → origin and its file, `via_dunder_all`; §4.2.3). Derived: `exports` (public access path → the seed declaration in the origin's file: an implementation before an `@overload` stub, then the one Pysa describes, then the last in source order; one row per `public_names` row, so a `.py`/`.pyi` pair gives an access path two rows, one seeding each file, told apart by `source_files.is_stub`; Pass A seeds from the source row) | 1 |
| `signatures` | raw: `parameter_syntax` (Ruff: ordinal, name, default text and span, annotation text), `pysa_functions` (Pyrefly: function key → name span, flags, signature count), `parameter_semantics` (Pyrefly Pysa undecorated signatures: kind, required, annotation), `class_ancestry` (Pyrefly: bases and reported MRO), C1 `pysa_classes` (one row per class, so a class without bases is keyed). Derived: `provider_node_map` (Stage C, name-span join), `signatures` (per `def`: its callable, stubs rolled up to the implementation, and its Pysa signature index), `parameters` (Ruff ⋈ Pysa on the ordinal); C1 `provider_class_map` (Stage C for classes), `synthetic_callables`, `ancestry_targets` and `override_targets` (bases, MRO entries and overridden methods resolved to nodes, a reason where an end does not resolve) | 1 |
| `calls` | raw: `call_syntax` and `arguments` (Ruff: span, owner, ordinal, keyword, starred, expression span; `call_syntax` rows are the call sites), `pysa_calls` (Pyrefly Pysa call graphs: targets, receiver, phase, unresolved reasons). Derived: `resolutions` (§3.6, one per call site), `call_targets` (joined on the full call-expression range, §4.2.3). C1: `arguments.node_id`, `pysa_calls.payload_id`, `argument_resolutions` (higher-order arguments: status and unresolved remainder); `call_targets` typed (a declaration, a synthetic callable or a dependency definition, with the higher-order argument). Pysa rows at non-call sites (property accesses, identifiers, artificial and format-string sites) stay raw, as a declared pending class of the lineage rule, until C2 and C3 give them nodes | 1 |
| `embedding_cache` | `embedding_cache` (spec_hash, input_hash, vector as `List<Float32>`, model identity). Global and append-only; not snapshot-qualified: its read mode is `global` (§6.2); written by an insert-only MERGE (ADR-0017 amendment); the key is unique | 1 |
| `coverage` | `coverage`, `boundaries` (§3.7) | 1 |
| `graph` | derived: `nodes`, `edges` (§3.8). Not a coverage unit | C1 |
| `syntax` | **Implemented and Tested (C2; revised by its compact review).** Raw `syntax_nodes` (Ruff): every statement, the clause nodes (`elif`/`else`, `except`, `case`, `with` items) and **every expression outside annotations** (the IP 2.1 exhaustive-exporter contract; placement depends on the source alone, never on a provider). Each row has its parent (the nearest placed ancestor), owner, field (`syntax_field`: body, test, orelse, handler, exc, cause, default, argument, …), ordinal in that field (a statement's block index), span, `kind` (Ruff's `NodeKind`, the `syntax_kind` codebook, an exhaustive match) and detail (a name, attribute, operator, literal as written, or a handler's name). A `def`, a `class` and a call are placed under their declaration and call-site ids; nothing inside an annotation is placed. Derived: `site_targets` (each Pysa attribute, artificial and format-string record → the deepest syntax node at its span, a chained comparison's pairwise site → its comparison → its typed target; a span with no node is our own failure, never a reason). Consumers: Pass B guards, raises, handlers and defaults; Pass C straight-line regions; FCA raised types (their type is C4's) | C2 |
| `lexical` | **Implemented and Tested (C3; revised by its compact review).** Raw, from our recognizer (surface `lctx-lexical`, `recognizer`, inside the Ruff walk): `scopes` (module, class, function, lambda, comprehension; owner, and parent = the scope the scope's position evaluates in, so a lambda in a default or decorator belongs to the enclosing scope), `bindings` (every binding event per scope, ordinals in source order: kind, site, span, the assigned value's span, and the innermost branch Pyrefly decides statically that it sits in: the deciding test's kind (`type_checking`, `version_info`, `platform`, `constant`, `combined`) and whether Pyrefly analyzes or prunes that branch, clause by clause exactly as `SysInfo::pruned_if_branches` decides, recursively: an `if` inside a pruned clause is never walked, so its bindings take the pruning clause's mark (H1 C1, H1 review F1: `SysInfo::evaluate_bool` per clause, the kind from the expression tree; `static_marks_agree_with_pyrefly_pruning` checks every fixture `if`, and `static_polarity_is_pyrefly_recursive_pruning` every assignment binding, against Pyrefly's own pruning, nested cases included); every event under `global`/`nonlocal` binds in the declared scope, a `nonlocal` target decided once every binding is known; a repeated name in one declaration is one event; the module's implicit globals and a method's `__class__` cell are `implicit` events), `references` (every name load outside annotations, and an augmented assignment's target, which reads before it binds; a role of its placed name, with its parent and field; nothing inside an annotation opens a scope or binds), `reference_resolutions` (Python's scoping rules as modelled: the scope's own bindings, else the nearest enclosing function scope with class scopes skipped, else the module, else the star imports whose wildcard set holds the name, else a builtin; comprehension first iterables and function defaults in the enclosing scope; walrus in the nearest non-comprehension scope; flow-insensitive candidates, except that a module or class body reading a name it binds only later also reads it from outside, as `LOAD_NAME` does; a builtin names itself, a builtin variable reads `variable_origin`, anything else `unresolved_target`). Every name set from outside the module's text is Pyrefly's: its `ImplicitGlobal` set, its `builtins` definitions that are real public names, each star import's `Transaction::get_wildcard` set (a star module Pyrefly cannot find stays a candidate for any otherwise unbound name). **Not modelled:** PEP 695 annotation scopes (class and alias type parameters), the implicit unbinding at the end of an `except … as` handler (C3 review O4, O5, deferred). `export_syntax.resolved_module` is each import's absolute module by Pyrefly's own `ModuleName::new_maybe_relative`. Derived: `identifier_targets` (Pysa's identifier sites → the reference at their span → typed target), `import_targets` (each import → the release or dependency module it names; `unresolved_target` only where Pyrefly's finder says not found, a `context_modules` row of origin `not_found`, or where the import climbs past the top package). Consumers: Pass B binding order (§4.2.4), Pass C bindings and values, the import graph, `if_called` targets, variable exports | C3 |
| `types` | **Implemented and Tested (C4; revised by its compact review).** Raw, from Pyrefly's native types (surface `pyrefly-types`, `native_structural`). `type_terms`: one row per distinct term, its id a Merkle hash over Pyrefly's own structure and identities (kind, detail, class pair, children with their roles; §3.4.1); the display is a label, and only a display-only kind (`other`, `truncated`) hashes it. Two structures that share an id but differ in kind, detail or display fail `unique:type_terms` (several runs may observe one term; `nodes` keeps it once). A class is a (module ref, class key) pair, an enum member keeps its class, and a recursive alias is a reference to its name, so a term is finite; a depth cap (32) makes that a guarantee. A type variable's id is Pyrefly's own identity (`QuantifiedIdentity`), so one variable is one term wherever it is observed and two unrelated `T`s are two; its bound, constraints and default are its children (`type_arg_role` `bound`, `constraint`, `default`). `type_term_kind` maps every `Type` variant by an exhaustive match; solver-internal and experimental variants are `other` (`display_only`). `type_term_args`: each child at its role and ordinal; a callable parameter carries its name, kind and requiredness. `type_observations` (§3.5.1): each parameter's type, each `def`'s return (`Key::ReturnType`; an annotated one is the annotation, §3.5.1), each call's result, each argument's value and each `raise`'s exception (Pyrefly's expression trace at the exact span; calls in annotations excluded). A subject Pyrefly records no type for (a `TypeVar(...)` declaration, a call in a lambda body, a branch Pyrefly skips for the platform) is a `types` boundary (`missing_evidence`) and the module's coverage is `partial`; a bare `raise` has no exception to type. `record_fields`: the fields a dataclass, attrs or pydantic class, `TypedDict` or `NamedTuple` declares itself (an inherited field a subclass assigns in a method stays its base's), with the flags as the field states them (default, `init`, alias and `kw_only` through `ClassField::dataclass_flags_of`; `TypedDict` required and read-only); the constructor they imply is Pyrefly's synthesized `__init__`, a `synthetic_callable` with its `parameter_semantics`. Derived: `type_class_targets` (each term's class → a release class or dependency definition; a miss is our failure, never a reason) and `type_binders` (each source-anchored variable → the innermost release declaration, type-alias or assignment statement holding Pyrefly's scope anchor, a joined fact; `scope_boundary` for an anchor outside the release). Consumers: Pass C type compatibility, FCA parameter, return and raised types, controls from record fields | C4 |
| `docs` | **Implemented and Tested (C5a, C5b).** A corpus run over the library's upstream tree at its pinned commit (§4.0), in the library's environment; its search path is the tree, then site-packages (the library run's search path), so a module both runs import is one file. Raw, parsed by markdown-rs 1.0 (MDX constructs and frontmatter; byte offsets, probe P5): `documents` (each selected file: path, digest, frontmatter title, whether it parsed; one that does not parse is `unavailable` with markdown-rs's message), `passages` (each top-level heading's section to the next, so a document's passages partition it, with level, heading and heading path; the text before the first heading is passage 0), `code_blocks` (fenced blocks at any depth, MDX components included: language, meta, code, digest; each in the passage its start falls in) and `doc_links` (URL, title, text). `mentions` (our recognizer, `lctx-docs`) against the library run's public names and declarations, two classes never merged: `exact` for inline code (or a dotted prose token) that is a public access path, an origin path or a public class's member (`FastMCP.tool`); `lexical` for inline code that is a bare public name of a class, function, method or module, or such a name in prose when it is distinctive (an underscore, or two capitals and a lower-case letter), one `candidate` per origin (re-exports collapse to the shortest access path). Embedding-based linking is §9.7's. Derived: `mention_targets` (→ the `export` node, or the member's release declaration by the seed rank). Consumers: exact doc links to APIs and extractive brief text, §9.4 co-mention. **The usage run (C5b)** is the same corpus run's code: the selected examples and tests, and every Python code block materialized as a module of its own (`_lctx_blocks/d_<document>/block_<n>.py`, named in `code_blocks.module_path`), with every code family but `exports`. The corpus names each installed file the release's distributions own by the library run's own site-relative `@path` (C5 review F2), so a usage call's target is the release's own declaration or synthetic callable (the same Pysa key, probe P4), a release class is one type term whichever run observes it, and an import of a library module targets the library's module node. A tree that holds its own copy of the package ahead of the installed one (a flat layout) would cut the usage code off the release, so it fails the compile, naming the module. Consumers: Pass C examples and tests, §10.3–§10.5 usage patterns, §9.4 co-use. Each usage module's text and role are in `source_files` (ADR-0015, closing C6 review F2), so a snippet and whether it is an example, a test or a doc block are read from Delta alone | C5 |
| `findings` | ADR-0019 (contracts in `cpg_schema::findings`; provenance in-row, no `fact_id`; not a coverage unit; outside the `nodes`/`edges` catalogs): `analysis_invocations` (method, parameters as canonical JSON, projection digest, seed, diagnostics), `findings`, `finding_members`, `witnesses` (path steps keyed by node ids, `edge_id` as lineage), `evidence` (evidence_id → one of: fact, span, passage, example, fixture run, with resolved text), `assertions`, `assertion_support` (assertion → finding / evidence, role `support` or `scope`), `briefs` (with `review_state`, outside `brief_id`), `brief_assertions`, `brief_members`, `brief_documents`, `assertion_policy`; `usage_patterns` with Pass C | 1 |

Deferred: CFG, dataflow and alias tables (§1.3, §13). Type structure and `record_fields` are built in C4 (ADR-0014).

> Decision: ADR-0014, ADR-0012, ADR-0015, ADR-0019

### §3.3 Physical profiles

**Tested** (`every_table_round_trips_through_delta_exactly`; C6 review, 2026-09-23). Every data
file is written with **zstd level 3** (H1 P6, `delta::writer_properties`; `data_files_are_zstd`),
with Parquet's default dictionary encoding and page statistics.

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
| Embedding vector | `FixedSizeList<Float32, 4096>` | `List<Float32>` in `embedding_cache` (child renamed `element`) | length 4096, finite, unit norm, checked on read |

**Conversion happens only at the Delta boundary**, checked in both directions.
- **Tested** (spike S6, 2026-09-22): FixedSizeBinary is written as generic BINARY and read back
  through the DataFusion provider as `BinaryView`. A direct `BinaryView → FixedSizeBinary` cast is
  unsupported.
- **Read path:** BinaryView → Binary → FixedSizeBinary(16), two `cast_with_options` steps with
  `CastOptions { safe: false }`. A wrong length is an error.
- **Timestamps:** µs normalization is lossy for ns, so only µs is ever written.

> Decision: ADR-0012

### §3.4 Identity rules

- **A qualified name is a label, not an identity.** Package versions, roots, stubs and
  redeclarations stay distinct; source and stub are linked by `STUB_FOR`.
- **A span alone is not a node ID.** Identity includes the syntax kind and the structural
  occurrence path.
- **Provider-local IDs** are mapped through `(run, module, provider kind, local key)`.
- **Codebooks are append-only.** Codes are never regenerated or reordered.
- **Source coordinates** are byte offsets into the exact UTF-8 parser input: the text Pyrefly
  loaded and parsed (BOM included), which is also the text the Ruff walk sees (§4.2.2). Other
  coordinates are converted, never assumed to match. Pysa locations (1-based line, 1-based UTF-8
  byte column) are converted with that module's own `LineIndex::offset(.., Utf8)`, the exact
  inverse of how Pyrefly produced them (**Tested**, spike S5: 29,279 of 29,279 exact).
- **Type variables keep binder identity:** two unrelated parameters named `T` are distinct.
- **Deduplicate repeated ingestion of the *same* assertion only.** Assertions from independent
  providers are separate facts, even when they agree.

> Decision: ADR-0012

### §3.4.1 ID derivation

**Proposed**, except the extractor's ids: **Implemented** in slice 1 and pinned by an id-recipe
snapshot (`extractor_id_recipes_snapshot`, **Tested** 2026-09-22).

**Encoding.** `BLAKE3("lctx-id/v1" ‖ len‖kind_tag ‖ len‖field …)`. Lengths are u64
little-endian. IDs are the first 16 bytes; digests are all 32. A version bump in the tag is a
migration (DM-51).

| ID | Derived from | Scope |
|---|---|---|
| `release_id` | the release distributions' names and versions and the sorted (path, sha256) of their `RECORD`-verified analyzer-readable files: what is analyzed, never the lock entry (a source tree: its label) | global |
| `node_id` | `release_id`, path within the release, then the structural occurrence path (syntax nodes: ruff `NodeKind` names and child ordinals) or the qualified name and occurrence (declarations) | stable across snapshots and runs. Syntax ids are producer-scoped: a ruff bump may rename a node kind |
| `context_id` | Python version, platform, ordered search and site-package paths (root-relative), config digest, environment digest (the installed distributions' dist-info names and every analyzer-readable file's site-relative path and content), lock digest | global |
| `producer_id` | tool, tool revision, adapter build digest | global |
| `run_id` | `release_id`, `context_id`, `producer_id`, sorted enabled families, the producer's own config digest | global |
| `fact_id` | `run_id`, record kind, subject id(s), canonical payload bytes. Provenance is outside the id: the same payload with different provenance fails the run (Tested) | per run |
| `finding_id`, `assertion_id`, `brief_id`, `evidence_id`, `invocation_id` (ADR-0019) | kind, subject `node_id`(s), canonical payload. **No config digest**, so an unchanged finding keeps its ID when parameters change; ablation diffs are joins. A finding's payload names its witness steps by call-site and callee node ids, never by `edge_id` (producer-scoped, ADR-0014 O6). An assertion's includes its sorted supports; a brief's, its seed, applicable case and sorted (section, ordinal, assertion); `review_state` is outside it. An invocation's is its method, parameters digest, projection digest, subject and seed. `capability_id` = `brief_id` | content |
| `edge_id` (C1, **Implemented** and **Tested**: `the_catalogs_hold_every_graph_shape`, `the_catalogs_are_the_same_across_runs_order_and_location`; byte-identical on a pilot rerun and relocation, C6 review 2026-09-23) | `edge`, edge kind, source and target node ids, then the kind's discriminator: an ordinal, or for a provider row joined at one site its run-independent payload digest (`pysa_calls.payload_id` = `pysa-call` over the row's payload). Never a `fact_id` | stable across snapshots and runs |
| Role and derived node ids (C1, C3, C4, C5 **Implemented**) | Argument: `argument`, call node, ordinal (Rust). Export: `export`, `release_id`, access path (SQL). Synthetic callable: `synthetic_callable`, module node, Pysa function key (SQL). External module: `external_module`, owner, owner version, module name (Rust), where the owner is the distribution whose `RECORD` lists the file and its version, else `pyrefly-bundled` and the fork revision, else `unowned` and the file's content digest. External symbol: `external_symbol`, the external module id, definition kind, Pysa key (Rust). Its qualified name is a label, because conditional definitions can share one. Reference (C3): `reference`, the name's syntax id. Type term (C4): `type`, kind, detail, class pair and type-variable identity, then each child's role, ordinal, id, parameter name, kind and requiredness; a variable's is its identity alone, a display-only kind's includes its display (Rust; a Merkle id with no SQL form, so no `id:` rule). It is producer-scoped like syntax and external-symbol ids: it hashes Pyrefly's detail text, Pysa class keys and anchor byte offsets, so a Pyrefly bump or an edit earlier in a module renames it. Field (C4): `field`, class node, name (Rust; `id:record_fields`). Document (C5): `document`, release, path. Passage and code block (C5): `passage` or `code_block`, document node, ordinal (Rust; `id:documents`, `id:passages`, `id:code_blocks`) | stable across snapshots and runs for the same inputs; Pysa keys make external symbols producer-scoped, like syntax ids |
| `snapshot_id` | a fresh random 128-bit value per compile attempt | execution identity (DM-12) |
| `content_digest` | sorted `run_id`s (each carrying its `release_id`, and the lock and environment through its context; the `lctx-compiler` run carries the analytics-config digest), compiler digest, embedding spec hash, and a digest of the sorted `(spec_hash, input_hash)` keys the snapshot used. Never the shared `embedding_cache` version, which another library's compile can move (ADR-0017 amendment). Slice 2 has the first two (**Implemented**; equal across a pilot rerun and relocation, C6 review 2026-09-23) | compares reruns |
| `compiler_digest` | the locked engines (DataFusion, Arrow, Parquet, object_store, delta-rs and its kernel, read from `Cargo.lock` by `cpg-core`'s build script), a hand-bumped compiler output version, every derivation query, table contract and validation rule. Stored on every `snapshots` row (**Implemented**, **Tested** by a unit test on each input) | per build |

- **Ids in SQL** (C1, **Implemented** in `cpg_core::udf`, **Tested** by its known-answer and
  plan-time refusal tests). Stage D computes its ids with one scalar UDF, `lctx_id(kind, …)`,
  registered in every session.
  - It implements `IdHasher` exactly: the kind through `IdHasher::new`, and every other argument
    in the `opt_*` encoding (a presence byte, then the length-prefixed value).
  - It accepts Utf8/Utf8View/LargeUtf8, Int16 and Int64 (hashed as i64), Binary, BinaryView and
    FixedSizeBinary, and Boolean. UInt64 (`row_number()`), Int32 and floats are rejected at plan
    time, never cast. The kind must be a non-null text literal, and the result field is
    non-nullable; both are checked when the query is planned (`return_field_from_args`; H1 P7).
    Each argument's type is resolved once per batch, and each row continues a hasher already
    seeded with the tag and kind.
  - Known-answer vectors are shared with the Rust tests, so an id computed in Rust (the
    extractor, a later pass) equals the one computed in SQL.
  - The UDF's recipe is part of `compiler_digest`.
- **Keys are snapshot-qualified.** Uniqueness is checked on `(snapshot_id, key)`, so an identical
  rerun re-emits the same `node_id` and `fact_id` in a new snapshot without conflict (**Tested**
  at pilot scale: a rerun's `nodes`, with `existence_fact_id`, are byte-identical; C6 review R1,
  2026-09-23).
- **Collisions are validator failures** (§8), never silently merged. `nodes` keeps one row per id
  only for the kinds several existence rows legitimately assert (an export read from a `.py` and
  its `.pyi`, a dependency module or symbol, or a type term two runs reference); any other
  repeated id fails `key:nodes` (review O2), and a type term's id with two kinds, details or
  displays fails `unique:type_terms`.
- **One recipe, two places.** The ids the extractor computes in Rust whose inputs are also
  columns (argument, external module, external symbol; C3's scope `H(scope, owner)`, binding
  `H(binding, site, name)` and reference `H(reference, name node)`) are `cpg_schema::id::recipe`
  functions in the UDF's `opt_*` encoding. A generated `id:*` rule recomputes each in SQL on every compile, and
  known-answer values are pinned (review F3).
- **Producer scope.** `call_target` and `higher_order_target` edge ids take Pysa's payload,
  function keys included, so a Pyrefly bump may rename them, like syntax and external-symbol ids
  (review O6).
- **The compiler has its own run** (ADR-0019; built in increment 1 slice 1.4). Analytics and
  synthesis are a run of producer `lctx-compiler` over the library release and its context, whose
  config digest is the analytics config's and whose tool revision is the `compiler_digest`. Its
  `runs` and `producers` rows are written with the raw tables (every input is known before
  Stage B), so `content_digest` includes it. It declares no families.
- **Overloads.** A public callable with `@overload` stubs is **one** declaration node (the
  implementation), with one `signatures` row per overload (`is_overload`) plus the implementation
  signature. Seeds and briefs attach to the declaration. **Implemented** (slice 2): a stub rolls
  up to the first later `def` of its name that is an implementation or that Pysa describes, else
  the group's last stub; Pysa gives one undecorated signature per stub, in source order, and none
  for an implementation.
- **Binding choice follows the analyzer.** Where one file binds a name more than once (a
  `sys.version_info` or `TYPE_CHECKING` branch, a redefinition), the `exports` seed and the
  overload roll-up prefer the `def` Pysa describes (a Stage-C key), because Pyrefly's binding pass
  drops the branches the context decides statically. The other reads `unreachable_in_context`.
  Under `TYPE_CHECKING` that means the typed facade wins over the runtime body, a real choice
  Pass A inherits. Calls inside an unbound `def` still read `missing_evidence` (Pysa has no record
  of them). **Tested** on `derive_cases` (2026-09-22).

> Decision: ADR-0013 (superseding ADR-0007), ADR-0019

### §3.5 Vocabularies and codebooks

**Implemented** and **Tested** (`registry_snapshot`, `codes_are_dense_from_zero_and_names_unique`,
the `codebook:boundaries.reason` case; C6 review, 2026-09-23).

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
| `boundary_reason` | native_unavailable, unresolved_target, unsupported_unpacking, ambiguous_binding, unsupported_control_flow, scope_boundary, budget_reached, missing_evidence, not_requested, provider_disagreement, outside_provider_model (a construct the provider's model does not cover, e.g. a call inside an annotation), syntax_error (facts from a recovered tree), undecodable_source (bytes are not UTF-8), no_source_declaration (a function the provider describes that has no `def` of its own: a synthesized member such as a dataclass `__init__`, or a callable class field), unreachable_in_context (a `def` the analyzer's context never binds) |
| `pysa_unresolved_reason` | the 14 variants of Pysa's unresolved-call reason at the pinned pyrefly, spelled as Pysa spells them; appended when the pin moves |
| `evidence_status` | structurally_observed, documented, statistically_derived, fixture_checked, unresolved |
| `finding_kind`, `assertion_kind`, `analytic_method` | Defined in `cpg-schema` as their consumers land (§9, §10) |
| `node_kind`, `edge_kind` (C1+) | The v1 node kinds (§3.1) and the edge kinds of the registry (§3.8), appended by slice |
| `syntax_kind`, `syntax_field` (C2), `lexical_scope_kind`, `binding_kind`, `static_branch` (C3; `constant` and `combined` appended by H1), `type_term_kind`, `type_arg_role`, `type_role`, `record_kind` (C4) | Defined in `cpg-schema` with their slice. `lexical_scope_kind` is separate from the coverage `scope_kind`. `binding_kind` is the recognizer's binding events: Ruff's kinds it emits plus `del` and the `global`/`nonlocal` declarations |
| `fact_family` additions | `graph` (C1; not a coverage unit), `syntax`, `lexical`, `types`, `docs` as their slices land |
| C5 codebooks | `mention_class` (exact, lexical), `mention_source` (inline code, prose); `scope_kind` `document` (the `docs` family's coverage unit); `source_role` (release, example, test, doc_block; ADR-0015) |
| `boundary_reason` addition (C1) | `variable_origin`: an export whose origin Pyrefly calls a variable-like symbol in a dependency module (a release-module origin targets its `binding` since C3) |
| C3 review appends | `binding_kind` `implicit` (a module's implicit globals, a method's `__class__` cell); `module_origin` `not_found` (an import Pyrefly's finder cannot find: a row, no node) |
| C1 codebooks | `module_origin`, `definition_kind`, `symbol_kind` (Pyrefly's export kinds, an exhaustive match), `derivation_class` |
| slice-1 extraction codebooks | `fact_family`, `scope_kind`, `declaration_kind`, `export_syntax_kind`, `parameter_kind`, `signature_form`, `argument_kind`, `pysa_site_kind`, `pysa_target_kind`, `pysa_callee_kind`, `implicit_receiver`, `ancestry_relation`; values in `cpg-schema`, whose snapshot-tested registry is authoritative |

- **Missing output is not negative evidence.** "Unavailable", "not requested", "failed" and
  "unresolved" are recorded separately.
- **`fidelity` values.** Each value says what structure a fact guarantees:
  - `raw`: text or bytes exactly as found in the source (spans, slices);
  - `native_structural`: the provider's own structure, field for field (a Ruff AST node, a native
    `pyrefly_types::Type`);
  - `normalized_structural`: our structure-preserving derivation;
  - `report_projection`: a provider's documented projection of richer internal state (Pysa's
    structs). What the projection carries is all there is;
  - `display_only`: a display string with no structure.

  A row's fidelity is that of its weakest semantic field.
- **`model_id` is not a codebook.** It is `<producer_id>/<surface>` (e.g. `ruff-ast`,
  `pyrefly-pysa`, `pyrefly-public`, `lctx-compiler/pass-a`), validated against `producers`.
- **Extractor tables use `extraction_mode = native_traversal`.** `report_decode` stays in the
  append-only codebook, unused in v1.
  - Exceptions (ADR-0014):
    - boundary facts compare two surfaces, so they are surface `compare` with
      `relational_derivation` (C1);
    - the lexical recognizer's rows are `recognizer` (C3);
    - docs mentions are `recognizer` (C5).
  - The mode is a parameter of the fact sink, never hard-coded.
- **Pyrefly-sourced values map through exhaustive matches.** Every Pyrefly enum → codebook mapping
  is a `match` with no wildcard arm, so a new upstream variant (a 15th unresolved reason, a new
  `OriginKind`) fails the build instead of degrading silently. `OriginKind` becomes
  `site_detail` text through our own exhaustive match, never upstream's `Display`.
- **`type_role` (C4)** says what an observation types, and follows from the subject: `parameter`,
  `return`, `call_result`, `argument`, `raised`. Whether the type is an annotation's or computed
  is the observation's `declared` flag, not a role. IP L576–623's contextual roles (`expected`,
  `narrowed`, `unnarrowed`, `contextual`, …) append when a consumer needs them; Pyrefly's
  `get_expected_type_trace` already reaches the first.

> Decision: ADR-0014, ADR-0012, ADR-0015

### §3.5.1 Type observations and class order

- `has_type` is a derived edge over `type_observations` that keeps the role on its evidence row.
  It is never an editable copy.
- **Declared types** come from native annotation data, never from TSP `getDeclaredType` (which
  returns the computed type). An observation is `declared` when the subject has an annotation (a
  parameter's `annotation_text`, a `def`'s return annotation): its type is Pyrefly's reading of
  that annotation. A subject without one gets Pyrefly's computed type with `declared` false
  (**Implemented**, C4). `Key::ReturnType` is the computed return, which for an annotated `async
  def` that is not a generator wraps the annotation as `Coroutine[Any, Any, <annotation>]`
  (`return_type_from_annotation`); that one rule is inverted exactly, so the declared return is
  the annotation (C4 review F2; **Tested**).
- **Pyrefly's MRO** is kept exactly as reported. It excludes the class itself and `object`, so it
  is labelled as ancestors, not as a complete runtime MRO. It is never re-derived by
  topologically sorting base edges.

### §3.6 Resolution is a set

**Implemented** and **Tested** (the derived-table snapshots; `modality_follows_the_variant_table`;
C6 review, 2026-09-23).

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
- **Override dispatch is never a single callee.** A Pysa `Target::Overrides(f)` is a `candidate`
  target to `f`, and its resolution has `candidate_set_complete_under_model = false`: any override
  of `f` can be reached, including subclasses outside the release. Pass A never reports it as
  `direct_delegation` without further evidence (§4.2.3).

> Decision: ADR-0012

### §3.7 Coverage and boundaries

- **`coverage(snapshot_id, run_id, scope_kind, scope_node_id, fact_family, status, reason)`**
  - `scope_kind` is release, module or callable.
  - Every family an extractor declares gets a row for every module in scope.
  - A module an extractor never reached is `failed` or `unavailable`, never absent.
- **`boundaries(snapshot_id, fact_id, subject_node_id, fact_family, boundary_reason, detail)`**
  - Resolution issues and analysis stops, with one row per stop.
  - Analytics and briefs read these rows instead of treating "the analysis stopped" as "the
    library has nothing more".
- **Two channels, one fact each.** `coverage` and `boundaries` describe the producer's run. Gaps
  that Stage C/D finds are `reason` columns on the derived rows (§3.2). Where both describe one
  fact (a call with no Pysa record), a rule requires them to agree per call site, both ways (§8).
- **What a projection can cite as completeness** (C1, **Implemented**). These state what the graph
  covers and where it stops:
  - `coverage` per family and module;
  - `boundaries`;
  - the unresolved remainders on `resolutions` and `argument_resolutions`;
  - the `external_module` and `external_symbol` nodes: the edge of the analyzed universe, with no
    outgoing call edges because their bodies are not analyzed;
  - `synthetic_callable` nodes: functions with no body in source (a dataclass `__init__`), so a
    call path ends there (279 call edges on the pilot);
  - `graph_gaps`: the raw rows the graph does not represent yet, with the slice that will. Empty
    since C3, and kept as a published table for the next family with such rows (review O2).

  A projection's spec (§5) states which of them it accepts. A non-finding outside that coverage
  is never read as absence (guidelines §7).

> Decision: ADR-0014

### §3.8 Graph catalog and edge registry

**Implemented** for C1 in `cpg_schema::graph` (registry, catalogs, rules) and `cpg_core::udf`, and
**Tested** (C1, 2026-09-22):
- `cpg-core/tests/graph.rs` on the `graph_shapes` fixture: the catalogs as an insta snapshot; an
  isolate; two parallel call sites giving two edge ids; a self-loop; a cross-file SCC and a
  diamond found by a petgraph projection built from the catalogs, with each arc's lineage; an
  unresolved call with no edge; a higher-order `potential` edge; `f(g(x))`'s argument distinct
  from the inner call; a variable export with its reason; identical catalogs across two locations
  and the reversed module order;
- every registry rule rejects an injected violation: lineage and endpoint from raw rows
  (`compile.rs`), and evidence, support, one-per-evidence, no-parallel and typed targets from a
  doctored catalog (`graph.rs`).

C2 and C3 are **Implemented** and **Tested** (`cpg-core/tests/syntax.rs`: the placed tree of
`syntax_shapes`, and every name of `lexical_shapes` with its resolution and bindings; each new rule
rejects an injected violation). C4 is **Implemented** and **Tested** (the same file: every
observation, type variable, class and record field of `type_shapes`, two unrelated `T`s as two
terms with two binders, the recursive alias as one finite term). C5 is **Implemented** and
**Tested** (`a_corpus_documents_its_library`: the selection, passages, a code block inside an MDX
component, links, mentions by class and an other-library member refused, an unparsable
document's coverage, and the usage run: a test module and a materialized block reaching the
release's own declarations, the release class one term. Also: the same corpus in two places is
one context and run, and another library release is another corpus; a tree shadowing the release
fails; an empty glob is refused; each C5 rule rejects an injected violation,
`the_corpus_rules_reject_their_violations`). This
is how
the typed family tables
become a graph without a second authority, following the operator's guidelines
(`docs/design_review/design_principles/rust_code_intelligence_data_graph_guidelines.md` §2–§4,
§10–§12).

**The registry** (`cpg_schema::graph`) is the one declaration the catalogs, references and graph
rules are generated from (DM-52).
- **Per node kind:** its **existence source**, a relation independent of every column that
  references the node. Its `node_id` column, `module_node_id` and existence `fact_id` column.
- **Per edge kind:**
  - its source relation (a family table or a derivation);
  - its source and target columns, with the allowed endpoint kinds;
  - its **direction meaning**;
  - whether parallel edges are allowed;
  - its **derivation class**: `extracted` (one provider row), `analyzer` (a provider's
    resolution), `joined` (a Stage-C/D join), `recognizer` (our analysis);
  - its evidence and support `fact_id` columns;
  - its ordinal;
  - its `edge_id` discriminator.

**The catalogs** (family `graph`, Stage D, after every other derivation):
- `nodes(snapshot_id, node_id, node_kind, module_node_id?, existence_fact_id)`: one row per node
  of every kind, from the existence sources. So an isolate (a public function nobody calls) is
  present.
- `edges(snapshot_id, edge_id, edge_kind, src_node_id, src_kind, dst_node_id, dst_kind, ordinal?,
  evidence_fact_id, support_fact_id?)`: one row per relationship. Payload, including phase,
  receiver, keyword and role, stays on the source row that the evidence cites.
- Ids come from `lctx_id` (§3.4.1). Edges get no `facts` rows (§B6).

**Edge kinds by slice.** Direction reads "source → target".

| Slice | Edge kind: source → target (derivation) |
|---|---|
| C1 | `declares`: module/class/function → class/function (extracted). `overload_of`: stub → callable (joined). `stub_for`: `.pyi` declaration → the `.py` declaration the `exports` seed rank picks, one shared rank (joined). `has_parameter`: function → parameter, ordinal (extracted). `exports`: export → declaration, external symbol, module or external module, one edge per access file, the file as discriminator (analyzer; parallel). `encloses_call`: owner → call site (extracted). `has_argument`: call site → argument, ordinal (extracted). `call_target`: call site → function, synthetic callable or external symbol (analyzer; discriminator `payload_id`; phase, receiver and modality on the evidence row). `higher_order_target`: argument → callable (analyzer; `potential`). `base_class`, `mro_entry`: class → class or external symbol, ordinal, MRO as reported (analyzer). `overrides`: function → function or external symbol (analyzer). `declared_in`: external symbol → external module (joined) |
| C2 (**Implemented**) | `ast_child`: module, declaration, call site or syntax node → the placed child, ordinal (extracted; one parent per node). `argument_value`: argument → its value's placed node (joined; one per argument of a call outside an annotation). `site_target`: the syntax node or call site at a Pysa attribute, artificial or format-string site → function, synthetic callable or external symbol, **not** `potential` records (analyzer; parallel, discriminator `payload_id`; `synthetic_model` for artificial sites) |
| C3 (**Implemented**) | `owns_scope`: module, declaration, lambda or comprehension → scope. `lexical_parent`: scope → enclosing scope. `binds`: scope → binding, ordinal. `introduces`: binding → the declaration, parameter or placed statement that makes it (recognizer). `reads_binding`: reference → binding in its own or the module scope (recognizer; `candidate` when several). `captures`: reference → a binding of an enclosing function scope (recognizer). `reads_builtin`: reference → the builtin function or class (recognizer). `shadows`: binding → the previous event of its name in its scope (joined). `potential_target`: reference (Pysa identifier sites) or syntax node (Pysa `if_called` at attribute sites) → callable (analyzer; `potential`). `imports_module`: module → the module an import names (joined; one per alias). **Deferred:** `imports_symbol` (Pyrefly's `find_definition` at an alias; reopen when a consumer needs symbol-level import targets, since `exports` already trace re-export origins) and separate `global`/`nonlocal` edges (their effect is the declared scope of the binding and the resolution) |
| C4 (**Implemented**) | `has_type`: parameter, function, call site, argument or `raise` syntax node → type term (analyzer; one per subject; role and `declared` on the evidence row). `type_arg`: term → term, ordinal (analyzer; parallel, discriminator the `type_arg_role`, since a callable returning its first parameter's type joins one pair twice at ordinal 0). `type_class`: term → class or external symbol (analyzer, through `type_class_targets`). `has_field`: class → field, ordinal (analyzer). `field_type`: field → term (analyzer) |
| C5 (**Implemented**) | `contains_passage`: document → passage, ordinal; `contains_block`: passage → code block, ordinal (extracted). `mentions`: passage → export, class or function, at the byte offset (recognizer; parallel; `exact` or `lexical` and the modality on the evidence row). `block_module`: Python code block → its materialized module (joined). A usage call's `call_target` reaches the release's own node directly (the corpus names the release's files by `@path`); `usage_link` (code 35) was retired by the C5 review before any snapshot relied on it |

**Rules** generated from the registry (§8):
- Endpoint kinds: both ends exist in `nodes` with an allowed kind, and `src_kind`/`dst_kind`
  equal it.
- **Node-valued references**, from the registry's `node_columns`: every node-valued column of every
  family table names a node of an allowed kind in `nodes`. The hand-kept `REFERENCES` holds only
  fact, provenance and composite references (slice-2 O8, closed).
- Evidence exists, in the kind's evidence table; support exists.
- `key:nodes`, where one id with two kinds is a collision, and `key:edges`.
- **Lineage from raw rows:** each source row yields exactly its declared edges, or its derived row
  carries a provider's reason. Where the edge is read straight off the same unfiltered table,
  the rule is an edit guard (§8), not a data check.
- **Partition of `pysa_calls`:** every row is a call-site row (lineage), an unresolved remainder
  counted on its `resolutions` or `argument_resolutions` row, a C2 site row (lineage through
  `site_targets`), or an identifier site (lineage through `identifier_targets`). No row is a gap
  since C3; the gaps rule stays as an edit guard over an empty table (review O2; §8). The
  remainders rule rejects a resolution that stops counting its remainder (C6 review F1).
- **Placement (C2):** every declaration, and every call outside an annotation, has its
  `syntax_nodes` row; a node is placed once; a placed child lies within its placed parent, in the
  same module; a Pysa site with no node, or a name with no reference, is a `typed:*` violation
  (never a reason).
- **Lexical (C3):** `id:scopes`, `id:bindings`, `id:references`; `typed:reference_resolutions` (no
  binding, builtin or reason); `typed:identifier_targets`; `typed:import_targets` (a reason only
  from Pyrefly's finder: an import we misname fails it, C3 review F2); every reference's name is a
  placed syntax node. Each rejects an injected violation (`compile.rs`, C3 review F7).
- **Earlier rules without a case until C6** (C6 review F1): `typed:exports`,
  `typed:ancestry_targets`, `typed:override_targets`, five `semantic:*`, `id:arguments`,
  `id:context_definitions`, `id:context_modules`, `placed:call_syntax` and "a run's release has
  modules or documents" now reject doctored views in `each_graph_rule_rejects_a_doctored_catalog`
  (`boundary-has-resolution` a raw mutation in `compile.rs`).
- **Source text and role (ADR-0015):** `semantic:source-text` (the text is present exactly when
  the bytes are UTF-8, with `byte_len` bytes); `semantic:source-role-by-run` (the `release` role
  exactly in a run that declares `exports`). Each rejects a doctored view.
- **Docs (C5):** `id:documents`, `id:passages`, `id:code_blocks`; `typed:mention_targets`,
  a run's release has modules or documents, and a document's release has a run (C6 review O6); a
  run declaring a family has what it covers (`coverage:family-has-scope`: a document for `docs`,
  a module for a code family); `coverage:complete` expects a
  `docs` row per document; `unique:release-paths` (an attempt's releases share no path, so a
  `@path` module reference names one file); `unique:type_terms` (one term id, one kind, detail and
  display, however many runs emit it). Each rejects an injected violation
  (`the_corpus_rules_reject_their_violations`, C5 review F7).
- **Types (C4):** `id:record_fields`; `typed:type_class_targets`; `typed:type_binders`; lineage
  for every observation, term argument, class-bearing term and record field. The three named
  rules reject injected violations on `type_shapes` (C4 review F1).
- **Typed targets:** a null target carries a reason.
- **Ids:** the Rust recipes equal their SQL form (`id:*`).
- A reference whose target is built from its own source column is not generated, because it
  cannot fail; the lineage rules that cannot are declared edit guards (§8).

**The registry as data.**
- `edge_kinds` publishes each kind's derivation class (`derivation_class`: extracted, analyzer,
  joined, recognizer), direction meaning, parallel policy, evidence table and endpoint kinds with
  every snapshot. A projection selects by them from the store, not from its own build (review F6).
- `graph_gaps` publishes each raw row the graph does not represent yet, with its reason
  (`not_requested`) and the slice that will represent it. After C1 these were Pysa's records at
  attribute, artificial and format-string sites (C2) and at identifiers (C3); since C3 it is
  empty. Nothing is left out silently (review F5).

**Measured** (`lctx compile fastmcp`, release build, FastMCP 4.0.5, this Linux host,
2026-09-22):
- 47,145 nodes and 65,176 edges, with every rule passing;
- every call target (17,174), ancestry entry (1,182) and overridden method (1,107) is a typed node;
- 978 exports have a target, and 335 were variables (`variable_origin`), every one a symbol Pyrefly
  itself calls a variable; after C3, 334 of them target their module-scope binding (one keeps
  `variable_origin`: its origin is a dependency);
- 17,282 Pysa rows were published gaps after C1: 8,083 artificial, 1,502 attribute and 1,942
  format-string sites for C2, and 5,755 identifier sites for C3. **After C2** only the 5,755
  identifier sites remain. Every other site lands on a placed node: 8,500 resolve to targets and
  3,027 are unresolved by Pysa itself (`unresolved_target`). Thirteen chained-comparison sites
  that no node spans attach to their comparison;
- **C2 on the pilot** (2026-09-23, as first committed): 83,627 placed syntax nodes; 113,897 nodes
  and 165,064 edges; every rule passing; 14.9 s, 2.91 GB. After its review every expression
  outside annotations is placed: 138,136 syntax nodes. 829 attribute-site records are `potential`
  and are `potential_target` edges;
- **C3 on the pilot** (2026-09-23): 3,667 scopes, 22,230 bindings, 39,950 references and 45,713
  resolutions. **No name is unresolved**: 45,591 resolutions read a binding (760 rows on 734
  references are closure captures) or a builtin function or class, and 122 read a builtin
  variable. Every Pysa identifier
  site lands on a reference (5,675 resolved, 80 unresolved by Pysa). All 4,526 imports resolve.
  `graph_gaps` is empty. 234,328 nodes and 335,442 edges; every rule passing; 17.5 s at 3.91 GB
  peak RSS, validation 3.4 s. The memory now reaches the C6 streaming trigger's question
  (§4.3). **After C3's compact review** (2026-09-23, snapshot `076151d4…`): 119 of those 122
  "builtin variable" reads were the module's own `__name__` and `__file__`, and now read its
  implicit bindings; 3 read real builtin variables (`NotImplemented`, `Ellipsis`). Still no name
  unresolved, now with the builtins, implicit globals and star sets taken from Pyrefly. Every
  import is found; one export keeps `variable_origin` (its origin is a dependency). The lambda in
  `debug.py`'s default now has the class scope as parent. 25,530 bindings (3,300 implicit),
  244,673 nodes and 391,193 edges; every rule passing; 19.0 s at 3.93 GB;
- **C4 on the pilot** (2026-09-23, fork `6a93da34`): 6,162 type terms and 6,013 term arguments;
  42,263 observations: 6,201 parameters (4,768 declared), 2,449 returns (2,284 declared), 13,871
  call results, 19,008 argument values and 734 raised exceptions. 773 record fields: 447 pydantic,
  258 dataclass and 68 `TypedDict`. All 2,416 class-bearing terms resolve to a node. 102 type
  variables, 90 with a binder; the 12 without are anchored in dependency modules. 126 terms are
  `other` (125 `super()` instances). No `types` boundary, no truncated term. 241,373 nodes and
  387,774 edges; every rule passing; the types pass takes 0.13 s; 18.4 s at 3.81 GB peak RSS
  (validation 4.3 s; a run on the unpushed fork clone peaked at 4.12 GB);
  **After C4's compact review** (2026-09-23, snapshot `69bc2a1e…`, a fresh store: the migration
  changes `type_terms`): 6,228 terms; the 792 annotated `async def` returns now read their
  annotation (0 declared coroutines; 28 unannotated ones keep Pyrefly's computed coroutine); 97
  anchored type variables, 92 bound to a release declaration and 5 anchored in dependencies
  (`scope_boundary`), and 0 variables split across terms (7 were); 17 enum literals carry their
  class; 771 record fields (2 inherited, method-assigned rows gone); 244 `types` boundaries for
  subjects Pyrefly does not type, in 65 modules whose `types` coverage is `partial`. 247,858 nodes
  and 397,961 edges; every rule passing; 20.1 s;
- **C5a on the pilot** (2026-09-23, snapshot `61ade81e…`): the corpus is FastMCP 4.0.5's tree at
  `004bf15a` (594 MDX pages, of which the selection keeps the 148 guide pages: `v2/`, `v3/` and the
  generated `python-sdk/` reference are excluded). 148 documents (144 parse; the 4 `snippets/`
  React components are `unavailable`), 1,608 passages, 1,357 code blocks (960 Python), 5,766
  links, 3,363 mentions: 77 exact and 3,286 lexical candidates, every one with a target. 247,786
  nodes and 397,521 edges; every rule passing; the documents take 0.20 s; 20.7 s at 3.91 GB;
- **C5b on the pilot** (2026-09-23, snapshot `82e00ed6…`): the usage run compiles 1,512 modules
  (125 examples, 427 tests and 960 materialized code blocks, every one parsing); 1,464 library
  definitions the usage code reaches all linked to the release's own (probe P4: the same keys), and
  18,529 usage call edges reached the release through them. 35 usage modules are `types`-partial.
  The whole CPG: 907,845 nodes and 1,452,970 edges; every rule passing; 46.3 s at 6.7–7.7 GB;
- **after the C5 review** (2026-09-23, snapshot `ab6d98a3…`): the corpus names the release's files
  by `@path`, so 18,564 usage call edges and 3,605 usage imports end on the release's own nodes
  with no link table, no class term names a release module by its dotted name, and `FastMCP` is one
  term; the corpus search path is `$release | $venv/lib/python3.14/site-packages`. Mentions are
  unchanged (77 exact, 3,286 lexical). 905,648 nodes and 1,449,162 edges;
- 329 dependency modules: 249 site-packages modules, all with their distribution, and 80 from
  Pyrefly's bundled typeshed;
- 1,913 external symbols;
- `nodes` and `edges` derive in 0.05 s and 0.08 s, and validation takes 1.3 s;
- the whole compile takes 14.4 s wall time at 2.49 GB peak RSS, against 7.9 s and 1.61 GB before
  C1. Of the difference, the Pyrefly check of the referenced dependency modules at
  `Require::Everything` and their definitions take 4.4 s, and validation (now 261 rules) most of
  the rest (review O4);
- `attrs` 26.1.0, which failed `key:edges` before review F2, publishes (4,061 nodes, 5,184
  edges, 1.5 s);
- two runs give one `content_digest`.

**What the catalogs never hold:**
- transitive closures, paths or all-pairs results (guidelines §7);
- a merged "best" target in place of a candidate set;
- graph-local indices (§5).

> Decision: ADR-0014

---

## §4 Pipeline

### §4.0 Library acquisition and the run contract

**Implemented** in `libraries/`, `cpg_extract::library` and `crates/lctx`, and **Tested**
(`crates/cpg-extract/tests/library.rs`, `crates/lctx/tests/acquire.rs`, `crates/lctx/src/propose.rs`,
`just pilot`, 2026-09-22), after a standard review corrected the first identity recipes and the
acquisition flags. Source: IP L1146–L1170 (Stage A), L536 (run supporting tables).

**A library is data.** Every analyzed library, the pilot included, is a committed uv project
`libraries/<name>/`. This is the one production path for any Python library, and none of them
needs to be a dependency of this project.
- `pyproject.toml`: a virtual project (`[tool.uv] package = false`) with exactly one pinned
  requirement and an exact `requires-python`.
- `[tool.lctx] release`: the first-party distributions whose code is compiled. Everything else
  installed is dependency context, analyzed only as far as imports reach.
- `[tool.lctx.source]`: the upstream repository, tag and the full 40-hex `commit` the tag names
  (a tag can move), for docs, examples and tests. Stage A checks the tag names the locked
  version. `documents`, `documents_exclude`, `examples` and `tests` select the corpus by glob
  from the tree's root, in globset's syntax (`*` and `?` within one name, `**` across directories,
  `[…]`, `{a,b}`; H1 C2); a module's role is the key that selected it, and a file both `examples`
  and `tests` select is refused (ADR-0015). The tree is walked once (walkdir), dot-directories
  skipped, **no link followed**: a symlink a glob selects is refused, naming it, and so is a
  directory link that could hold a selection (it and a glob's literal prefix lie one under the
  other) unless an exclude covers it. An exclude covers a link only when it names the link, or
  matches any name under it (tested with two unrelated probe names); `documents`, `examples` and
  `tests` each have an `_exclude` list (ADR-0018; H1 review F5; **Tested**:
  `symlinks_in_a_tree_are_refused_unless_excluded`, with a partial exclude that does not cover and
  a loop under an excluded path that is harmless). A source tree (`Release::from_tree`) refuses
  any link. `lctx acquire` fetches the tree
  hermetically (every `GIT_*` variable removed, no system or global git configuration, no
  prompts; `git init`, a shallow fetch of the one commit, checkout, then `rev-parse HEAD` checked
  every time; no ambient git attributes, `core.autocrlf` off) into
  `build/sources/<name>/<commit>`; **Tested** by a stub `git`. `pyproject.toml` is read through typed serde structs:
  `[tool.lctx]` and `[tool.lctx.source]` refuse unknown keys (a misspelled `[tool.lctx.sourse]`
  fails, naming it; H1 C3) and mistyped values, and each include glob must select a file. The corpus release's id hashes its label
  (`repository@commit`), the library's `release_id` (its vocabulary and the files it reaches) and
  every selected file's path, content and role. It runs in the library's environment under its own
  context (the tree ahead of site-packages on its search path, each entry relative to its root).
- `.python-version`: the exact interpreter.
- `uv.lock`: the acquisition lock. Every distribution of the closure with the sha256 of each of
  its artifacts, reviewed and committed.

**Acquisition** is `lctx acquire <name>`: `uv sync --project libraries/<name> --frozen
--no-install-project --no-config --python <.python-version> --link-mode copy` into
`build/envs/<name>`.
- `--frozen` never re-resolves, and uv verifies every artifact hash.
- `--no-config` ignores user and system uv configuration, `--python` enforces the pin, and
  `--link-mode copy` keeps the environment's files from sharing inodes with the uv cache and other
  environments (each file's link count is 1 on the pilot).
- Every other `UV_*` variable and `VIRTUAL_ENV` is removed from uv's environment.
- **Tested** by a stub `uv` that records its arguments and environment.
- `lctx acquire <name> --reinstall` rebuilds every package: the remedy when Stage A finds a changed
  file.
- The environment is gitignored and rebuildable; the lock is the record.
- `lctx library init` locks without `--no-config`: a library's own `[tool.uv]` applies there, and
  the lock is reviewed.

**Stage A** (`cpg_extract::library::acquired`) reads the definition, the lock, `pyvenv.cfg` and
every `*.dist-info`, with no network and no interpreter. It works under one equivalence: **the
analyzer-readable bytes**, the files Pyrefly's module finder reads (`.py`, `.pyi`, `py.typed`;
never `.pth`).
- Every installed distribution must be the version the lock names, and the interpreter
  `.python-version`'s; otherwise the attempt stops.
- **Every** distribution's analyzer-readable `RECORD` entries must match their sha256, in the
  release and in its dependencies (the remedy: `lctx acquire --reinstall`).
- A release distribution the lock records without artifact hashes (a git or local source) is
  refused.
- The release's modules are exactly its distributions' `.py`/`.pyi` `RECORD` entries. Nothing
  walks a directory, and the root is site-packages.
- `release_id` hashes the release distributions' names, versions and verified content (§3.4.1). A
  re-listed artifact or a dependency-only upgrade leaves it unchanged.
- A source tree (a fixture or a local checkout) is the other input, `Release::from_tree`, whose
  `release_id` hashes its label: one label on two trees gives one id.

**Adding and upgrading** (`libraries/README.md`):
- `lctx library init <name> --requirement REQ` writes the definition, locks it and acquires it. It
  proposes `release` as the requested distribution plus every installed distribution that shares
  a repository root (`host/owner/repo` under a source label; sponsor and funding links never
  count). **Tested** by unit tests over METADATA. Observed, not a repo test (2026-09-22): for
  FastMCP it proposed exactly the three distributions, and `attrs` 25.3.0 went from nothing to a
  published snapshot in 0.8 s.
- Upgrading is: edit the pin, `uv lock --project libraries/<name> --upgrade-package <dist>`,
  review the lock diff, `lctx compile <name>`. For FastMCP the skill moves with it (§1.4).

**Context.** The context records:
- the Python version (from `pyvenv.cfg`) and platform;
- the **ordered** search paths;
- the **environment digest**: the installed distributions' dist-info names (which carry their
  versions) and every analyzer-readable file's site-relative path and content digest. `RECORD`
  lines outside site-packages (console scripts carry the environment's absolute path) and files
  Pyrefly never reads stay out, so a moved environment keeps its identity and every
  analyzer-visible change moves it. It costs well under a tenth of a second on the pilot;
- the **lock digest**;
- the digests of every configuration file an analyzer receives.

**Explicit inputs only.** Extractors receive their environment and configuration as arguments.
- **Pyrefly reads only the environment's `site-packages`.** It never runs the interpreter.
- **Pyrefly's configuration is a constructed value, not a discovered file** (§4.2.1):
  - explicit search path and site-package path;
  - explicit Python version and platform;
  - heuristics, walk-up fallback and the interpreter query all disabled.
- **Recorded.** `contexts` stores the digest of the configured `ConfigFile` (keys sorted:
  DataFusion turns on serde_json's `preserve_order`, and no digest may depend on the build
  graph), the search and site-package paths **relative to their roots** (release, environment),
  the sys info from its fields (not `Debug`), the environment digest and the lock digest. Where a
  checkout, tempdir or environment sits never changes an identity; what the dependencies contain
  always does (**Tested**: two fixture locations, two acquired environment paths with
  location-dependent `RECORD`s, two environments, an unowned stub inside a package, a loose file,
  a lock-only change; on the pilot, two environment paths give one `content_digest`; and at
  pilot scale for both runs, a freshly synced environment in another directory plus a copied
  source tree give byte-identical `contexts`, `nodes` and `edges`, a C6 review observation
  (R2, 2026-09-23), not a repo test). A changed
  analyzer-readable byte a `RECORD` owns is refused.
- `releases` and `distributions` record the library, requirement, lock digest, release
  distributions, installer and every installed distribution (§3.2).

So nothing ambient can change an answer without changing `context_id` (G4). That covers `PATH`,
`VIRTUAL_ENV`, `PYTHONPATH`, `CONDA_PREFIX`, the working directory and an upward
`pyproject.toml`. **Tested** (spike S2, 2026-09-22): no process was spawned, and output was
byte-identical with each of them perturbed.

**Run.** A run is one producer applied to one context for a declared set of families under one
analysis configuration. `runs` records that. `producers` records the tool, the revision (for the
extractor: the fork revision and patch digest, which `just deps` checks against `Cargo.lock` and
the patch file, and the ruff line) and the adapter build digest (an output version bumped by hand
when mapping output changes, which the variant and id snapshots show).

**Measured** (`just pilot`, release build, warm environment and uv cache so acquisition is a
no-op, this Linux host, 2026-09-22): FastMCP 4.0.5, 275 modules, 103 distributions; acquire +
extract 7.1 s, the whole compile 7.9 s wall time, 1.61 GB peak RSS. Two runs, and two environment
paths, give the same `release_id` and `content_digest`.
- **After C1 (ADR-0014)** the compile takes 13.6 s at 2.53 GB. `lctx compile` now prints each
  stage (§4.3): acquire 0.01 s, Stage A 0.04 s, the release check 2.1 s, per-module extraction 4.5 s
  (the Pysa collectors 4.4 s, the Ruff walk 0.08 s), the dependency check 3.7 s, dependency
  definitions 0.7 s, the Delta stages 1.6 s.
- A published snapshot is inspected with `lctx query --store DIR --snapshot HEX "SQL"`: read-only,
  every table at its recorded version, filtered to the snapshot (§6.2). `lctx compile` prints
  the attempt's id first, and `--unpublished` reads an attempt validation rejected at the commits
  carrying its own `lctx.snapshot_id`, found in each table's kept log whatever was written after
  it; a table it did not write is left out, so a query naming it fails (`attempt_versions`;
  ADR-0017, H1 review F2; `a_rejected_attempt_is_inspected_at_its_own_commits`). For inspecting a
  failure, never for a reader.

> Decision: ADR-0013 (superseding ADR-0007), ADR-0012, ADR-0015, ADR-0017, ADR-0018

### §4.1 Stages

| Stage | Owner | Output |
|---|---|---|
| A. Source and analysis universe | uv (`lctx acquire`) + `cpg_extract::library` (§4.0) | the verified release files and context; `releases`, `distributions`, `source_files` |
| B. Typed provider facts | Pyrefly and Ruff in-process (§4.2) + Arrow builders (§4.3) | raw family batches, written to Delta |
| C. Provider-local identity | A DataFusion name-span join (`cpg_schema::derived`) | `provider_node_map`, its keys checked unique and injective before publication (after use by D, which is safe because nothing publishes on failure) |
| D. Semantic relationships | DataFusion over the written raw tables, written back through Delta (§4.3) | derived family tables and views |
| E. Projections and analytics | DataFusion (projection SQL on the attempt's session) → `lctx-analytics` (petgraph / leiden-rs / FCA; Arrow in, Arrow out) → the attempt's write path (§5, §9) | `findings` family, provenance in-row (ADR-0019), with lineage |
| F. Synthesis | `cpg-core::synth`: evidence, kind policy and status propagation in DataFusion; Rust templates + extractive selection (§10); brief documents embedded through the cache | assertions, briefs |
| G. Publication | Rust (§6) | `snapshots` row; serving bundle |

Unmapped rows stay in the derived table with a null node and, where one applies, a reason
column. No inner join drops them.

> Decision: ADR-0012, ADR-0019

### §4.2 Extraction

**Labels.** A line that cites a spike result (S1–S7, `spike/pyrefly-inproc`, FastMCP 4.0.3,
2026-09-22) is **Tested** or **Measured**. The rest is **Implemented** in `cpg-extract` and
**Tested** by `crates/cpg-extract/tests` (slice 1, 2026-09-22), where each test names the claim
it checks: the variant table (every site kind, `is_attribute`, potential remainders), module and
class keys, `__all__` forms, `_invalid/` modules, BOM/CRLF offsets against the stored bytes, two
install locations, two dependency environments, module order and cross-process determinism, the
id recipes, the panic abort, the ambient refusal and harness equivalence with the CLI. The
`catch_unwind` and per-module-thread ban is an ast-grep rule. Pyrefly is linked from the pinned fork (§B8). A run is one call of the driver over one context, and everything below happens in
one process.

| Provider surface (`model_id` suffix) | Mode | Raw tables (v1) |
|---|---|---|
| Ruff `=0.0.11` walk over Pyrefly's parse (`ruff-ast`) | `native_traversal` | `declarations` (with docstrings), `export_syntax`, `parameter_syntax`, `call_syntax`, `arguments`; `syntax_nodes` (C2) |
| Pyrefly's Pysa collectors, in memory (`pyrefly-pysa`) | `native_traversal` | `parameter_semantics`, `pysa_calls`, class ancestry; `type_observations` (increment 3) |
| Pyrefly's public-name helpers (`pyrefly-public`) | `native_traversal` | `public_names`. **This defines "public"** |
| Our local-binding recognizer over the Ruff AST | `recognizer` | `lexical` (C3) |

### §4.2.1 Driver

1. **Refuse ambient knobs.** The driver exits before any analysis if `PYREFLY_STACK_SIZE`,
   `PYREFLY_FIXPOINT_DETAILS` or any `PYSA_DUMP*` is set; clearing them would need `unsafe`
   (edition 2024), which the workspace forbids. Every path passed in is absolute, because Pyrefly
   reads the working directory to absolutize relative paths.
2. **Configuration.** Build a `ConfigFile` with:
   - `search_path_from_args`;
   - `disable_search_path_heuristics: true`, `disable_project_excludes_heuristics: true` and
     `enable_fallback_search_path: false`;
   - `python_environment.{python_version, python_platform, site_package_path: Some(..)}`;
   - `interpreters.skip_interpreter_query = true`;
   - no build system.

   `configure()` must return no errors. `ConfigFinder::new_constant` rules out discovery and
   walk-up.
3. **State.** `State::new(finder, ThreadCount::Inline)` on a driver-owned thread whose stack size
   is part of the producer config (the spike used 512 MiB). The check and the lazy solves all run
   on that thread; it is the producer's config digest, so changing it changes `run_id`. One
   thread is a precaution (upstream's cycle placeholders are per thread) with no difference
   shown. FastMCP is identical at `Inline`, `NumThreads(1)` and `NumThreads(4)` (S3). The
   import-cycle fixture (a return-type cycle and a global cycle whose solved types reach
   `pysa_calls`) is identical at `NumThreads(8)`, 6 runs in each module order (probe,
   2026-09-22); Pyrefly 1.3.1 iterates cycles to a fixpoint. **Tested:** sorted and reversed
   module order and separate processes give identical output.
4. **Handles.** One per project module, from `cfg.handle_from_module_path`, sorted by module name.
5. **Run.** Install a `PysaReporter` with `write_files: false` and `ModuleIds::new(&handles)`, then
   call `transaction.run(&handles, Require::Everything, None)`. Keep the reporter installed during
   extraction (borrow it with `pysa_reporter()`), because dependency modules solve lazily.
6. **Extract** per module, in sorted order (§4.2.2–§4.2.3). Then emit coverage (§3.7).

**Measured** (spike S7; FastMCP 4.0.3, 257 modules, release build): 5.4 s cold at `NumThreads(1)`
(2.5 s check, 2.8 s extraction), peak RSS about 918 MB; 4.1 s and about 765 MB with `Inline`.

### §4.2.2 Syntax: one walk over Pyrefly's parse

- **Order (C2).** Per module the Pysa collectors run, then the walk. Placement depends on the
  source alone: every statement, clause and expression outside annotations is placed
  (`syntax_nodes`, §3.2; C2 review), and Pysa's sites are matched to those nodes in Stage C.
- **One walk.** A single `SourceOrderVisitor` walks `Transaction::get_ast(handle)`, the unmodified
  ruff parse Pyrefly analyzed, which is kept at `Require::Everything`. The text is
  `get_module_info(handle)`'s contents. There is no second parse.
- **Built-ins used:** `SourceOrderVisitor` with `walk_annotation`, `Arguments::iter_source_order`,
  `ArgOrKeyword` and `StringLiteralValue::to_str`.
- **Ours:**
  - the parameter list (the five lists in declaration order, which is source order) and the
    docstring check (a first-statement string literal), a few lines each;
  - the structural occurrence path (each ancestor's ruff `NodeKind` name and child ordinal).
    Ruff's `node_index` is always unset, so it can't be used;
  - the qualified-name stack;
  - the `@overload` decorator match.
- **Recovered and unreadable files.** A module whose acquired bytes fail our own UTF-8 check is
  `unavailable` for every family; Pyrefly would load it as an empty module, which must not read as
  "no API". Parse errors are read from Pyrefly's per-module errors (the `parse-error` kind). A
  recovered tree still yields facts, but **every** family of that module is `partial`, with a
  `boundaries` row, so recovery artefacts never read as complete.

### §4.2.3 Semantics: Pyrefly's own collectors

- **Collectors.** Per module: `PysaResolver::new`, `ModuleAnswersContext::create`,
  `collect_captured_variables_for_module`, `create_reversed_override_graph_for_module`, then
  `export_module_definitions` and `export_module_call_graphs`, the functions behind
  `--report-pysa`. The in-memory structs equal the CLI's JSON (S4: 257/257 modules; definitions
  equal as sets).
- **Locations → bytes.** Every `PysaLocation` is converted with its module's `LineIndex` (§3.4).
- **Join key.** `pysa_calls` joins `call_syntax` on the **full call-expression range**. This
  matched 13,104 of 13,292 calls (S5). Every unmatched call is inside an annotation, which Pysa's
  call model does not cover. Each becomes a `boundaries` row with `outside_provider_model`.
- **Mapping** (§3.6). Every Pysa variant has one row. The mapper is exhaustive, so a variant
  missing here fails the build:

  | Pysa variant | Row | Phase | Modality | Origin |
  |---|---|---|---|---|
  | `call_targets` with `Target::Function` | call target | `call` | `definite` if it is the only target and nothing is unresolved, else `candidate` | analyzer_assertion |
  | `Target::Overrides(f)` (any list) | candidate target to `f`; the resolution has `candidate_set_complete_under_model = false` | the list's phase | `candidate` | analyzer_assertion |
  | `init_targets`, `new_targets` | call targets | `init`, `new` | as `call_targets` | analyzer_assertion |
  | `higher_order_parameters[i]` | target attached to argument `i` | `call` | `potential` | analyzer_assertion |
  | `if_called` (identifier or attribute) | target on a `Reference`, and its unresolved remainder | `call` | `potential` | analyzer_assertion |
  | `property_getters`, `property_setters` | call targets | `property_get`, `property_set` | as `call_targets`, but at most `candidate` when `is_attribute` | analyzer_assertion |
  | `AttributeAccessCallees.is_attribute` (some flow reads a plain attribute) | a column on the attribute access's rows | — | — | — |
  | `ArtificialCall`, `ArtificialAttributeAccess`, format-string callees | as the callee kind above, keeping the `OriginKind` | as above | as above | synthetic_model |
  | `Unresolved::True(reason)` | an `unresolved` row with the reason: `has_unresolved_remainder` on the resolution | `call` | `definite`, or `potential` under `if_called` and higher-order lists | as the site |
  | receiver fields (`implicit_receiver`, `receiver_class`, `implicit_dunder_call`, class and static method flags) | columns on the target row | — | — | — |
  | `Target::FormatString`, `Return` shims, `global_targets`, `captured_variables`, `return_type` | **not carried** in v1: synthetic or no consumer | — | — | — |
  | `Define` | **not carried**: it links a nested `def` to the function it creates, which `declarations` already records | — | — | — |

- **Unmatched calls.** The walker marks calls inside annotations (`visit_annotation`). An
  unmatched call there is a `boundaries` row with `outside_provider_model`. Any other unmatched
  call is a `missing_evidence` boundary, and that module's `calls` coverage is `partial`.
- **Module and class keys.** Every Pysa row carries its file's `module_node_id` (a `.py` and its
  `.pyi` share a module name). A reference resolves through Pysa's `ModuleId` (never stored) to a
  *module ref*: `@<release-relative path>` for a release file, else the module name (one file per
  name in a context). Classes are `<module ref>:<Name>#<ClassId>`, functions
  `<module ref>::<key>`; Stage C takes spans from the `pysa_functions` and `class_ancestry` name
  spans. **Tested** (`pysa_keys`: a `.py`/`.pyi` pair, two nested `Config` classes).
- **Set-valued lists** (`captured_variables`, a union's `class_names`) come out of hash sets in
  varying order. Every record set is sorted by its declared key.
- **Public names.** Per public module (`is_public_module`): `explicit_dunder_all_names` if
  present, else local definitions plus explicit re-exports, each traced by `trace_export_origin`
  to a row (access path, origin, `via_dunder_all`). The flattened set must equal
  `compute_public_fqns` (behind `coverage report --public-only`) or the run fails. Pyrefly stays
  the only definition of "public".
  - **A completeness detector replaces Ruff's corroboration.** Pyrefly reads a non-literal
    `__all__` (`sub.__all__ + [...]`, a call) as absent or in part, a blind spot every check
    sharing its code shares. So `exports` is `partial`, with an `outside_provider_model`
    boundary, when `unresolvable_dunder_all_range()` is set or the Ruff walk finds an `__all__`
    that is not a literal list or tuple of strings.
- **Fidelity.** Pysa-model facts are `report_projection`, in memory or not: Pysa's types are a
  display string, scalar properties and class names, and `parameter_semantics` keeps all three
  (a row keeping only the string would be `display_only`). Native `pyrefly_types::Type`
  (`native_structural`) is reachable through `Answers` when a consumer needs it (§13).

### §4.2.4 Binding rule (conservative)

- **The rule.** Any binding of a parameter's name that appears lexically before a guard or
  forwarding site in the same scope marks that site `ambiguous_binding` (a boundary).
- **The motivating case** is pyarrow's `write_dataset`. Its `schema` guard only applies to
  caller-supplied scanners, because earlier branches rebind `schema`.
- **Pyrefly's flow-sensitive `Bindings` are not used for this rule.** Its binding pass drops
  branches it decides statically (`TYPE_CHECKING`, `sys.version_info`), so a rebinding there is
  invisible.

**Deferred:** Ruff's full semantic model, which nothing public drives (§13).

### §4.2.5 Failure, determinism and the parity oracle

- **Panics abort the attempt.** Any panic in code that touches Pyrefly (`run`, the collectors, the
  public-name helpers, lazy solves during extraction) aborts it, and nothing is published (§6.1).
  There is no `catch_unwind`: Pyrefly treats its state as unsupported after any panic (a poisoned
  lock, unpublished cycle answers), so continuing with the next module is unsafe.
  - Load and parse errors are not panics; they still become coverage rows (§4.2.2). Per-module
    isolation would need ADR-0012's Option 4, a separate process.
- **Determinism oracle.** Reruns, reversed module order, separate processes and perturbed ambient
  variables give byte-identical sorted tables (S2, S3, fixture tests).
- **Harness-equivalence oracle.** A test runs the pinned Pyrefly CLI (the `uv` dev group, same
  revision) with an equivalent generated `pyrefly.toml` and asserts, per project module, that the
  in-process Pysa structs equal its `--report-pysa-format json` output as sets (`module_id`
  removed), and that the public set explains its `--public-only` report (an exact match or a
  public parent prefix).

  It shares the collectors with the CLI, so it checks our driver (configuration, reporter
  lifecycle, lazy solving), not the correctness of Pysa. It is **Tested** (S4, and a nextest test
  on two fixtures, 2026-09-22). The CLI is never a production input.

### §4.2.6 Upgrading Pyrefly

1. Rebase the fork commit onto the new tag and regenerate `third_party/pyrefly-<ver>.patch`.
   - A **logic change** is anything beyond visibility changes, accessors that borrow or compose
     existing upstream queries, and fields whose default reproduces upstream behaviour.
   - A patch that needs a logic change, or grows past about 60 changed lines, needs an ADR
     (ADR-0012's revisit trigger).
2. `just deps` checks that `Cargo.lock`, the driver's `PYREFLY_REV`/`PYREFLY_PATCH_SHA256` and
   pins.md name one revision, that it is the tag plus the patch, and that every `env::var` read
   in the pinned source is classified against the refused list (§4.2.1).
3. Move the ruff pin to the line the new Pyrefly compiles against (`pin-check`).
4. Fix compile errors in the mappers, and append codebook values where exhaustive matches demand
   them. Record how many lines the Pyrefly-facing module changed, because port cost is also a
   revisit trigger.
5. Run the harness-equivalence and determinism oracles, and record the pin with its date in
   `docs/pins.md`.

> Decision: ADR-0012

### §4.3 Fact construction and persistence

**Implemented** in `cpg-schema` (build, canonicalize, ids, derivation SQL, rules) and `cpg-core`
(create, open, write, derive, validate, publish, read), and **Tested** there (slices 1–2,
2026-09-22): every table round-trips exactly through Delta; open refuses drifted or missing
CHECKs; the helper refuses writes; the `INSERT INTO` bypass is asserted; the derived tables are
snapshot-tested on three fixtures; each rule kind rejects an injected violation and nothing
publishes. This section says which built-in owns each step; §6 and §8 hold the protocol and the
rules.

| Stage | Built-in | Ours |
|---|---|---|
| Build | Typed builders against the table's `cpg-schema` `SchemaRef` (`FixedSizeBinaryBuilder`, `Int16Builder`, `BooleanBuilder::append_option`, `GenericListBuilder`, `StructBuilder`). `RecordBatch::try_new` with default options is the local type check: exact types, nested names, nullability, metadata | One builder per table. The arrow-json serde path is tests-only: it expects hex for `FixedSizeBinary` |
| Canonicalize | `lexsort_to_indices` + `take_record_batch` on the table's declared **total** key, over the in-memory batch (Arrow's sort is unstable). The Delta writer may store the rows in another order (it fans partitions into one writer), so readers sort (`read_at`, every rendered query) and never rely on storage order (H1 review O1) | Key declarations |
| Ids | The `blake3` crate (`=1.8.6`, shared with Pyrefly) inside one `IdHasher` (§3.4.1). Not `RowConverter` bytes: the encoding may change between releases. Not SQL `digest`: it can't write length prefixes | `IdHasher` |
| Create | `DeltaTable::create().with_columns(..).with_configuration_property(TableProperty::AppendOnly, Some("true"))` plus, from C1, `EnableExpiredLogCleanup = "false"` and `LogRetentionDuration = "interval 36500 days"` (§6.1), then `add_constraint()` with the table's **immutable** per-row CHECKs: span order and non-negative offsets. delta-rs counts a NULL result as a violation (**Tested**), so a CHECK on a nullable column reads `c IS NULL OR …`. Codebook membership is not a CHECK, because codebooks grow (§8). `CreateBuilder` rejects `delta.constraints.*` keys (Interface-checked: observed in S6, not asserted) | CHECK declarations |
| Open | When an attempt opens a table, compare its `delta.constraints.*`, `delta.appendOnly` and (C1) the two retention properties, as exact strings, with the generated set, in delta-rs's normalized form, and abort on a mismatch. A table left without its constraints (a crash between create and `add_constraint`) is refused | The verify helper |
| Write raw | `DeltaTable::write(batches)` (`WriteBuilder`), with `CommitProperties::with_metadata` carrying `lctx.snapshot_id`. That metadata is audit only; `snapshots` stays the authority. **Tested:** CHECK is enforced, `appendOnly` rejects deletes, and the metadata reads back through `history()` | — |
| Derive | A session over the attempt's tables at their written versions, each filtered to the snapshot (§6.2). The derivation SQL from `cpg-schema` computes derived ids with the `lctx_id` UDF (§3.4.1, C1) and runs through the one helper, `ctx.sql_with_options` with DDL, DML and statements disallowed; no other code calls `ctx.sql`. The result is collected, cast strictly to the declared schema, sorted canonically and written like a raw table, so it passes the same local type check. `with_input_plan` streaming (Tested in S6) is not needed at pilot scale: the review probe on FastMCP 4.0.5 (2026-09-22, Measured) had 33,012 `pysa_calls`, 15,772 `call_targets` and 93,101 `facts` rows. Derived tables can be rebuilt from Delta (DM-23) | SQL per derived table |
| Validate | DataFusion queries generated from the contracts (§8), over the session read once into memory, 8 rules at a time (H1 P2). No float aggregate exists yet; when one does, its query fixes its own reduction order | The generator, semantic rules, and a finite-float loop (there is no built-in `isfinite`) |
| Publish | `snapshots.write([rows])` in one commit (§6.1). **Tested:** a rejected append is classified unpublished by re-reading | Classification after an ambiguous error |
| Read | `DeltaTableBuilder::from_url(..)?.with_version(v).load()`, assert `version()`, `update_datafusion_session`, `table_provider()`. Ids come back through the two-step cast (§3.3). **Tested:** with two snapshots in one table, dropping the version pin or the snapshot filter changes the result. Reading a missing table creates nothing | One helper |

Operations are methods on `DeltaTable`. `DeltaOps` does not exist at this pin.

**Never** (the write, SQL and Parquet-scan items are ast-grep rules):
- DataFusion `INSERT INTO` or `DataFrame::write_table` into a Delta table. The `DeltaDataSink`
  path skips CHECK constraints and invariants; a test asserts the bypass at the pinned revision,
  so an upstream fix gets noticed.
- delta-rs's low-level `RecordBatchWriter` or `JsonWriter` on fact tables (no constraint
  handling; Interface-checked).
- `SaveMode::Ignore`; deletion vectors (they switch off Parquet pushdown); column mapping; raw
  Parquet scans; vacuum or optimize.

**Snapshot-scoped reads (H1 P3; closes the known limit).** Delta log statistics skip the Binary
`snapshot_id` (`writer/stats.rs:212-229` at `58f07cd`), so a filter alone skipped no file and
cost one footer read per file of every snapshot. Instead each pinned read opens **only the files
its version's commit added**: `LogStore::read_commit_entry(v)` + `logstore::get_actions` → the
`Add` actions → `TableProviderBuilder::with_adds` (`snapshot::commit_provider`). A snapshot's rows
of a table are exactly one commit's (one commit per table per attempt, §6.1), the JSON commits are
kept (log cleanup off, verified at open), and a selected file that is gone fails the scan rather
than returning fewer rows (**Tested**: `a_pinned_read_opens_only_its_commits_files`). The
`snapshot_id` filter stays as the row predicate.

**Metrics** (C1, **Implemented**; guidelines §12; `cpg_schema::metrics`).
- `lctx compile` reports, per stage, wall time and the process's peak RSS so far (`VmHWM`, which
  the kernel updates lazily, so it only grows approximately; it includes allocator retention,
  §4.3 Measured): acquire, Stage A, the Pyrefly check, per-module extraction (with its Ruff walk and
  Pysa collectors), public names, the dependency check and definitions, raw write per table,
  derive per table, validate and publish (review O4).
- They are returned with the published attempt and never stored in Delta: they are not content.

**Measured, the whole CPG (C6, 2026-09-23;** `just pilot` after the C5 and C6 review fixes, with
per-rule validation costs; FastMCP 4.0.5 and its corpus; fresh store, snapshot `ddee0669…`; a
32-thread, 188 GB host; the default glibc allocator**):**
- 905,648 nodes and 1,449,162 edges; every one of the 493 rules passing; **44.6 s** in all (the
  C6 review reproduced 44.2 s and 44.5 s). (At the C5b build, before its review: 907,845 nodes,
  1,452,970 edges, 503 rules, 50.0 s, snapshot `063b8eb3…`.)
- **Extraction, 31.3 s.** The library run: the Pyrefly check 2.1 s, per-module extraction 5.1 s
  (4.4 s of it the Pysa collectors), and the dependency check 3.8 s. The corpus run: its check
  3.4 s, per-module extraction 7.8 s, its dependency check 3.7 s, and the documents 0.2 s.
- **Raw writes, 0.7 s. Derivation, 2.4 s** (`edges` 1.1 s, `nodes` 0.5 s).
- **Validation, 10.0 s:** 493 queries, the slowest 0.15 s (`unique:type_terms`), so the cost is
  their number, each re-scanning its Delta views. `lctx compile` reports the three slowest rules and
  the one that raised the peak most.
- **Peak RSS, and what it measures** (C6 review F3). With the default allocator the peak is
  7.1 GB here, and 6.7–8.0 GB across seven runs with identical inputs. It climbs from 3.8 GB after
  the raw writes to 5.1 GB after derivation and 7.1 GB after validation. **About 40–45% of it is
  glibc arena retention, not working set:** under `MALLOC_ARENA_MAX=2` the same compile peaks at
  4.2 GB (3.7 GB after extraction, +0.5 GB in derivation, nothing in validation), but takes
  61.2 s (derivation 12.2 s, validation 16.9 s). `VmHWM` is updated lazily, so "only grows" holds
  approximately. The raw batches are now released once written (`attempt::compile_owned`): the
  effect is inside the default allocator's spread, and 0.1 GB under the arena limit (4.3 → 4.2 GB).
- **Decision:** streaming derive stays deferred, since its trigger is not met: derivation is 5% of
  the wall time, and its working set about 0.5 GB.

**Measured, after H1 (2026-09-23;** `just pilot` on a fresh store at `fff5aa5`: jemalloc
(ADR-0016), validation over cached tables 8 at a time, per-commit reads, zstd, fork `a07b7bae`;
FastMCP 4.0.5 and its corpus; snapshot `15fecdab…`, content `10e56541…`; the same host**):**

| Stage | Before H1 (baseline, `1a4c4406…`) | After H1 |
|---|---|---|
| Total | 45.0 s | **29.9 s** |
| Extraction | 31.4 s | 25.7 s (library: check 1.8 s, per-module 4.2 s, dependencies 3.1 s; corpus: 3.1 s, 6.7 s, 3.0 s, documents 0.2 s) |
| Raw writes | 0.73 s | 0.95 s (zstd) |
| Derivation | 2.56 s | 2.24 s (`edges` 1.02 s, `nodes` 0.50 s) |
| Validation | 10.16 s | **0.90 s** (the slowest rule's compute 0.28 s, `key:edges`; the largest hash build 160 MiB) |
| Peak RSS (`VmHWM`, MiB as `lctx` prints) | 7,587 MiB (6,678–8,044 MiB across seven runs) | **3,646 MiB** (3.6 GiB), flat from extraction on |
| Store (`du -h`) | 272 MiB | 235 MiB |

- 905,648 nodes and 1,449,162 edges, all 496 rules passing, before and after.
- **What stayed the same.** Through H1b, every runtime-only commit left the content digest
  (`f01077be…`) and the table fingerprints (the sorted ids of `facts`, `nodes`, `edges`,
  `type_terms`, `syntax_nodes`, `bindings`, `mentions`, hashed) identical to the baseline.
- **What moved.** The fork revision bump (D6) moves producer, run and fact ids. It also moves the
  1,085 nodes of Pyrefly's bundled stubs (their identity is the Pyrefly revision, §3.4.1) and the
  91,278 edges that touch them. Every other node and edge is byte-identical to the pre-D6 build.
- **The test suite** runs in 83 s instead of 225 s.
- **After the H1 review's fixes** (2026-09-23, fresh stores): content `19c3e8e2…`. F1's extractor
  version bump moved producer, run and fact ids; the node, edge, type-term, syntax and binding
  fingerprints are identical to the table above. 29.9–33.4 s across three runs, peak
  3,640–3,649 MiB, 235 MiB. A compile's stderr carries no warning (the known Binary-statistics
  lines are quieted, and tables are created without a failed load; H1 review F6).

**Deferred, with triggers.**
- `datafusion-tracing` (compatible with 55.1 per its skill): until per-operator spans are needed.
- **Streaming derive:** `WriteBuilder::with_input_plan(LogicalPlan)` streams per partition and
  still enforces CHECKs (read in the pinned source, `write/execution.rs:405-431`, 2026-09-22).
  - It would need its own schema and foreign-snapshot checks, and row counts from write metrics.
  - Reopen when derivation dominates the per-stage time, or its working set dominates the peak.
    Under jemalloc (ADR-0016) the reported peak tracks the working set (flat and repeatable,
    within 7 MiB across runs; jemalloc still holds freed pages for its decay period); the earlier
    `MALLOC_ARENA_MAX=2` reading applied only to glibc. At C6 it did neither (above).
- **Validation over cached tables and concurrent rules: taken** (H1 P2, operator 2026-09-23).
  `validate` reads every registered table once through its pinned, snapshot-filtered Delta view
  into a `MemTable` (`cached_session`), then runs the rules 8 at a time on spawned tasks, putting
  violations back in `rules()` order; every session plans on `TARGET_PARTITIONS = 8`. Still one
  query per rule, the same SQL, the one shared validator (§B3). Per-rule cost is read from each
  rule's own physical plan (wall time, summed `elapsed_compute`, largest `build_mem_used`; H1 P5),
  since a process-wide peak delta means nothing under concurrency. **Measured** on the pilot:
  validation 10.2 s → 0.89 s, the peak unchanged (3,646 MiB under jemalloc).
- **Peak memory:** taken by ADR-0016 (jemalloc: the peak tracks the working set, 3,646 MiB on the
  pilot, flat from extraction on) and the raw batches released once written. Reopen when the
  peak nears the host's memory.
- **File skipping on `snapshot_id`: taken** (H1 P3, above): no schema, partition or store change.

> Decision: ADR-0012, ADR-0014, ADR-0016

---

## §5 Projections

**Interface-checked** (petgraph skill). Source: IP L1335–L1415, L2233–L2362. The invocation
projection, its declared spec (`cpg_schema::projection`) and the adapter (`lctx_analytics::graph`)
are **Implemented** and **Tested** (slice 1.4, 2026-09-23: `the_adapter_keeps_parallel_arcs_isolates_and_refuses_disorder`;
`pass_a_finds_the_known_answers_on_analysis_shapes` checks the property-getter arc, the
override-open candidate arc and a module-level caller on the real pipeline).

**Declaring a projection.** A projection declares:
- the snapshot;
- node and edge kinds;
- the accepted origins and fidelities;
- candidate-target and unknown-target policies;
- the algorithm and parameter set.

The vertex universe is selected separately from the edges, so isolated public APIs survive.

**The invocation projection (v1)**
- It is built by joining call-site ownership with call targets in DataFusion: `encloses_call` ⋈
  `call_target`, plus the non-potential `site_target` arcs of property getters and setters, whose
  site's owner is read from `syntax_nodes`.
- **Definition arcs** (slice 1.4 review F1): a function → each function it declares
  (`declares`), with the nested callable as the arc's site, no phase and `arc_kind = definition`.
  A decorator factory's `decorator`, or a closure it registers, is then reachable, and so is what
  it calls. A definition arc is never a `direct_delegation`. Every arc carries its `arc_kind`
  (`call` | `definition`).
- Call and property arcs read the site's owner through one fragment
  (`cpg_schema::graph::owner_of`), so they cannot disagree about who calls (review O7).
- It keeps `call_site_id` (which is also the resolution's id: a `ResolutionSet` is keyed by its
  call site, §3.6), `edge_id`, `invocation_phase`, the arc's modality and
  `has_unresolved_remainder` on every arc.
- **Parallel call sites are preserved.**
- **Excluded:** `potential` targets (so every `higher_order_target`) and `synthetic_model` arcs.
- **Constructor phases stay tagged.**
- **Candidate arcs** (Pysa `Overrides` dispatch) are kept with their modality, so a pass can
  treat them as candidates (§3.6).
- **Unknown targets.** A call site with no target, or with an unresolved remainder, gets no arc
  to a placeholder. It is listed in the projection's `unresolved_sites` relation, which Pass A
  turns into `incomplete_resolution` findings.
- **Callers** are the enclosing function, or the module or class whose body holds the call, as
  typed vertices.
- The projection's rows are persisted where a result cites them: `witnesses` holds each path
  step's call site, callee and `edge_id` (ADR-0019).

**Three identities**

| Identity | Scope | Persisted? |
|---|---|---|
| Canonical `node_id` / `fact_id` | persistent | yes |
| Projection row | links an arc to its evidence rows | yes |
| petgraph `NodeIndex` / `EdgeIndex` | temporary coordinates | **never** |

**Container and adapter** (the library-leverage review, D4; the recipe slice 4 builds).
`petgraph::Graph<(), u32, Directed, u32>`, whose edge weight is the arc's row index (the arc columns
stay in Arrow).
- **Two queries on the pinned snapshot:** the vertex universe (`ORDER BY node_id`, isolates
  included) and the arcs with the columns above (`ORDER BY src, dst, call_site_id, edge_id`: total, since `edge_id` is unique; H1 review F9).
- **The dense index is the sorted domain ids:** read the `FixedSizeBinaryArray` directly (no hex
  strings); domain → dense is a `binary_search`.
- `Graph::with_capacity(n, m)`, then `try_add_node` in order so `NodeIndex(i) == i`, and
  `try_add_edge` in canonical arc order so `EdgeIndex(k)` is arc row k (`graph_impl/mod.rs:656-681`).
- Immutable once built, parallel edges kept, nothing ever removed (removal silently re-points held
  indices).
- **Scopes and callers without copies:** `EdgeFiltered`/`NodeFiltered`, and `Reversed` (which keeps
  the original edge ids). Filtered views keep the base graph's `node_bound`.
- **Not** `Csr` or `GraphMap` (they drop parallel edges), nor `StableGraph` (unneeded, and it blocks
  9 algorithms); no `rayon` or `serde-1` features.
- **Order is a precondition, not a repair** (review O4): the adapter refuses arcs out of
  canonical order, and the arcs query's total `ORDER BY` supplies it. Tests: the refusal
  (`the_adapter_keeps_parallel_arcs_isolates_and_refuses_disorder`), and whole-attempt identity
  across module order (`pass_a_is_identical_across_module_order_and_location`).

**Determinism rules**
- `edges_directed` returns the newest edge first (`graph_impl/mod.rs:919, 982`). So `Bfs` visits
  siblings newest-first (`visit/traversal.rs:294-306`), while `Dfs` pushes every successor and
  pops the last, taking the oldest first (`:108-121`). Probe (2026-09-23): with arcs r→1, r→2, r→3,
  Bfs = [0,3,2,1,4] and Dfs = [0,1,4,2,3].
- So traversals never rely on walker order. They collect a node's edges and **sort by
  (target canonical id, arc key)** before choosing.
- SCC members are sorted by canonical id.

> Decision: ADR-0011, ADR-0019

---

## §6 Persistence and publication

**Tested** where a line cites a spike or slice 2 (`cpg-core/tests/compile.rs`, 2026-09-22),
otherwise **Interface-checked** (deltalake skill probes). Source: IP L1603–L1623, L2967–L3006. The ADR-0009 Delta probe ran in full on 2026-09-22: S6
(CHECK, read cast), P1 (Binary statistics), P2 (a failed validation publishes nothing), P3 (an
ambiguous append is classified by re-reading) and P4 (a byte-identical bundle rebuild).

### §6.1 Canonical tables and publication

- **Family tables.** Each fact family is a set of append-only Delta tables. Every row carries
  `snapshot_id` as a plain column; tables are not partitioned.
- **A compile attempt:**
  1. writes its rows;
  2. runs local and cross-table validation (§8);
  3. then, **and only then**, appends one row per table to `snapshots`: (snapshot_id,
     content_digest, table, Delta version, schema digest, compiler digest, row count), in a
     single commit (**Tested**: a validation failure publishes nothing, and the published
     snapshot's reader sees only its rows). A snapshot is published at most once: `publish`
     refuses a `snapshot_id` that `snapshots` already holds.
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
- **Writes go through `DeltaTable::write` only** (§4.3), except the global `embedding_cache`,
  written by an insert-only `DeltaTable::merge` so that concurrent attempts cannot duplicate a
  key (ADR-0017 amendment; probed in slice 1.6 before use). Both paths enforce the tables' CHECK
  constraints and `delta.appendOnly` where the table features are present, and the open-time
  verify asserts they are. DataFusion `INSERT INTO` does not, so it is never used (**Tested**,
  spike S6).
- **`SaveMode::Ignore` is never used.** It appends to existing tables (`delta.write.3`).
- **No vacuum or optimize on fact tables** in stage 1. Vacuum defaults to `dry_run=false` and Lite
  mode.
- **Retention keeps every published snapshot readable** (C1, ADR-0014; read in the pinned
  delta-rs 58f07cd and kernel 8ba063f sources; **Tested** by
  `retention_keeps_old_versions_loadable`, 2026-09-22).
  - **The problem.**
    - By default, every commit's post-commit hook writes a checkpoint every 100 versions.
    - It also runs expired-log cleanup (`delta.enableExpiredLogCleanup` true,
      `delta.logRetentionDuration` 30 days), which deletes the commits and checkpoints below the
      newest checkpoint older than the cutoff.
    - After that, `with_version(v).load()` of an older `v` fails ("No files in log segment"). A
      snapshot published more than 30 days ago would then be unreadable at its recorded version,
      breaking §6.2.
  - **The fix.**
    - Every table, `snapshots` included, is created with `delta.enableExpiredLogCleanup =
      false` and `delta.logRetentionDuration = interval 36500 days`, set through
      `with_configuration_property`. Never `with_configuration`, which replaces the whole map.
    - Open-time verify compares both strings exactly, because delta-rs silently falls back to its
      default on a value it cannot parse.
    - Checkpoints stay on, so a load replays at most 99 commits.
  - Tested by a table created with `checkpointInterval = 2` and zero retention: an old version is
    unloadable under the defaults and loadable under ours.

### §6.2 Readers

1. Resolve the snapshot's row set in `snapshots`.
2. Load each table **at its recorded version** with
   `DeltaTableBuilder::from_url(..)?.with_version(v).load()`, then assert
   `table.version() == Some(v)`.

   A provider built on an already-loaded handle ignores the requested version
   (`delta.open.2`, `delta.read.4`).
3. **A snapshot-qualified table: read only that commit's files** (`snapshot::commit_provider`, H1
   P3; ADR-0017). A snapshot's rows of a table are exactly the commit at its recorded version (one
   commit per table per attempt), and the commit's `lctx.snapshot_id` must name the snapshot:
   another snapshot's commit is refused (`ForeignCommit`), never read as empty (H1 review F2).
   Then **filter `snapshot_id`** as the row predicate. **A global table that accumulates across
   attempts** (`embedding_cache`) is read at its recorded version over **all** its active files,
   with no commit selection and no snapshot filter (H1 review F3). Each table declares its read
   mode (`snapshot` or `global`) in `cpg-schema`, and every reader honours it (ADR-0017
   amendment).
4. Project columns by name through the DataFusion provider. Never use `scan_table().with_columns`,
   which returned the wrong column for a partition-first schema (`delta.read.3`).
5. Never scan the Parquet directory directly (`delta.read.2`).
6. Register one object store per table root per session (`delta.storage.4`).

### §6.3 Schema evolution

- **Additive nullable columns only**, via `SchemaMode::Merge`. Anything else is a new table
  (`edges_v2`), with the move recorded in `snapshots`.
- Schema digests are `cpg-schema`'s digest of the declared contract, never of Arrow read back, because read-back
  changes `Utf8` → `Utf8View` and renames list children to `element`.
- **A change to a table's CHECK set is a migration**, like a column change: a new table, or an
  explicit constraint step recorded by ADR. The open-time verify (§4.3) refuses a table whose
  constraints differ from the declared set.

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
    **serving schema digests**: SHA-256 over a language-neutral canonical form (field name, a
    declared type grammar, nullability, sorted metadata), which Python recomputes with its standard
    library; known answers are shared by the Rust and Python tests. It is not the store's
    `canonical_schema` (§6.3), whose Rust type display Python cannot reproduce (ADR-0019).
- **Activation.**
  - The bundle is smoke-queried by the serving code's own test entry point.
  - Then the `generations/active` symlink is switched by atomic rename.
  - A running server keeps the generation it loaded; restarting it picks up the new one.

> Decision: ADR-0017 (superseding ADR-0009), ADR-0012, ADR-0019, ADR-0014

---

## §7 Pinned dependency family

| Component | Pin | Source of truth |
|---|---|---|
| Rust toolchain | 1.98.1 | `rust-toolchain.toml` |
| DataFusion | =55.1.0 (`sql`, `parquet`) | `Cargo.toml` |
| Arrow / Parquet | =59.3.0 | `Cargo.toml` |
| object_store | 0.13.2 | `Cargo.lock` (no crate depends on it directly; single version by `check_family.py`; H1 O2) |
| delta-rs | git `58f07cd62bfbce3649a7e1c87c696288068ae184` (`datafusion`, `rustls`) | `Cargo.toml` |
| delta kernel | `buoyant_kernel` 0.25.1, git `8ba063f8`, branch `buoyant/main` | `Cargo.lock` |
| petgraph | =0.8.3 | `Cargo.toml` |

**Tested:** the `cpg-schema::family_smoke` tests (2026-09-22) write Delta and query it through
DataFusion, and `just deps` checks for single versions.

The analyzer, analytics-crate, Python-stack and model pins are in `docs/pins.md`, which is
authoritative for them (ADR-0010, ADR-0011, ADR-0012).

> Decision: ADR-0002

---

## §8 Validation

**Implemented** for the cross-table rules below and **Tested** (slice 2, 2026-09-22; C6 review
F1, 2026-09-23): every hand-written rule rejects an injected violation or is a declared edit
guard (below), and every generated template (`key`, `ref`, `fact`, `fact-payload`, `codebook`,
`endpoint`, `evidence`, `one-per-evidence`, `no-parallel`, `lineage`) has at least one case;
`every_rule_is_exercised_or_declared_an_edit_guard` fails when a rule is added without either.
The brief rules are **Proposed**. Source: IP L1581–L1601.

**Local** (Arrow/Rust), at every materialization boundary:
- exact physical types, nullability and widths. `RecordBatch::try_new` with default options
  enforces these against the declared schema (§4.3);
- codebook membership;
- numeric bounds;
- finite floats and unit-norm vectors.

**Per-row, at every Delta write.** Invariants that never change (span order, non-negative
offsets) are also Delta CHECK constraints, generated from the `cpg-schema` declarations and
enforced by `DeltaTable::write` (§4.3). The open-time verify keeps them identical to the
declarations, so they are enforcement at the storage boundary, not a second definition.
**Codebook membership is not a CHECK:** codebooks grow append-only, and a stored range would
reject the next code. The local validators above check it against the current codebook.

**Cross-table** (DataFusion), **one query per rule**, generated in `cpg_schema::rules` from the
contracts and snapshot-tested; the rules read the session's tables cached once in memory and run
concurrently, their violations reported in rule order (§4.3; `violations_come_back_in_rule_order`;
`every_table_is_read_by_some_rule` walks every rule's plan):
- `key`: uniqueness of every table's declared total key via `GROUP BY … HAVING count(*) > 1`;
- `ref`: each declared reference via `LEFT ANTI JOIN` returning zero rows;
- `fact`: every raw row has its `facts` row and every `facts` row its raw row;
- `codebook`: every codebook column holds a code of its codebook (a query, since not a CHECK);
- `coverage`: a row for every declared family × module of the run's release, and every declared
  family is in the codebook;
- `semantic` (hand-written, one query each): every Pysa function with signatures is some
  signature row's callable; a
  `resolutions` reason and the extractor's call boundary agree per call site, both ways;
  Stage C is injective; `parameter_semantics` names an existing Pysa function; `facts.model_id`
  names its run's producer; a module's stored text is its bytes, and its role matches its run
  (ADR-0015);
- **graph** (C1, **Implemented** and **Tested**; ADR-0014), generated from the registry (§3.8), so
  the references and the mapping are one authority:
  - endpoint kinds;
  - node-valued references (from `node_columns`);
  - evidence and support exist;
  - `key:nodes`/`key:edges`;
  - lineage from raw rows (each source row yields its declared edges, or its derived row carries
    a provider's reason);
  - the partition of `pysa_calls` (lineage or counted remainder; the published-gap arm is empty
    since C3);
  - typed targets: a null target carries a reason, and no derivation supplies a catch-all one
    (this replaces slice 2's "a release call target names a declaration or gives a reason");
  - ids: each Rust recipe equals its SQL form.

  **Edit guards** (C3 review O2; C6 review F1). A lineage rule that re-reads its edge kind's own
  unfiltered source cannot fail on today's SQL: it guards an edit of that edge's derivation, not
  a data condition. Eighteen rules are such guards, declared in `cpg_schema::rules::EDIT_GUARDS`
  and counted apart: the lineage of `declares`, `has_parameter`, `encloses_call`,
  `has_argument`, `ast_child`, `owns_scope`, `lexical_parent`, `binds`, `reads_binding`,
  `captures`, `declared_in`, `has_type`, `type_arg`, `has_field`, `field_type`,
  `contains_passage` and `contains_block`, and `partition:pysa_calls-gaps` (its table is empty
  by construction since C3). The other 15 lineage rules compare a filtered or joined source with
  the edges, and each can fail. A reference whose target is built from its own source column is
  not generated. **Count** (2026-09-23, `rules().len()`): 496 rules, 478 of them falsifiable;
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

> Decision: ADR-0014, ADR-0012, ADR-0015

---

## §9 Analytics

**Proposed** (methods); **Interface-checked** (the libraries named). Source: IP L1735–L1885,
L2233–L2578. Scope extended by ADR-0005 to cover community detection, concept analysis and
embeddings.

**Rules for every technique:**
- it must have a **named consumer** in the brief;
- it must be **deterministic** for fixed inputs and parameters;
- it must record its method, parameters, projection and diagnostics in `analysis_invocations`
  (ADR-0019);
- its effect must be measurable by ablation (§9.8).

**Parameters.** Analysis parameters and the subsystem declaration (§1.4) live in one versioned,
pre-registered analytics config. Its digest is the `lctx-compiler` run's config digest and part of
`content_digest`. Defaults below are starting budgets, not measured optima.

### §9.1 Pass A — public entry point and delegation

**Implemented** and **Tested** (slice 1.4, 2026-09-23): `lctx_analytics::pass_a` with its
hand-worked projection (`pass_a_finds_delegations_boundaries_and_gaps_with_witnesses`,
`budgets_truncate_and_say_so`), and on `fixtures/python/analysis_shapes` through the whole attempt,
identical across module order and location (`pass_a_is_identical_across_module_order_and_location`).
Seeds resolve through `exports`, then each remaining name as a member of its own body or MRO. The
walk covers the whole MRO, external ancestors included, and **fails closed** (slice 1.4 review F2):
the compile is refused when an ancestor is unresolved, when a class in the chain binds the name
other than by `def` or `class` (an assignment or import in its body), or when a non-release class
comes before the definition (it could define the name, and we cannot see it). Tested:
`a_seed_that_could_name_another_method_is_refused`.

- **Question.** Which public API exposes the mechanism, and what does it already coordinate?
- **Method.**
  1. Map public access paths → declarations (`exports`).
  2. From each seed (the declaration node, §3.4.1), run an **explicit BFS with parent pointers**
     over the invocation projection (§5).
     - **Arcs** are calls and definitions (§5). A definition arc reaches a nested callable, so a
       decorator factory's behaviour is in its neighbourhood (slice 1.4 review F1).
     - **Witnesses.** The first witness to a target is the BFS path under sorted adjacency. Up to
       two alternatives are the next **shortest** paths that differ in their final arc, taken in
       canonical arc order. Parallel call sites are distinct arcs, so each can be a witness. A
       longer route to a target already reached is neither shown nor flagged (review O1).
     - **Kind.** It is decided over every shortest final arc, never the kept witnesses: a
       `direct_delegation` needs a definite call arc at depth 1, and its first witness is that
       call (review F3; `semantic:direct-delegation-is-definite`).
     - **Truncation.** `witnesses_omitted = true` when more shortest final arcs existed than the
       witness budget kept (presentation only; `stop_reason` `witness_limit` is reserved). A
       vertex or arc budget is operational truncation: the invocation is `partial`, and Stage F
       reports it as a limit. The depth bound and the boundaries are the stated model (ADR-0019
       review F4). The depth bound is recorded (`depth_limit`) when a frontier vertex has an arc
       or an unresolved site that was not followed (review O2).
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
- **Library.** leiden-rs 0.8.1 (`default-features = false` and no features, so it runs
  sequentially; not its `petgraph` adapter, whose `from_petgraph` would read §5's `u32` arc-row
  weight as the edge weight), with the **RBER** quality function: CPM with γ relative to the
  graph's density, so γ is scale-free on unit-normalized layers (ADR-0011 amendment). γ comes
  from our own fixed-seed grid in the analytics config; leiden-rs's `resolution_scan` and
  `resolution_profile` change seeds between points. It is built with `GraphDataBuilder` from §5's
  dense index. `run_multiplex` is never used: it ignores `layer_weights` after the first level,
  so our weighted sum of layers is the objective.
- **Input normal form** (the library-leverage review, D2; Tested by probe). The undirected builder
  does not normalize orientation, and shuffled input changed an LFR partition at μ=0.5. So
  DataFusion aggregates each (min,max) pair as **integer counts** under a named weight policy and
  sorts (a multi-partition f64 `SUM` is not bit-stable); normalization, hub down-weighting and
  layer weighting happen in Rust in canonical order; and a fixture
  asserts that shuffled and flipped edges give an identical partition. **Owed by the §9.4 slice**
  (H1 review F9): each aggregated pair keeps its contributing arcs, so a community can cite them
  (guidelines §4, §10).
- **Determinism.**
  - The seed is always set (`None` draws OS entropy) and recorded; `track_quality_history` stands
    in for the missing converged flag.
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
- **Stability.** Consensus over N seeds (default 10): pairwise `leiden_rs::metrics::{try_nmi,
  try_ari}` over the partitions, and per community a membership agreement score (our own
  max-Jaccard matching against the reference seed).
- **Outputs.** `statistically_derived` findings.

### §9.5 Centrality

- **Consumer.** The primary entry point within a community, which orders seeds for briefs.
- **Method.** Our own weighted power iteration (about 40 lines) over the usage projection in
  canonical order. The edge weights are the usage counts from examples and tests (a named weight
  policy), with dangling-mass redistribution. It records iterations, the final L1 residual and a
  converged flag (guidelines §8). petgraph's `page_rank` is rejected (the library-leverage review,
  D1). **Owed by the §9.5 slice** (H1 review F9): the input projection (§5 declares only the
  invocation projection), and the damping, tolerance, iteration budget and dangling target,
  recorded in the analytics-config digest. The dangling target is uniform, so
  `leiden_rs::compute_flow` (weighted, directed, uniform teleport) is the reference oracle.
- **Tests:**
  - a hand-computed 3-node fixture;
  - two parallel arcs counted with their weights;
  - a budget too small to converge, reported as not converged;
  - shuffled rows giving identical scores.
- **Output.** A `statistically_derived` ranking finding.

### §9.6 Formal and relational concept analysis

- **Consumer.** Applicable cases and modes, shared controls, and implication-style assertions
  (e.g. "every writer accepting `filesystem` also accepts `format`").
- **FCA (increment 2).**
  - Our own NextClosure (Ganter, ICFCA 2010) over `fixedbitset`, which also yields the
    Duquenne–Guigues implication basis; FCbO (Outrata & Vychodil 2012) only if the concept count
    exceeds the budget. No usable crate exists: odis is AGPL, fcars enumerates concepts only. So
    `fcars =0.2.2` is a dev-dependency **oracle** for concept sets, and Python `concepts` 0.9.2 for
    the cover relation.
  - Objects: the public APIs of one **structurally defined scope**: the subsystem, one module, or
    one class hierarchy. Communities are never an FCA scope, because their membership is
    statistical.
  - Attributes: parameter names, parameter and return types, raised exception types, decorators.
  - A support threshold is applied. There is no stability index: it is #P-hard.
- **RCA (increment 3).** Adds one relational-scaling step (∃-scaling over calls and handoffs), a
  DataFusion join that adds attribute columns to the same FCA.
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

> Decision: ADR-0011, ADR-0019, ADR-0005

---

## §10 Synthesis and briefs

**Proposed.** Source: IP L1695–L1731, L1886–L1966, L2580–L2637. Changed by ADR-0005: there is no
LLM interpreter. The increment-1 kinds, the kind policy, status propagation, the Outcome order and
the grounding rules below are **Implemented** and **Tested** (slice 1.5, 2026-09-23:
`cpg-core::synth`; `briefs_are_synthesized_from_findings_and_verbatim_evidence` snapshots each
brief of `analysis_shapes` and checks every evidence text byte-for-byte against its source past a
non-ASCII byte; each rule rejects an injected violation in `the_analysis_rules_reject_their_violations`).

### §10.1 Findings

**A finding is a typed record,** carrying:
- `finding_kind`;
- its subject and related nodes;
- ordered witness steps (call site, callee, modality and phase; the `edge_id` as lineage) and
  cited facts (ADR-0019);
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

**The count of `unresolved` slots, by section, is the §B11 gap metric.** Read from a published
snapshot with `lctx query`:
`SELECT p.brief_section, count(*) FROM assertions a JOIN assertion_policy p ON
p.assertion_kind = a.assertion_kind AND p.evidence_status = a.evidence_status WHERE
a.evidence_status = 4 GROUP BY p.brief_section ORDER BY 1`.

**How increment 1 fills the sections** (slice 1.5, `cpg-core::synth`):
- **Outcome:** the seed's docstring summary line, located in the literal's own source bytes (the
  first non-blank line; Pyrefly's `Docstring::clean` renders Markdown and loses spans), else the
  first UAX #29 sentence (`unicode-segmentation`) of the first prose paragraph of the first passage,
  by document path and ordinal, holding an **exact** mention of the seed or of an export naming it;
  else `unresolved`. Either is verbatim, and its evidence is the byte span.
- **Public access:** the seed's configured access path and every other public access path naming
  it (Pass A's `public_alias`).
- **Already coordinates:** one assertion per delegation finding: a direct call with its number of
  call sites, or a bounded path with its first intermediate; a path through an override-open call
  says so.
- **Important controls:** one `parameter` assertion per parameter of the seed's own signature (the
  receiver aside): kind, default, requiredness and annotation, with the parameter's fact and span as
  evidence. Parameter docs join with Pass B (increment 2).
- **Limits:** one `analysis_boundary` assertion per stop reason (dependencies, synthesized
  callables, release code outside the subsystem, unresolved sites), plus the depth bound or a
  budget truncation of the seed's invocation.
- **The brief document** (§11.1): outcome, public APIs, control names and limits. Over
  2,048 tokens (a declared proxy of 4 bytes per token until the embedder counts, slice 1.6) it
  fails the compile in increment 1; applicable cases split it from increment 2.
- A template or extractive-rule change bumps `synth::TEMPLATE_VERSION`; the analysis output is
  pinned to the versions in a test ledger (ADR-0019 review O4).

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

> Decision: ADR-0005, ADR-0019

---

## §11 Serving and agent interface

**Tested** where a line cites spike E1–E3 (`spike/pyrefly-inproc`, 2026-09-22). Otherwise
**Interface-checked** (fastmcp skill and installed FastMCP; vLLM 0.30.0 source; model card).
Source: IP L2037–L2071, L2640–L2965.

### §11.1 Embedding spec and vectors

**Model.** Qwen3-Embedding-8B at a pinned revision (`docs/pins.md`), served by a **separate vLLM
0.30.0 service** (`vllm serve … --runner pooling --max-model-len 8192`).
- **Output:** 4,096 dimensions, `Float32`, cosine.
- **Normalization (Tested, E1).** vLLM L2-normalizes: every norm was 1 ± 1e-7. The pooling
  (`LAST`, with activation) comes from the model's sentence-transformers config.
- **Never send `dimensions`.** We use the full 4,096, and vLLM rejects the parameter without a
  Matryoshka override.
- **Measured (E1):** ~15.5 GiB of weights, ~27 GB of the 5090 at 0.80 utilization, 85 s to start.

**Spec.** The spec is hashed into `spec_hash`. It covers the model and revision, tokenizer
revision, vLLM version and served dtype (bfloat16), pooling, instruction template, document
template, dimensions, output dtype and normalization.
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
- **Conformance (Tested, E2).** Over the fixed conformance inputs, both clients build
  byte-identical request texts, apply the same rejections, and return vectors that agree to cosine
  ≥ 0.9995. vLLM is not bitwise deterministic across requests (identical inputs differed by up to
  3.8e-3 in a component), so an exact vector match is never expected.

**Implemented** and **Tested** in slice 1.6 (2026-09-23):
- **The spec** is committed canonical JSON, `specs/embedding/qwen3-embedding-8b.json`, whose
  SHA-256 is the spec hash (`cpg_core::embed::Spec`; `the_committed_spec_is_its_canonical_form`).
- **The Rust client** (`lctx-embed`) builds its request bodies with `serde_json` and is held to
  `specs/embedding/request_bodies.json` for the shared conformance inputs, which the Python
  client is held to as well; every rejection fires (`every_rejection_fires`); a stub service
  exercises the real HTTP path; an unreachable service is `blocked`, never fake. It uses the
  `reqwest` 0.12.28 already in the lock, without TLS (the service is local).
- **Token counts** come from the service's `/tokenize`; an over-cap document fails the compile.
- **The fake embedder** (`FakeEmbedder`, its own spec) draws a unit vector by splitmix64 from the
  request text's SHA-256, so Python reproduces it with its standard library.
- **The cache** is written by an insert-only MERGE (`delta::merge_global`), probed at the pinned
  delta-rs: only Add actions on an append-only table, only missing keys inserted, CHECKs enforced,
  and four racing merges of one key leave one row
  (`an_insert_only_merge_adds_only_missing_keys_to_an_append_only_table`,
  `a_merge_enforces_the_immutable_checks`, `concurrent_merges_of_one_key_leave_one_row`).
- **`lctx compile --embedder vllm|fake|none`**; the service is `just embed-serve`, from the
  locked `services/vllm` project.

**Cache.** Vectors are keyed by `spec_hash + input_hash` in the canonical `embedding_cache` Delta
table (§3.2), because vLLM numerics vary between requests (E2). Snapshots record the cache version they
read, and bundles copy the vectors they need from it.
- A **deterministic fake embedder**, with its own spec hash, is used by tests and `just check`.
- Mixing spec hashes within one generation is rejected.

### §11.2 Retrieval

**In-process, over the pinned generation.**

1. **Lexical.** BM25 over `lexical_text`, scored by `bm25s` 0.3.11 (numpy backend,
   `get_scores`) over our own tokenization (ADR-0010 amendment). That text is the brief's text
   plus split forms of public names (`write_dataset` → `write dataset`), computed in Rust when the
   bundle is built.
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

- **Package.** `python/lctx_mcp`. It depends on `fastmcp` 4.0.x, `pyarrow` 25.0.1 (the bundle
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
- **Transport.** stdio, started with `mcp.run(transport="stdio", show_banner=False)` and
  `FASTMCP_CHECK_FOR_UPDATES=off`: FastMCP 4.0.5's banner otherwise makes an HTTP GET to PyPI at
  every start. Nothing may write to stdout, at import or in the lifespan either; a
  `StdioTransport` subprocess test checks it (ADR-0010 amendment).
- **Tests.** `fastmcp.Client(mcp)` in both the auto and legacy protocol modes, asserting
  `.structured_content`, plus generations with a mismatched schema or spec, which must fail at
  connect.
  **Tested** (E3): `auto` negotiated `2026-07-28` and `legacy` `2025-11-25`; both mismatch
  fixtures failed at connect.

> Decision: ADR-0010, ADR-0013

---

## §12 Evaluation

**Proposed.** Source: IP L2116–L2131, L3026–L3072.

**Fixtures.** The IP L3030 table, plus:
- guard after parameter rebinding;
- shuffled input gives identical communities and concepts;
- LFR planted-partition graphs for community detection;
- FCA over a hand-computed context.

**Gold scoring** (§1.4). The gold and the analyzed release are one FastMCP version, guarded by
`scripts/check_gold.py`. A small committed extract of the gold families records each family's
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

> Decision: ADR-0004, ADR-0013

---

## §13 Deferred

Each item returns by ADR when a consumer needs it.

| Deferred | Where it is described | Trigger |
|---|---|---|
| Full ontology tables beyond the CPG families (the native type graph and `record_fields` are built in C4, §3.2) | IP L469–L717, L334–L409 | an analytic or brief needs the detail |
| Ruff semantic-model port | IP L99–L166 | binding kinds or typing-only context are needed |
| Cross-references (Pyrefly's Glean collector, `report::glean::convert::glean(&Transaction, &Handle)`, reachable in-process today). Probe (2026-09-23): 42 xref targets on a fixture, including the typed attribute xref `self.helper()` → `pkg.mod.C.helper`, in `declarations.qualified_name` form. One target per use, flow-sensitive and pruned, so it complements `reference_resolutions` and never replaces it | IP L209–L231 | a consumer needs attribute cross-references (Pass C handoffs, §10 "see also") |
| CinderX located types (narrowed, unnarrowed, contextual), TSP query surfaces | IP L290–L332, L410–L425 | narrowing or contextual types are needed |
| Python CFG, dominance, dataflow, aliasing, summaries | IP L821–L884, L1505–L1563 | a pass needs path-sensitive facts |
| SCC condensation, dominators on projections: condensation from `kosaraju_scc` membership keeping every arc's evidence, never petgraph's `condensation` (it merges parallel edges; ADR-0011) | IP L1416–L1503 | a consumer beyond recursion labelling |
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
| 2026-09-22 | Pyrefly (patched fork) and Ruff 0.0.11 linked in-process: §B1, §B2, §B8 revised; §3.2–§3.5, §4.0, §4.1 amended; §4.2 rewritten as §4.2.1–§4.2.6; §4.3 added; §6.1, §6.2, §8 and §13 amended; budget raised to ~1,450 lines | ADR-0012 |
| 2026-09-22 | ADR-0009 probe ran in full (P1–P4) and ADR-0009 was accepted. ADR-0010 spikes (E1–E3) ran and ADR-0010 was accepted, with the embedding model changed to Qwen3-Embedding-8B (4,096 dimensions) by operator decision: §3.3, §4.3, §6 and §11 amended | ADR-0009, ADR-0010 |
| 2026-09-22 | ADR-0012 standard review F1–F11, O1: abort on any panic; full Pysa variant table and `Overrides` as open candidates (§3.6); `__all__` completeness detector; immutable CHECKs verified at open; fidelity definitions; `Inline` thread; root-relative context paths; labels corrected | ADR-0012 |
| 2026-09-22 | Slice-1 compact review F1–F9: `is_attribute` and potential remainders; module and class keys; site-packages digest in `context_id`; sorted config keys; revision tied to `Cargo.lock`; §3.4.1, §4.2.1–§4.2.5 reconciled to the code | ADR-0012 |
| 2026-09-22 | Slice 2: Stage C/D derivations, generated validators, `snapshots` publication and the pinned reader; §3.2, §4.1, §4.3, §6, §8 amended (derived rows carry `fact_id`s, reasons in-row) | ADR-0008 |
| 2026-09-22 | Slice-2 compact review F1–F5: reasons on `provider_node_map`/`call_targets`, binding choice follows Stage C, per-site resolution/boundary rule, stored `compiler_digest` with locked engines, `.py`/`.pyi` seed cardinality; §B6 scoped; ADR-0008 accepted | ADR-0008 |
| 2026-09-22 | Pilot moved to FastMCP 4.0.5; libraries are pinned uv projects acquired with `uv sync --frozen`, Stage A reads the acquired environment (RECORD-verified release files, lock-derived `release_id`, environment and lock digests), `releases`/`distributions` added, `lctx` CLI; §1.2, §1.4, §3.2, §3.4.1, §4.0, §4.1, §11, §12 amended | ADR-0013 |
| 2026-09-22 | ADR-0013 standard review F1–F9: one equivalence (verified analyzer-readable bytes) for `release_id` and the environment digest; hermetic acquisition (`--no-config --python --link-mode copy`, `--reinstall`); hash-less releases refused; docs source pinned by commit; `distributions` keyed by context; writer rule; schema drift refused at open; `just gold`; ADR-0013 accepted | ADR-0013 |
| 2026-09-22 | CPG first (operator): slices C1–C6 before Pass A (§1.2). The node and edge catalogs, persistent `edge_id`, typed external and synthetic endpoints, the graph registry and its generated rules (§3.1, §3.2, §3.4.1, §3.5, §3.7, new §3.8, §8); `lctx_id` UDF, metrics, deferred streaming and file skipping (§4.3); retention for pinned reads (§6.1); the syntax, lexical, types and docs families specified for C2–C5, with type structure and record fields no longer deferred. Aligned with the operator's Rust code-intelligence guidelines; probe P1 (dependency definitions) recorded | ADR-0014 (supersedes ADR-0008), ADR-0004 and ADR-0009 amendments |
| 2026-09-23 | ADR-0014 standard review F1–F6 fixed and ADR-0014 accepted: reasons only from providers, export edges per access file, one id recipe in Rust and SQL, generated node references, `graph_gaps`, `edge_kinds` (§3.2, §3.4.1, §3.5, §3.7, §3.8, §8). CPG slice C2: the `syntax` family (`syntax_nodes`, `site_targets`, `ast_child`/`argument_value`/`site_target`) (§3.2, §3.8, §4.2.2) | ADR-0014 |
| 2026-09-23 | C2 compact review F1–F4: every expression outside annotations placed (placement no longer provider-dependent), `if_called` attribute sites as `potential_target`, no catch-all reason for a node-less site, `argument_value` lineage, `unique:syntax_nodes`, the `default` field. CPG slice C3: the `lexical` family (scopes, bindings, references, full Python name resolution, identifier and import targets; ten edge kinds) (§3.2, §3.4.1, §3.5, §3.8, §8) | ADR-0014 |
| 2026-09-23 | CPG slice C4: the `types` family (`type_terms`, `type_term_args`, `type_observations`, `record_fields`; derived `type_class_targets`; `type` and `field` nodes; five edge kinds); `type_role` as positions plus a `declared` flag; fork revision `6a93da34` (`ClassField::dataclass_flags_of` public, `Transaction::get_wildcard`) (§B8, §3.2, §3.4.1, §3.5, §3.5.1, §3.8, §13) | ADR-0014, ADR-0012 |
| 2026-09-23 | C3 compact review F1–F7: names from outside a module's text from Pyrefly (implicit globals as `implicit` bindings, real public builtins, star imports' wildcard sets before builtins); provider-backed import reasons (`not_found` context rows, Pyrefly's relative-import naming) and `variable_origin` only for dependency origins; module and class bodies read a later-bound name from outside too; scope parents by position; every binding event follows `global`/`nonlocal`, `nonlocal` targets decided at the end; nothing binds inside annotations; one event per repeated declared name; four more rules tested by injected violations (§3.2, §3.5, §3.7, §3.8, §4.2.2, §8) | ADR-0014 |
| 2026-09-23 | CPG slice C5a: the source corpus fetched hermetically at its pinned commit, the corpus release and run (several runs per attempt over distinct releases), the `docs` family (`documents`, `passages`, `code_blocks`, `doc_links`, `mentions`; derived `mention_targets`; `document`, `passage`, `code_block` nodes; three edge kinds) with markdown-rs 1.0 (probe P5) (§3.1, §3.2, §3.4.1, §3.5, §3.8, §4.0, §4.1, §8) | ADR-0014, ADR-0013 amendment |
| 2026-09-23 | C4 compact review F1–F7: no catch-all in `type_class_targets`; declared async returns are the annotation; term ids from Pyrefly's structure and identities alone (the display a label; one term per variable), binders derived in Stage D (`type_binders`); boundaries for untyped subjects; enum classes and type-variable bounds, constraints and defaults; inherited method-assigned fields skipped; three rules tested by injected violations (§B8, §3.2, §3.4.1, §3.5.1, §3.8, §4.2.6) | ADR-0014 |
| 2026-09-23 | CPG slice C5b: the usage run (examples, tests and materialized Python code blocks, every code family but `exports`) in the corpus run, whose search path puts the tree ahead of site-packages; `usage_targets` and the `block_module` and `usage_link` edges (probe P4); release-scoped module-name joins; one node and edge for what two runs both assert; `unique:release-paths`, `unique:type_terms`; an augmented assignment's target is a reference; `lctx query --unpublished` (§3.2, §3.4, §3.8, §4.0, §8) | ADR-0014, ADR-0013 amendment |
| 2026-09-23 | CPG slice C6: the whole-CPG measurement (§4.3): 50.0 s and 8.0 GB on the pilot (44.6 s and 7.1 GB after the C5 and C6 reviews; 4.2 GB with `MALLOC_ARENA_MAX=2`) and its corpus; streaming derive stays deferred (its trigger is not met), and validation's cost is recorded per rule, with a new deferred item and trigger | ADR-0014 |
| 2026-09-23 | C5 compact review F1–F8: the corpus run names the release's installed files by their `@path`, so usage calls, type terms and imports reach the release's own nodes and `usage_targets` and `usage_link` retire; a tree shadowing the release fails; the corpus identity is location-free and includes the library release; hermetic git attributes; source keys and globs checked, `coverage:family-has-scope`; `exact` members need their prefix; injective block paths; injected violations for every C5 rule (§3.2, §3.4.1, §3.8, §4.0, §8) | ADR-0014; ADR-0013 |
| 2026-09-23 | C6 deep review (Accept, claims narrowed): §8 states the 18 edit-guard rules (`EDIT_GUARDS`) and counts them apart (493 rules, 475 falsifiable), and a meta-test holds every other hand-written rule to an injected case; 14 new cases, `ref:documents.release_id`, unique tie-breaks; the C6 memory figures restated with their allocator conditions and range, the arena-limited peak, retargeted triggers, and the raw batches released once written; the usage run's §10 consumers narrowed, with the text-and-role decision open in §13; labels raised where verified (§B2, §B3, §B6, §B7, §3.3, §3.4.1, §3.5, §3.6, §4.0, §8) | ADR-0014 amendment |
| 2026-09-23 | ADR-0015 (operator decision, closing C6 review F2): every analyzed module's text and role are stored in `source_files` (`source_role` appended); `[tool.lctx.source]` `examples` and `tests` replace `usage`; `semantic:source-text` and `semantic:source-role-by-run` (496 rules); the §13 open row removed (§3.2, §3.5, §4.0, §8, §13) | ADR-0015 |
| 2026-09-23 | H1, the library-leverage hardening slice (operator: every review item adopted). Static branches are Pyrefly's own decisions (`constant`, `combined` appended); globset/walkdir selection that follows no link; typed library definitions; `RECORD` as CSV; clap CLI; hermetic `git init`; Pyrefly's own predicates; hash and sort known answers. jemalloc (ADR-0016); cached, concurrent validation with plan metrics; per-commit Delta reads; zstd; the UDF's literal kind; a log subscriber; fs-err/anyhow; `cargo shear`. The declared return annotation read from Pyrefly (fork `a07b7bae`); ADR-0011 amended (own PageRank, normalized Leiden input, own FCA with an oracle, condensation from SCCs, §5's adapter recipe). Pilot 45.0 s → 29.9 s, 7,587 → 3,646 MiB peak, 272 → 235 MiB store (§3.2, §3.3, §3.4.1, §3.5, §4.0, §4.3, §5, §6.2, §7, §8, §9.4–§9.6, §13) | ADR-0016; ADR-0011; ADR-0009, ADR-0012, ADR-0013, ADR-0002 amendments |
| 2026-09-23 | Remaining-scope plan, Phase 0: analysis results are typed tables with in-row provenance, content ids and new crates `lctx-analytics`/`lctx-embed` (§B6, §3.2, §3.4.1, §4.1); global read mode and the `embedding_cache` MERGE, a key digest in `content_digest` (§3.2, §3.4.1, §6.1, §6.2); hybrid retrieval moves into increment 1 and increment 4 becomes the generation lifecycle (§1.2); FCA within a structural scope (§1.2); §5's resolution id, unknown-target policy, typed callers and property arcs; RBER, integer aggregation, the γ grid, `compute_flow` oracle, `kosaraju_scc` (§B4, §9.4, §9.5, §13); bm25s and the stdio start flags (§11.2, §11.3) | ADR-0019; ADR-0004, ADR-0010, ADR-0011, ADR-0017 amendments |

> Decision: ADR-0012
| 2026-09-23 | Slice 1.4: table groups for analysis results, the `lctx-compiler` run, the declared invocation projection (§5) and its petgraph adapter, Pass A (§9.1) with `analysis_invocations`/`findings`/`finding_members`/`witnesses`; `libraries/fastmcp/analytics.toml` (docs-only authored); the ADR-0019 standard review's P1 items (identity and lineage per contract, completion mapping, status policy, `adr lint` checks for Decision lines and cited records) (§B6, §5, §6.4, §9, §9.1, §10.1) | ADR-0019 (amended), ADR-0004 |

| 2026-09-23 | Slice 1.5: Stage F synthesis (`cpg-core::synth`): the increment-1 assertion kinds, the kind policy published as `assertion_policy`, derived statuses, verbatim evidence by byte span, briefs, brief members and the brief document; eight tables and eight rules (policy, propagation, text, supports, evidence bytes, exported members, briefs cite analysis); `unicode-segmentation` pinned; the analysis-output ledger (§10, §10.2, §10.3) | ADR-0019 |
| 2026-09-23 | Slice 1.6: the embedding spec, the `Embedder` trait with a fake and the vLLM client (`lctx-embed`), request-body conformance, the global `embedding_cache` (read mode, insert-only MERGE probed at the pin, empty when unused), the keys digest in `content_digest`, `services/vllm` and `just embed-serve`/`pilot-live` (§3.2, §6.1, §6.2, §11.1) | ADR-0017 amendment, ADR-0010 amendment |

