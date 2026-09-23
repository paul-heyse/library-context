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
| 1 | **One complete path.** Real FastMCP 4.0.5 (`libraries/fastmcp`, §4.0), with one hand-registered seed, `fastmcp.FastMCP.tool` (analytics config, §1.4), plus 2–4 distractor briefs for other public entry points. Families provenance, exports, signatures, calls, coverage, findings, embedding_cache. Pass A → increment-1 assertion kinds (§10.2) → brief → bundle → embeddings → exact-cosine search → hydration → both MCP tools | deep |
| 2 | **Analytics families on synthetic fixtures.** Passes B and C with the `syntax` and `lexical` families; single-layer community detection with seed-consensus stability; `page_rank`; FCA within one community | compact |
| 3 | **Pilot corpus, ~15–25 reviewed briefs** (§10.4 manual review). Docs family; compile-time embeddings for doc links and labels; RCA and extra community layers, each kept only if the §9.8 ablation shows it changes published output; gold scoring (§12) | deep |
| 4 | **Reliable serving.** Generations, lexical + RRF fusion, exact-symbol promotion, degraded lexical-only mode when the embedding service is down | compact |
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

> Decision: ADR-0004, ADR-0014

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
  2026-09-22): 275 modules, 103 distributions, every validation rule passing.
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

**Proposed.**

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
- **Scope** (ADR-0008, carried by ADR-0014): `facts` rows are the extracted assertions, and later
  the analytic ones (findings). A derived join row (Stage C/D, §4.1) is not a `facts` row: it is
  traced by the `fact_id`s it cites plus its snapshot's `compiler_digest` (§3.4.1, §6.1), and it
  is rebuildable from them. That includes the `nodes` and `edges` catalogs (§3.8). An edge is a
  first-class relationship through its persistent `edge_id` and its evidence `fact_id`s, not
  through a `facts` row of its own.
- Independent assertions are kept, including disagreement. They are never collapsed into mutable
  node properties.

> Decision: ADR-0014

### §B7 Delta canonical store, published by a `snapshots` append

**Interface-checked.**

- Fact tables are append-only Delta tables, one per fact family.
- A snapshot becomes visible only through one append to the `snapshots` table, made after
  validation passes.
- Readers resolve table versions through that row and filter by `snapshot_id` (§6).

