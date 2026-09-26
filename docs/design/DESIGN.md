# library-context — design

**This file plus `docs/design/sections/` is the architectural authority**; the
[architecture map](README.md) routes each responsibility to its owner. This file holds scope
(§1), the binding decisions (§2, §B1–§B14), the dependency-family rule (§7) and durable deferrals
(§13); focused owners hold the rest. Labels distinguish implemented behavior from accepted
targets: an accepted target is not an implementation claim. Current ADRs say why and what was
rejected ([index](../adr/README.md)); the
[forward plan](../plans/behavioral-model-forward-plan_2026-09-24.md) owns execution and the
disposition of known defects.

**Changing a section.** Change a governed section in the same commit as the ADR that decides it
and keep its `> Decision: ADR-NNNN` line. Section IDs are never renumbered or reused: insert
`§3.2.1` rather than shifting `§3.3`, and a moved live section leaves a relocation pointer. Keep
sections current rather than appending history (ADR-0042). There is no line budget: the detail the
design needs comes before length. Column-level contracts live in `cpg-schema` and are
snapshot-tested; they are not repeated here.

**Labels.** Every claim carries a principles §D label (`Proposed`, `Interface-checked`,
`Implemented`, `Tested`, `Measured`, `Formally established`). A section's label applies unless a
line says otherwise. `Interface-checked` means the named library surface was read at the pinned
version (skill brief, probe or pinned source), not that our code uses it yet.

---

<a id="section-1"></a>

## §1 Scope

<a id="section-1-1"></a>

### §1.1 Objective and the promise

**Accepted scope** (ADR-0021); operation states as labeled below.

A pinned Python library compiles into an **evidence-carrying behavioral model of its whole public
surface** for coding agents:
- which operations accept a value, pass it where, under which configuration, raising what, and
  needing which lifecycle;
