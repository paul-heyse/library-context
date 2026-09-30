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

**Accepted target; PR1 catalog Implemented, remaining product scope Proposed (ADR-0071/0072, 2026-09-28).** A pinned Python library
compiles into an **API and evidence catalog for feature discovery and correct use**. A coding agent
can identify a built-in feature, select its public invocation/configuration, inspect original examples
and deployment details, and follow precise evidence and uncertainty. The comprehensive target is
[§14](sections/api-and-evidence-product.md); it replaces general behavioral completion as the first-
product release route.

Whole-public-surface identities and existing behavioral evidence remain valuable. Exhaustive results
name their supported domain and coverage; ranked discovery is not exhaustive; unavailable analysis
and unresolved behavior are local, explicit states. Briefs are optional renderings. No generative
model runs in compilation or the query path (§B11).

**Implemented and Tested, 2026-09-28:** catalog-only readiness, ordered source/provider API contracts,
stable public members, declaration evidence and optional behavioral/brief tools through PG serving.
Broader effective-surface normalization, contextual evidence/deployment search and typed
per-requirement states remain **Proposed**.
[§11.3](sections/synthesis-and-serving.md#section-11-3) describes current tools and
[§14.9](sections/api-and-evidence-product.md#section-14-9) owns their replacement target.

> Decision: ADR-0071, ADR-0078, ADR-0025

<a id="section-1-2"></a>

### §1.2 Increments

**Accepted sequencing; PR0–PR1 implemented, comparative admission open (ADR-0071/0072).** The
[forward plan §3.0](../plans/behavioral-model-forward-plan_2026-09-24.md#30-consolidated-execution)
owns PR0–PR6: task/baseline freeze, mandatory catalog/contracts, bounded surface/options,
scenarios/deployment, typed retrieval, agent usability and comparative confirmation.

**Retained implementation:** earlier increments 1–3 built facts/analytics/whole-surface tools;
Stage 3 has partial conditions/models/summaries and remains incomplete. PG0–PG17 local exact
storage/serving deployment is qualified within its recorded scope. Neither is a Context7 comparison.
Remaining Stage 3–5 work is deferred research unless an exposed claim or named product task needs it.
No former semantic exit is relabeled passed.

**Current increment: the semantic model cutover (accepted 2026-09-29; ADR-0085/0083/0084).** The
[cutover plan](../plans/semantic-model-cutover-plan_2026-09-29.md) runs these phases in order:
0. core contracts;
1. the store (PostgreSQL replaces Delta);
2. facts;
3. normalized relations;
4. analysis and catalog;
5. serving.

Each phase exits only with no legacy code in its layer. PR6 and new product features pause until phase
5. The target is [§15](sections/semantic-model.md); until each layer cuts over, the sections below
describe the implemented legacy pipeline.

The CPG precedes its analytics (ADR-0086). Canonical facts keep their identity and provenance;
mandatory catalog construction is independent of optional analysis. Libraries remain pinned
uv projects (ADR-0046). ADR-0040 owns review cadence. ADR-0079 owns the current-tree editable development loop on dated nightly Cargo,
workspace feature unification, a CLI-only Hakari crate and shared intermediates with fine-grain
locking. Final artifacts stay in local `target/`: workspace O2 with incremental compilation,
imported dependencies O3 with sccache, and the existing release test workflow. No wheel
project is selected.

> Decision: ADR-0071, ADR-0086, ADR-0087, ADR-0046, ADR-0079, ADR-0040

<a id="section-1-3"></a>

### §1.3 Non-goals

**Accepted** (ADR-0071).

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

> Decision: ADR-0071

<a id="section-1-4"></a>

### §1.4 Pilot, subsystem and gold reference

**Implemented and Tested** for the pilot and the gold guard; the subsystem and freezes are
**accepted** (ADR-0071, ADR-0046).

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

> Decision: ADR-0071, ADR-0046

<a id="section-1-5"></a>

### §1.5 Definition of done

**Accepted target, implementation/evaluation Proposed (ADR-0071).**

The first **usable pilot** has accurate supported API contracts, typed options, contextual scenarios,
deployment evidence, field-specific uncertainty and a functioning real-embedding PG/MCP path. Valid
APIs remain available without briefs or completed deep analysis; invalid claimed evidence still
fails. The product is **differentiated** only after the independent matched-condition task comparison
in [§14.12](sections/api-and-evidence-product.md#section-14-12) demonstrates its declared material
improvement over Context7 and the internal unstructured-evidence baseline. Unknown-count reduction,
retrieval rank or nicer presentation alone is not that result.

Existing behavioral/brief evaluation retains its own rubric and sealed data; incomplete Stage 3–5
exits remain incomplete. No answer states `refuted_under_model` outside complete coverage. Every
emitted brief still meets its grounding/Outcome validators; missing optional briefs are a catalog
availability concern, not permission to invent a documented outcome.

> Decision: ADR-0071

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


**Implemented policy (ADR-0040/0093, 2026-09-30).** These are the load-bearing choices. The
repository owns core 3.2 and declares its standard in
`docs/design_review/design_principles/standard.toml`. The six foundations organize review of
responsibilities, contracts, composition, explicit domain models and scoped semantic authority,
constraints and local reasoning. The principles and review skill own the domain-model criterion;
assess it within bounded design/review periods at the binding's cadence. AGENTS.md routes to
that process without imposing a standing modeling mandate. Investigation depth follows the
scoped question; flow tracing is optional where it resolves a concrete uncertainty.
A2 retains model adequacy and authoritative behavior as acceptance criteria. A1–A3 judge architecture
independently of correctness and domain-fidelity gates. Supporting DP/CI IDs retain their
historical meaning at the version cited by each review.

The library-context binding maps §B decisions to the standard and owns review cadence and
finding disposition. Changing a §B decision needs an ADR and a design/target review. Library
selection follows the owned capability, expected changes and total integration burden.
Early design favors coherent boundaries and inexpensive revision, with compatibility and
operational obligations scoped to real consumers and claimed behavior.

DESIGN owns accepted architecture and labeled targets; executable schemas and rule declarations
own their detailed contracts. ADRs record decisions and alternatives. Reviews preserve dated
evidence, while the active plan owns current disposition of scheduled findings. STATUS links to
that owner. Acceptance, implementation and verification remain distinct facts.

> Decision: ADR-0040, ADR-0093

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
  catalog), `sha2` and `biodivine-lib-bdd` (the condition kernel, whose general allowance ADR-0085
  accepts); never DataFusion, Delta, object_store, an async runtime or I/O. DataFusion
  validators live in core crates.
- **No inferred schemas**: none is inferred from JSON or a first batch, including the serving
  generation ([§6.4](sections/storage-and-publication.md#section-6-4)) and the embedding exchange
  ([§11.1](sections/synthesis-and-serving.md#section-11-1)).
- A schema change is a reviewed snapshot change and a declared migration; codebook codes are
  never renumbered or reordered.

**Accepted target, implementation Proposed (ADR-0073).** `cpg-schema` also owns finite typed
catalog requests, contextual witnesses and wire envelopes. Schemars is selected as a pure
Rust→JSON-Schema derivation dependency; Rust `jsonschema` is selected initially for offline dev
conformance. These wire representations do not replace or infer canonical Arrow schemas.
Matching flat components derive from existing declarations; nominal domain wrappers preserve
the canonical ID encodings. SQLx codecs, effectful loading and transport remain outside schema.
[§14.7/§14.11](sections/api-and-evidence-product.md#section-14-7) own the contract and library policy.

**Accepted target, implementation Proposed (ADR-0085, 2026-09-29).** Relation declarations in the new
crate `lctx-model` become the single authority ([§15.2](sections/semantic-model.md#section-15-2)).
One declaration per relation generates:
- the Rust and Arrow types;
- PostgreSQL DDL and constraints;
- codebook foreign keys, validators and inventories;
- serving views and wire DTOs.

`cpg-schema` is retired by the cutover.

> Decision: ADR-0085, ADR-0086, ADR-0073

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
- **Modeled source reads complete before modeled returns.** A source operand keeps its raw
  value-flow fact separate from its direct-parameter normal-read witness. Exact ty reaching
  evidence is preferred; a direct lexical parameter resolution with no same-function deletion
  or exception-handler frame supplies the bounded fallback for ty's approximate `try` regions
  (ADR-0055). Missing evidence stays unknown.
- **Closed argument expressions have their own witness kind.** A direct literal is
  `literal_normal`; a bounded unary, numeric binary, decisive two-operand Boolean expression
  or direct-literal-selected conditional expression is `closed_expression_normal` and cites its
  whole Ruff syntax fact (ADR-0056). Neither status
  implies dispatch, an enclosing return or an evaluated value for an unproved operand.
- **Proof identity.** A finite summary is identified by its callable, source contribution,
  input/output paths, transfer kind, condition, exit and ordered typed proof steps; each step cites checked source
  or model evidence. Nested call provenance for value transfers is a **Proposed** refinement
  (ADR-0028). Exact nested total-identity model calls now compose through bounded, ordered
  per-call argument evidence in focused cases; general nested evaluation remains a target.
  A boundary is keyed to that contribution and condition, so a proof for one
  origin does not discharge a sibling on the same raw fact (ADR-0054).
- Known gaps (effectful finalizers, predecessor completion, recursive members, access
  normalization) stay explicit unknowns; the forward plan owns them.

The accepted Stage 3 consolidation target uses shared typed evaluation, completion, argument
binding and coverage contracts across summary channels (ADR-0057; implementation remains
partial in §9.9). Providers observe; the pure semantic layer owns these conclusions.

**Accepted target, Proposed implementation (2026-09-26).** Channel composition uses typed
subjects, independent phase/coverage and shared proof admission (§9.9). Current transfer and
RCA projections preserve semantic alternatives and modality; deferred execution remains Stage 5.

**Implemented and focused Tested (2026-09-27; ADR-0059), partial:** synchronous `nullcontext` and
`suppress` protocols bind authored runtime transitions to pinned classes and fresh source sites.
Constructor signatures, provider method observations and manager occurrences remain separate;
a separate certificate proves the bounded parameter-to-entry-result identity inside the active
context body. Pure completion owns entry and reverse cleanup order. Shared
source/native admission checks source-derived ordered return obligations, including condition
scope; no missing exit observation is fabricated as a call. Full Stage 3 qualification is pending.

> Decision: ADR-0045, ADR-0028, ADR-0054, ADR-0055, ADR-0056, ADR-0057, ADR-0058, ADR-0059, ADR-0063

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

**Accepted target, Proposed implementation (2026-09-26).** Coverage certificates distinguish
complete empty channels from missing analysis and omitted witnesses; non-value channels cite
their actual subjects and sources instead of invented parameter-return identities (§9.9).

**Accepted target, implementation Proposed (ADR-0085, 2026-09-29).** Provenance stays in-row, and it
is also indexed: each proof and step relation declares itself a derivation source, and generated
`derivations`/`derivation_premises` views separate joint premises from alternative derivations.
Obligations replace boundary-reason encodings. Invocations record their input invocations
([§15.8–§15.9](sections/semantic-model.md#section-15-8)).

> Decision: ADR-0085, ADR-0086, ADR-0045, ADR-0058

<a id="section-b7"></a>

### §B7 PostgreSQL generations are the canonical store

**Accepted target (ADR-0086, 2026-09-29); implementation in progress (cutover phase 1).**
PostgreSQL 18 is the single relational store.
- Each generation owns an ordinary schema generated from the typed model, with keys, references
  and generated constraints.
- An attempt writes by binary COPY.
- Publication happens after the database constraints and the model's validators pass over the
  stored, sealed contents.
- Published relations are read-only by privilege.
- Readers pin one generation ([§15.11](sections/semantic-model.md#section-15-11)).

**Implemented (plan P1.5–P1.12, focused-Tested 2026-09-29):**
- the service baseline and the verified, non-elevated owner;
- the generated install with a live-catalog `store check` and a phased, resumable reset;
- the attempt-owned lifecycle: failed and interrupted states, facts admission, frontier-scoped
  schemas;
- the generation catalog;
- generation-bound provider sessions over a driver-neutral lease;
- the `lctx store|generation|query` commands and the operator transition.

The Delta store was removed at P1.3/P1.4. Until phase 2 publishes facts, no product generation
exists.

> Decision: ADR-0086

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

DataFusion, Arrow/Parquet and object_store each resolve to exactly one version in the core
workspace; there is no delta-rs and no DataFusion federation. An extra family is allowed only when
declared with its scope, and no type crosses its boundary (§7).

> Decision: ADR-0090

<a id="section-b10"></a>

### §B10 Exclusions

**Accepted** for the exclusions. The condition-kernel allowance is **Implemented**, and ADR-0085
accepts it as the canonical condition representation ([§15.7](sections/semantic-model.md#section-15-7)).

Excluded: a graph database; a generic workflow engine; JSON-inferred canonical schemas; a
whole-ontology petgraph instance; a general composition planner or constraint solver; neural
reranking; graph embeddings.

**Allowed:** text embeddings; a *rebuildable* search projection of the published generation
(§B12); a bounded Boolean decision-diagram kernel and a narrow typed theory for proven stable
primitive places, which decide compatibility and implication over evaluation atoms and return
`unknown` at node, work, stability or type boundaries ([§3.9](sections/behavior-model.md#section-3-9)).
A theory solver (z3) stays excluded unless a registered query needs a theory the bounded
lowering cannot express (forward plan §5). ADR-0045 accepts the persisted, validated analysis
condition catalog and the BDD predecessor-compatibility screen. ADR-0085 accepts the general allowance,
with occurrence-keyed atoms and a rendering-only DNF.

> Decision: ADR-0005, ADR-0045, ADR-0085

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
  built from a published snapshot plus its selected immutable retrieval realization. Complete
  inputs replay byte-for-byte; changed rendering/specification can yield another identified
  realization over the same snapshot. Any future search index derives from that generation.
- No cross-store transactions: the generation manifest names the snapshot it came from.

**Accepted target, implementation Proposed (ADR-0086, 2026-09-29).** There is one store. The published
generation is canonical, and serving reads that same generation through generated views, grants and
indexes ([§15.12](sections/semantic-model.md#section-15-12)). The bundle import and the serving copy
are removed. Arrow IPC remains only for derived, content-addressed caches. Serving still pins exactly
one generation per process, and a missing or corrupt required relation is a refusal.

> Decision: ADR-0086, ADR-0077

<a id="section-b13"></a>

### §B13 FastMCP pins one immutable generation; Rust owns PostgreSQL effects

**Implemented and bounded Tested under the live-embedding waiver, 2026-09-28** (ADR-0077/0078;
[current evidence](../design_review/evidence/2026-09-28_pr4/README.md)). The native
semantic executor remains **Partially implemented** under a **Proposed** decision (ADR-0025).

- `lctx_mcp` serves the §1.1 operations from exactly one pinned, immutable generation per process,
  with no Delta, DataFusion or compiler code. The PostgreSQL route uses
  `lctx_storage`/`lctx-postgres` for bounded relational
  selection and complete evidence hydration; PostgreSQL is an explicit serving dependency
  ([§11.3](sections/synthesis-and-serving.md#section-11-3)).
- **Target (ADR-0025):** a pinned in-process Rust/PyO3 extension executes bounded semantic queries
  (compatibility, implication, effect/role filters, witness traversal) over that generation with
  typed inputs; row, node, pair-work and depth budgets yield `unknown`/`truncated`, never a
  negative or `complete` claim; no semantic decision is duplicated in Python. Today it provides
  path-local value inspection only; admission and decoding defects are plan items W1–W3.

- **Implemented (ADR-0073/0077):** generated input/output schemas and
  typed Rust decoding/envelopes replace duplicate semantic Python declarations through the
  [§14.9 Tool adapter](sections/api-and-evidence-product.md#section-14-9). Existing generation,
  lifetime, budgets and cancellation ownership remain; custom Tool validation is explicit.

- **Accepted target (ADR-0086, cutover phase 5):** the pinned generation is the canonical PostgreSQL
  generation itself.
  - Wire DTOs derive from relation declarations.
  - The native executor reads generation relations, optionally through a derived artifact cache, and
    shares `lctx-model`'s verdict, discharge and condition functions.

> Decision: ADR-0078, ADR-0025, ADR-0073, ADR-0086

<a id="section-b14"></a>

### §B14 One embedding spec, exact consumed-vector receipts

**Implemented and Tested, 2026-09-27; PostgreSQL receipts are linked below.**

- One hashed embedding spec ([§11.1](sections/synthesis-and-serving.md#section-11-1)) governs
  every vector. Standard output is 1024 float32 dimensions, MRL prefix then L2 normalization;
  format-2 spec identity includes launch admission and reduction. Rust and Python retain their
  shared conformance oracle.
- PostgreSQL reuses one immutable winner per `spec_hash + input_hash`. An attempt retains each
  exact value before operation/E0/brief consumption and publishes snapshot-local Delta receipts
  for all consumed values, including analytics-only inputs. Value digests enter content identity.
- Canonical vectors replay from the selected snapshot; addressable retrieval fragments have a
  complete immutable materialization receipt. Current reconstruction needs no live cache/provider.
  Online MCP explicitly requires PostgreSQL and pins a ready generation/exact profile for its
  lifespan. Database discovery cannot authorize an unpublished snapshot. PR4 live embedding
  qualification is operator-waived; no current hybrid-quality claim follows from fixture controls.
- [§6.5](sections/storage-and-publication.md#section-6-5) owns database effects and conditional
  capabilities. The [PostgreSQL workstream](../plans/behavioral-model-forward-plan_2026-09-24.md#postgresql-workstream) owns
  current qualification; W9/W16 retain live-client and endpoint-identity boundaries.
- **Accepted target (ADR-0086):** consumed-vector receipts and exact vectors become canonical
  PostgreSQL relations of the generation, and they stay in its content digest.

> Decision: ADR-0078, ADR-0086

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

**Tested** (`cpg-schema` `family_smoke` queries Arrow batches through DataFusion SQL;
`just deps` checks single versions, the declared extra families and the Pyrefly fork).

DataFusion, Arrow/Parquet and object_store resolve to exactly one version each in the core
workspace (§B9). The owned PostgreSQL table-provider fork is pinned by revision without federation. Extra families are allowed only when declared in `scripts/check_family.py`
with their scope; the `cpg-flow` ty/ruff 0.0.14 line with salsa pinned exactly is the one in use.
[`docs/pins.md`](../pins.md) is authoritative for every pin and its dated verification, and the
`pin-check` skill governs changes.

> Decision: ADR-0090

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

**Accepted deferrals** (ADR-0071, ADR-0046). These capabilities are outside the current
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
- **Lance/LanceDB.** pgvector is selected for the current exact projection; an alternative
  needs a named capability beyond that qualified route and an isolated workspace, Arrow IPC as the only interface and a derived index outside the byte-identical
  generation, because Lance writes are not byte-reproducible.
- **A recursion engine (Ascent/datafrog).** A same-state finite-base relation probe found no
  integration advantage for the first value channel over the bounded SCC-local producer;
  compare again when several recursive channels share rules (§9.9, ADR-0053), or a catalog consumer
  needs genuinely recursive multi-relation association (§14.11, ADR-0073). A simple finite closure
  remains an indexed worklist; optional engines do not gate product delivery.
- **Catalog fine-grained memoization and extra serving caches.** Coarse immutable rebuilds remain
  selected. Salsa (including optional disposable persistence), Moka, interning and compressed/dense
  collections require §14.11's named consumers and equivalence/resource evidence. Existing ty's
  Salsa database remains provider-private; no provider-family upgrade is selected.
- **Graph-FCA in the pipeline; on-demand RCA at serve time.**
- **Generative interpretation** (§B11), graph embeddings, neural reranking and composition planning.

> Decision: ADR-0046, ADR-0071, ADR-0073