> Decision: ADR-0009

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
  `release_id` and never append (ADR-0013, amended for C5). An attempt holds several runs only over
  distinct releases (the library and its corpus), since the family keys carry no run. The corpus run
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
| `provenance` | `releases` (library, requirement, lock digest, or a source tree's label), `distributions` (every installed distribution: version, artifact sha256s, `RECORD` digest, in the release or not), `source_files` (C1: with its release `distribution`), `contexts`, `producers`, `runs`, `facts`. C1: `context_modules` (each dependency module a fact references: name, site-relative path or Pyrefly's bundled typeshed, distribution and version) and `context_definitions` (the Pysa definitions of those modules: the existence source of `external_symbol` nodes; §3.8) | 1 |
| `exports` | raw: `declarations` (Ruff: qualified name, kind, parent, span, docstring text and span, `is_overload`), `export_syntax` (Ruff: import aliases, `__all__` statement span; syntax evidence only), `public_names` (Pyrefly: access path → origin and its file, `via_dunder_all`; §4.2.3). Derived: `exports` (public access path → the seed declaration in the origin's file: an implementation before an `@overload` stub, then the one Pysa describes, then the last in source order; one row per `public_names` row, so a `.py`/`.pyi` pair gives an access path two rows, one seeding each file, told apart by `source_files.is_stub`; Pass A seeds from the source row) | 1 |
| `signatures` | raw: `parameter_syntax` (Ruff: ordinal, name, default text and span, annotation text), `pysa_functions` (Pyrefly: function key → name span, flags, signature count), `parameter_semantics` (Pyrefly Pysa undecorated signatures: kind, required, annotation), `class_ancestry` (Pyrefly: bases and reported MRO), C1 `pysa_classes` (one row per class, so a class without bases is keyed). Derived: `provider_node_map` (Stage C, name-span join), `signatures` (per `def`: its callable, stubs rolled up to the implementation, and its Pysa signature index), `parameters` (Ruff ⋈ Pysa on the ordinal); C1 `provider_class_map` (Stage C for classes), `synthetic_callables`, `ancestry_targets` and `override_targets` (bases, MRO entries and overridden methods resolved to nodes, a reason where an end does not resolve) | 1 |
| `calls` | raw: `call_syntax` and `arguments` (Ruff: span, owner, ordinal, keyword, starred, expression span; `call_syntax` rows are the call sites), `pysa_calls` (Pyrefly Pysa call graphs: targets, receiver, phase, unresolved reasons). Derived: `resolutions` (§3.6, one per call site), `call_targets` (joined on the full call-expression range, §4.2.3). C1: `arguments.node_id`, `pysa_calls.payload_id`, `argument_resolutions` (higher-order arguments: status and unresolved remainder); `call_targets` typed (a declaration, a synthetic callable or a dependency definition, with the higher-order argument). Pysa rows at non-call sites (property accesses, identifiers, artificial and format-string sites) stay raw, as a declared pending class of the lineage rule, until C2 and C3 give them nodes | 1 |
| `embedding_cache` | `embedding_cache` (spec_hash, input_hash, vector as `List<Float32>`, model identity). Global and append-only; not snapshot-qualified; the key is unique | 1 |
| `coverage` | `coverage`, `boundaries` (§3.7) | 1 |
| `graph` | derived: `nodes`, `edges` (§3.8). Not a coverage unit | C1 |
| `syntax` | **Implemented and Tested (C2; revised by its compact review).** Raw `syntax_nodes` (Ruff): every statement, the clause nodes (`elif`/`else`, `except`, `case`, `with` items) and **every expression outside annotations** (the IP 2.1 exhaustive-exporter contract; placement depends on the source alone, never on a provider). Each row has its parent (the nearest placed ancestor), owner, field (`syntax_field`: body, test, orelse, handler, exc, cause, default, argument, …), ordinal in that field (a statement's block index), span, `kind` (Ruff's `NodeKind`, the `syntax_kind` codebook, an exhaustive match) and detail (a name, attribute, operator, literal as written, or a handler's name). A `def`, a `class` and a call are placed under their declaration and call-site ids; nothing inside an annotation is placed. Derived: `site_targets` (each Pysa attribute, artificial and format-string record → the deepest syntax node at its span, a chained comparison's pairwise site → its comparison → its typed target; a span with no node is our own failure, never a reason). Consumers: Pass B guards, raises, handlers and defaults; Pass C straight-line regions; FCA raised types (their type is C4's) | C2 |
| `lexical` | **Implemented and Tested (C3; revised by its compact review).** Raw, from our recognizer (surface `lctx-lexical`, `recognizer`, inside the Ruff walk): `scopes` (module, class, function, lambda, comprehension; owner, and parent = the scope the scope's position evaluates in, so a lambda in a default or decorator belongs to the enclosing scope), `bindings` (every binding event per scope, ordinals in source order: kind, site, span, the assigned value's span, and the statically decided branch it sits in with its polarity; every event under `global`/`nonlocal` binds in the declared scope, a `nonlocal` target decided once every binding is known; a repeated name in one declaration is one event; the module's implicit globals and a method's `__class__` cell are `implicit` events), `references` (every name load outside annotations, a role of its placed name, with its parent and field; nothing inside an annotation opens a scope or binds), `reference_resolutions` (Python's scoping rules as modelled: the scope's own bindings, else the nearest enclosing function scope with class scopes skipped, else the module, else the star imports whose wildcard set holds the name, else a builtin; comprehension first iterables and function defaults in the enclosing scope; walrus in the nearest non-comprehension scope; flow-insensitive candidates, except that a module or class body reading a name it binds only later also reads it from outside, as `LOAD_NAME` does; a builtin names itself, a builtin variable reads `variable_origin`, anything else `unresolved_target`). Every name set from outside the module's text is Pyrefly's: its `ImplicitGlobal` set, its `builtins` definitions that are real public names, each star import's `Transaction::get_wildcard` set (a star module Pyrefly cannot find stays a candidate for any otherwise unbound name). **Not modelled:** PEP 695 annotation scopes (class and alias type parameters), the implicit unbinding at the end of an `except … as` handler (C3 review O4, O5, deferred). `export_syntax.resolved_module` is each import's absolute module by Pyrefly's own `ModuleName::new_maybe_relative`. Derived: `identifier_targets` (Pysa's identifier sites → the reference at their span → typed target), `import_targets` (each import → the release or dependency module it names; `unresolved_target` only where Pyrefly's finder says not found, a `context_modules` row of origin `not_found`, or where the import climbs past the top package). Consumers: Pass B binding order (§4.2.4), Pass C bindings and values, the import graph, `if_called` targets, variable exports | C3 |
| `types` | **Implemented and Tested (C4; revised by its compact review).** Raw, from Pyrefly's native types (surface `pyrefly-types`, `native_structural`). `type_terms`: one row per distinct term, its id a Merkle hash over Pyrefly's own structure and identities (kind, detail, class pair, children with their roles; §3.4.1); the display is a label, and only a display-only kind (`other`, `truncated`) hashes it. Two structures that share an id but display differently are both emitted, so `key:nodes` rejects the snapshot. A class is a (module ref, class key) pair, an enum member keeps its class, and a recursive alias is a reference to its name, so a term is finite; a depth cap (32) makes that a guarantee. A type variable's id is Pyrefly's own identity (`QuantifiedIdentity`), so one variable is one term wherever it is observed and two unrelated `T`s are two; its bound, constraints and default are its children (`type_arg_role` `bound`, `constraint`, `default`). `type_term_kind` maps every `Type` variant by an exhaustive match; solver-internal and experimental variants are `other` (`display_only`). `type_term_args`: each child at its role and ordinal; a callable parameter carries its name, kind and requiredness. `type_observations` (§3.5.1): each parameter's type, each `def`'s return (`Key::ReturnType`; an annotated one is the annotation, §3.5.1), each call's result, each argument's value and each `raise`'s exception (Pyrefly's expression trace at the exact span; calls in annotations excluded). A subject Pyrefly records no type for (a `TypeVar(...)` declaration, a call in a lambda body, a branch Pyrefly skips for the platform) is a `types` boundary (`missing_evidence`) and the module's coverage is `partial`; a bare `raise` has no exception to type. `record_fields`: the fields a dataclass, attrs or pydantic class, `TypedDict` or `NamedTuple` declares itself (an inherited field a subclass assigns in a method stays its base's), with the flags as the field states them (default, `init`, alias and `kw_only` through `ClassField::dataclass_flags_of`; `TypedDict` required and read-only); the constructor they imply is Pyrefly's synthesized `__init__`, a `synthetic_callable` with its `parameter_semantics`. Derived: `type_class_targets` (each term's class → a release class or dependency definition; a miss is our failure, never a reason) and `type_binders` (each source-anchored variable → the innermost release declaration, type-alias or assignment statement holding Pyrefly's scope anchor, a joined fact; `scope_boundary` for an anchor outside the release). Consumers: Pass C type compatibility, FCA parameter, return and raised types, controls from record fields | C4 |
| `docs` | **Implemented and Tested (C5a, C5b).** A corpus run over the library's upstream tree at its pinned commit (§4.0), in the library's environment; its search path is the tree, then site-packages (the library run's search path), so a module both runs import is one file. Raw, parsed by markdown-rs 1.0 (MDX constructs and frontmatter; byte offsets, probe P5): `documents` (each selected file: path, digest, frontmatter title, whether it parsed; one that does not parse is `unavailable` with markdown-rs's message), `passages` (each top-level heading's section to the next, so a document's passages partition it, with level, heading and heading path; the text before the first heading is passage 0), `code_blocks` (fenced blocks at any depth, MDX components included: language, meta, code, digest; each in the passage its start falls in) and `doc_links` (URL, title, text). `mentions` (our recognizer, `lctx-docs`) against the library run's public names and declarations, two classes never merged: `exact` for inline code (or a dotted prose token) that is a public access path, an origin path or a public class's member (`FastMCP.tool`); `lexical` for inline code that is a bare public name of a class, function, method or module, or such a name in prose when it is distinctive (an underscore, or two capitals and a lower-case letter), one `candidate` per origin (re-exports collapse to the shortest access path). Embedding-based linking is §9.7's. Derived: `mention_targets` (→ the `export` node, or the member's release declaration by the seed rank). Consumers: exact doc links to APIs and extractive brief text, §9.4 co-mention. **The usage run (C5b)** is the same corpus run's code: the selected examples and tests, and every Python code block materialized as a module of its own (`_lctx_blocks/d_<document>/block_<n>.py`, named in `code_blocks.module_path`), with every code family but `exports`. Derived `usage_targets`: a dependency definition in a module that is one of the release's installed files (same site-relative path under the same distribution's `RECORD`) → the release's own declaration or synthetic callable by the same Pysa key (probe P4), a key nothing has being our failure. Consumers: Pass C examples and tests, §10.5 usage patterns, §9.4 co-use | C5 |
| `findings` | `findings`, `witnesses`, `evidence` (evidence_id → one of: fact, span, passage, example, fixture run), `assertions`, `assertion_support` (assertion → finding / evidence), `briefs` (with `review_state`), `brief_members`, `usage_patterns` | 1 |

Deferred: CFG, dataflow and alias tables (§1.3, §13). Type structure and `record_fields` are built in C4 (ADR-0014).

> Decision: ADR-0014, ADR-0012

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
| `finding_id`, `assertion_id`, `brief_id`, `evidence_id` | kind, subject `node_id`(s), canonical payload. **No config digest**, so an unchanged finding keeps its ID when parameters change; ablation diffs are joins. `capability_id` = `brief_id` | content |
| `edge_id` (C1, Proposed) | `edge`, edge kind, source and target node ids, then the kind's discriminator: an ordinal, or for a provider row joined at one site its run-independent payload digest (`pysa_calls.payload_id` = `pysa-call` over the row's payload). Never a `fact_id` | stable across snapshots and runs |
| Role and derived node ids (C1, C3, C4, C5 **Implemented**) | Argument: `argument`, call node, ordinal (Rust). Export: `export`, `release_id`, access path (SQL). Synthetic callable: `synthetic_callable`, module node, Pysa function key (SQL). External module: `external_module`, owner, owner version, module name (Rust), where the owner is the distribution whose `RECORD` lists the file and its version, else `pyrefly-bundled` and the fork revision, else `unowned` and the file's content digest. External symbol: `external_symbol`, the external module id, definition kind, Pysa key (Rust). Its qualified name is a label, because conditional definitions can share one. Reference (C3): `reference`, the name's syntax id. Type term (C4): `type`, kind, detail, class pair and type-variable identity, then each child's role, ordinal, id, parameter name, kind and requiredness; a variable's is its identity alone, a display-only kind's includes its display (Rust; a Merkle id with no SQL form, so no `id:` rule). It is producer-scoped like syntax and external-symbol ids: it hashes Pyrefly's detail text, Pysa class keys and anchor byte offsets, so a Pyrefly bump or an edit earlier in a module renames it. Field (C4): `field`, class node, name (Rust; `id:record_fields`). Document (C5): `document`, release, path. Passage and code block (C5): `passage` or `code_block`, document node, ordinal (Rust; `id:documents`, `id:passages`, `id:code_blocks`) | stable across snapshots and runs for the same inputs; Pysa keys make external symbols producer-scoped, like syntax ids |
| `snapshot_id` | a fresh random 128-bit value per compile attempt | execution identity (DM-12) |
| `content_digest` | sorted `run_id`s (each carrying its `release_id`, and the lock and environment through its context), compiler digest, analytics-config digest, embedding spec hash, `embedding_cache` version used. Slice 2 has the first two | compares reruns |
| `compiler_digest` | the locked engines (DataFusion, Arrow, Parquet, object_store, delta-rs and its kernel, read from `Cargo.lock` by `cpg-core`'s build script), a hand-bumped compiler output version, every derivation query, table contract and validation rule. Stored on every `snapshots` row (**Implemented**, **Tested** by a unit test on each input) | per build |

- **Ids in SQL** (C1, **Implemented** in `cpg_core::udf`, **Tested** by its known-answer and
  plan-time refusal tests). Stage D computes its ids with one scalar UDF, `lctx_id(kind, …)`,
  registered in every session.
  - It implements `IdHasher` exactly: the kind through `IdHasher::new`, and every other argument
    in the `opt_*` encoding (a presence byte, then the length-prefixed value).
  - It accepts Utf8/Utf8View/LargeUtf8, Int16 and Int64 (hashed as i64), Binary, BinaryView and
    FixedSizeBinary, and Boolean. UInt64 (`row_number()`), Int32 and floats are rejected at plan
    time, never cast.
  - Known-answer vectors are shared with the Rust tests, so an id computed in Rust (the
    extractor, a later pass) equals the one computed in SQL.
  - The UDF's recipe is part of `compiler_digest`.
- **Keys are snapshot-qualified.** Uniqueness is checked on `(snapshot_id, key)`, so an identical
  rerun re-emits the same `node_id` and `fact_id` in a new snapshot without conflict.
- **Collisions are validator failures** (§8), never silently merged. `nodes` keeps one row per id
  only for the kinds several existence rows legitimately assert (an export read from a `.py` and
  its `.pyi`, a dependency module or symbol two runs reference); any other repeated id fails
  `key:nodes` (review O2).
- **One recipe, two places.** The ids the extractor computes in Rust whose inputs are also
  columns (argument, external module, external symbol; C3's scope `H(scope, owner)`, binding
  `H(binding, site, name)` and reference `H(reference, name node)`) are `cpg_schema::id::recipe`
  functions in the UDF's `opt_*` encoding. A generated `id:*` rule recomputes each in SQL on every compile, and
  known-answer values are pinned (review F3).
- **Producer scope.** `call_target` and `higher_order_target` edge ids take Pysa's payload,
  function keys included, so a Pyrefly bump may rename them, like syntax and external-symbol ids
  (review O6).
- **The compiler has its own run** (Proposed, with analytics). Analytics, synthesis and
  publication are a run of producer `lctx-compiler`, whose config digest is the analytics
  config's. Until then its identity is the `compiler_digest` stored on `snapshots`.
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

> Decision: ADR-0013 (superseding ADR-0007)

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
| `boundary_reason` | native_unavailable, unresolved_target, unsupported_unpacking, ambiguous_binding, unsupported_control_flow, scope_boundary, budget_reached, missing_evidence, not_requested, provider_disagreement, outside_provider_model (a construct the provider's model does not cover, e.g. a call inside an annotation), syntax_error (facts from a recovered tree), undecodable_source (bytes are not UTF-8), no_source_declaration (a function the provider describes that has no `def` of its own: a synthesized member such as a dataclass `__init__`, or a callable class field), unreachable_in_context (a `def` the analyzer's context never binds) |
| `pysa_unresolved_reason` | the 14 variants of Pysa's unresolved-call reason at the pinned pyrefly, spelled as Pysa spells them; appended when the pin moves |
| `evidence_status` | structurally_observed, documented, statistically_derived, fixture_checked, unresolved |
| `finding_kind`, `assertion_kind`, `analytic_method` | Defined in `cpg-schema` as their consumers land (§9, §10) |
| `node_kind`, `edge_kind` (C1+) | The v1 node kinds (§3.1) and the edge kinds of the registry (§3.8), appended by slice |
| `syntax_kind`, `syntax_field` (C2), `lexical_scope_kind`, `binding_kind`, `static_branch` (C3), `type_term_kind`, `type_arg_role`, `type_role`, `record_kind` (C4) | Defined in `cpg-schema` with their slice. `lexical_scope_kind` is separate from the coverage `scope_kind`. `binding_kind` is the recognizer's binding events: Ruff's kinds it emits plus `del` and the `global`/`nonlocal` declarations |
| `fact_family` additions | `graph` (C1; not a coverage unit), `syntax`, `lexical`, `types`, `docs` as their slices land |
| C5 codebooks | `mention_class` (exact, lexical), `mention_source` (inline code, prose); `scope_kind` `document` (the `docs` family's coverage unit) |
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

> Decision: ADR-0014, ADR-0012

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
component, links, mentions by class, an unparsable document's coverage, and the usage run: a
test module and a materialized block reaching the installed library, linked to its own
declarations). This
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
| C5 (**Implemented**) | `contains_passage`: document → passage, ordinal; `contains_block`: passage → code block, ordinal (extracted). `mentions`: passage → export, class or function, at the byte offset (recognizer; parallel; `exact` or `lexical` and the modality on the evidence row). `block_module`: Python code block → its materialized module (joined). `usage_link`: dependency definition (external symbol) → the release's own function, class or synthetic callable (joined, through `usage_targets`), so a usage call's `call_target` reaches the release |

**Rules** generated from the registry (§8):
- Endpoint kinds: both ends exist in `nodes` with an allowed kind, and `src_kind`/`dst_kind`
  equal it.
- **Node-valued references**, from the registry's `node_columns`: every node-valued column of every
  family table names a node of an allowed kind in `nodes`. The hand-kept `REFERENCES` holds only
  fact, provenance and composite references (slice-2 O8, closed).
- Evidence exists, in the kind's evidence table; support exists.
- `key:nodes`, where one id with two kinds is a collision, and `key:edges`.
- **Lineage from raw rows:** each source row yields exactly its declared edges, or its derived row
  carries a provider's reason.
- **Partition of `pysa_calls`:** every row is a call-site row (lineage), an unresolved remainder
  counted on its `resolutions` or `argument_resolutions` row, a C2 site row (lineage through
  `site_targets`), or an identifier site (lineage through `identifier_targets`). No row is a gap
  since C3; the gaps rule stays, and checks an empty table (review O2).
- **Placement (C2):** every declaration, and every call outside an annotation, has its
  `syntax_nodes` row; a node is placed once; a placed child lies within its placed parent, in the
  same module; a Pysa site with no node, or a name with no reference, is a `typed:*` violation
  (never a reason).
- **Lexical (C3):** `id:scopes`, `id:bindings`, `id:references`; `typed:reference_resolutions` (no
  binding, builtin or reason); `typed:identifier_targets`; `typed:import_targets` (a reason only
  from Pyrefly's finder: an import we misname fails it, C3 review F2); every reference's name is a
  placed syntax node. Each rejects an injected violation (`compile.rs`, C3 review F7).
- **Docs (C5):** `id:documents`, `id:passages`, `id:code_blocks`; `typed:mention_targets`,
  `typed:usage_targets`; a run's release has modules or documents; `coverage:complete` expects a
  `docs` row per document; `unique:release-paths` (an attempt's releases share no path, so a
  `@path` module reference names one file); `unique:type_terms` (one term id, one kind, detail and
  display, however many runs emit it).
- **Types (C4):** `id:record_fields`; `typed:type_class_targets`; `typed:type_binders`; lineage
  for every observation, term argument, class-bearing term and record field. The three named
  rules reject injected violations on `type_shapes` (C4 review F1).
- **Typed targets:** a null target carries a reason.
- **Ids:** the Rust recipes equal their SQL form (`id:*`).
- A rule whose target is built from its own source column is not generated, because it cannot
  fail.

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
  definitions the usage code reaches all link to the release's own (probe P4: the same keys), and
  18,529 usage call edges reach the release through them. 35 usage modules are `types`-partial.
  The whole CPG: 907,845 nodes and 1,452,970 edges; every rule passing; extraction 32.7 s (the
  corpus run 17.8 s), derivation 2.5 s, validation 10.4 s; 46.3 s at 6.7–7.7 GB peak RSS, reached
  in validation (the C6 streaming question, §4.3);
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
  version. `documents`, `documents_exclude` and `usage` select the corpus by glob from the tree's
  root (`**` spans directories, `*` stays within one name). `lctx acquire` fetches the tree
  hermetically (every `GIT_*` variable removed, no system or global git configuration, no
  prompts; `git init`, a shallow fetch of the one commit, checkout, then `rev-parse HEAD` checked
  every time) into `build/sources/<name>/<commit>`; **Tested** by a stub `git` (C5a). The corpus
  release's id hashes its label (`repository@commit`) and every selected file's path and content,
  and it runs in the library's context. The earlier plan was to digest the tree into the docs
  run's context and path-map it when the docs
  family lands (increment 3).
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
  a lock-only change; on the pilot, two environment paths give one `content_digest`). A changed
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
  the attempt's id first, and `--unpublished` reads an attempt validation rejected, at each
  table's latest version: for inspecting a failure, never for a reader.

> Decision: ADR-0013 (superseding ADR-0007), ADR-0012

### §4.1 Stages

| Stage | Owner | Output |
|---|---|---|
| A. Source and analysis universe | uv (`lctx acquire`) + `cpg_extract::library` (§4.0) | the verified release files and context; `releases`, `distributions`, `source_files` |
| B. Typed provider facts | Pyrefly and Ruff in-process (§4.2) + Arrow builders (§4.3) | raw family batches, written to Delta |
| C. Provider-local identity | A DataFusion name-span join (`cpg_schema::derived`) | `provider_node_map`, its keys checked unique and injective before publication (after use by D, which is safe because nothing publishes on failure) |
| D. Semantic relationships | DataFusion over the written raw tables, written back through Delta (§4.3) | derived family tables and views |
| E. Projections and analytics | DataFusion → petgraph / leiden-rs / FCA → DataFusion (§5, §9) | `findings` family, with lineage |
| F. Synthesis | Rust templates + extractive selection (§10) | assertions, briefs |
| G. Publication | Rust (§6) | `snapshots` row; serving bundle |

Unmapped rows stay in the derived table with a null node and, where one applies, a reason
column. No inner join drops them.

> Decision: ADR-0012

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
| Canonicalize | `lexsort_to_indices` + `take_record_batch` on the table's declared **total** key. Arrow's sort is unstable | Key declarations |
| Ids | The `blake3` crate (`=1.8.6`, shared with Pyrefly) inside one `IdHasher` (§3.4.1). Not `RowConverter` bytes: the encoding may change between releases. Not SQL `digest`: it can't write length prefixes | `IdHasher` |
| Create | `DeltaTable::create().with_columns(..).with_configuration_property(TableProperty::AppendOnly, Some("true"))` plus, from C1, `EnableExpiredLogCleanup = "false"` and `LogRetentionDuration = "interval 36500 days"` (§6.1), then `add_constraint()` with the table's **immutable** per-row CHECKs: span order and non-negative offsets. delta-rs counts a NULL result as a violation (**Tested**), so a CHECK on a nullable column reads `c IS NULL OR …`. Codebook membership is not a CHECK, because codebooks grow (§8). `CreateBuilder` rejects `delta.constraints.*` keys (Interface-checked: observed in S6, not asserted) | CHECK declarations |
| Open | When an attempt opens a table, compare its `delta.constraints.*`, `delta.appendOnly` and (C1) the two retention properties, as exact strings, with the generated set, in delta-rs's normalized form, and abort on a mismatch. A table left without its constraints (a crash between create and `add_constraint`) is refused | The verify helper |
| Write raw | `DeltaTable::write(batches)` (`WriteBuilder`), with `CommitProperties::with_metadata` carrying `lctx.snapshot_id`. That metadata is audit only; `snapshots` stays the authority. **Tested:** CHECK is enforced, `appendOnly` rejects deletes, and the metadata reads back through `history()` | — |
| Derive | A session over the attempt's tables at their written versions, each filtered to the snapshot (§6.2). The derivation SQL from `cpg-schema` computes derived ids with the `lctx_id` UDF (§3.4.1, C1) and runs through the one helper, `ctx.sql_with_options` with DDL, DML and statements disallowed; no other code calls `ctx.sql`. The result is collected, cast strictly to the declared schema, sorted canonically and written like a raw table, so it passes the same local type check. `with_input_plan` streaming (Tested in S6) is not needed at pilot scale: the review probe on FastMCP 4.0.5 (2026-09-22, Measured) had 33,012 `pysa_calls`, 15,772 `call_targets` and 93,101 `facts` rows. Derived tables can be rebuilt from Delta (DM-23) | SQL per derived table |
| Validate | DataFusion queries generated from the contracts (§8). `target_partitions = 1` for float aggregates | The generator, semantic rules, and a finite-float loop (there is no built-in `isfinite`) |
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

**Known limit (Tested, P1).** Delta log statistics skip the Binary `snapshot_id`, so Delta skips
no files. The Parquet footers do carry binary statistics, and row groups are pruned (3 → 1 in
P1). A `snapshot_id` filter therefore costs one footer read per file, not a full scan.

**Metrics** (C1, **Implemented**; guidelines §12; `cpg_schema::metrics`).
- `lctx compile` reports, per stage, wall time and the process's peak RSS so far (`VmHWM`, which
  only grows): acquire, Stage A, the Pyrefly check, per-module extraction (with its Ruff walk and
  Pysa collectors), public names, the dependency check and definitions, raw write per table,
  derive per table, validate and publish (review O4).