- each claim with its evidence and derivation;
- an exhaustive answer where the analysis is complete, a ranked candidate where only discovery
  applies, and a named `unknown` where the analysis stopped ([§3.9](sections/behavior-model.md#section-3-9)).

The universe is the library's `public_paths`; every analysis runs per public callable. **Briefs
remain one rendering** for a curated subset ([§10](sections/synthesis-and-serving.md#section-10)).

| Operation | State | Returns | Does not promise |
|---|---|---|---|
| `search_capabilities(library, query, limit)` | **Implemented** | Published briefs relevant to a task, including ones whose API names differ from its wording | Exhaustive discovery |
| `get_capability(snapshot_id, capability_id)` | **Implemented** | The full brief | A synthesized solution for arbitrary requirements |
| `get_operation(snapshot_id, operation)` | **Implemented** (behavioral detail grows by stage) | One public operation's record: paths, signature, control fates and verdicts, behaviors with conditions, raises, callbacks, ambient reads, evidence, boundaries; its brief if one exists | Behavior the analysis did not reach, which is stated as `unknown` or `not_analyzed` |
| `find_operations(library, where, limit, cursor)` | **Implemented** for declared and materialized facets; semantic filters **Proposed** (Stage 3) | **Exhaustive** matches over the pinned generation, with `complete` and the operations whose answer is unknown; never vectors | Completeness where a boundary intervenes, which it names |
| `search_operations(library, query, filters, limit)` | **Implemented** | **Ranked** discovery over operations, labelled as such | Exhaustiveness |
| `lookup_concepts(text)` | **Proposed** (Stage 4) | Candidate capability concepts with scope notes | A single interpretation of ambiguous wording |
| `explain(snapshot_id, claim)` | **Proposed** (Stage 4) | The stored derivation: rule, premises, source spans | Proof of runtime behavior beyond the stated model |

Compile time publishes facts, summaries and lossless condition nodes. A request selects
materialized rows or, as a **Proposed** target, runs a bounded Rust semantic query over the same
immutable generation ([§11.3](sections/synthesis-and-serving.md#section-11-3), ADR-0025). No
generative model runs in the request path (§B11).

> Decision: ADR-0021, ADR-0025

<a id="section-1-2"></a>

### §1.2 Increments

**Implemented** for increments 1–3; increment 4 **in progress**; increment 5 **Proposed**.
Each increment is a working vertical slice; the [forward plan](../plans/behavioral-model-forward-plan_2026-09-24.md)
owns remaining execution and exit criteria.

| # | Deliverable | State |
|---|---|---|
| 1 | One complete path on real FastMCP: acquisition, extraction, the CPG, Pass A, briefs, the serving bundle, embeddings, hybrid search and both brief tools | Built |
| 2 | Passes B and C, community detection, centrality and FCA over declared structural scopes | Built (default analytics decided by §9.8) |
| 3 | The whole public surface: the `behavior` family, per-callable passes, the operation catalog and the three operation tools, structured evaluation v0 | Built |
| 4 | Stages 2.9–3: the `flow` family, conditions and verdicts, the condition kernel, models, L2 fates, transfer summaries and native serving | In progress |
| 5 | Stages 4–5: the capability registry, `lookup_concepts` and `explain`, framework models and protocols, then the held-out evaluation and the §B11 decision | Proposed |

**The CPG precedes its analytics** (ADR-0047): node and edge catalogs, then the `syntax`,
`lexical`, `types` and `docs`/usage-corpus families; every family names the consumer that reads
it. Libraries are pinned uv projects acquired by `lctx` (ADR-0046).

**Reviews** follow the repository binding's cadence (ADR-0040): design/target reviews for
architectural choices and assembled stages, change/conformance reviews for bounded slices; a
coincident stage/increment end needs one review.

**Development loop** (**Implemented**, ADR-0026): pinned stable Rust 1.98.1, 16 Cargo jobs, the
default single rustc frontend thread, sccache with incremental compilation off, Clang with mold
for Linux links; agents work on `main` in the current tree and use a separate worktree only for
truly concurrent production-code edits. Build performance of this configuration remains
**Proposed** until measured.

> Decision: ADR-0021, ADR-0047, ADR-0046, ADR-0026, ADR-0040

<a id="section-1-3"></a>

### §1.3 Non-goals

**Accepted** (ADR-0021).

- Analysis of the caller's own codebase.
- Comparison across library versions.
- Arbitrary composition planning.
- Constraint solving over requirements such as "under 500 MB". Conditions are Boolean functions
  over evaluation atoms with a narrow typed theory ([§3.9](sections/behavior-model.md#section-3-9),
  §B10), not a general solver.
- **Unbounded query-time traversal.** Paths are precomputed as summaries and witnesses; a request
  selects materialized rows or runs a bounded, budgeted semantic query
  ([§11.3](sections/synthesis-and-serving.md#section-11-3)).
- Native-extension bodies.
- **A general ontology, an RDF store or a reasoner.** The capability registry is a closed,
  executable vocabulary in `cpg-schema` ([§9.9](sections/behavioral-analysis.md#section-9-9)).
- **General alias analysis (points-to).** Flow is intraprocedural over bounded places (§3.9);
  summaries are interprocedural (§9.9).

> Decision: ADR-0021

<a id="section-1-4"></a>

### §1.4 Pilot, subsystem and gold reference

**Implemented and Tested** for the pilot and the gold guard; the subsystem and freezes are
**accepted** (ADR-0021, ADR-0046).

- **Library.** FastMCP **4.0.5**, acquired as `libraries/fastmcp`
  ([§4.0](sections/acquisition-and-extraction.md#section-4-0)) with the `fastmcp` skill's install
  line, `fastmcp[anthropic,openai,gemini,azure,apps,code-mode,tasks]==4.0.5`. The release is three
  distributions: the facade `fastmcp`, `fastmcp-slim` (the code) and `fastmcp-tasks`. `mcp` and
  `mcp-types` are dependency context, resolved but outside the analyzed boundary.
- **Subsystem.** The server-components surface, taken **only** from the pre-registered analytics
  config (`libraries/fastmcp/analytics.toml`: module prefixes and public roots), whose digest is part
  of `content_digest`. It scopes brief seeds and seed passes only. The behavioral model's universe
  is `public_paths` under the config's public roots (1,534 declarations on the pilot, Measured
  2026-09-24), and its scan follows parameters into any release function.
- **Gold reference.** The skill's reviewed capability families are used **only to evaluate**
  ([§12](sections/validation-and-evaluation.md#section-12)). Nothing under `.claude/skills/` is
  ever a compiler input: acquisition fetches its own pinned artifacts even when identical bytes
  exist in a skill cache. Gold scores are a record of brief retrieval; techniques are judged by
  the structured evaluation (§9.8), and the gold never tunes parameters.
- **One version.** `scripts/check_gold.py` (`just gold`, part of `just test-all`) fails when the
  skill's install line or resolved release versions differ from `libraries/fastmcp`.
- **Freezes.** The analytics config, the analytics and selection parameters in code and the
  variant policies are frozen by digest in `eval/gold/analytics-freeze.json`; `just gold` fails on
  an edit. Any edit needs a new ADR that names the gold its author has seen, recorded as the
  freeze file's `adr` (accepted records are not amended with new decisions, ADR-0042).
- **Serving** runs FastMCP from the project's own environment; that environment is never an
  analysis input.

> Decision: ADR-0021, ADR-0046

<a id="section-1-5"></a>

### §1.5 Definition of done

**Accepted** (ADR-0021).

The behavioral model is done when:
- each stage's exit criterion has passed on the pilot (stage exits are in the
  [forward plan §3–§5](../plans/behavioral-model-forward-plan_2026-09-24.md#5-evaluation-and-tooling));
- every stage's structured evaluation (`eval/behavior/`, targets pre-registered) has been assessed
  and reviewed by the operator;
- the held-out structured evaluation has run;
- no answer states `refuted_under_model` outside complete coverage (§3.9).

Briefs remain a rendering under two standing rules: no brief reaches publication by bypassing the
analytics (a brief explained by documentation alone is allowed and labelled so, with a
`documented` Outcome), and "analysis-backed" means citing a derivation family's positive finding
(`findings::ANALYSIS_BACKED`). `semantic:brief-cites-analysis` and
`semantic:documentation-only-has-outcome` enforce both (**Implemented and Tested**).

> Decision: ADR-0021

---

<a id="section-2"></a>

## §2 Binding decisions

**Implemented documentation policy and publisher** (ADR-0041, 2026-09-25). Focused
architecture pages retain stable section IDs; a moved live section leaves a relocation pointer.
One resolver drives ADR references and the generated section directory. Publication derives
navigation and search from sources; it does not prove implementation. [Publishing
operations](../publishing.md) owns the commands. Documentation upkeep follows changes to meaning,
boundaries and workflows; improved reading/change cost remains **Proposed**.

**Documentation lifecycle** (ADR-0042, 2026-09-25; tooling **Tested** by
`tests/scripts/test_adr.py`, reading-cost benefit **Proposed**). The checkout holds a current
working set: architecture owns contracts and targets, current ADRs own reasons and open choices,
the active plan owns execution and scheduled finding disposition, STATUS the checkpoint. A reader
never needs a supersession chain or retired plan. Retired records, reviews, plans and evidence are
recovered from Git ([historical recovery](../README.md#historical-recovery)); governing references
must resolve, historical `supersedes` entries need not, and ADR ids are never reused.

> Decision: ADR-0041, ADR-0042


**Implemented policy (ADR-0040, 2026-09-25).** These are the load-bearing choices. The
repository owns core 3.0 and declares its standard in
`docs/design_review/design_principles/standard.toml`. The six foundations organize review of
responsibilities, contracts, composition, authority, constraints and local reasoning. A1–A3
judge architecture independently of correctness and domain-fidelity gates. Supporting DP/CI IDs
retain their historical meaning at the version cited by each review.

The library-context binding maps §B decisions to the standard and owns review cadence and
finding disposition. Changing a §B decision needs an ADR and a design/target review. Library
selection follows the owned capability, expected changes and total integration burden.
Early design favors coherent boundaries and inexpensive revision, with compatibility and
operational obligations scoped to real consumers and claimed behavior.

DESIGN owns accepted architecture and labeled targets; executable schemas and rule declarations
own their detailed contracts. ADRs record decisions and alternatives. Reviews preserve dated
evidence, while the active plan owns current disposition of scheduled findings. STATUS links to
that owner. Acceptance, implementation and verification remain distinct facts.

> Decision: ADR-0040

<a id="section-b1"></a>

### §B1 Ruff and Pyrefly are the only semantic front ends

**Implemented and Tested** (in-process extraction, harness equivalence with the Pyrefly CLI).

- **Pyrefly**, linked in-process from a pinned, minimally patched fork (§B8,
  [§4.2](sections/acquisition-and-extraction.md#section-4-2)), supplies definitions and
  signatures, call resolution through its own Pysa collectors, class order, public names and
  types.
- **Ruff library crates** (the `=0.0.11` line Pyrefly compiles against) supply syntax by walking
  Pyrefly's own parse, so there is one parse and one byte coordinate system.
- **Binding history** comes from our own scope-aware recognizer over the Ruff AST
  ([§4.2.4](sections/acquisition-and-extraction.md#section-4-2-4)): Pyrefly's binding IR drops
  statically decided branches, and Ruff's semantic model has no public driver.
- **One declared exception:** `cpg-flow` reads ty's semantic index over a second parse (the ruff
  0.0.14 line) and contributes only flow facts, joined to ours by module and byte range under
  two-way parity rules ([§3.9](sections/behavior-model.md#section-3-9)). Every other fact comes
  from Pyrefly's parse. Otherwise gaps are closed with adapters, normalization and our own
  analyses, never a second type checker.

> Decision: ADR-0046

<a id="section-b2"></a>

### §B2 Arrow schemas are the authoritative data contract

**Implemented and Tested** for the fact, derived, catalog and analysis tables (contract, registry
and codebook snapshots).

- `cpg-schema` holds the Arrow `Schema` definitions, append-only codebooks, logical-id newtypes,
  key and reference declarations, the graph registry
  ([§3.8](sections/facts-and-identity.md#section-3-8)), physical storage mappings and
  Arrow-only batch builders and local validators. Its dependencies are `arrow-*`, `blake3` (ids), `serde`/`serde_json`/`toml` (typed declarations such as the model
  catalog), `sha2` and `biodivine-lib-bdd` (the condition kernel, whose general allowance is
  proposed in ADR-0024); never DataFusion, Delta, object_store, an async runtime or I/O. DataFusion
  validators live in core crates.
- **No inferred schemas**: none is inferred from JSON or a first batch, including the serving
  generation ([§6.4](sections/storage-and-publication.md#section-6-4)) and the embedding exchange
  ([§11.1](sections/synthesis-and-serving.md#section-11-1)).
- A schema change is a reviewed snapshot change and a declared migration; codebook codes are
  never renumbered or reordered.

> Decision: ADR-0047

<a id="section-b3"></a>

### §B3 DataFusion constructs and validates relations

**Implemented and Tested.**

- Derived nodes, edges and analysis inputs are joins, projections and unions over extracted
  facts, in DataFusion SQL declared beside their contracts.
- Cross-table invariants are DataFusion queries, one per rule
  ([§8](sections/validation-and-evaluation.md#section-8)); the same validators run in tests and
  before publication.

<a id="section-b4"></a>

### §B4 Graph algorithms have named owners

**Implemented** for traversal, SCCs, communities, PageRank and FCA/RCA; dominators, control
dependence and summary composition beyond the acyclic case are **Proposed** targets
([§9](sections/analytics.md#section-9)).

| Owner | Algorithms |
|---|---|
| petgraph 0.8.3 | Traversal, dominators and SCCs over immutable, explicitly declared projections ([§5](sections/storage-and-publication.md#section-5)) |
| leiden-rs | Community detection, fed a normalized, sorted edge list |
| Our own code | Weighted PageRank with convergence diagnostics; formal and relational concept analysis; control dependence; bounded summary composition |

- **Each algorithm has a named consumer** in the served model, a tool's output or a brief.
- **A relationship does not need a graph algorithm** just because it has two endpoints.
- **What runs by default is decided by the §9.8 keep rule**: Passes A–C, direct usage and
  selection. Communities, FCA, kNN, PageRank, RCA and extra layers are variants, off by default
  and reachable with `lctx compile --analytics`.
- Summary scheduling uses iterative petgraph SCCs, callees first (ADR-0052). The first recursive
  value channel uses an SCC-local bounded worklist owned by our finite producer (ADR-0053);
  multi-relation effect/exception/role recursion triggers a fresh engine comparison (W12).

> Decision: ADR-0044, ADR-0020, ADR-0052, ADR-0053

<a id="section-b5"></a>

### §B5 Python semantics are custom Rust passes

**Implemented and Tested in focused cases** for the flow IR, conditions, verdicts, pinned models
and finite acyclic summaries; recursive and operation-wide composition is **Proposed**.

- Python-specific semantics are our own Rust code with stated abstractions: the structural
  recognizers ([§9.1–§9.3](sections/analytics.md#section-9-1)) and the **flow IR**
  ([§3.9](sections/behavior-model.md#section-3-9)), our stated **runtime** abstraction:
  statement-level control flow with exceptional exits and reaching definitions over bounded
  places, with `TYPE_CHECKING` false and version/platform tests following the analyzed context.
- **No provider's IR is the model.** ty supplies the use-def index in `cpg-flow`; Pyrefly's
  inference graph and ty's own decisions are never relabelled as runtime dataflow. Providers
  observe; our stated rules conclude.
- **Meaning comes from pinned models, propagation from summaries**
  ([§9.9](sections/behavioral-analysis.md#section-9-9)). A call alone never propagates a
  capability. A model's exception classes and class relationships bind to pinned context facts;
  total normal completion is an explicit model assertion, separate from transfer.
- **Proof identity.** A finite summary is identified by its callable, source contribution,
  input/output paths, transfer kind, condition, exit and ordered typed proof steps; each step cites checked source
  or model evidence. Nested call provenance for value transfers is a **Proposed** refinement
  (ADR-0028). A boundary is keyed to that contribution and condition, so a proof for one
  origin does not discharge a sibling on the same raw fact (ADR-0054).
- Known gaps (effectful finalizers, predecessor completion, recursive members, access
  normalization) stay explicit unknowns; the forward plan owns them.

> Decision: ADR-0045, ADR-0028, ADR-0054

<a id="section-b6"></a>

### §B6 Facts are first-class assertions with provenance

**Implemented and Tested.**

- Every extracted assertion is a `facts` row with its run, `origin`, `extraction_mode`,
  `modality`, `fidelity` and model. Independent assertions, including disagreement, are kept and
  never collapsed into mutable node properties.
- A derived join row, including the `nodes`/`edges` catalogs, is not a `facts` row: it is traced
  by the `fact_id`s it cites plus its snapshot's `compiler_digest`, and is rebuildable from them.
  An edge is first-class through its persistent `edge_id` and evidence ids.
- **Analysis results carry provenance in-row**: findings, assertions, briefs and behavioral
  results hold their compiler run, model, method invocation and evidence status, cite the
  facts, edges and nodes they rest on, and are rebuildable from the snapshot, the analytics
  config and the `compiler_digest`. Summary proof steps retain source evidence and condition ids
  in the same snapshot.

> Decision: ADR-0047, ADR-0045

<a id="section-b7"></a>

### §B7 Delta canonical store, published by a `snapshots` append

**Implemented and Tested.**

- Fact tables are append-only Delta tables, one per fact family.
- A snapshot becomes visible only through one append to `snapshots`, made after validation passes.
- Readers resolve table versions through that row, open only the pinned commit's files and filter
  by `snapshot_id` ([§6](sections/storage-and-publication.md#section-6)).

> Decision: ADR-0047

<a id="section-b8"></a>

### §B8 Pyrefly and Ruff link in-process; one workspace, one process

**Implemented and Tested.**

- **Pyrefly** is a git dependency on the fork `paul-heyse/pyrefly`, pinned by revision: the
  upstream tag plus `third_party/pyrefly-<ver>.patch`, which changes visibility, adds a no-write
  reporter switch and borrow-only accessors, and changes no logic. [`docs/pins.md`](../pins.md)
  records the revision.
- **Ruff** library crates are pinned to the line Pyrefly compiles against.
- **One workspace, one process.** Extraction, construction, analytics and publication run in one
  Rust process with no IPC. Isolation is by contract: an explicit constructed configuration,
  refusal of ambient variables that cannot be cleared, and abort on any panic
  ([§4.2](sections/acquisition-and-extraction.md#section-4-2)).
- The Pyrefly CLI of the same revision is only a parity-test oracle.

> Decision: ADR-0046

<a id="section-b9"></a>

### §B9 One pinned Rust dependency family

**Tested** (§7).

DataFusion, Arrow/Parquet, object_store and delta-rs each resolve to exactly one version in the
core workspace. An extra family is allowed only when declared with its scope, and no type crosses
its boundary (§7).

> Decision: ADR-0002

<a id="section-b10"></a>

### §B10 Exclusions

**Accepted** for the exclusions; the condition-kernel allowance is **Implemented** and its
decision record remains **Proposed** (ADR-0024).

Excluded: a graph database; a generic workflow engine; JSON-inferred canonical schemas; a
whole-ontology petgraph instance; a general composition planner or constraint solver; neural
reranking; graph embeddings.

**Allowed:** text embeddings; a *rebuildable* search projection of the published generation
(§B12); a bounded Boolean decision-diagram kernel and a narrow typed theory for proven stable
primitive places, which decide compatibility and implication over evaluation atoms and return
`unknown` at node, work, stability or type boundaries ([§3.9](sections/behavior-model.md#section-3-9)).
A theory solver (z3) stays excluded unless a registered query needs a theory the bounded
lowering cannot express (forward plan §5). ADR-0045 accepts the persisted, validated analysis
condition catalog and the BDD predecessor-compatibility screen; the general allowance remains
ADR-0024's open proposal.

> Decision: ADR-0005, ADR-0045, ADR-0024

<a id="section-b11"></a>

### §B11 Insight synthesis is programmatic; no LLM in the query path

**Implemented** (templates and extractive selection); the generative-model trigger is **Proposed**.

- Assertions come from typed findings through deterministic templates and extractive text
  selection ([§10](sections/synthesis-and-serving.md#section-10)). Statistical output may
  nominate and order, never state a control, a limit or a behavioral claim.
- There is no generative model in the pipeline. Adding one (a local model, compile time only,
  under mechanical grounding) needs an ADR triggered by the §12 gap metric.
- **No generative model ever runs in the query path.**

> Decision: ADR-0005

<a id="section-b12"></a>

### §B12 Canonical store vs serving projections

**Implemented and Tested.**

- The Delta store is authoritative for facts, findings and assertions. The serving generation
  ([§6.4](sections/storage-and-publication.md#section-6-4)) is a derived, immutable projection,
  built only from a published snapshot and rebuildable byte-for-byte; any search index is derived
  from it in turn.
- No cross-store transactions: the generation manifest names the snapshot it came from.

> Decision: ADR-0047

<a id="section-b13"></a>

### §B13 The agent interface is a FastMCP server over file-based generations

**Implemented** for the materialized server; the native semantic executor is **Partially
implemented** under a **Proposed** decision (ADR-0025).

- `lctx_mcp` serves the §1.1 operations from exactly one pinned, immutable generation per process,
  with no Delta, DataFusion, compiler code or network access. Direct lookup and ranked retrieval
  use materialized rows and cached vectors
  ([§11.3](sections/synthesis-and-serving.md#section-11-3)).
- **Target (ADR-0025):** a pinned in-process Rust/PyO3 extension executes bounded semantic queries
  (compatibility, implication, effect/role filters, witness traversal) over that generation with
  typed inputs; row, node, pair-work and depth budgets yield `unknown`/`truncated`, never a
  negative or `complete` claim; no semantic decision is duplicated in Python. Today it provides
  path-local value inspection only; admission and decoding defects are plan items W1–W3.

> Decision: ADR-0043, ADR-0025

<a id="section-b14"></a>

### §B14 One embedding spec, cached vectors

**Implemented and Tested** for the spec, cache and conformance oracle.

- One hashed embedding spec ([§11.1](sections/synthesis-and-serving.md#section-11-1)) governs
  every vector; compile-time (Rust) and query-time (Python) clients are held to shared
  conformance vectors.
- Vectors are cached by `spec_hash + input_hash` in the canonical `embedding_cache`; a snapshot
  records the cache version it read and the generation copies vectors from it. A view (signature
  and docstring, source body) is a column, not part of the cache key.
- The two cache-fill routes now share admission and committed-value readback in focused tests
  ([plan W9](../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)).
  Deployment identity beyond the operator-controlled launch remains open (plan W16).

> Decision: ADR-0043

---

<a id="section-3"></a>

## §3 Fact model

<!-- relocated-section -->

Owner: [Facts and identity](sections/facts-and-identity.md#section-3).

- <a id="section-3-1"></a>[§3.1 Layers and node kinds](sections/facts-and-identity.md#section-3-1)
- <a id="section-3-2"></a>[§3.2 Fact families and authority](sections/facts-and-identity.md#section-3-2)
- <a id="section-3-3"></a>[§3.3 Physical profiles](sections/facts-and-identity.md#section-3-3)
- <a id="section-3-4"></a>[§3.4 Identity rules](sections/facts-and-identity.md#section-3-4)
- <a id="section-3-4-1"></a>[§3.4.1 ID derivation](sections/facts-and-identity.md#section-3-4-1)
- <a id="section-3-5"></a>[§3.5 Vocabularies and codebooks](sections/facts-and-identity.md#section-3-5)
- <a id="section-3-5-1"></a>[§3.5.1 Type observations and class order](sections/facts-and-identity.md#section-3-5-1)
- <a id="section-3-6"></a>[§3.6 Resolution is a set](sections/facts-and-identity.md#section-3-6)
- <a id="section-3-7"></a>[§3.7 Coverage and boundaries](sections/facts-and-identity.md#section-3-7)
- <a id="section-3-8"></a>[§3.8 Graph catalog and edge registry](sections/facts-and-identity.md#section-3-8)
- <a id="section-3-9"></a>[§3.9 Behavior model: places, conditions and verdicts](sections/behavior-model.md#section-3-9)


<a id="section-4"></a>

## §4 Pipeline

<!-- relocated-section -->

Owner: [Acquisition and extraction](sections/acquisition-and-extraction.md#section-4).

- <a id="section-4-0"></a>[§4.0 Library acquisition and the run contract](sections/acquisition-and-extraction.md#section-4-0)
- <a id="section-4-1"></a>[§4.1 Stages](sections/acquisition-and-extraction.md#section-4-1)
- <a id="section-4-2"></a>[§4.2 Extraction](sections/acquisition-and-extraction.md#section-4-2)
- <a id="section-4-2-1"></a>[§4.2.1 Driver](sections/acquisition-and-extraction.md#section-4-2-1)
- <a id="section-4-2-2"></a>[§4.2.2 Syntax: one walk over Pyrefly's parse](sections/acquisition-and-extraction.md#section-4-2-2)
- <a id="section-4-2-3"></a>[§4.2.3 Semantics: Pyrefly's own collectors](sections/acquisition-and-extraction.md#section-4-2-3)
- <a id="section-4-2-4"></a>[§4.2.4 Binding rule (conservative)](sections/acquisition-and-extraction.md#section-4-2-4)
- <a id="section-4-2-5"></a>[§4.2.5 Failure, determinism and the parity oracle](sections/acquisition-and-extraction.md#section-4-2-5)
- <a id="section-4-2-6"></a>[§4.2.6 Upgrading Pyrefly](sections/acquisition-and-extraction.md#section-4-2-6)
- <a id="section-4-3"></a>[§4.3 Fact construction and persistence](sections/acquisition-and-extraction.md#section-4-3)


<a id="section-5"></a>

## §5 Projections

<!-- relocated-section -->

Owner: [Storage and publication](sections/storage-and-publication.md#section-5).



<a id="section-6"></a>

## §6 Persistence and publication

<!-- relocated-section -->

Owner: [Storage and publication](sections/storage-and-publication.md#section-6).

- <a id="section-6-1"></a>[§6.1 Canonical tables and publication](sections/storage-and-publication.md#section-6-1)
- <a id="section-6-2"></a>[§6.2 Readers](sections/storage-and-publication.md#section-6-2)
- <a id="section-6-3"></a>[§6.3 Schema evolution](sections/storage-and-publication.md#section-6-3)
- <a id="section-6-4"></a>[§6.4 Serving generations](sections/storage-and-publication.md#section-6-4)

<a id="section-7"></a>

## §7 Pinned dependency family

**Tested** (`cpg-schema` `family_smoke` writes Delta and queries it through DataFusion;
`just deps` checks single versions, the declared extra families and the Pyrefly fork).

DataFusion, Arrow/Parquet, object_store and delta-rs resolve to exactly one version each in the
core workspace (§B9). Extra families are allowed only when declared in `scripts/check_family.py`
with their scope; the `cpg-flow` ty/ruff 0.0.14 line with salsa pinned exactly is the one in use.
[`docs/pins.md`](../pins.md) is authoritative for every pin and its dated verification, and the
`pin-check` skill governs changes.

> Decision: ADR-0002

---

<a id="section-8"></a>

## §8 Validation

<!-- relocated-section -->

Owner: [Validation and evaluation](sections/validation-and-evaluation.md#section-8).



<a id="section-9"></a>

## §9 Analytics

<!-- relocated-section -->

Owner: [Analytics](sections/analytics.md#section-9).

- <a id="section-9-1"></a>[§9.1 Pass A — public entry point and delegation](sections/analytics.md#section-9-1)
- <a id="section-9-2"></a>[§9.2 Pass B — controls and local restrictions](sections/analytics.md#section-9-2)
- <a id="section-9-3"></a>[§9.3 Pass C — direct handoff](sections/analytics.md#section-9-3)
- <a id="section-9-4"></a>[§9.4 Community detection](sections/analytics.md#section-9-4)
- <a id="section-9-5"></a>[§9.5 Centrality](sections/analytics.md#section-9-5)
- <a id="section-9-6"></a>[§9.6 Formal and relational concept analysis](sections/analytics.md#section-9-6)
- <a id="section-9-7"></a>[§9.7 Embeddings in analytics](sections/analytics.md#section-9-7)
- <a id="section-9-8"></a>[§9.8 Determinism and ablation](sections/analytics.md#section-9-8)
- <a id="section-9-9"></a>[§9.9 Summaries, models and the capability registry](sections/behavioral-analysis.md#section-9-9)


<a id="section-10"></a>

## §10 Synthesis and briefs

<!-- relocated-section -->

Owner: [Synthesis and serving](sections/synthesis-and-serving.md#section-10).

- <a id="section-10-1"></a>[§10.1 Findings](sections/synthesis-and-serving.md#section-10-1)
- <a id="section-10-2"></a>[§10.2 Assertions](sections/synthesis-and-serving.md#section-10-2)
- <a id="section-10-3"></a>[§10.3 Brief structure and the Outcome order](sections/synthesis-and-serving.md#section-10-3)
- <a id="section-10-4"></a>[§10.4 Grounding checks](sections/synthesis-and-serving.md#section-10-4)
- <a id="section-10-5"></a>[§10.5 Usage patterns](sections/synthesis-and-serving.md#section-10-5)


<a id="section-11"></a>

## §11 Serving and agent interface

<!-- relocated-section -->

Owner: [Synthesis and serving](sections/synthesis-and-serving.md#section-11).

- <a id="section-11-1"></a>[§11.1 Embedding spec and vectors](sections/synthesis-and-serving.md#section-11-1)
- <a id="section-11-2"></a>[§11.2 Retrieval](sections/synthesis-and-serving.md#section-11-2)
- <a id="section-11-3"></a>[§11.3 FastMCP contract](sections/synthesis-and-serving.md#section-11-3)


<a id="section-12"></a>

## §12 Evaluation

<!-- relocated-section -->

Owner: [Validation and evaluation](sections/validation-and-evaluation.md#section-12).

<a id="section-13"></a>

## §13 Deferred

**Accepted deferrals** (ADR-0021, ADR-0046). These capabilities are outside the current
architecture on purpose; each returns by ADR when a named consumer needs it. The
[forward plan §7](../plans/behavioral-model-forward-plan_2026-09-24.md#7-deferred-each-with-a-trigger)
owns the scheduling triggers.

- **Ontology tables beyond the CPG families.** The CPG holds typed families and derived catalogs
  (§3); the capability registry (§9.9) is a closed, executable vocabulary, not an ontology store.
- **A port of Ruff's semantic model.** Our scope-aware recognizer supplies binding history
  ([§4.2.4](sections/acquisition-and-extraction.md#section-4-2-4)).
- **Pyrefly cross-references (Glean collector)** and **located/contextual types**. Reachable
  in-process, but no consumer needs them; cross-references would complement, never replace,
  `reference_resolutions`.
- **General alias analysis (points-to).** Flow runs over bounded places and summaries (§3.9, §9.9).
- **SCC condensation on projections.** If needed, it is built from SCC membership keeping every
  arc's evidence, never from petgraph's `condensation`, which merges parallel edges (§9).
- **An ANN index or Lance/LanceDB.** Exact search serves today; adoption needs an isolated
  workspace, Arrow IPC as the only interface and a derived index outside the byte-identical
  generation, because Lance writes are not byte-reproducible.
- **A recursion engine (Ascent/datafrog).** A same-state finite-base relation probe found no
  integration advantage for the first value channel over the bounded SCC-local producer;
  compare again when several recursive channels share rules (§9.9, ADR-0053).
- **Graph-FCA in the pipeline; on-demand RCA at serve time.**
- **Generative interpretation** (§B11), graph embeddings, neural reranking and composition planning.

> Decision: ADR-0046, ADR-0021
