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
  **Measured** (slice 3.3, 2026-09-23, snapshot `1a7244fe`; the gold's operations joined to the
  public-callables relation and `exports`, each declaration's module tested against the config's
  prefixes). The six families hold 30 operations. 25 resolve to release declarations; the other
  five are `pydantic` and `mcp` names outside the release. 22 of the 25 lie in the subsystem, and
  every miss is a `fastmcp.Client` method, which the config's rule excludes. By family, in
  subsystem / operations: `fm.register` 6/6, `fm.inputs` 3/4, `fm.outputs` 2/4, `fm.errors` 3/4,
  `fm.resources` 4/8, `fm.middleware` 4/4. The kept default's 20 briefs touch `fm.register`,
  `fm.inputs` and `fm.resources` among the six. Each brief is one operation (§10.3), so a family's
  best Jaccard is at most 1 / |operations|.
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
  - **Measured** (slice 1.9, 2026-09-23): live Qwen3-Embedding-8B vectors, pilot generation
    `9470c532be0a9e8d`, 4 briefs.
    - For "decorate callable imperative add tool registry original function",
      `fastmcp.FastMCP.tool` ranks 1st.
    - For "Register functions and expose component metadata", it ranks 2nd, behind
      `FastMCP.prompt`. The vector leg ranks it 1st; the lexical leg ranked the briefs by length,
      on the one shared word "register".
  - **Outcome: `failed`** (`scripts/ranking_check.py`, 2026-09-23): first for 1 of the 2
    `fm.register` aliases.
  - The fusion policy has since changed to a gold-independent rule: a leg votes only where it
    discriminates (ADR-0010 amendment, deviation log D19). The check is restated for increment 3's
    15–25 briefs: the primary seed's brief first for every alias of its family, with
    `just ranking-check <generation> vllm` exiting 0. It is re-run with live vectors at 3.3.

In addition:
- the §12 evaluation must have run;
- no brief may reach publication by bypassing the analytics. A brief that is honestly explained
  by documentation alone is allowed, and is labelled that way.
  - "Analysis-backed" means citing a derivation family's positive finding about behaviour
    (`findings::ANALYSIS_BACKED`: delegations and implementation boundaries; Pass B and C kinds
    join as they land). A `public_alias`, an unresolved site or a traversal stop does not count.
  - `semantic:brief-cites-analysis` holds the label both ways. `semantic:documentation-only-has-outcome`
    requires that a documentation-only brief has a `documented` Outcome (slice 1.5 review F4).

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
- **What runs by default is decided by the §9.8 keep rule** (ADR-0020, Measured 2026-09-23):
  Passes A–C, direct usage and selection. Communities (leiden-rs), FCA, kNN, PageRank, RCA and
  the extra layers are variants, off by default; each is reachable with `lctx compile
  --analytics`.

