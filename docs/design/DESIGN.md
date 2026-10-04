# library-context — design

**This file plus `docs/design/sections/` is the architectural authority**; the
[architecture map](README.md) routes each responsibility to its owner. This file holds scope
(§1), the binding decisions (§2, §B1–§B14), the dependency-family rule (§7) and durable deferrals
(§13); focused owners hold the rest. Labels distinguish implemented behavior from accepted
targets: an accepted target is not an implementation claim. Current ADRs say why and what was
rejected ([index](../adr/README.md)); the
[cutover plan](../plans/semantic-model-cutover-plan_2026-09-29.md) owns current layer execution and cross-phase findings; the
[forward plan](../plans/behavioral-model-forward-plan_2026-09-24.md) retains the product sequence and its findings.

**Changing a section.** Change a governed section in the same commit as the ADR that decides it
and keep its `> Decision: ADR-NNNN` line. Section IDs are never renumbered or reused: insert
`§3.2.1` rather than shifting `§3.3`, and a moved live section leaves a relocation pointer. Keep
sections current rather than appending history (ADR-0042). There is no line budget: the detail the
design needs comes before length. Column-level contracts derive from `lctx-model::domain` and are
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

**Accepted product target; typed catalog Implemented, qualification in progress (2026-10-01).** A pinned Python library
compiles into an **API and evidence catalog for feature discovery and correct use**. A coding agent
can identify a built-in feature, select its public invocation/configuration, inspect original examples
and deployment details, and follow precise evidence and uncertainty. The comprehensive target is
[§14](sections/api-and-evidence-product.md); it replaces general behavioral completion as the first-
product release route.

Whole-public-surface identities and existing behavioral evidence remain valuable. Exhaustive results
name their supported domain and coverage; ranked discovery is not exhaustive; unavailable analysis
and unresolved behavior are local, explicit states. Briefs are optional renderings. No generative
model runs in compilation or the query path (§B11).