- They are returned with the published attempt and never stored in Delta: they are not content.

**Deferred, with triggers.**
- `datafusion-tracing` (compatible with 55.1 per its skill): until per-operator spans are needed.
- **Streaming derive:** `WriteBuilder::with_input_plan(LogicalPlan)` streams per partition and
  still enforces CHECKs (read in the pinned source, `write/execution.rs:405-431`, 2026-09-22).
  - It would need its own schema and foreign-snapshot checks, and row counts from write metrics.
  - Reopen when C6's per-stage RSS shows derive dominating.
- **File skipping on `snapshot_id`:** the writer records no Delta-log statistics for Binary
  columns (`writer/stats.rs:214-238`), so the known limit above stands. Reopen when a published
  read's latency is measured to matter.

> Decision: ADR-0012

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
- **Writes go through `DeltaTable::write` only** (§4.3). That path enforces the tables' CHECK
  constraints and `delta.appendOnly`. DataFusion `INSERT INTO` does not, so it is never used
  (**Tested**, spike S6).
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
3. **Filter `snapshot_id`.** The version is an audit coordinate and a lower bound, not a row
   selector.
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
    **schema digests** in `cpg-schema`'s canonical schema form (field name, type, nullability,
    metadata; conformance-tested in Rust and Python).
- **Activation.**
  - The bundle is smoke-queried by the serving code's own test entry point.
  - Then the `generations/active` symlink is switched by atomic rename.
  - A running server keeps the generation it loaded; restarting it picks up the new one.