> Decision: ADR-0011, ADR-0020

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
| `signatures` | raw: `parameter_syntax` (Ruff: ordinal, name, default text and span, annotation text), `parameter_docs` (slice 2.1, **Implemented** and **Tested**: Pyrefly's `parse_parameter_documentation` over each `def`'s docstring, Sphinx and Google styles; one row per documented parameter of the signature, its text as Pyrefly normalizes it, its span the description's verbatim bytes. The entry is anchored on its own header line (Sphinx `:param [type] name:`, Google `name:` or `name (type):`), never on a substring of the name, and runs over the lines indented deeper than the header; it is accepted only when its trimmed lines equal Pyrefly's text, or begin with it: Pyrefly's Google parser ends an entry at a continuation line holding a colon, and such a description is **extended** to its entry's end, its text then those lines (slice 2.1 review F2, F3). A description no header, or more than one, locates is a `signatures` boundary (`provider_disagreement`), never guessed; `docstring_tests` in `walk.rs`; on the pilot (Measured, 2026-09-23): 1,580 rows, 46 of them extended, and one description (`truncation_suffix`) a boundary), `pysa_functions` (Pyrefly: function key → name span, flags, signature count), `parameter_semantics` (Pyrefly Pysa undecorated signatures: kind, required, annotation), `class_ancestry` (Pyrefly: bases and reported MRO), C1 `pysa_classes` (one row per class, so a class without bases is keyed). Derived: `provider_node_map` (Stage C, name-span join), `signatures` (per `def`: its callable, stubs rolled up to the implementation, and its Pysa signature index), `parameters` (Ruff ⋈ Pysa on the ordinal); C1 `provider_class_map` (Stage C for classes), `synthetic_callables`, `ancestry_targets` and `override_targets` (bases, MRO entries and overridden methods resolved to nodes, a reason where an end does not resolve) | 1 |
| `calls` | raw: `call_syntax` and `arguments` (Ruff: span, owner, ordinal, keyword, starred, expression span; `call_syntax` rows are the call sites), `pysa_calls` (Pyrefly Pysa call graphs: targets, receiver, phase, unresolved reasons). Derived: `resolutions` (§3.6, one per call site), `call_targets` (joined on the full call-expression range, §4.2.3). C1: `arguments.node_id`, `pysa_calls.payload_id`, `argument_resolutions` (higher-order arguments: status and unresolved remainder); `call_targets` typed (a declaration, a synthetic callable or a dependency definition, with the higher-order argument). Pysa rows at non-call sites (property accesses, identifiers, artificial and format-string sites) stay raw, as a declared pending class of the lineage rule, until C2 and C3 give them nodes | 1 |
| `embedding_cache` | `embedding_cache` (spec_hash, input_hash, vector as `List<Float32>`, model identity). Global and append-only; not snapshot-qualified: its read mode is `global` (§6.2); written by an insert-only MERGE (ADR-0017 amendment); the key is unique | 1 |
| `coverage` | `coverage`, `boundaries` (§3.7) | 1 |
| `graph` | derived: `nodes`, `edges` (§3.8). Not a coverage unit | C1 |
| `syntax` | **Implemented and Tested (C2; revised by its compact review).** Raw `syntax_nodes` (Ruff): every statement, the clause nodes (`elif`/`else`, `except`, `case`, `with` items) and **every expression outside annotations** (the IP 2.1 exhaustive-exporter contract; placement depends on the source alone, never on a provider). Each row has its parent (the nearest placed ancestor), owner, field (`syntax_field`: body, test, orelse, handler, exc, cause, default, argument, …), ordinal in that field (a statement's block index), span, `kind` (Ruff's `NodeKind`, the `syntax_kind` codebook, an exhaustive match) and detail (a name, attribute, operator, literal as written, or a handler's name). A `def`, a `class` and a call are placed under their declaration and call-site ids; nothing inside an annotation is placed. Derived: `site_targets` (each Pysa attribute, artificial and format-string record → the deepest syntax node at its span, a chained comparison's pairwise site → its comparison → its typed target; a span with no node is our own failure, never a reason). Consumers: Pass B guards, raises, handlers and defaults; Pass C straight-line regions; FCA raised types (their type is C4's) | C2 |
| `lexical` | **Implemented and Tested (C3; revised by its compact review).** Raw, from our recognizer (surface `lctx-lexical`, `recognizer`, inside the Ruff walk): `scopes` (module, class, function, lambda, comprehension; owner, and parent = the scope the scope's position evaluates in, so a lambda in a default or decorator belongs to the enclosing scope), `bindings` (every binding event per scope, ordinals in source order: kind, site, span, the assigned value's span, and the innermost branch Pyrefly decides statically that it sits in: the deciding test's kind (`type_checking`, `version_info`, `platform`, `constant`, `combined`) and whether Pyrefly analyzes or prunes that branch, clause by clause exactly as `SysInfo::pruned_if_branches` decides, recursively: an `if` inside a pruned clause is never walked, so its bindings take the pruning clause's mark (H1 C1, H1 review F1: `SysInfo::evaluate_bool` per clause, the kind from the expression tree; `static_marks_agree_with_pyrefly_pruning` checks every fixture `if`, and `static_polarity_is_pyrefly_recursive_pruning` every assignment binding, against Pyrefly's own pruning, nested cases included); every event under `global`/`nonlocal` binds in the declared scope, a `nonlocal` target decided once every binding is known; a repeated name in one declaration is one event; the module's implicit globals and a method's `__class__` cell are `implicit` events), `references` (every name load outside annotations, and an augmented assignment's target, which reads before it binds; a role of its placed name, with its parent and field; nothing inside an annotation opens a scope or binds), `reference_resolutions` (Python's scoping rules as modelled: the scope's own bindings, else the nearest enclosing function scope with class scopes skipped, else the module, else the star imports whose wildcard set holds the name, else a builtin; comprehension first iterables and function defaults in the enclosing scope; walrus in the nearest non-comprehension scope; flow-insensitive candidates, except that a module or class body reading a name it binds only later also reads it from outside, as `LOAD_NAME` does; a builtin names itself, a builtin variable reads `variable_origin`, anything else `unresolved_target`). Every name set from outside the module's text is Pyrefly's: its `ImplicitGlobal` set, its `builtins` definitions that are real public names, each star import's `Transaction::get_wildcard` set (a star module Pyrefly cannot find stays a candidate for any otherwise unbound name). **Not modelled:** PEP 695 annotation scopes (class and alias type parameters), the implicit unbinding at the end of an `except … as` handler (C3 review O4, O5, deferred). `export_syntax.resolved_module` is each import's absolute module by Pyrefly's own `ModuleName::new_maybe_relative`. Derived: `identifier_targets` (Pysa's identifier sites → the reference at their span → typed target), `import_targets` (each import → the release or dependency module it names; `unresolved_target` only where Pyrefly's finder says not found, a `context_modules` row of origin `not_found`, or where the import climbs past the top package). Consumers: Pass B binding order (§4.2.4), Pass C bindings and values, the import graph, `if_called` targets, variable exports | C3 |
| `types` | **Implemented and Tested (C4; revised by its compact review).** Raw, from Pyrefly's native types (surface `pyrefly-types`, `native_structural`). `type_terms`: one row per distinct term, its id a Merkle hash over Pyrefly's own structure and identities (kind, detail, class pair, children with their roles; §3.4.1); the display is a label, and only a display-only kind (`other`, `truncated`) hashes it. Two structures that share an id but differ in kind, detail or display fail `unique:type_terms` (several runs may observe one term; `nodes` keeps it once). A class is a (module ref, class key) pair, an enum member keeps its class, and a recursive alias is a reference to its name, so a term is finite; a depth cap (32) makes that a guarantee. A type variable's id is Pyrefly's own identity (`QuantifiedIdentity`), so one variable is one term wherever it is observed and two unrelated `T`s are two; its bound, constraints and default are its children (`type_arg_role` `bound`, `constraint`, `default`). `type_term_kind` maps every `Type` variant by an exhaustive match; solver-internal and experimental variants are `other` (`display_only`). `type_term_args`: each child at its role and ordinal; a callable parameter carries its name, kind and requiredness. `type_observations` (§3.5.1): each parameter's type, each `def`'s return (`Key::ReturnType`; an annotated one is the annotation, §3.5.1), each call's result, each argument's value and each `raise`'s exception (Pyrefly's expression trace at the exact span; calls in annotations excluded). A subject Pyrefly records no type for (a `TypeVar(...)` declaration, a call in a lambda body, a branch Pyrefly skips for the platform) is a `types` boundary (`missing_evidence`) and the module's coverage is `partial`; a bare `raise` has no exception to type. `record_fields`: the fields a dataclass, attrs or pydantic class, `TypedDict` or `NamedTuple` declares itself (an inherited field a subclass assigns in a method stays its base's), with the flags as the field states them (default, `init`, alias and `kw_only` through `ClassField::dataclass_flags_of`; `TypedDict` required and read-only); the constructor they imply is Pyrefly's synthesized `__init__`, a `synthetic_callable` with its `parameter_semantics`. Derived: `type_class_targets` (each term's class → a release class or dependency definition; a miss is our failure, never a reason) and `type_binders` (each source-anchored variable → the innermost release declaration, type-alias or assignment statement holding Pyrefly's scope anchor, a joined fact; `scope_boundary` for an anchor outside the release). Consumers: Pass C type compatibility, FCA parameter, return and raised types. Controls from record fields are **not yet read**: Pass B follows parameters only (C4 O3, deferred until a seed's controls are a record's fields) | C4 |
| `docs` | **Implemented and Tested (C5a, C5b).** A corpus run over the library's upstream tree at its pinned commit (§4.0), in the library's environment; its search path is the tree, then site-packages (the library run's search path), so a module both runs import is one file. Raw, parsed by markdown-rs 1.0 (MDX constructs and frontmatter; byte offsets, probe P5): `documents` (each selected file: path, digest, frontmatter title, whether it parsed; one that does not parse is `unavailable` with markdown-rs's message), `passages` (each root-level heading's section, whatever its depth, to the next, so a document's passages partition it, with level, heading and heading path; the text before the first heading is passage 0), `code_blocks` (fenced blocks at any depth, MDX components included: language, meta, code, digest; each in the passage its start falls in) and `doc_links` (URL, title, text). **Components (the holistic assessment's A3; Implemented and Tested, 2026-09-24):** `doc_components` holds each MDX JSX element, flow or text form (`component_form`), in pre-order with its parent ordinal and depth, its name (none for a fragment), its span, its inner span (first child to last; none when self-closing) and its lead (the first direct paragraph), in the passage its start falls in; `doc_component_attributes` holds its attributes in order, each by kind (`attribute_value_kind`: a literal's value as written; an expression's as source text, never evaluated; a bare name without a value; a spread without a name). Fenced and inline code never yield a component, and a heading inside one opens no passage. Components are span facts, not graph nodes. On the pilot: 1,250 components and 1,605 attributes (Measured, 2026-09-24). Consumers: §10.3's documented warnings and `<ParamField>` parameter descriptions. `mentions` (our recognizer, `lctx-docs`) against the library run's public names and declarations, two classes never merged: `exact` for inline code (or a dotted prose token) that is a public access path, an origin path or a public class's member (`FastMCP.tool`); `lexical` for inline code that is a bare public name of a class, function, method or module, or such a name in prose when it is distinctive (an underscore, or two capitals and a lower-case letter), one `candidate` per origin (re-exports collapse to the shortest access path). Embedding-based linking is §9.7's. Derived: `mention_targets` (→ the `export` node, or the member's release declaration by the seed rank). Consumers: exact doc links to APIs and extractive brief text, §9.4 co-mention. **The usage run (C5b)** is the same corpus run's code: the selected examples and tests, and every Python code block materialized as a module of its own (`_lctx_blocks/d_<document>/block_<n>.py`, named in `code_blocks.module_path`), with every code family but `exports`. The corpus names each installed file the release's distributions own by the library run's own site-relative `@path` (C5 review F2), so a usage call's target is the release's own declaration or synthetic callable (the same Pysa key, probe P4), a release class is one type term whichever run observes it, and an import of a library module targets the library's module node. A tree that holds its own copy of the package ahead of the installed one (a flat layout) would cut the usage code off the release, so it fails the compile, naming the module. Consumers: Pass C examples and tests, §10.3–§10.5 usage patterns, §9.4 co-use. Each usage module's text and role are in `source_files` (ADR-0015, closing C6 review F2), so a snippet and whether it is an example, a test or a doc block are read from Delta alone | C5 |
| `findings` | ADR-0019 (contracts in `cpg_schema::findings`; provenance in-row, no `fact_id`; not a coverage unit; outside the `nodes`/`edges` catalogs): `analysis_invocations` (method, parameters as canonical JSON, projection digest, seed, diagnostics), `findings`, `finding_members`, `witnesses` (path steps keyed by node ids, `edge_id` as lineage), `evidence` (evidence_id → one of: fact, span, passage, example, fixture run, with resolved text), `assertions`, `assertion_support` (assertion → finding / evidence, role `support` or `scope`), `briefs` (with `review_state`, outside `brief_id`), `brief_assertions`, `brief_members`, `brief_documents`, `assertion_policy`; and `public_paths` (the holistic assessment's A1: every public path under the config's roots, own and inherited, with its export, kind, `own` and one `preferred` per node; §9's opening). A usage pattern is a `usage_pattern` assertion, not a table (D24) | 1 |

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
| `compiler_digest` | the locked engines (DataFusion, Arrow, Parquet, object_store, delta-rs and its kernel, read from `Cargo.lock` by `cpg-core`'s build script) and analysis libraries (`lctx_analytics::LIBRARIES`), a hand-bumped compiler output version, the UDF version, the synthesis template version (`synth::TEMPLATE_VERSION`, which stands for Stage F's queries and templates; the analysis ledger test fails any output change made without bumping it; increment-1 deep review F1), every derivation query, declared projection digest and Pass B relation digest, the public-path relation, every table contract and every validation rule; and (the holistic assessment's A2(e), 2026-09-24) a digest of every `.rs` file of `cpg-core`, `lctx-analytics` and `cpg-schema`, computed by the same build script, so a code change no version names still moves run and producer ids (`every_compiler_source_is_hashed`). `TEMPLATE_VERSION` stays the published lineage and the ledger the alarm. Stored on every `snapshots` row (**Implemented**, **Tested** by a unit test on each input) | per build |

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
| A3 codebooks | `component_form` (flow, text), `attribute_value_kind` (literal, expression, bare, spread) |
| A2(d) codebook | `unfollowed_reason` (rebound, computed, unmapped): Pass B's reasons, still stored as text in `finding_members.label`, read back by name, never by a default |
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
- **Docs components (A3):** the CHECKs `span_order`, `parent_before`, `inner_within` and
  `lead_within`; `semantic:doc-component-parent` (a parent exists, precedes its child, contains it
  and is one level up; a top-level component is at depth 0); `semantic:doc-component-in-passage`;
  `semantic:doc-attribute-component` (an attribute's component exists, and it has no name exactly
  when it is a spread and no value exactly when it is bare); and `semantic:docs-span-in-document`
  (every passage, code block, component, link and mention lies within its document's bytes: the
  docs family's missing span rule). Each semantic rule rejects an injected violation in the same
  test.
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
- **As implemented** (slice 2.1; review F4). Stricter than "before": a parameter whose name has
  any second binding event in its scope (after the site, or in a statically pruned branch) is
  never followed, and a guard on it is no guard. The trace is an analysis finding, not a
  `boundaries` row (extraction's table): a read of the name at a call into the subsystem is an
  `unfollowed_argument` with reason `rebound`, stated in the brief's Limits (§9.2).
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
  from `cpg-schema` (`cpg_schema::bundle`). Codebook values are served as their text. The files:
  - `briefs`, with each brief's Outcome and its status;
  - `assertions`, one row per (brief, ordinal), with kind, section and status;
  - `supports`, each assertion's findings (by kind) and evidence, by role and ordinal;
  - `evidence`, with its resolved text and its file's or document's path;
  - `brief_members`, and `symbol_map` (exact public access path → brief). **`FORMAT` 2** (the
    holistic assessment's A1, 2026-09-24; pre-registered in ADR-0010's amendment): every public
    path of a brief's seed, own and inherited, with `own` (`semantic:brief-member-public`), so any
    public spelling promotes the brief (`fastmcp.FastMCP.http_app` promotes
    `TransportMixin.http_app`'s). The server shows a brief by its own paths. `FORMAT` 1 served
    the seed's aliases only;
  - `public_paths`: the whole public surface (node, path, kind, `own`, `preferred`), what a
    query's or a gold operation's spelling resolves against;
  - `lexical_text`: each brief's documents, then the **distinct tokens** of its public names, each
    once however many spellings or splits produce it (the path, its segments and their words at
    underscores and case changes; `cpg_schema::bundle::name_tokens`), for BM25 (§11.2). The
    tokenizer is Unicode lower-casing, then ASCII letter-and-digit runs, identical in Rust and
    Python, with shared known answers in `specs/serving/tokens.json`;
  - `embedding_spec` and `vectors` (§11.1), whose vector type is `fixed_size_list(float32 not null
    "item", D)`, `D` the spec's dimensions. The spec comes from the snapshot's `embedding_specs`
    row, and each document's key is `(spec_hash, input_hash)` (slice 1.7).
  - Usage patterns are `usage_pattern` assertions in `assertions` and `evidence` (slice 2.2,
    deviation log D24), so no separate served file exists.
- **`MANIFEST.json`** has sorted keys. It lists:
  - the format, the snapshot, and its content and compiler digests;
  - the spec hash (none for a lexical-only generation);
  - per file, its sha256, rows and **serving schema digest**. That digest is a SHA-256 over a
    language-neutral canonical form: field name, a declared type grammar, nullability and sorted
    metadata. Python recomputes it with its standard library, and the known answers are shared
    by the Rust and Python tests (`specs/serving/schema_digests.json`). It is not the store's
    `canonical_schema` (§6.3), whose Rust type display Python cannot reproduce (ADR-0019).
  - a coverage summary: coverage by scope, family and status; boundaries by reason; invocations
    by completion; briefs by review state and analysis backing; unresolved slots by section (the
    §B11 gap metric).
  - The **generation key** is the first 16 hex digits of the SHA-256 of the manifest without its
    key. It moves with any file, and with the snapshot's provenance.
- **Normalization** (slice 1.7), so a rebuild is byte-identical:
  - each file is one query, sorted by its declared key;
  - every column is cast to its declared type and rebuilt through a builder, so no view type,
    scan metadata or byte under a null slot reaches the file;
  - one record batch per file, written in the Arrow IPC file format: V5, 64-byte alignment,
    uncompressed.
  - One generation never mixes vector spaces: a snapshot with two specs is refused, by
    `semantic:one-embedding-spec` and by the builder.
  - **Tested** (2026-09-23): `a_generation_rebuilds_to_the_same_bytes` rebuilds from the store,
    and from a second compile in another location and module order. Also
    `a_changed_generation_is_refused`, `mixed_embedding_specs_are_refused` and
    `serving_schema_digests_are_the_shared_known_answers`.
  - `lctx compile` builds the generation after publishing; `lctx bundle` rebuilds it.
  - A reader session registers its own `snapshots` rows, so the manifest's digests are read
    from the store (C6 review O5).
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

**The public paths** (the holistic assessment's A1; Implemented and Tested 2026-09-24, consumers
moving in Phase 2 of its plan). One relation, `cpg_schema::public::public_paths`, names every
public spelling of every public function and class, and it is persisted as the analysis table of
the same name, written before Stage E. Its roots are bound as `$roots`.
- **Exports:** each declaration exported under a root by a path with no private segment; where a
  path names several, the seed resolution's pick (an implementation before a stub).
- **Members:** each public member, `__init__` or `__call__` an exported class declares or inherits
  along its MRO. The rule is the seed resolution's (§9.1 step 1): the nearest definer wins, with
  the seed rank among its definitions. Nothing is inherited past an unresolved base or an ancestor
  outside the release that precedes the definition, and nothing where a class up to the definer
  binds the name by an assignment or import.
- **`own`** marks a path whose export declares the node. **`preferred`** marks one path per node:
  own first, then the fewest segments, then the least.
- Rules: `semantic:public-path-exported`, `-preferred` and `-own`, each with an injected case. On
  `analysis_shapes`, every brief's seed is its row, and the relation refuses `pkg.Shadowed.run`
  and `pkg.Aliased.tool`, as the seed resolution does (`public_paths_agree_with_the_seed_resolution`).
  `public_shapes` covers inheritance, rebinding, an outside ancestor, overloads, nested classes and
  private names. A hand-built session covers an unresolved base, which no validated snapshot holds.

**Parameters.** The subsystem declaration (§1.4), the seeds and the pass budgets live in one
versioned, pre-registered analytics config. Its digest is the `lctx-compiler` run's config digest
and part of `content_digest`. The community and PageRank parameters, added after that config was
frozen (D21), are pre-registered code (`communities::Params`, `ranking::Params`; D28, D29): each
invocation records them, the compiler digest includes them, and the gold freeze pins their digest
beside the config's (ADR-0011 review F4). Defaults below are starting budgets, not measured optima.

### §9.1 Pass A — public entry point and delegation

**Implemented** and **Tested** (slice 1.4, 2026-09-23): `lctx_analytics::pass_a` with its
hand-worked projection (`pass_a_finds_delegations_boundaries_and_gaps_with_witnesses`,
`budgets_truncate_and_say_so`), and on `fixtures/python/analysis_shapes` through the whole attempt,
identical across module order and location (`pass_a_is_identical_across_module_order_and_location`).
A seed is its **`public_paths` row** (§9's opening; the holistic assessment's A1, 2026-09-24), and
its aliases are the rows naming its node through the same container (the same exported class, or
the module level for a direct export). The relation carries the member rule that `member()` used
to walk here (a whole-MRO walk, external ancestors included, that **fails closed**; slice 1.4
review F2): nothing is inherited past an unresolved ancestor or a non-release class before the
definition, nor where a class in the chain binds the name other than by `def` or `class`. A seed
with no row is refused, naming it. Tested: `a_seed_that_could_name_another_method_is_refused`,
`public_paths_agree_with_the_seed_resolution`. On the pilot the lookup replaced about 0.9 s of
seed-resolution queries (Measured: "analyze: seed selection" 0.89 s before, 0.02 s after,
2026-09-24).

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
    `implementation_boundary`, `incomplete_resolution`, and `traversal_stop`. The last is
    emitted when the depth bound or a budget left something unfollowed, so the Limits entry that
    states it cites a finding (slice 1.5 review F1).
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

- **Supported predicate** (slice 2.1 review F5). An expression of the function's own parameters
  (each bound once), builtin names and literals, joined by comparisons, `not`/`and`/`or`,
  arithmetic, tuples, lists and sets, where every call's callee is a builtin's name. An
  attribute, subscript, other call, lambda, comprehension or walrus reads state the analysis does
  not model, so it is not a guard.

- **Never guessed:**
  - `*args`/`**kwargs` passthrough;
  - ambiguous overloads;
  - multiple writes;
  - property or subscript access;
  - any §4.2 `ambiguous_binding` site.
- **Visited key.** `(callable, formal, source parameter)` (slice 2.1 review O2: the mapping
  context is the source parameter, since the mapping itself is per arc).
- **Promotion.** A `conditional_raise` is reported as "the implementation raises in this
  branch". It becomes a public precondition only with supporting evidence, and never when an
  enclosing handler may catch it.
- **What a path must say** (slice 2.1 review F1). Every Pass B template shares one path-qualifier
  rule: each override-open hop is named, and each call its caller makes only on some paths is
  said. A raise is not reported when a call on the path may be absorbed (inside a `try` or a
  `with`, whose context manager may suppress it) or sits in a construct of its caller that also
  tests the flowing value (the caller may pass only values the callee accepts).
- **What is declined leaves a trace** (slice 2.1 review F4, §4.2.4). A reached parameter read at
  a call into the subsystem in a form not followed (rebound in its scope, inside an expression,
  unpacked, or taken by no single formal) is an `unfollowed_argument` finding, stated in Limits,
  so "not listed as passed on" is never read as "not passed on".

**Implemented** and **Tested** in slice 2.1, revised by its compact review (2026-09-23;
deviation log D17, D18, D26):
- **Relations.** Declared SQL relations (`cpg_schema::flows`), whose digest joins the compiler
  digest. Their arcs take the invocation projection's accepted evidence (review O1), and one
  mapping fragment serves Pass B and Pass C (review O7).
  - *Argument flows:* each argument of a `call` or `init` arc mapped to one formal of its
    target, the implicit receiver counted. A positional argument before any `*` argument maps by
    index; a keyword argument maps by name to a positional-or-keyword or keyword-only formal,
    also after a `*` argument. Each value is classed as a parameter bound once, one identity
    alias assigned directly in the caller's body, a literal as written, or other. Each row says
    whether its call site sits in a `try` or `with` of the caller, in a conditional construct
    (`if`, loop, `match`, conditional expression, boolean operator, comprehension, lambda,
    `except` clause), and whether such a construct also reads the flowing value.
  - *Guards:* an `if` directly in a function's body whose test is a supported predicate reading
    at least one of its parameters, with a `raise` directly in its branch.
  - *Parameter reads:* each argument of an arc whose value reads a name the caller binds under
    one of its parameters' names: rebound or not, bare or not, unpacked or not.
  - *Receivers:* a method's first positional parameter unless Pysa says it is static, by the
    declaration's kind and never by the parameter's name (review F8; the seed parameters and the
    brief's parameter list both use it).
- **Worklist.** `lctx_analytics::pass_b`, keyed by `(callable, formal, source parameter)` and
  bounded by Pass A's depth. It follows parameters and aliases into subsystem callees only.
  - It reports `forwarding`, `transformed_argument` (a literal the seed itself supplies),
    `conditional_raise` and `unfollowed_argument` (reason `rebound`, `computed` or `unmapped`).
  - A call its caller makes only on some paths is a `conditional_call` member of each finding
    whose path crosses it.
  - A raise is declined when a call on the way may be absorbed or is tested by its caller, or
    when the tested parameter is rebound.
  - Never mapped: starred arguments and any positional after one, `**` arguments, catch-all
    formals, property or subscript values, and literals below the seed.
- **Stage F.** Pass B's findings become assertions, all `structurally_observed` (the kind policy
  permits nothing more; review F7). Documented text reaches Controls and Limits through §10.3's
  parameter descriptions and documented warnings, not through Pass B:
  - `control`: per seed parameter, where it is passed on, with the path qualifiers;
  - `transformed_control`, with the path qualifiers;
  - `restriction`: "`callee` raises (`raise E`) when `test`; its `formal` receives `p`", with the
    path qualifiers, citing the test's and the raise's syntax facts;
  - `unfollowed_control` (Limits): per seed parameter, the callees it reaches in forms the
    analysis does not follow, and why. Like `analysis_boundary`, it stays out of the brief
    document (D14).
- **Tests.** `pass_b_finds_the_known_answers_on_analysis_shapes` (`pkg.configure`) covers
  forwarding directly, through an alias, over a bound receiver, over a class receiver
  (`Registry.create`) and into a constructor (`Widget(name)`); two mappings of one parameter; a
  literal; a depth-2 chain and the depth bound's stop; raises reached by each. It shows no raise
  for the `try`-guarded call, the `contextlib.suppress` call, the call under `if name is not
  None`, the attribute guard, the rebound guard or `**options`, and an `unfollowed_argument` for
  each of a rebound value, a computed one, a positional after `*extra`, `*extra` itself and
  `**options`. `templates_say_what_the_findings_show` checks the overridable and conditional
  qualifiers in the published text.
- **Pilot (Measured, 2026-09-23, snapshot `c9309c78`).** 35 `forwarding`, 4 `conditional_raise`
  and 5 `unfollowed_argument` findings. `FastMCP.tool`'s `name_or_fn` reaches
  `ToolDecoratorMixin.tool`'s `isinstance(name_or_fn, classmethod)` guard, now stated with its
  overridable hop; `FastMCP.mount` has `server is self`. The unfollowed ones are `FastMCP.tool`'s
  `meta` (rebound) and `task` (computed), `FastMCP.resource`'s `mime_type` and `meta` (rebound),
  and `FastMCP.mount`'s `namespace` (computed).

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
- **Output.** A `handoff` finding per (other callable, consumer formal), with its status, its
  occurrence count as score, and its first occurrences' producer and consumer sites (slice 2.2
  review O2: the region and binding are the sites' statements).

**Implemented** and **Tested** in slice 2.2, revised by its compact review (2026-09-23; deviation
log D24, D25, D30):
- **Relation.** A declared relation (`cpg_schema::flows::handoffs_sql`) over the official examples,
  tests and doc blocks. Producer and consumer are both callables the release declares, never a
  helper the usage code defines itself (review F2). It holds each occurrence of:
  - `x = producer(...)`, the statement's only target, then `consumer(..., x)`, in a later
    statement of the same block, with the call as that statement's value (awaited or not). `x`
    must be bound once and read once, except as a receiver;
  - or `consumer(..., producer(...))`.
  - The argument maps to a formal by the mapping fragment Pass B shares (`maps_formal`).
- **Setup is not a consumer.** A read of `x` as the object of a called attribute or a decorator
  (`x.method()`, `@x.tool`) configures the produced object (D25, narrowed by review F4). A second
  argument use, any other attribute read, a return, a store or a reassignment rejects the named
  occurrence, and so does a `with` item. The nested form is counted wherever it occurs (review O5).
- **Paths.** A doc block is named by its document and fence number (`docs/x.mdx, code block 3`),
  never by the module the compiler materialized it as (review F5; `flows::usage_files_sql`,
  checked by `semantic:no-materialized-block-path`).
- **Kernel.** `lctx_analytics::pass_c` groups occurrences per `(other callable, formal)` into one
  `handoff` finding. Its score is the count. Its members are the formal and three occurrences:
  examples first, then doc blocks, then tests.
- **Tests.** `handoffs_and_usage_patterns_come_from_official_code` covers a named handoff after a
  receiver use and a nested one (counted), and two consumers, a reassignment, a usage-defined
  helper, a chained assignment and a passed-on attribute (not counted).
  `pass_c_and_usage_patterns_are_identical_across_location_and_module_order` (review F6).
- **Pilot (Measured, 2026-09-23, snapshot `8a882a72`, after the review).** 205 seed occurrences
  in 7 handoff findings, all between release callables: `FastMCP(...)` → `FastMCP.mount(server=…)`
  153 (156 before one-target producers and the narrowed receiver rule), `require_scopes(...)` →
  `FastMCP.tool(auth=…)` 24, `create_proxy(...)` → `mount` 13. The three findings on usage-defined
  helpers (`require_tenant`, `require_access_level`, `two_question_server`) are gone. Every seed
  has a pattern; doc blocks are named by document and fence (`docs/servers/authorization.mdx,
  code block 10`), and `custom_route`'s pattern now comes from a test that imports what it reads.

### §9.4 Community detection

**Off by default since ADR-0020** (the §9.8 keep rule, 2026-09-23): the `+communities` variant. Its record below stands as the variant's.

- **Label.** Implemented and Tested (slice 2.3, revised by the ADR-0011 standard review;
  ADR-0011 accepted with it).
- **Consumers:**
  - **seed selection**, which decides which entry points get briefs within the brief budget;
  - the brief's **Related** field (§10.3).

  A community is not a brief boundary: a brief is one outcome at one public operation
  (IP L1697).
- **Library.** leiden-rs 0.8.1 (`default-features = false` and no features, so it runs
  sequentially; not its `petgraph` adapter, whose `from_petgraph` would read §5's `u32` arc-row
  weight as the edge weight), with the **RBER** quality function: CPM with γ relative to the
  graph's density, so γ is scale-free on unit-normalized layers (ADR-0011). leiden-rs's
  `resolution_scan` and `resolution_profile` are not used: they change seeds between points.
  `run_multiplex` is never used: it ignores `layer_weights` after the first level, so our weighted
  sum of layers is the objective. The graph is built with `GraphDataBuilder` from the kernel's own
  dense index.
- **Input normal form** (the library-leverage review, D2; Tested). The undirected builder does not
  normalize orientation, and shuffled input changed an LFR partition at μ=0.5. So each layer is
  **integer counts** per `(min, max)` pair (counted in Rust over the declared relations' rows,
  which is as bit-stable as an integer aggregate), in a `BTreeMap`; normalization, hub
  down-weighting and layer weighting follow in canonical order. Each pair keeps its least
  contributing site as lineage (H1 F9), and a community cites the sites behind its strongest pairs.
- **Determinism.** The seed is always set and recorded; `track_quality_history` stands in for the
  missing converged flag; rand is pinned on its 0.9 line; crate versions are in each invocation's
  `library_versions`.

**Implemented** and **Tested** (slice 2.3 and the ADR-0011 review, 2026-09-23; deviation log D28):
- **Relations** (`cpg_schema::communities`, digested with the invocation projection's digest into
  every community invocation's `projection_digest`). The co-use occurrences (each usage-code call
  to a function with its enclosing scope, over the flows' call targets) and the public callables
  (the function and async-function rows of `public_paths`, §9's opening; before 2026-09-24 a
  relation of their own, which the holistic assessment's A1 folded into it).
- **Layers**, each a named policy in `Params` (review F3). *Invocation:* every
  invocation-projection arc between two distinct subsystem functions (calls, property accesses and
  definitions; definite and candidate), 1 per arc. On the pilot, 370 of its 712 pairs come from
  candidate arcs, whose sites nearly always have that one target, and 33 from definitions.
  *Co-use:* every official-usage scope calling two distinct subsystem functions, 1 per scope.
- **Weights.** Hubs: an end whose strength (its summed counts) exceeds the layer's 95th-percentile
  strength scales the count by threshold / strength. Each layer is normalized to unit total, and
  the layers are summed with weight 0.5 each. The vertices are the subsystem functions some pair
  touches; an isolated function is in no community, and is not in RBER's density term.
- **Resolution.** RBER at **γ = 1**, its own density scale, over seeds 0–9 (review F2: choosing
  the γ with the highest mean ARI from a grid depended on the seed block, not the data). The
  profile γ ∈ {0.5, 1, 2, 4} runs too, and its stability is recorded (mean and SD of pairwise ARI,
  mean and min NMI, community count, largest community), never chosen from. γ = 1 is degenerate
  when its seed-0 partition puts more than half the vertices in one community or has no community
  of three: then nothing is reported and the diagnostics say so.
- **Reported.** Each community of the seed-0 partition with at least two public members whose
  **co-assignment** (the mean, over seeds 1–9, of the share of its public-member pairs sharing a
  community) is at least 0.5 (review F5: the score measures the published grouping). A
  `community` finding lists its public APIs (`community_member`, with access path and strength),
  cites up to three supporting sites (`supporting_site`), and has the co-assignment as score.
- **Records.** One `leiden` invocation per run (γ, seed, iterations, convergence, quality
  history) and one `community_consensus` invocation, whose `diagnostics` (a declared migration)
  hold the profile, the choice and what was reported.
- **Tests.** `communities::tests`: LFR planted partitions (n = 250, μ = 0.1 and 0.3) recovered at
  γ = 1 with NMI ≥ 0.9; shuffled and flipped edges give the identical consensus; hub
  down-weighting; a trivially small graph is degenerate; the co-assignment score; the parameters
  against their gold freeze. `the_digest_follows_the_invocation_projection` (cpg-schema).
  `communities_are_stable_and_projected_onto_public_apis` on `analysis_shapes`; the co-use layer
  on `docs_shapes`; the module-order and location tests cover the rows.
- **Pilot (Measured, 2026-09-23, snapshot `48bf9454`).** 516 vertices and 1,298 combined pairs
  (712 invocation, 598 co-use). All runs converged. At γ = 1: mean pairwise ARI 0.749 (SD 0.082),
  mean NMI 0.909, min NMI 0.866; 45 communities, the largest of 101. 29 are reported (2–78 public
  members; co-assignment 0.681–1.0, mean 0.954); 1 falls below 0.5, and 15 have fewer than two
  public members. Four seeds (`FastMCP.tool`, `mount`, `resource`, `prompt`) share the 78-member
  server surface (co-assignment 0.831); `custom_route` (declared on `TransportMixin`) is in no
  reported community. The profile's mean ARI is 0.700, 0.749, 0.793 and 0.739 at γ = 0.5, 1, 2
  and 4.
- **Closed** (the review's F6; the holistic assessment's A1, 2026-09-24): the public-callables
  relation's MRO walk skipped unresolved or outside ancestors and class-level rebindings, where
  §9.1's `member()` refused. Both now read `public_paths`, which refuses as `member()` did. On the
  pilot the move changed no published output (`lctx diff` `5dc48895` → `3c2be6da`); it dropped one
  unpublished `direct_usage` finding, `FastMCP.instructions`' getter, because a property's getter
  and setter share one path and the seed rank names the setter (deviation log D48).

**Extra layers, Implemented and Tested in slice 3.2** as variants, off by default (2026-09-23;
D41):
- **`+type-layer`** (`cpg_schema::communities::shared_types_sql`): two subsystem functions that
  name one release class anywhere in their declared parameter types (a recursive walk of
  `type_term_args`, the receiver aside) are a pair, 1 per class.
- **`+mention-layer`** (`co_mention_sql`; C5 O1): two subsystem functions that one doc passage's
  exact mentions name, 1 per passage.
- **`+knn-layer`**: each subsystem public API with its three nearest other APIs by embedding cosine
  at or above the kNN floor (§9.7's parameters), 1 per direction found.
- **Combination.** With extra layers, every layer weighs 1 / (number of layers)
  (`EXTRA_WEIGHT_RULE`); each layer keeps its own hub down-weighting and normalization. The
  default's two layers keep `Params`' 0.5/0.5. The consensus records the layers, their policies
  and the rule, and its diagnostics give each extra layer's pairs and hub threshold. The layers'
  SQL joins the community relations' digest (`extra_digest`). A supporting site names its layer.
  Since the holistic assessment's A2(c) (2026-09-24), a layer is a `LayerSpec` (type, mention,
  or kNN with its embedding spec hash, `k` and similarity floor) matched exhaustively: the kNN
  layer's lineage joins `extra_digest` and the consensus's parameters (`knn_layer`), so two
  embedding specs never share an invocation id (`the_knn_layer_digest_follows_its_lineage`).
- **Tests:** `variants_add_relational_attributes_and_layers` (type and mention layers on
  `analysis_shapes`; the default's diagnostics unchanged) and
  `mention_and_knn_layers_come_from_the_corpus` (`docs_shapes` with the words embedder; the kNN
  layer without an embedder is refused).
- **Pilot (Measured, 2026-09-23, snapshot `904d86eb`, fake vectors).** The type layer has 4,068
  pairs (hub threshold 126), the mention layer 7 and the kNN layer 0: fake vectors clear no
  floor. 536 vertices; 27 communities are reported, against 29 by default. The 3.3 ablation
  judges each layer with live vectors.

**The consumers, Implemented and Tested in slice 2.6** (2026-09-23; deviation log D34), **revised
by the increment-2 review** (U1; ADR-0011 amendment; deviation log D37):
- **Seed selection** (`lctx_analytics::selection`). Communities and direct usage (§9.5) run
  first, before any per-seed pass. The configured seeds come first. While the brief budget
  allows, the **eligible** public APIs follow by rank: the most direct official-usage calls first
  (PageRank in the `+pagerank` variant), ties to the smaller id. Eligible means official usage
  calls it at least once and its docstring has a summary, the Outcome its brief will state
  (`synth::summary_span`; the review's O1). Communities **cap** rather than choose: an API is
  skipped once its community holds ⌈budget / 3⌉ seeds, configured ones included; an API in no
  community is capped by nothing. Each selected API is named by its preferred path (below); one
  whose path resolves to another declaration is dropped and recorded. A `seed_selection`
  invocation records the rule and its choices (`selection::Params`, frozen with the analytics
  parameters: the review's F7) and names the configured, selected and dropped seeds and the
  eligible count. Each selected seed gets Passes A–C, FCA of its scope and a brief. Tested by
  `select`'s unit test, `selection_needs_official_usage` (`analysis_shapes` has no usage code:
  nothing is eligible) and `selection_takes_what_usage_calls_within_the_budget` (`docs_shapes`,
  budget 4: `pkg.Server.run` is chosen). The pilot's budget equals its five seeds, so nothing is
  selected there until increment 3 raises it (an ADR-0004 amendment). *Superseded:* 2.6's rule,
  rounds over the communities each taking its most PageRank-central documented member, which the
  review showed would fill a larger budget with helpers (F3).
- **The preferred path** (the review's F4). A public callable is shown by one path wherever it is
  named (community members, Related, selected seeds, kNN texts):
  `public_paths`' `preferred` column, read by `cpg_schema::public::preferred_callables`. It is a path through the class that declares
  the method first, so a classmethod is never named through a subclass it would bind
  differently, then the fewest segments, then the least. *Superseded:* the least path, which
  named `Tool.from_function` as `FastMCPProviderTool.from_function` on the pilot.
- **Related** (§10.3): the seed's community co-members, at most five, the most called in official
  usage first (by PageRank in its variant; by name, and saying so, when usage calls none of them),
  citing the community and each listed member's `direct_usage` finding, so its status derives
  `statistically_derived`.
  The policy case holds by construction and by rule: the kind policy permits
  `statistically_derived` only for `related`, and a control citing a community finding or stated
  statistically is rejected (two injected cases in `the_analysis_rules_reject_their_violations`).
- **Pilot (Measured, 2026-09-23, snapshot `8e1e1f19`).** Nothing is selected (budget five, five
  seeds). Four seeds list Related operations from the 78-member server community (co-assignment
  0.83), led by `FastMCP.__init__` and `FastMCPProviderTool.from_function`; `custom_route`, in no
  reported community, has none.
- **Pilot after the increment-2 review (Measured, 2026-09-23, snapshot `a8591982`, generation
  `4a789baa`, fake vectors).** 196 public APIs are eligible; nothing is selected (budget = seeds).
  The four Related lines name the server community by preferred path, the most called first:
  `fastmcp.FastMCP.__init__`, `tool`, `resource`, `call_tool`, `list_tools` (`mount` for two).
  No line names a method through an unrelated subclass. Stage E takes 3.9 s; the compile 35.1 s.

### §9.5 Centrality

- **Consumer.** Which operations get briefs (seed selection, §9.4) and the order of a Related
  line: the question both ask is **which operations official usage calls** (the increment-2
  review's U1; ADR-0011 amendment).
- **Method (the default): direct usage** (`lctx_analytics::ranking::usage_counts`, the review's
  U1). Each call arc (definite or candidate, any phase) from an official-usage caller (a function
  or module of an example, test or doc block) into a subsystem function; each call site counts
  once, split evenly among its targets (`USAGE_POLICY`, digested with the invocation projection).
  A method call on an instance is a `candidate` arc (Pysa's override marking), almost always with
  one target, so a definite-only count would see constructors and module functions only (pilot:
  11,794 of 11,854 candidate usage sites have one target; D37). One `usage_count` invocation;
  each public API usage calls is a `direct_usage` finding (`structurally_observed`: a count of
  observed calls) whose score is its count. It orders, and never states behaviour.
- **Method (the `+pagerank` variant, kept for the §9.8 ablation).** Our own weighted power
  iteration (about 40 lines) over the usage projection in canonical order. The edge weights are
  the usage counts from examples and tests (a named weight policy), with dangling-mass
  redistribution. It records iterations, the final L1 residual and a converged flag (guidelines
  §8). petgraph's `page_rank` is rejected (the library-leverage review, D1). The input projection
  and the damping, tolerance, iteration budget and dangling target are pre-registered code (D29),
  frozen by digest in `eval/gold/analytics-freeze.json`. The dangling target is uniform, so
  `leiden_rs::compute_flow` (weighted, directed, uniform teleport) is the reference oracle.
  PageRank ranks what usage reaches **through the library's own delegation**, so implementation
  sinks rank high (the review's F3: on the pilot, its 6th and 8th public APIs have no direct
  usage call, while `FastMCP.call_tool`, 320 sites, is 16th). It replaces direct usage as the
  order only in its variant, and is kept only if the ablation shows it helps.
- **Tests:**
  - a hand-computed 3-node fixture;
  - two parallel arcs counted with their weights;
  - a budget too small to converge, reported as not converged;
  - shuffled rows giving identical scores.
- **Output.** A `statistically_derived` ranking finding.

**Implemented** and **Tested** in slice 2.4 (2026-09-23; deviation log D29); since the increment-2
review, the `+pagerank` variant's method:
- **The usage projection** (`lctx_analytics::ranking`, H1 F9 answered). The invocation projection
  restricted and weighted by a named policy (`WEIGHT_POLICY`, digested with the invocation
  projection into the compiler digest and the invocation's `projection_digest`). Vertices: every
  subsystem function, and every official-usage caller (a function or module of an example, test
  or doc block) with an arc into one. Arcs: each invocation arc into a subsystem function from a
  subsystem function or a usage caller (call and definition arcs, any accepted modality: the
  review's F3(c), now named in the policy), weighted by the arc count per ordered pair. A function
  ranks by the official usage that reaches it, directly or through the library's own delegation.
- **Iteration.** `r'ⱼ = (1−d)/n + d·(Σᵢ rᵢ·wᵢⱼ/Wᵢ + D/n)`, from the uniform start, over the arcs
  in canonical order, where `D` is the dangling mass; damping 0.85, L1 tolerance 1e-10, 100
  iterations (pre-registered code, D29). The `pagerank` invocation records iterations, the final
  L1 residual, `converged` and diagnostics; each public API (`cpg_schema::communities`'
  public callables) is a `centrality` finding whose score is its rank.
- **Tests** (`ranking::tests`): the hand-computed 3-node fixed point; parallel arcs counted with
  their weights; a 2-iteration budget reported as not converged; a permuted projection giving an
  identical usage graph, scores and counts (the review's F6(b); the earlier reversed-rows test
  could not fail); `compute_flow` agreeing to 1e-9 on one hand-made graph and on 30 random
  weighted digraphs of 5–44 vertices, a quarter dangling (the review's probe 2); direct usage
  ranking what usage calls above the sink its delegation reaches, which PageRank ranks first.
  **The usage branch is Tested** (the review's F6(a)): the unit test's usage caller, and
  `selection_takes_what_usage_calls_within_the_budget` on `docs_shapes` (counts `make_server` 12,
  `Server.tool` 9, `run` 2, `stop` 2). On `analysis_shapes`, with no usage code,
  `public_apis_are_ranked_over_the_usage_projection` runs the `+pagerank` variant.
- **Pilot (Measured, 2026-09-23, snapshot `25e8465c`).** 4,444 vertices (607 subsystem
  functions, 3,837 usage callers), 8,080 weighted arcs (total weight 9,346), 273 dangling;
  converged in 38 iterations. 328 public APIs ranked: `FastMCP.tool` 7th, `resource` 13th,
  `mount` 43rd, `prompt` 65th, `custom_route` 169th. The top of the ranking is helpers the whole surface delegates
  to (`fastmcp.decorators.get_fastmcp_meta`, `fastmcp.server.dependencies.get_http_request`),
  which is what PageRank over delegation measures, and why the review moved the default to direct
  usage.
- **Direct usage on the pilot (Measured, 2026-09-23, snapshot `a8591982`).** 7,030.5 calls reach
  257 subsystem functions; 219 public APIs are `direct_usage` findings. The top: `FastMCP.__init__`
  1,185.5, `tool` 674, `resource` 429, `call_tool` 320, `list_tools` 249, `mount` 198,
  `add_transform` 180, `add_provider` 174, `Tool.from_function` 162, `add_middleware` 142: the
  review's direct-usage list (§8 of that review), which it computed independently.

### §9.6 Formal and relational concept analysis

**Off by default since ADR-0020** (the §9.8 keep rule, 2026-09-23): the `+fca` variant, with `+rca`. The record below stands as the variants'.

- **Consumer.** Applicable cases and modes, shared controls, and implication-style assertions
  (e.g. "every writer accepting `filesystem` also accepts `format`").
- **FCA (increment 2).**
  - Our own NextClosure (Ganter, ICFCA 2010) over `fixedbitset`, which also yields the
    Duquenne–Guigues implication basis; FCbO (Outrata & Vychodil 2012) only if the concept count
    exceeds the budget. No usable crate exists: odis is AGPL, fcars enumerates concepts only. So
    `fcars =0.2.2` is a dev-dependency **oracle** for concept sets. No cover relation is computed,
    so none is checked (the increment-2 review's F8; Python `concepts` 0.9.2 would be its oracle).
  - Objects: the public APIs of one **structurally defined scope**: the subsystem, one module, or
    one class hierarchy. Communities are never an FCA scope, because their membership is
    statistical.
  - Attributes: parameter names, parameter and return types, raised exception types, decorators.
  - A support threshold is applied. There is no stability index: it is #P-hard.
- **RCA (increment 3).** Adds one relational-scaling step (∃-scaling over calls and handoffs), a
  DataFusion join that adds attribute columns to the same FCA.
  It is kept only if the ablation shows it changes published output.
  **Implemented and Tested in slice 3.2** as the `+rca` variant (2026-09-23; D41):
  `lctx_analytics::concepts::relational` adds, for each object of a scope, `calls X` for each
  call arc (definite or candidate) into a subsystem function, and `hands off to X` / `takes from
  X` for each handoff the Pass C relation holds. X is the partner's preferred public path, else its
  qualified name. The pairs come from the in-memory projection and Pass C's relation, so no new
  SQL is needed; `RCA_POLICY` joins the FCA invocations' parameters and relation digest. Stage F
  reads each seed's attributes as the context held them (`AnalysisRows.seed_attributes`), and
  the templates say "calls `X`", "has its result passed to `X` in official usage". Tested by
  `relations_become_attributes_of_the_objects_in_scope` and
  `variants_add_relational_attributes_and_layers`. **Pilot (Measured, 2026-09-23, fake vectors,
  snapshot `904d86eb`, `+rca,+type-layer,+mention-layer,+knn-layer`):** the `fastmcp.FastMCP`
  scope grows from 193 to 363 attributes, 95 to 111 concepts and 104 to 131 implications; 16
  relational attributes appear in concepts or implications.
- **Output.**
  - Concepts become `applicable_case` findings (the kind's name is historical: a brief states one
    as a **shared signature** under Related, not as the Applicable case; the increment-2 review's
    U2).
  - Implications with confidence 1 over the support threshold become `implication` findings.
  - Both are `structurally_observed`, because they are exact over the extracted attributes, with
    the attribute scope stated.

**Implemented** and **Tested** in slice 2.5 (2026-09-23; deviation log D32):
- **Scope.** Each seed's structural scope is the public namespace its access path names: the
  public APIs `container.x` of the exported class or module `container` (`fastmcp.FastMCP` for
  `fastmcp.FastMCP.tool`: its public methods, declared or inherited; a package's own module when
  no export names it). One FCA invocation runs per distinct scope with at least two APIs.
- **Attributes** (`cpg_schema::concepts::attributes_sql`, digested): `parameter NAME` (`*`/`**`
  for the catch-alls; the receiver aside by the method's kind), `parameter type T` (declared),
  `returns T` (declared), `raises E` (the typed `raise`s directly in the body; a bare re-raise has
  no type, C4 review O4) and `decorator D`.
- **Kernel** (`lctx_analytics::concepts`): our own NextClosure over `fixedbitset` enumerating every
  set closed under the implications found so far, so one pass gives the frequent concepts and the
  frequent Duquenne–Guigues basis. Pruning infrequent candidates is exact: a larger set never has
  a larger extent. Parameters (pre-registered code, frozen with the others): support 2 APIs,
  budget 20,000 closed sets (a budget stop is `concept_budget`, completion `partial`).
- **Findings.** Each frequent concept with a non-empty intent is an `applicable_case` finding
  (`extent_member` APIs, `intent_attribute`s; score = extent size); each basis implication an
  `implication` finding (`premise`, `conclusion`; score = support). The subject is the scope.
- **Stage F** (as revised by the increment-2 review, F1 and U2; deviation log D38). A seed's
  `shared_signature` assertion (section Related) is the concept of **its own** scope that holds it
  with another API, shares at least two attributes, and has the most (other API, shared
  attribute) pairs, |intent| · (|extent| − 1), then the larger intent: "Like `A`, `B` and `C` (4
  public APIs of `S` in all), `X` declares … and raises `E` directly in its body." Up to three
  `implication` assertions (section Important controls) are the best-supported implications of
  its own scope whose premise the seed meets. Both are `structurally_observed`, and the scope is
  named in the text. Neither enters the brief document's header. The Applicable-case slot stays
  **absent** until an input-or-mode source exists, so the §B11 gap metric sees it. The floor,
  caps and counts are `selection::Params`, frozen (F7). *Superseded:* 2.5 published the concept
  as the Applicable case ("belongs with"), from any scope holding the seed, in the document
  header.
- **Scopes are nodes** (the review's F1). Each seed's access-path container resolves to its class
  or module node; container strings naming one node are one scope, one invocation, labelled by
  the fewest-segment, then least, of them. Each seed records its scope (`AnalysisRows.
  seed_scopes`); Stage F reads concepts and implications of that scope only, so a method inherited
  into two scopes is never described through the other one. Tested by
  `fca_scopes_are_nodes_and_state_only_their_own_apis` (the review's probes 3 and 4: two paths to
  `Catalog` compile as one scope; `Widgets(Catalog)` adds a family, and `Catalog.add_tool`'s lines
  name only `pkg.Catalog` APIs).
- **Attributes from term structure** (the review's F2). A type Pyrefly could not determine is no
  attribute: a term that is, or holds anywhere in its structure (`type_term_args`, a recursive
  walk), an `Any` of style `error` or `implicit` (displayed `Unknown`). A raised class is one
  attribute whether raised as the class (`raise E`, a `ClassObject` term) or an instance; only
  class-typed raises count. The text says what an API does with each attribute's scope
  ("declares …", "raises `E` directly in its body", "is decorated with …"). The rule
  `semantic:concept-attribute-known` is a tripwire over labels. Tested in
  `concepts_come_from_each_seeds_structural_scope` (`Catalog.load`/`reload` return an unresolvable
  type and raise `KeyError` both ways: one `raises KeyError`, no `Unknown`).
- **Tests** (`concepts::tests`): the hand-computed context of §12 (seven concepts; the basis
  `b → a`, `c → a`, `{a,d} → {b,c}`); fcars 0.2.2 agreeing on random contexts at supports 0, 2
  and 4; the basis sound and complete (closing every subset under it gives its Galois closure);
  the budget stop; the frequent basis sound, complete over every frequent set and non-redundant at
  supports 1–4 on 29 random contexts (the review's probe 1, F6(c)).
  `concepts_come_from_each_seeds_structural_scope` on `analysis_shapes` (`pkg.Catalog`: a
  registration family, two loaders and an unrelated method).
- **Pilot (Measured, 2026-09-23, snapshot `57039be8`; before the increment-2 review).** One scope,
  `fastmcp.FastMCP`: 51 public APIs, 195 attributes, 97 frequent concepts and 107 implications from 205 closed sets, far under
  the budget. `tool`, `resource` and `prompt` share an applicable case of eight parameters
  (`auth`, `description`, `icons`, `meta`, `name`, `tags`, `title`, `version`) and six declared
  parameter types; `mount`'s is five APIs sharing a `str | None` parameter and raising
  `ValueError`. Every seed had an applicable case, which the review showed filled the slot by
  construction (U2), and 1–3 implications, 22 of the 107 implications carrying `Unknown` (F2).
  The longest brief document is 1,429 bytes: none is split on the pilot.
- **Pilot after the review (Measured, 2026-09-23, snapshot `a8591982`).** The one scope,
  `fastmcp.FastMCP`: 51 APIs, 193 attributes, 95 concepts and 104 implications from 200 closed
  sets; no attribute is `Unknown`. `tool`, `resource` and `prompt` each carry the shared signature
  of the eight registration parameters and six declared types; `mount`'s ("Like `http_app`,
  `run_http_async`, `__init__` and `resource` … declares a parameter typed `str | None` and raises
  `ValueError` directly in its body") and `custom_route`'s (`name` and a `str` parameter) are the
  coincidental overlaps the review named, now under Related and out of the retrieval header. The
  Applicable-case slot is absent on all five briefs (`absent_slots`: `applicable_case` 5).

### §9.7 Embeddings in analytics

**Off by default since ADR-0020** (the §9.8 keep rule, 2026-09-23): the `+knn` variant. The record below stands as the variant's.

- **Consumers:**
  - linking doc passages to APIs, supplementing explicit mentions;
  - labelling communities by their nearest doc heading;
  - kNN as an optional community layer.
- **Method.** Vectors come from the cache (§11.1). kNN runs in Rust with a fixed `k` and a
  similarity margin.
- **Output.** `statistically_derived` findings.

**Implemented** and **Tested** in slice 3.1 (2026-09-23; deviation log D35):
- **E0** (`cpg-core::embed::embed_texts`), in Stage E after PageRank, with an embedder configured.
  It embeds the corpus passages (`cpg_schema::neighbours::passages_sql`) and the subsystem's
  public APIs (`path(parameters)` and the docstring; `api_texts_sql`, text version 1) as
  documents under the spec. Each text is cut into windows of at most 4,096 bytes at line ends,
  never dropping text. Vectors are read from the cache or embedded and merged; E0's keys join the
  snapshot's key set in `content_digest`.
- **kNN** (`lctx_analytics::neighbours`): exact, the best cosine over window pairs. Each API's
  three nearest passages at or above 0.5 are `doc_link` findings; each reported community's
  centroid takes its nearest passage's heading as a `community_label`. Both are
  `statistically_derived`, and one `knn` invocation records the candidate set (APIs × passages)
  and the counts. The parameters (k 3, floor 0.5, 4,096-byte windows) are pre-registered code,
  frozen with the others.
- **Stage F.** A `doc_link` assertion (section Related) lists the operation's nearest
  documentation with its cosines; the Related line names its community's label. Neither feeds the
  Outcome.
- **kNN as a community layer:** the `+knn-layer` variant (slice 3.2, §9.4). E0 now runs before
  the communities whenever kNN or the kNN layer reads its vectors.
- **Tests.** `neighbours::tests`: windows never drop text; exact search, the floor and node-order
  ties; a community labelled by its centroid's nearest heading.
  `doc_links_come_from_embedding_similarity` runs `docs_shapes` end to end with a bag-of-words
  test embedder: `Server.tool` links the quickstart's "Install" section (0.77), and its brief
  carries the link, `statistically_derived`.
- **Pilot (Measured, 2026-09-23).** Fake vectors (snapshot `0f8b911f`): E0 covers 2,079 windows
  (328 APIs; 1,751 windows of 1,608 passages), 527,424 candidate pairs, and no link clears the
  floor, as expected of unrelated hash vectors; Stage E takes 3.8 s instead of 2.2. **Live
  vectors** (Qwen3-Embedding-8B via `just pilot-live`, snapshot `89d3d4d0`; the service was
  stopped afterwards): 957 links, 323 of 328 APIs linked, cosines 0.50–0.91 (mean 0.70), and all
  29 communities labelled. Each seed's links are its own documentation (`FastMCP.tool`: "tools.mdx §
  The @tool Decorator" 0.85; `mount`: "composition.mdx § Namespacing" 0.84); Stage E takes 46.6 s,
  embedding included. The §1.5 ranking check on that generation is **failed**: `FastMCP.tool`
  is first for 0 of 2 `fm.register` aliases (ranks 4 and 2; 1 of 2 at slice 1.9). 3.3's
  pre-registered evaluation judges it.

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

**The ablation, Measured in slice 3.3** (2026-09-23; ADR-0020; deviation log D42, D43). Every
variant was compiled at budget 20 with live vectors, scored by `scripts/score_gold.py --embedder
vllm` (deterministic: two rescorings were identical) and diffed by `lctx diff`. Hits are counted
over all 44 gold aliases:

| Variant | hit@5 | hit@1 | (a) | touched | (c) | briefs +/−/~ |
|---|---|---|---|---|---|---|
| default then (communities, FCA, kNN) | 14 | 7 | 0.0444 | 8 | 4 | — |
| `-communities` | 13 | 9 | 0.0430 | 7 | 5 | 12/12/7 |
| `-fca` | 14 | 7 | 0.0444 | 8 | 4 | 0/0/14 |
| `-knn` | 14 | 7 | 0.0444 | 8 | 4 | 0/0/20 |
| `+pagerank` | 13 | 8 | 0.0442 | 8 | 4 | 10/10/7 |
| `+rca` | 14 | 7 | 0.0444 | 8 | 4 | 0/0/10 |
| `+type-layer` | 14 | 8 | 0.0509 | 9 | 6 | 3/3/17 |
| `+mention-layer` | 14 | 7 | 0.0444 | 8 | 4 | 0/0/18 |
| `+knn-layer` | 13 | 7 | 0.0444 | 8 | 4 | 1/1/19 |
| `-communities,-fca,-knn` (the kept set) | 13 | 9 | 0.0430 | 7 | 5 | — |

- **Keep decisions.** Communities improve hit@5 by one and lower (c) by one, so they are not kept.
  FCA and kNN change output but leave hit@5 unchanged, so they are not kept. No off-by-default
  variant improves hit@5, so none is adopted. **The default is now Passes A–C, direct usage and
  selection** (ADR-0020); the rest are variants.
- **Every difference is one or two units.** No margin is added after the fact. The increment-5
  held-out evaluation is the unbiased check, and it is ADR-0020's revisit trigger.
- **The §1.5 ranking check** is **failed** on both the old default (`FastMCP.tool` first for 1 of
  2 `fm.register` aliases) and the kept set (0 of 2).

> Decision: ADR-0020

**Variants, Implemented in slices 3.2 and 3.3 groundwork** (2026-09-23; deviation log D41):
- `lctx compile --analytics <variant>`: `default`, or changes to it by name, `+name`/`-name`
  (`cpg_core::analyze::Techniques`). Since ADR-0020, every technique is off by default:
  `communities`, `fca`, `knn`, `pagerank` (the increment-2 review's U1), `rca`, `type-layer`,
  `mention-layer` and `knn-layer`. A community layer needs `communities`, the kNN layer needs
  an embedder, and `rca` needs `fca` (`Techniques::parse` refuses each otherwise).
- **The whole technique set** (its JSON, every flag) joins the compiler run's config digest (the
  holistic assessment's A2(a), 2026-09-24, closing the ADR-0020 review's F3), so no two sets share
  a run id, and a variant snapshot never shares a content digest with another set's. The label
  names the techniques that are on (`none` when none are). The selection invocation records the
  set, so its parameters, and so its id, differ per set; every other invocation records only what
  its technique changed (`rca` in the FCA parameters; `extra_layers` and the weight rule in the
  consensus's). A finding the variant does not touch keeps its id, so an ablation diff is a join
  (`variants_add_relational_attributes_and_layers`). `lctx diff` compares findings, assertions,
  evidence, briefs and documents, not invocations.

> Decision: ADR-0011, ADR-0019, ADR-0005

---

## §10 Synthesis and briefs

**Proposed.** Source: IP L1695–L1731, L1886–L1966, L2580–L2637. Changed by ADR-0005: there is no
LLM interpreter. The increment-1 kinds, the kind policy, status derivation, the Outcome order and
the grounding rules below are **Implemented** and **Tested** (slice 1.5 and its review fixes,
2026-09-23; `cpg-core::synth`):
- `briefs_are_synthesized_from_findings_and_verbatim_evidence` snapshots each brief of
  `analysis_shapes`. It checks every spanned evidence text byte-for-byte against its source, some
  past a non-ASCII byte.
- `templates_say_what_the_findings_show` checks the call-site floor, depth-2 boundaries and the
  override-open hop.
- `an_outcome_from_the_docs_is_the_sentence_that_mentions_the_seed` covers Outcome leg 2 on a
  corpus, with passage evidence byte-checked.
- `synthesis_ids_ignore_the_runs_they_cite`, `synthesis_ids_follow_their_identity_columns` and
  the determinism test cover identity.
- Each rule rejects an injected violation in `the_analysis_rules_reject_their_violations`.
  The status rule, a floor and a ceiling, has three cases.

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
- **Status derivation** (`findings::derive_status`; slice 1.5 review F1). An assertion's status is
  a function of its supports alone, never chosen:
  - `unresolved` if it has no `support` row, or if any supporting finding is unresolved;
  - otherwise `statistically_derived` if any supporting finding, **or any finding that defined
    its scope**, is `statistically_derived`;
  - otherwise the strongest status among its supports, by `STATUS_STRENGTH`
    (structurally observed < documented < fixture checked).
  - A finding supports with its own status. An evidence row supports with its kind's
    (`EVIDENCE_STATUS`): a fact is structural; a docstring span, a passage or an example is
    documented; a fixture run is fixture checked.
- **Validator.** `semantic:assertion-policy` checks the kind policy.
  `semantic:assertion-status-derived` recomputes the derivation in SQL and requires equality: a
  floor and a ceiling.

**Increment-1 assertion kinds**

| Kind | Brief section | From | Permitted statuses |
|---|---|---|---|
| `outcome` | Outcome | docstring summary or explicit doc mention | documented, unresolved |
| `public_access` | Public access | Pass A `public_alias`, `exports` | structurally_observed |
| `coordinates` | Public access, "already coordinates" | Pass A `direct_delegation`, `bounded_delegation_path` | structurally_observed |
| `parameter` | Important controls | `parameters` (name, kind, default, required) and, from 2.1, `parameter_docs` (the description, its span cited); since A3, else a top-level `<ParamField>`'s lead (§10.3), citing its anchoring mention as `scope` | structurally_observed, documented (with parameter-doc evidence, or a `<ParamField>` lead under `semantic:documented-parameter-cites-its-doc`; deviation log D8) |
| `control` (2.1) | Important controls | Pass B `forwarding`, per seed parameter, with the path qualifiers | structurally_observed (documented from 3.4, with its rule; slice 2.1 review F7) |
| `transformed_control` (2.1) | Important controls | Pass B `transformed_argument` | structurally_observed |
| `restriction` (2.1) | Limits and prerequisites | Pass B `conditional_raise`, with the test's and raise's syntax facts and the path qualifiers | structurally_observed (documented only with precondition documentation, from 3.4 with its rule; review F7) |
| `unfollowed_control` (2.1 review) | Limits and prerequisites | Pass B `unfollowed_argument`, per seed parameter | structurally_observed |
| `usage_pattern` (2.2) | Usage pattern | `cpg_core::usage`: a verbatim statement subset of official usage code, its `example` spans, and each handoff it shows | documented, fixture_checked |
| `handoff` (2.2) | Usage pattern | Pass C `handoff`: the other callable, the formal and the occurrence count | structurally_observed |
| `analysis_boundary` | Limits and prerequisites | `boundaries` on the seed's neighbourhood | structurally_observed |
| `related` | Related | community co-membership, ordered by direct usage (PageRank in its variant) | statistically_derived |
| `applicable_case` | Applicable input or mode | reserved: no v1 source (the increment-2 review's U2) | structurally_observed |
| `implication` (2.5) | Important controls | an FCA `implication` of the seed's own scope whose premise it meets | structurally_observed |
| `doc_link` (3.1) | Related | kNN `doc_link` findings | statistically_derived |
| `shared_signature` (increment-2 review) | Related | the FCA concept of the seed's own scope with the most shared pairs | structurally_observed |
| `documented_warning` (3.4; A3) | Limits and prerequisites | a `<Warning>` component of a passage that mentions the seed exactly, its inner bytes cited, and its anchoring mention cited as `scope` (§10.3; `semantic:documented-warning-anchored`) | documented |

### §10.3 Brief structure and the Outcome order

**One brief = one outcome, anchored to one public operation**, optionally with one direct handoff.

| Section | Filled from |
|---|---|
| Outcome | the order below |
| Public access | Pass A `public_alias` / exports, plus "already coordinates" from Pass A delegation findings (with the note that these are implementation details the caller does not need to rebuild) |
| Applicable input or mode | an input or mode source: none in v1, so the slot is absent (the increment-2 review's U2; FCA concepts are shared signatures under Related) |
| Important controls | Pass B forwarding + parameter docs |
| Usage pattern | Pass C handoff or official example, with setup preserved |
| Limits and prerequisites | Pass B restrictions, documented warnings, boundaries |
| Evidence | all cited findings and evidence ids |
| Related | other public APIs of the seed's community, at most five, the most called in official usage first (`statistically_derived`; slice 2.6, the increment-2 review's U1); the seed's shared signature (`structurally_observed`); doc links (`statistically_derived`) |

**Outcome order.** Take the first source that applies:
1. the entry point's docstring summary: the first sentence of the docstring's first paragraph;
2. the lead sentence of a doc paragraph that **explicitly** mentions the entry point, when the
   mention lies inside that sentence;
3. otherwise `unresolved`.

Sentences are found with UAX #29 on a view in which each line break is a space, so a hard-wrapped
sentence is one. The evidence is the source bytes; the assertion reads the sentence with its
whitespace collapsed. Change-log documents (`synth::CHANGELOG_STEMS`) never give an Outcome
(slice 1.5 review F2, O1; deviation log D7).

The nearest doc passage by embedding is never an Outcome. It is published as a doc link
(`statistically_derived`), which keeps the gap metric honest.

**The count of `unresolved` slots, by section, is the §B11 gap metric.** Read from a published
snapshot with `lctx query`:
`SELECT p.brief_section, count(*) FROM assertions a JOIN assertion_policy p ON
p.assertion_kind = a.assertion_kind AND p.evidence_status = a.evidence_status WHERE
a.evidence_status = 4 GROUP BY p.brief_section ORDER BY 1`.

**How increment 1 fills the sections** (slice 1.5, `cpg-core::synth`):
- **Outcome:** the seed's docstring summary.
  - It is located in the literal's own source bytes. Pyrefly's `Docstring::clean` renders
    Markdown and loses spans.
  - Its paragraph ends at a blank line, a section header or the closing quote.
  - Else the first exact mention, by document path, passage ordinal and position, whose
    paragraph's lead sentence holds it. The mention is of the seed's declaration, or of an export
    whose target is the seed: a class's export does not name its method.
  - Else `unresolved`. The evidence is the byte span.
- **Public access:** the call form the seed's declaration gives:
  - a function is called, and a class constructed;
  - a method, class method, static method or property is named with its class and how to use it.
  - Then its configured access path, and every other public access path naming it (Pass A's
    `public_alias`).
- **Already coordinates:** one assertion per delegation finding, total over its fields:
  - at depth 1: a direct call with its number of call sites ("or more" when the witness cap
    omitted some), a definition, a property read or set, or an override-open call;
  - deeper: the intermediate callables, with each hop that is a definition, a property access or
    an override-open call named at the step where it occurs.
- **Important controls:** one `parameter` assertion per parameter of the seed's own signature (the
  receiver aside): kind, default, requiredness and annotation.
  - Its evidence is the syntax fact and span, plus Pysa's semantics fact for requiredness.
  - Where Pysa has no semantics row (an overloaded seed), it says "requiredness not observed".
  - Its description, when the documentation gives one (`documented`): the docstring's
    (`parameter_docs`, citing its span). Otherwise (A3; Implemented and Tested, 2026-09-24) the
    lead paragraph of a **top-level** `<ParamField>` whose literal `body` is the parameter's
    name, in a passage that mentions the seed exactly (changelogs are never read), citing that
    paragraph's bytes as Passage evidence and the anchoring mention as `scope` (R1 F1). *Top-level*
    means **no `<ParamField>` ancestor**: a `<Card>` or `<Expandable>` around the field does not
    count (R1 F3; the schema's depth 0 is another thing). If several such fields disagree after
    normalization, none is used. A `<ParamField>` nested in another describes a field of its
    parent's value, never a parameter. Fields named by `path`, `query` or `header` are not read
    (R1-3).
- **Limits:** one `analysis_boundary` assertion per stop reason (dependencies, synthesized
  callables, release code outside the subsystem, unresolved sites). What the seed calls itself
  is kept apart from what the callables it reaches call. Plus the depth bound or a budget
  truncation, citing Pass A's `traversal_stop` finding.
- **Documented warnings** (Limits; slice 3.4, rebuilt on components by A3, Implemented and
  Tested 2026-09-24): one `documented_warning` assertion, `documented`, per `<Warning>` component
  of a passage that mentions the seed exactly (changelogs are never read). Its text is the
  warning's inner bytes, verbatim, with where it is ("in `docs/x.mdx` § Heading (which mentions
  `seed`)") and its literal `title` when it has one, citing those bytes as Passage evidence.
  Inside a `<ParamField>`, the warning is about the **nearest** such field's parameter and says so
  ("about the parameter `p`"); if `p` is not a parameter of the seed other than its receiver, the
  warning is skipped. A `<Warning>` in fenced or inline code is not a component, so it is never
  read. The anchoring mention is cited as `scope`, and `semantic:documented-warning-anchored`
  checks the quote, the anchor and the scoping (R1 F1). Warnings enter the brief document as the
  capability's own limits.
- **Tested** (R1 F2, 2026-09-24; `docs_shapes`, `a_documented_warning_is_a_limit`,
  `documentation_statements_are_anchored_to_their_seed`): a warning under a field that is no
  parameter is skipped; the nearest field decides (a warning under `backoff`, itself inside the
  parameter `retries`, is skipped); a field nested under `ParamField > Expandable` describes
  nothing; a field inside a `<Card>` describes its parameter; a warning in a passage that mentions
  no seed is not stated; each rule rejects a doctored anchor, quote, scope or nesting.
- **The components are Mintlify's** (`cpg_schema::mdx`, read by Stage F and the rules; R1 F5).
  A library documented another way has none, so a zero there means no such source.
- **Measured on the pilot** (2026-09-24, snapshot `8c76bc75…`, 20 briefs): the corpus has 1,250
  components (450 nested, depth at most 3; 1,605 attributes: 1,504 literal, 70 expression, 31
  bare), including 93 `<Warning>`s and 173 `<ParamField>`s. **None reaches a brief:** no documented
  warning is stated and no parameter is described from a `<ParamField>` (60 of 97 are described
  by their docstrings). Only one passage holding a warning has any exact mention. The passages
  that document the seeds' arguments (`docs/servers/tools.mdx` § Decorator Arguments, and the like
  for `resource` and `prompt`) name the operation as `@mcp.tool`, a receiver-variable spelling
  that is no exact mention. A code-block call anchor is **a probe, not a measurement** (R1 F4): the
  author's count was 13 warnings in 7 briefs, the reviewer's reconstruction 14 in 9, with no
  committed query; both reach no `<ParamField>` (that block's `mcp` is unbound), and the anchor is
  noisy (`TransportMixin.run` anchors 44 passages through `mcp.run()`). **The exact anchor stays**
  (R1 §10; deviation log D45). One widening candidate, a receiver-resolved mention, is registered
  as a content variant in the ADR-0020 re-registration and decided by the structured evaluation;
  the heading/title and code-block anchors are rejected, each with a reopening trigger.
- **The brief document** (§11.1): outcome, applicable case (when one exists: none in v1),
  public APIs, control names, usage and limits. Over 2,048 tokens (a declared proxy of 4 bytes per
  token) it is split into chunks of whole parts, each under the header (the outcome and any
  applicable case), never truncated; a part too long for any chunk fails the compile (slice 2.5,
  `chunked`). Retrieval scores a brief by its best chunk.
- A template or extractive-rule change bumps `synth::TEMPLATE_VERSION`; the analysis output is
  pinned to the versions in a test ledger (ADR-0019 review O4).

### §10.4 Grounding checks

These are mechanical and run before publication.
- Every named public symbol and parameter exists in this snapshot.
- Every cited finding, witness path and evidence id exists in this snapshot.
- Every snippet parses, and every name it loads is bound in it, imported by it or a builtin (§10.5).
  Whether each imported API exists is the release's to say: a pattern is verbatim official code
  (slice 2.2 review F1(d), narrowed).
- No assertion's status exceeds its evidence (see the §10.2 rule).
- Warnings and unresolved conditions are never dropped for length. An over-long brief's document
  is chunked at whole parts instead (§10.3); the brief itself is never split (the increment-2
  review's F8).

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

**Implemented** and **Tested** in slice 2.2, revised by its compact review (2026-09-23;
`cpg_core::usage`; deviation log D24, D30):
- **Selection.** Candidates are the seed's call and decorator sites in official examples, then doc
  blocks, then tests; within a role a handoff's sites are tried first. The pattern that shows a
  whole handoff occurrence (producer and consumer site) wins within its role, then the smallest.
- **The pattern.** It is the statement holding the site, in a module or function body only (never
  under a `with`, `if`, loop or `try` header it would drop, such as `with pytest.raises`), plus,
  transitively, the same-block statements before it that bind a name it reads, and the imports
  binding one. A candidate that reads a name bound anywhere else, or bound nowhere (neither a
  binding nor a builtin: a continuation doc block's, C5 O2), is not self-contained and is refused.
- **Publication.** Each statement is dedented by its own indentation. The code must parse (Ruff),
  and every name it loads, annotations included, must be bound in it, imported by it or a builtin
  of the analyzed Python (`ruff_python_stdlib`); otherwise the candidate is refused (review F1).
  `lctx_mcp.smoke` parses every served pattern again.
- **Assertion.** The pattern is a `usage_pattern` assertion (`documented`) citing each
  statement's `example` span verbatim, and each handoff it shows whole (review F3;
  `semantic:usage-pattern-shows-its-handoff`). Its code enters the brief document as §11.1's
  usage description.

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
  case (when one exists), public APIs, controls, usage description and limits, capped at 2,048
  tokens. An over-long document is chunked at whole parts under its header, never truncated
  (§10.3; the increment-2 review's F8).
  - The limits are the capability's own: Limits-section kinds other than `analysis_boundary`.
  - What the analysis did not follow (dependency and synthetic boundaries, unresolved sites, the
    depth bound) stays in the served brief, out of retrieval. Slice 1.9 measured this with live
    vectors (deviation log D14).
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
  client is held to as well; every rejection fires (`every_rejection_fires`). Both clients judge
  a response alike, Rust by its serde types and Python by pydantic strict models (the holistic
  assessment's A7: a JSON boolean is never an index or a component), held to one corpus of 25
  bodies, `specs/embedding/responses.json` (`responses_are_judged_as_the_shared_corpus_says` in
  both suites); a stub service
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

**Implemented** and **Tested** in slice 1.8 (2026-09-23; `lctx_mcp.retrieval`):
- the tokenizer is lower-cased runs of letters and digits;
- bm25s (`method="lucene"`, k1 1.5, b 0.75) equals a hand computation of the Lucene formula
  (`test_bm25_scores_are_the_lucene_formula`); unknown words count for nothing;
- a brief's vector score is its best chunk's cosine;
- RRF ties go to the lower brief id (`test_fusion_ranks_ties_by_id_and_promotes_exact_symbols`);
- a promoted brief ranks first;
- a down embedder gives `lexical-only` with its reason (`test_a_down_embedder_degrades_to_lexical_and_says_so`).
- Hybrid ranking with **real** vectors is 1.9's check. The fixture's fake vectors carry no
  meaning.

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
- **Implemented** and **Tested** in slice 1.8 (2026-09-23; `python/lctx_mcp`, 19 tests over the
  `just py-fixture` generation):
  - The round trip in both eras (`test_the_tools_round_trip_in_both_protocol_eras`).
  - Promotion, and degraded mode.
  - A tool error for an unknown library, snapshot or id, and for out-of-bounds arguments.
  - The resource as Markdown. An unknown id is invalid params, −32602, because `CapabilityError`
    is FastMCP's `ValidationError` (the holistic assessment's A7; it was `ResourceError`, which
    reached the wire as an internal error, −32603).
  - A schema digest, spec or key mismatch fails at load and at connect
    (`test_a_mismatched_generation_fails_at_connect`).
  - Byte-identical request bodies (`test_request_bodies_are_byte_identical_to_rusts`), and the
    fake twin (`test_the_fake_twin_reproduces_rusts_vectors`).
  - The serving digests' known answers.
  - A `StdioTransport` subprocess that writes nothing but the protocol
    (`test_the_server_speaks_only_the_protocol_on_stdout`).
  - Started as `python -m lctx_mcp --generation DIR --embedder vllm|fake|none`.

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

**The matcher, version 2** (pre-registered 2026-09-24 in ADR-0010's amendment, before any rescore;
the holistic assessment's A1, the ADR-0020 review's F1 and F9). Identity is the declaration node:
a gold operation resolves by exact path equality against the served `public_paths`, with no
fuzzy fallback, and a class operation matches only a brief seeded by that class. (a) is node
Jaccard, where a brief contributes its seed and unresolved operations stay in the union as
strings. (b) searches every task alias, and a hit is a returned brief whose seed is in the
family's node set. (c) is unchanged. All three are counted over all gold units (22 families, 44
aliases, 157 spans). Lexical text holds each distinct token once, promotion matches any public
spelling of the seed, and each score records `matcher_version`. Version 1 compared access-path
strings; the scores below are version 1's. **Implemented** (2026-09-24): `scripts/gold_match.py`,
shared by `score_gold.py` and `ranking_check.py`, reads the served `public_paths` (bundle
`FORMAT` 2); each alias records its mode, degraded reason and ranked hits, and a live run with a
degraded alias is `blocked` (exit 2); `just score <generation> [embedder]`. Tested over
constructed rows (`tests/scripts/test_gold_match.py`: an inherited spelling, a class operation,
unresolved operations, a degraded live run).

**Ablation** (§9.8). Diff the published output with each technique disabled, apply the keep
rule, and record the result in the increment-3 review. **Done in slice 3.3** (2026-09-23): the
table and the keep decisions are in §9.8 and ADR-0020.

**Gold scoring, Measured** (2026-09-23, `scripts/score_gold.py`, live vectors, budget 20): the
old default scored (a) 0.0444 over 22 families, 8 touched; (b) hit@5 14 and hit@1 7 of 44
aliases; (c) 4 of 157 spans. The kept default scores (a) 0.0430, 7 touched; (b) hit@5 13 and
hit@1 9; (c) 5 of 157. 19 of the gold's 125 operations are outside the release (`mcp`,
`mcp_types`, `fastmcp_tasks`, `pydantic`, `starlette`, `uncalled_for`), so no brief can name
them; the scorer reports them apart, and they count against (a).

**Agent evaluation** (increment 5).
- About 20 held-out task prompts and 5–10 usage fixtures.
- Raw-evidence retrieval is compared against compiled briefs under similar context budgets.
- Questions: was the right built-in capability found; did the agent discover the important
  control; did it use a supported handoff; did it respect the limits; did it avoid rebuilding
  library behavior?

**LLM trigger (§B11).** After increment 3, `unresolved` slot counts by brief section are reported.
If the increment-5 evaluation attributes failures to those slots, an ADR adds a local generation
model under §10.4 grounding.

> Decision: ADR-0004, ADR-0013, ADR-0020

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

> Decision: ADR-0012

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
| 2026-09-23 | Slice 1.4: table groups for analysis results, the `lctx-compiler` run, the declared invocation projection (§5) and its petgraph adapter, Pass A (§9.1) with `analysis_invocations`/`findings`/`finding_members`/`witnesses`; `libraries/fastmcp/analytics.toml` (docs-only authored); the ADR-0019 standard review's P1 items (identity and lineage per contract, completion mapping, status policy, `adr lint` checks for Decision lines and cited records) (§B6, §5, §6.4, §9, §9.1, §10.1) | ADR-0019 (amended), ADR-0004 |
| 2026-09-23 | Slice 1.5: Stage F synthesis (`cpg-core::synth`): the increment-1 assertion kinds, the kind policy published as `assertion_policy`, derived statuses, verbatim evidence by byte span, briefs, brief members and the brief document; eight tables and eight rules (policy, propagation, text, supports, evidence bytes, exported members, briefs cite analysis); `unicode-segmentation` pinned; the analysis-output ledger (§10, §10.2, §10.3) | ADR-0019 |
| 2026-09-23 | Slice 1.6: the embedding spec, the `Embedder` trait with a fake and the vLLM client (`lctx-embed`), request-body conformance, the global `embedding_cache` (read mode, insert-only MERGE probed at the pin, empty when unused), the keys digest in `content_digest`, `services/vllm` and `just embed-serve`/`pilot-live` (§3.2, §6.1, §6.2, §11.1) | ADR-0017 amendment, ADR-0010 amendment |
| 2026-09-23 | Slice 1.7: `lctx bundle`, byte-identical serving generations: nine served files declared in `cpg_schema::bundle`, the language-neutral serving schema digest with shared Rust/Python known answers, normalization by builders, the manifest and generation key; `brief_documents.spec_hash` and `embedding_specs` (§6.4, §11.1) | ADR-0019 |
| 2026-09-23 | Slice 1.8: `python/lctx_mcp` (FastMCP server, hybrid retrieval, the query embedder and its fake twin, conformance), the uv workspace, the gold extract (§11, §12) | ADR-0010 |
| 2026-09-23 | Slice 1.9: increment 1 end to end: the pilot's stdio smoke, the rerun oracle, the §1.5 ranking check measured with live vectors (failed on one alias), `just embed-conformance` (§1.5, §11.1) | ADR-0010 |
| 2026-09-23 | Slice 2.0: the seed audit and the pre-registered seed `fastmcp.FastMCP.mount` (§1.4) | ADR-0004 |
| 2026-09-23 | Slice 2.1: Pass B (`cpg_schema::flows`, `lctx_analytics::pass_b`), `parameter_docs`, the control/transformed-control/restriction kinds (§3.2, §9.2, §10.2) | ADR-0019 |
| 2026-09-23 | Increment-1 deep review fixes: the template version in `compiler_digest` (§3.4.1); the fusion policy and pre-registered retrieval evaluation (§11.2); absent slot sections served (§10.3); the budget enforced and `serve_unreviewed` removed; truthful call-site, may-call and definition templates; a raw-pipe stdio test; the gold freeze at 1.9 | ADR-0004, ADR-0010, ADR-0019 |
| 2026-09-23 | Slice 2.2: Pass C (`cpg_schema::flows::handoffs_sql`, `lctx_analytics::pass_c`) and usage patterns (`cpg_core::usage`), the `usage_pattern` and `handoff` kinds (§9.3, §10.2, §10.5) | ADR-0019 (amended) |
| 2026-09-23 | Slice 2.1 compact review fixes: the path-qualifier rule, handler (`try`/`with`), conditional and tested calls, `unfollowed_argument` and `unfollowed_control`, supported predicates, receivers by the declaration's kind, one mapping fragment, header-anchored and extended parameter descriptions; §4.2.4's trace; `control` and `restriction` narrowed to `structurally_observed` (§3.2, §4.2.4, §9.2, §10.2) | ADR-0019; deviation log D26, D27 |
| 2026-09-23 | Slice 2.3: communities (`cpg_schema::communities`, `lctx_analytics::communities`; leiden-rs RBER consensus), `analysis_invocations.diagnostics` (§3.2, §9.4) | ADR-0011 (amended); deviation log D28 |
| 2026-09-23 | Slice 2.4: centrality (`lctx_analytics::ranking`, weighted PageRank over the usage projection) (§9.5) | ADR-0011; deviation log D29 |
| 2026-09-23 | Slice 2.2 compact review fixes: release handoff endpoints, one-target producers, the narrowed receiver rule, doc blocks by document, self-contained usage patterns (block and free-name checks), citations of shown handoffs only; two rules; §10.2 rows for `usage_pattern` and `handoff`; §3.2, §9.3, §10.4, §10.5 (§3.2, §9.3, §10.2, §10.4, §10.5) | ADR-0019 (dated amendment); deviation log D30 |
| 2026-09-23 | ADR-0011 standard review fixes and ADR-0011 accepted: γ = 1 fixed with a recorded profile, named layer policies, the invocation projection in the community digest, the public co-assignment score, the parameters in the gold freeze (§9, §9.4) | ADR-0011 (accepted); ADR-0004 (amended); deviation log D31 |
| 2026-09-23 | Slice 2.5: FCA (`cpg_schema::concepts`, `lctx_analytics::concepts`: NextClosure concepts and the Duquenne–Guigues basis), `applicable_case` and `implication` findings and assertions, the applicable case in the brief document, over-cap documents split into chunks (§9.6, §10.2, §10.3) | ADR-0011; deviation log D32 |
| 2026-09-23 | Slice 2.6: seed selection within the brief budget (`lctx_analytics::selection`, a `seed_selection` invocation), Related from communities and centrality, the statistical-policy cases (§9.4, §10.3) | ADR-0011; deviation log D34 |
| 2026-09-23 | Slice 3.1: embeddings in analytics: E0 through the cache (`embed_texts`), exact kNN (`lctx_analytics::neighbours`), `doc_link` and `community_label` findings, the doc-link assertion and the Related label (§9.7) | ADR-0011; deviation log D35 |
| 2026-09-23 | Slice 3.3: `lctx diff`; the budget raised to 20 (ADR-0004 amendment); the ablation with live vectors; the keep rule applied, so communities, FCA and kNN are off by default and no variant is adopted (§B4, §9.4, §9.6, §9.7, §9.8) | ADR-0020; ADR-0004 amendment; deviation log D42, D43 |
| 2026-09-23 | Slice 3.2: RCA (`+rca`) and the type, mention and kNN community layers as analytics variants, off by default (§9.4, §9.6, §9.7, §9.8) | ADR-0011; deviation log D41 |
| 2026-09-23 | Increment-2 compact review fixes: direct usage as the default ranking and selection by it with a community cap (§9.4, §9.5; U1), the PageRank variant, one preferred path per callable (F4), FCA scopes keyed by node (F1), attributes from term structure (F2), the concept as a shared signature under Related with the Applicable-case slot absent (§9.6, §10.2, §10.3; U2), Stage E/F choices frozen (F7), the finding-kind-by-method rule; stale sentences fixed (F8) | ADR-0011 amendment; ADR-0019 amendment; deviation log D37–D40 |
| 2026-09-24 | Holistic assessment A3 (with slice 3.4's warnings): `doc_components` and `doc_component_attributes` from markdown-rs's mdast (codebooks `component_form`, `attribute_value_kind`; four semantic rules, `docs-span-in-document` among them); documented warnings from `<Warning>` components, scoped to their `<ParamField>`; a top-level `<ParamField>` as a parameter's description after the docstring; `EXTRACTOR_OUTPUT_VERSION` 19, `TEMPLATE_VERSION` 16; zero of either on the pilot, Measured (§3.2, §9.2, §10.3) | Deviation log D44, D45 |