**Implemented, qualification in progress, 2026-10-01:** model-owned catalog members and ordered
source/effective contracts, contextual evidence, declaration selection, optional analysis, synthesis
and retrieval inputs publish through the cumulative generation store. Phase 5 serving remains
unavailable. [§15](sections/semantic-model.md) owns these contracts;
[§14.9](sections/api-and-evidence-product.md#section-14-9) owns the proposed tool adapter.
Fixture qualification does not establish a real-library pilot or comparative product acceptance.

> Decision: ADR-0071, ADR-0078, ADR-0025

<a id="section-1-2"></a>

### §1.2 Increments

**Accepted sequencing; comparative admission and post-cutover serving open (ADR-0071/0072).** The
[forward plan §3.0](../plans/behavioral-model-forward-plan_2026-09-24.md#30-consolidated-execution)
owns PR0–PR6: task/baseline freeze, mandatory catalog/contracts, bounded surface/options,
scenarios/deployment, typed retrieval, agent usability and comparative confirmation.

Earlier product receipts retain their recorded boundaries; they do not qualify the reconstructed
pipeline. Remaining research is activated only by an exposed claim or a named product task.

**Current increment: the semantic model cutover (accepted 2026-09-29; ADR-0085/0083/0084).** The
[cutover plan](../plans/semantic-model-cutover-plan_2026-09-29.md) runs these phases in order:
0. core contracts;
1. the store (PostgreSQL replaces Delta);
2. facts;
3. normalized relations;
4. analysis and catalog;
5. serving.

Each phase exits only with no legacy code in its layer. PR6 and new product features pause until phase
5. [§15](sections/semantic-model.md) owns the reconstructed layers. Phase 4 implementation and
retirement qualification are in progress; Phase 5 contracts remain explicitly unavailable.

The CPG precedes its analytics (ADR-0086). Canonical facts keep their identity and provenance;
mandatory catalog construction is independent of optional analysis. Libraries remain pinned
uv projects (ADR-0117). ADR-0040 owns review cadence. ADR-0079 owns the current-tree editable development loop on dated nightly Cargo,
workspace feature unification, a CLI-only Hakari crate and shared intermediates with fine-grain
locking. Final artifacts stay in local `target/`: workspace O2 with incremental compilation,
imported dependencies O3 with sccache, and the existing release test workflow. No wheel
project is selected. Agents run functional tests and, at scope end, every non-functional check
(`just hygiene`), fixing what fails. An end-of-turn hook runs only automatic steps: formatting,
generators, readiness and the library catalog. It fixes nothing else (ADR-0110).

**Implemented workflow, 2026-09-30 (ADR-0109):** a coordinator owns design, integration and
acceptance, using reusable evidence, design-review, execution, implementation-review and functional
testing roles. Shared responsibilities and permitted effects live in [agent role contracts](../../.agents/roles/README.md);
native Codex and Claude definitions select models only: no role file restricts tools or sandbox, so the
contracts alone bound each role's effects.
`library-research` may write the evidence locations AGENTS.md names and an assigned shared library
skill; the other evidence roles are read-only (ADR-0113). Paired process skills prepare and carry
out reviews, plan creation and execution. Plan creation includes focused assessment of the existing
foundations it will use. Delegation is selective: independent coverage, context isolation, capability or judgment must
justify coordination cost. Worker startup follows the brief and relevant authorities; concrete
evidence conflicts and repeated repair failures trigger coordinator reassessment. Review cadence, decision authority and
single-owner finding disposition remain with their existing owners. Runtime settings are policy defaults, with explicit stronger-worker
routes; resource savings and broader workflow effectiveness remain **Proposed**.

> Decision: ADR-0071, ADR-0086, ADR-0087, ADR-0117, ADR-0079, ADR-0040, ADR-0110, ADR-0109, ADR-0113

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
  executable vocabulary in `lctx-model::domain` ([§9.9](sections/behavioral-analysis.md#section-9-9)).
- **General alias analysis (points-to).** Flow is intraprocedural over bounded places (§3.9);
  summaries are interprocedural (§9.9).

> Decision: ADR-0071

<a id="section-1-4"></a>

### §1.4 Pilot, subsystem and gold reference

**Implemented and Tested** for the pilot and the gold guard; the subsystem and freezes are
**accepted** (ADR-0071, ADR-0117).

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

> Decision: ADR-0071, ADR-0117

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

### §B1 Independent Ruff syntax, Pyrefly typing and ty flow have distinct provider roles

**Implemented with scoped receipts, 2026-10-03; assembled acceptance pending.** Latest independent Ruff owns canonical
syntax and contextual lexical observations. Native Pyrefly owns typing, callable/class metadata,
Pysa candidates and public-name evidence. ty supplies flow/index, timing and precision observations
only when requested; ty inference is an offline oracle. None of these observations independently
grants runtime truth or dispatch closure.

The latest Ruff/ty family and Pyrefly's embedded Ruff family are separate nominal Rust types.
Owned attachment checks source snapshot/view, byte range, node kind, role, context and uniqueness;
synthetic variants have optional source correspondence. The ty TYPE_CHECKING runtime view is
explicit. Model-owned normalization and question policies interpret these inputs.

The working tree links the migrated families and canonical parser;
[the coordinator](../plans/code-facts-expansion-plan_2026-10-03.md) owns scoped qualification,
remaining enrichment and assembled acceptance. Earlier
in-process extraction/CLI parity receipts retain their original scope.

> Decision: ADR-0117

<a id="section-b2"></a>

### §B2 Typed relation declarations are the authoritative data contract

**Implemented; Phase 4 qualification in progress, 2026-10-01.** `lctx-model` domain structs and
attributed tagged enums own relations, nominal identities, codebooks, keys/references, provenance,
coverage, policies and shared validation ([§15.2](sections/semantic-model.md#section-15-2)).
A validated model supplies the one registry/inventory. Derived Rust metadata and explicit Arrow
codecs lower those declarations; `lctx-postgres` lowers the same model to physical tables,
constraints and indexes. No independent row DSL or schema inventory supplies semantic meaning.

- **No inferred schemas:** neither JSON, a first batch nor an embedding response defines a
  canonical contract. Arrow is the explicit transport/compute representation, not a second owner.
- **Reviewed evolution:** a schema snapshot change is a declared migration; stored codebook codes
  are never renumbered or reordered.
- **Pure contract ownership:** effectful loading, SQLx codecs, COPY and transport stay outside
  domain policy. Handwritten computations/validators express behavior against the same types.
- **Implemented in the Phase 5 working tree; qualification pending, 2026-10-02:** wire DTOs and serving views derive from declarations
  or explicit mappings; Rust owns request classification and contextual validation. Schemars
  derives wire schemas, with independent offline conformance tests. Schema validity does not
  establish evidence closure. [§14.7/§14.11](sections/api-and-evidence-product.md#section-14-7)
  own the retained query/library contract.

Phase 5 has retired the `cpg-schema` wire owner and re-homed independent controls under
`lctx-model` and the real canonical-generation serving fixtures. No compatibility reader remains.

> Decision: ADR-0085, ADR-0086, ADR-0073

<a id="section-b3"></a>

### §B3 DataFusion constructs and validates relations

**Implemented; Phase 4 qualification in progress, 2026-10-01.** DataFusion supplies in-process
relational compute over model-declared inputs. Joins, projections and unions construct suitable
normalized and analysis relations; native kernels handle owned graph/semantic operations.
`cpg-core` registers generation-bound providers and runs the declared stage adapters, without
becoming another semantic authority.

Cross-relation invariants use shared model validators and suitable DataFusion queries
([§8](sections/validation-and-evaluation.md#section-8)). Tests and stored publication invoke the
same validation contract. Complete read grants and invariant/reference closure are required;
missing required input cannot be interpreted as a valid empty relation.

> Decision: ADR-0085, ADR-0086

<a id="section-b4"></a>

### §B4 Graph algorithms have named owners

**Implemented; Phase 4 qualification in progress, 2026-10-01.** Model-owned operations consume
borrowed immutable petgraph snapshots with canonical IDs, lineage, explicit universes and budgets.
`lctx-analytics` exposes four native entry points for scheduling, ranking, concepts and neighbours;
community conversion and the other analytic policies belong to the typed Analytic owner too.
[§9](sections/analytics.md#section-9) owns their method and uncertainty contracts.

| Owner | Algorithms |
|---|---|
| petgraph | Traversal and iterative SCC scheduling over declared projections; dominator capability only when a named consumer activates it |
| leiden-rs adapter | Community detection with explicit direction/weight conversion, pair universe and canonical membership |
| Model-owned native kernels | Weighted PageRank and diagnostics, bounded FCA/one-step RCA, exact neighbours and finite Summary composition |

- Every algorithm needs a named consumer; a relationship does not require a graph algorithm
  merely because it has two endpoints.
- Structural traversal/controls/handoffs and direct-usage/selection policies replace the legacy
  Pass A/B/C producer paths. Communities, PageRank, FCA/RCA, analytic kNN and extra layers remain
  off by default; RCA requires FCA and extra community layers require communities.
- Summary uses a canonical callee-first SCC schedule and bounded SCC-local worklist. Exact
  invocation-distinct guards and finite witnesses survive residual uncertainty; ancestry/cost
  are not semantic equality. General all-channel recursion remains outside the restored envelope.
- Parameter identity, replay and deterministic output do not establish measured performance or
  product benefit. [§9.8](sections/analytics.md#section-9-8) retains the separate ablation boundary.

> Decision: ADR-0044, ADR-0020, ADR-0052, ADR-0053, ADR-0106

<a id="section-b5"></a>

### §B5 Python semantics are custom Rust passes

**Implemented; Phase 4 qualification in progress, 2026-10-01.** Typed Local, execution, Model,
Summary and obligation owners implement the stated bounded runtime abstraction. `cpg-flow`
supplies ty observations; provider IR is never relabeled as the semantic model. Normalized
occurrence/binding identity and explicit native premises precede the owned conclusions.
[§3.9](sections/behavior-model.md#section-3-9) and
[§9.9](sections/behavioral-analysis.md#section-9-9) own the detailed contracts.

- Runtime-view `TYPE_CHECKING`, version/platform context, bounded places, entry values and
  occurrence-keyed conditions remain explicit. A type or provider reachability observation is
  not an execution witness.
- Local → BaseEvaluation → BaseCompletion → SourceCall → EnrichedExecution → Model → Summary
  is acyclic. Reached eager inputs, source/body/default admission, normal/exceptional completion
  and frame/finalizer release have independent proofs. A returned value cannot certify its own
  input availability or release.
- Closed expression evaluation cites exact syntax and admitted operand outcomes. It does not
  imply dispatch, enclosing completion or the value of an unproved operand. Use, Value and Guard
  entry/stability domains cannot substitute for one another.
- Pinned authored models bind exact targets, phase and independent channel coverage. A call
  alone never propagates a capability; normal-body, default-availability, transfer, effect,
  callback, resource and exception promises are separate.
- Supported synchronous context protocols retain fresh resource identity, entry/body/exit order
  and pending-outcome replacement. Opaque construction, unsupported cleanup/handlers and broad
  lifecycle state retain their specific uncertainty.
- Finite recursive value composition keeps exact call-site binding/substitution, per-origin
  closure, witness DAGs and checked depth/cost equations. Depth/work/proof residuals prevent
  universal discharge without erasing admitted positives. A negative needs exact false support,
  complete relevant coverage and its checked Refutation proof.
- Symbolic constructor→field→reader associations remain separate from temporal heap identity.
  Unknown reader alternatives retain proof=None; a known field location does not establish
  allocation, alias or mutation stability.

General heap/alias completeness, arbitrary async/generator lifecycle and recursive all-channel
closure remain outside the restored finite envelope. Named obligations preserve those gaps;
no compatibility engine or parallel verdict policy closes them by inference.

> Decision: ADR-0045, ADR-0054, ADR-0055, ADR-0056, ADR-0057, ADR-0058, ADR-0059, ADR-0063, ADR-0085, ADR-0106, ADR-0108, ADR-0028

<a id="section-b6"></a>

### §B6 Facts are first-class assertions with provenance

**Implemented for facts/normalized relations; Phase 4 Implemented with qualification in progress,
2026-10-01.** Attributed provider records retain run/input/context, origin, modality, fidelity and
coverage. Independent observations, including disagreement, are preserved rather than collapsed
into mutable node properties. Derived relations cite typed source identity and provenance under
their declared semantic key; generic legacy fact/node/edge catalogs are not a parallel authority.

Nominal producing owners retain their actual invocation, definition, qualification, coverage,
inputs and support. Complete empty coverage differs from unrequested/missing analysis; an omitted
witness cannot establish absence. Proof/step declarations generate derivation lookup views while
retaining their typed payloads. Joint premises and alternative derivations stay distinct and the
combined premise graph is acyclic ([§15.8–§15.9](sections/semantic-model.md#section-15-8)).

S0 alone writes shared findings and programmatic assertions from earlier qualified conclusions.
Summary witnesses cite source/earlier witnesses; final aggregates cite those witnesses after SCC
closure and never serve as their own premises. Coverage membership is independent of witness
deduplication. Obligations and one shared qualification/verdict policy preserve the exact scope;
rendered text and presentation limits cannot strengthen it.

> Decision: ADR-0085, ADR-0086, ADR-0045, ADR-0058, ADR-0106, ADR-0108

<a id="section-b7"></a>

### §B7 PostgreSQL generations are the canonical store

**Implemented for store/facts/normalized generations; Phase 4 qualification in progress,
2026-10-01.** PostgreSQL 18 is the single relational store.
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

The Delta store was removed at P1.3/P1.4. Facts and normalized generations are implemented;
upper-frontier qualification is in progress. Publication never selects a generation.

> Decision: ADR-0086

<a id="section-b8"></a>

### §B8 Pinned analyzer forks link in-process with observational seams

**Accepted target, 2026-10-03; migration in progress.** Pyrefly and independent Ruff/ty are exact
upstream tags plus one aggregate reviewed patch each. Patches expose native observation and
precision provenance while preserving ordinary inference/lint/algebra behavior. Parent/tag,
immutable revision, patch digest, environment classification and paired parity controls are
required; changing upstream semantic algorithms requires another decision.

Extraction, analysis and publication stay in one Rust process. Explicit configuration isolates
inputs; unsafe ambient variables are refused and provider panics abort the attempt. Native
Pyrefly types are encoded during the live transaction. The matching Pyrefly CLI is a parity
oracle only. Exact current pins and dated verification belong to [pins](../pins.md).

**Implemented, 2026-10-03.** Finite stage grants name each family’s actual provider
(§15.11). Native typing conclusions retain explicit model-owned premises (§3.9), so
linked observation does not become unconditional runtime evidence.

> Decision: ADR-0117, ADR-0119, ADR-0120

<a id="section-b9"></a>

### §B9 One pinned Rust dependency family

**Tested** (§7).

DataFusion, Arrow/Parquet and object_store each resolve to exactly one version in the core
workspace; there is no delta-rs and no DataFusion federation. An extra family is allowed only when
declared with its scope, and no type crosses its boundary (§7).

> Decision: ADR-0118

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
lowering cannot express (forward plan §5). ADR-0045 accepts the bounded BDD compatibility screen. ADR-0085 makes
model-owned conditions, atoms and Merkle nodes canonical, with occurrence-keyed evaluations and a
rendering-only DNF; no legacy condition catalog is a second authority.

> Decision: ADR-0106, ADR-0045, ADR-0085

<a id="section-b11"></a>

### §B11 Insight synthesis is programmatic; no LLM in the query path

**Phase 4 Implemented; qualification in progress, 2026-10-01.** S0 uses typed deterministic
templates and extractive selection; the generative-model trigger remains **Proposed**.

- S0 alone emits shared findings/assertions from typed documentary and qualified analysis
  conclusions through deterministic templates and extractive text selection ([§10](sections/synthesis-and-serving.md#section-10)). Statistical output may
  nominate and order, never state a control, a limit or a behavioral claim.
- There is no generative model in the pipeline. Adding one (a local model, compile time only,
  under mechanical grounding) needs an ADR triggered by the §12 gap metric.
- **No generative model ever runs in the query path.**

> Decision: ADR-0106

<a id="section-b12"></a>

### §B12 Canonical store vs serving projections

**Implemented / Tested for facts and normalized storage; Phase 4 qualification in progress,
2026-10-01.** PostgreSQL is the single canonical relational store. Typed declarations generate
generation tables and validation; immutable vocabulary prefixes and completed receipts bound reads.
There is no Delta store, bundle import or canonical serving copy. Numerical caches are disposable
physical derivatives of canonical exact vector values.

**Implemented in the Phase 5 working tree; qualification pending, 2026-10-02:** serving reads the same pinned canonical generation through
generated views, grants and indexes ([§15.12](sections/semantic-model.md#section-15-12)). Missing or
corrupt required relations refuse admission; derived caches cannot replace semantic authority.

> Decision: ADR-0086, ADR-0077

<a id="section-b13"></a>

### §B13 FastMCP pins one immutable generation; Rust owns PostgreSQL effects

**Implemented in the Phase 5 working tree; qualification pending, 2026-10-02.** The pinned generation is the
canonical PostgreSQL generation. Wire DTOs derive from declarations; Rust validates requests,
evaluates bounded semantic queries and hydrates complete evidence. Python is the thin validated
FastMCP adapter and repeats no selection, condition, verdict or discharge policy.

The old schema, bundle and native IPC authorities are retired. Current model contracts and real
store/native/MCP controls retain their independent expectations. Activation follows the
[Phase 5 qualification](../plans/semantic-model-phase5-detailed-plan_2026-10-01.md). Row, node, pair-work, depth and response
budgets retain explicit refusal/truncation semantics. [§11.3](sections/synthesis-and-serving.md#section-11-3)
and [§14.9](sections/api-and-evidence-product.md#section-14-9) own that serving boundary.

> Decision: ADR-0114, ADR-0078, ADR-0025, ADR-0073, ADR-0086

<a id="section-b14"></a>

### §B14 One embedding spec, exact consumed-vector receipts

**Implemented, qualification in progress, 2026-10-01.**

- One hashed embedding spec ([§11.1](sections/synthesis-and-serving.md#section-11-1)) governs
  every vector. Standard output is 1024 float32 dimensions, MRL prefix then L2 normalization;
  format-2 spec identity includes launch admission and reduction. Rust and Python retain their
  shared conformance oracle.
- `lctx-model::domain::embedding` owns specification, exact value codec, text membership, availability
  and analytic/retrieval consumption. PostgreSQL retains one immutable winner per specification and
  input hash. Each attempt publishes the exact consumed value, including analytics-only inputs;
  value digests enter content identity.
- Snapshot-local values and consumer receipts replay without a cache/provider effect. Fake-service
  controls qualify that seam and cache behavior; live embedding and retrieval quality are unqualified.
  Phase 5 MCP must pin a ready generation and exact profile; database discovery cannot authorize an
  unpublished generation.
- [§6.5](sections/storage-and-publication.md#section-6-5) owns database effects and conditional
  capabilities. The [PostgreSQL workstream](../plans/behavioral-model-forward-plan_2026-09-24.md#postgresql-workstream) owns
  current qualification; W9/W16 retain live-client and endpoint-identity boundaries.
- Consumed-vector receipts and exact values are canonical typed PostgreSQL relations in the
  generation and enter its content digest. Phase 4 qualification remains in progress; no live
  provider or retrieval-quality claim follows from service-free replay.

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
- <a id="section-4-2-2"></a>[§4.2.2 Canonical syntax and provider correspondence](sections/acquisition-and-extraction.md#section-4-2-2)
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

**Implemented; Phase 4 qualification in progress, 2026-10-01** (typed model records flow through generation-bound DataFusion providers;
`just deps` checks single versions, the declared extra families and the Pyrefly fork).

DataFusion, Arrow/Parquet and object_store resolve to exactly one version each in the core
workspace (§B9). The owned PostgreSQL table-provider fork is pinned by revision without federation. Extra families are allowed only when declared in `scripts/check_family.py`
with version and nominal Cargo source scopes. The accepted analyzer target uses one latest
Ruff/ty fork revision in extraction/flow and a separate registry Ruff family only inside the
Pyrefly adapter; salsa's three crates are pinned exactly for the ty graph. M1 owns adoption.
[`docs/pins.md`](../pins.md) is authoritative for every pin and its dated verification, and the
`pin-check` skill governs changes.

> Decision: ADR-0118

---

<a id="section-8"></a>

## §8 Validation

<!-- relocated-section -->

Owner: [Validation and evaluation](sections/validation-and-evaluation.md#section-8).



<a id="section-9"></a>

## §9 Analytics

<!-- relocated-section -->

Owner: [Analytics](sections/analytics.md#section-9).

- <a id="section-9-1"></a>[§9.1 Public entry point and delegation](sections/analytics.md#section-9-1)
- <a id="section-9-2"></a>[§9.2 Controls and local restrictions](sections/analytics.md#section-9-2)
- <a id="section-9-3"></a>[§9.3 Direct handoff](sections/analytics.md#section-9-3)
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

**Accepted deferrals** (ADR-0071, ADR-0117). These capabilities are outside the current
architecture on purpose; each returns by ADR when a named consumer needs it. The
[forward plan §7](../plans/behavioral-model-forward-plan_2026-09-24.md#7-deferred-each-with-a-trigger)
owns the scheduling triggers.

- **Ontology tables beyond the CPG families.** The CPG holds typed families and derived catalogs
  (§3); the capability registry (§9.9) is a closed, executable vocabulary, not an ontology store.
- **A bespoke port of Ruff's semantic model.** Native contextual observations are scheduled
  through Ruff's existing Checker; model normalization owns interpretation, not an analyzer port.
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

> Decision: ADR-0117, ADR-0071, ADR-0073