> Decision: ADR-0009, ADR-0012

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
authoritative for them (ADR-0010, ADR-0011, ADR-0012).

> Decision: ADR-0002

---

## §8 Validation

**Implemented** for the cross-table rules below and **Tested** (slice 2, 2026-09-22: each rule
kind rejects an injected violation); endpoint kinds and the brief rules are **Proposed**. Source:
IP L1581–L1601.

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
contracts and snapshot-tested:
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
  names its run's producer;
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

  A rule that could only pass (its target built from its own source column) is not generated;
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

> Decision: ADR-0014, ADR-0012

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

**Cache.** Vectors are keyed by `spec_hash + input_hash` in the canonical `embedding_cache` Delta
table (§3.2), because vLLM numerics vary between requests (E2). Snapshots record the cache version they
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
- **Transport.** stdio. Nothing may write to stdout.
- **Tests.** `fastmcp.Client(mcp)` in both the auto and legacy protocol modes, asserting
  `.structured_content`, plus generations with a mismatched schema or spec, which must fail at
  connect.
  **Tested** (E3): `auto` negotiated `2026-07-28` and `legacy` `2025-11-25`; both mismatch
  fixtures failed at connect.

> Decision: ADR-0010

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

> Decision: ADR-0004

---

## §13 Deferred

Each item returns by ADR when a consumer needs it.

| Deferred | Where it is described | Trigger |
|---|---|---|
| Full ontology tables beyond the CPG families (the native type graph and `record_fields` are built in C4, §3.2) | IP L469–L717, L334–L409 | an analytic or brief needs the detail |
| Ruff semantic-model port | IP L99–L166 | binding kinds or typing-only context are needed |
| Cross-references (Pyrefly's Glean collector, now in-process under `report::glean`) | IP L209–L231 | a consumer needs cross-references |
| CinderX located types (narrowed, unnarrowed, contextual), TSP query surfaces | IP L290–L332, L410–L425 | narrowing or contextual types are needed |
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
