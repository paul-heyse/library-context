# library-context — design

**This file plus `docs/design/sections/` is the architectural authority**; the
[architecture map](README.md) routes each responsibility to its owner. This file holds scope
(§1), the binding decisions (§2, §B1–§B14), the dependency-family rule (§7) and durable deferrals
(§13); focused owners hold the rest. Labels distinguish implemented behavior from accepted
targets: an accepted target is not an implementation claim. Current ADRs say why and what was
rejected ([index](../adr/README.md)); the
[graph-native coordinator](../plans/graph-native-pivot-plan_2026-10-05.md) owns replacement execution and cross-package findings; the
[forward plan](../plans/behavioral-model-forward-plan_2026-09-24.md) retains the product sequence and its findings.

**Changing a section.** Change a governed section in the same commit as the ADR that decides it
and keep its `> Decision: ADR-NNNN` line. Section IDs are never renumbered or reused: insert
`§3.2.1` rather than shifting `§3.3`, and a moved live section leaves a relocation pointer. Keep
sections current rather than appending history (ADR-0042). There is no line budget: the detail the
design needs comes before length. Column-level contracts derive from `lctx-model::domain` and are
snapshot-tested; they are not repeated here.

**Labels.** Every claim carries a principles §D label (`Proposed`, `Interface-checked`,
`Implemented`, `Tested`, `Measured`, `Formally established`). A section's label applies unless a
line says otherwise. `Interface-checked` means the named library surface was read at the locked
version (skill brief, probe or source at that version), not that our code uses it yet.

---

<a id="section-1"></a>

## §1 Scope

<a id="section-1-1"></a>

### §1.1 Objective and the promise

**Implemented catalog and native serving, 2026-10-06; real-library product acceptance remains Proposed.** A pinned Python library
compiles into an **API and evidence catalog for feature discovery and correct use**. A coding agent
can identify a built-in feature, select its public invocation/configuration, inspect original examples
and deployment details, and follow precise evidence and uncertainty. The comprehensive target is
[§14](sections/api-and-evidence-product.md); it replaces general behavioral completion as the first-
product release route.

Whole-public-surface identities and existing behavioral evidence remain valuable. Exhaustive results
name their supported domain and coverage; ranked discovery is not exhaustive; unavailable analysis
and unresolved behavior are local, explicit states. Briefs are optional renderings. No generative
model runs in compilation or the query path (§B11).

**Implemented, 2026-10-06:** model-owned catalog members and ordered
source/effective contracts, contextual evidence, declaration selection, optional analysis, synthesis
and retrieval inputs compile into admitted artifacts and publish through sealed native realizations.
All ten serving tools have targeted actual native/MCP evidence in the graph-native coordinator. [§15](sections/semantic-model.md) owns these contracts;
[§14.9](sections/api-and-evidence-product.md#section-14-9) owns the proposed tool adapter.
Fixture qualification does not establish a real-library pilot or comparative product acceptance.

> Decision: ADR-0071, ADR-0131, ADR-0025

<a id="section-1-2"></a>

### §1.2 Increments

**Accepted sequencing; comparative admission and post-cutover serving open (ADR-0071/0072).** The
[forward plan §3.0](../plans/behavioral-model-forward-plan_2026-09-24.md#30-consolidated-execution)
owns PR0–PR6: task/baseline freeze, mandatory catalog/contracts, bounded surface/options,
scenarios/deployment, typed retrieval, agent usability and comparative confirmation.

Earlier product receipts retain their recorded boundaries; they do not qualify the reconstructed
pipeline. Remaining research is activated only by an exposed claim or a named product task.

**Selected next increment / Proposed implementation, 2026-10-07:** [Persisted graph execution](../plans/persisted-graph-execution-plan_2026-10-07.md) replaces store-free compilation with native completed inputs and direct sealing, and consolidates matching inefficiencies across consumers. Operator-confirmed RC01/RC02 and dependency foundations inform this target; PG0 records the superseding decision and updates §B3/§B7/§B12 before dependent code. Earlier implementation receipts do not establish this target.

**Current increment: graph-native replacement (Implemented, 2026-10-06; ADR-0128).**
The [coordinator](../plans/graph-native-pivot-plan_2026-10-05.md) owns G0/M1/C1/C2, native realization,
serving, projections and acceptance. The store-free compiler stage is user-accepted; native
realization, projections and serving are implemented with targeted functional evidence. Compilation
returns an admitted artifact; publication and reader selection remain separate operations.
This execution uses the user-selected targeted controls in coordinator §6; broad qualification
and the stopped compiler suite are not_run.

This is a hard pivot: remove replaced runtime/code at its ownership boundary, rebuild from pinned
inputs, and retain no compatibility reader, ID bridge, intermediate PostgreSQL repair or dual write.
Pure typed semantics, source fidelity and independent functional expectations remain. Mandatory
catalog construction is independent of optional analytics and brief availability. Native SurrealDB
querying is selected; FP-07/A4 guide efficient physical design without detailed accounting or
performance proof requirements. Operator reconstruction and activation remain separate Q1 work.

Libraries remain pinned
uv projects (ADR-0117). ADR-0040 owns review cadence. ADR-0079 owns the current-tree editable development loop on dated nightly Cargo,
workspace feature unification, a CLI-only Hakari crate and shared intermediates with fine-grain
locking. Final artifacts stay in local `target/`: workspace O2 with incremental compilation,
imported dependencies O3 with sccache, and the existing release test workflow. No wheel
project is selected. **Accepted verification target, 2026-10-04 (ADR-0126):** agents run compile
checks and focused affected contract families during implementation, then affected controls and
applicable non-functional leaves at functional scope completion. A shared model/receipt/trust/
transport change or unresolved cross-boundary uncertainty requires assembled `just qualify`:
required families and representative journeys, compile-fail/doc contracts, full keep-going Clippy
and applicable leaves on one tree. This assurance pivot requires that assembled acceptance; a
focused pass does not establish it. Minor unrelated documentation/library changes do not
implicitly trigger it. Failures rerun the affected boundary or leaf. The root agent runs
`just turn-end` (formatting, generators) at the end of a turn that changed files, and `just ready`
after an environment change. The default operator store is inspected
only by explicit operator action; qualification owns disposable store configuration.

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

> Decision: ADR-0128, ADR-0071, ADR-0086, ADR-0087, ADR-0117, ADR-0079, ADR-0040, ADR-0126, ADR-0109, ADR-0113

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
- **One version.** `scripts/check_gold.py` (`just gold`, the applicable evaluation-policy leaf) fails when the
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

The primary engineering evaluation loop is programmatic; grounded outer agentic evaluation feeds
both system changes and evaluator models/coverage/judgments ([§12](sections/validation-and-evaluation.md#section-12), ADR-0130).
This accepted target is not implemented evaluation or comparative evidence.

The first **usable pilot** has accurate supported API contracts, typed options, contextual scenarios,
deployment evidence, field-specific uncertainty and a functioning real-embedding SurrealDB/MCP path. Valid
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


**Implemented policy (ADR-0040/0093/0130, 2026-10-06).** These are the load-bearing choices. The
repository owns core 3.3 and CI profile 1.5 and declares its standard in
`docs/design_review/design_principles/standard.toml`. The seven foundations organize review of
responsibilities, contracts, composition, explicit domain models and scoped semantic authority,
constraints, local reasoning and physical execution fit. The principles and review skill own the domain-model criterion;
assess it within bounded design/review periods at the binding's cadence. AGENTS.md routes to
that process without imposing a standing modeling mandate. Investigation depth follows the
scoped question; flow tracing is optional where it resolves a concrete uncertainty.
A2 retains model adequacy and authoritative behavior as acceptance criteria. A1–A4 judge architecture
independently of correctness and domain-fidelity gates. Supporting DP/CI IDs retain their
historical meaning at the version cited by each review. FP-07/A4 qualitatively assess access paths, necessary
versus repeated work, library composition, locality, working sets, reuse, assurance and lifecycle
scope against the intended workload. Semantic authority does not prescribe physical decomposition.
Static evidence can establish unjustified amplification; speed/capacity claims need measurement.
Explain material tradeoffs in plain language. This assessment does not require numerical estimates,
cost models, runtime accounting, execution-planning machinery, instrumentation or formal cost proofs;
such mechanisms need a separate concrete functional or operational requirement. No additional proof
artifact, mandatory benchmark, extra checklist or standing implementation review is introduced.

The [Heuristics for Efficient Architecture](../design_review/design_principles/core/efficient-architecture-heuristics.md)
(version 1.0, Implemented guidance, 2026-10-05) accompany the principles under FP-07/A4.
Use relevant patterns during design and planning before material physical choices become fixed,
and for consequential choices left open during implementation. Existing principles and lenses
remain; this companion adds no independent acceptance rules or recurring review obligation.

The library-context binding maps §B decisions to the standard and owns review cadence and
finding disposition. Changing a §B decision needs an ADR and a design/target review. Library
selection follows the owned capability, expected changes and total integration burden.
Early design favors coherent boundaries and inexpensive revision, with compatibility and
operational obligations scoped to real consumers and claimed behavior.

DESIGN owns accepted architecture and labeled targets; executable schemas and rule declarations
own their detailed contracts. ADRs record decisions and alternatives. Reviews preserve dated
evidence, while the active plan owns current disposition of scheduled findings. STATUS links to
that owner. Acceptance, implementation and verification remain distinct facts.

> Decision: ADR-0040, ADR-0093, ADR-0130

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

### §B2 Typed graph declarations own the data contract

**Implemented, 2026-10-06; targeted evidence is in the coordinator.** `lctx-model` owns nominal entities,
attributed assertions, participant roles, provenance, coverage, policies and shared validation.
Typed Arrow views support computation; the old relation registry is not the published schema.
Generate codecs and envelopes from one declaration owner. No first batch, JSON sample or embedding
response defines a schema. Codebooks remain append-only; snapshot changes are explicit migrations.
Semantic compatibility uses canonical declarations and policy/invariant revisions, separately from
implementation provenance and physical realization. Original bytes remain authoritative.

> Decision: ADR-0128, ADR-0085, ADR-0088, ADR-0073

<a id="section-b3"></a>

### §B3 Completed inputs and shared semantic operations

**Implemented, 2026-10-06; targeted evidence is in the coordinator.** Pure model operations and useful
native kernels retain their semantic ownership. The compiler consumes completed attempt-local
streams and spillable Arrow/DataFusion views without database roles, grants or readback. Share
ordered inputs and analytical topology; perform meaningful completion checks over their actual
required inputs. Tests and admission use the same semantic validators. Do not replay every producer
or turn design principles into an accounting/proof subsystem.

> Decision: ADR-0128, ADR-0126

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

### §B7 Separate admitted content from its native realization

**Implemented / focused Tested, 2026-10-06.** Compilation returns an immutable
admitted graph without a database. The publisher separately realizes it in strict per-snapshot
SurrealDB databases on the selected managed local RocksDB server. Checked bulk writes, one complete
stored reconciliation after writers drain, ready indexes/functions and sealed executable definitions
precede visibility. Publication does not select; readers pin one complete realization.

> Decision: ADR-0128

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

**Accepted policy, 2026-10-06; optional capabilities Proposed implementation.**
Excluded: a generic workflow engine, JSON-inferred canonical schemas, a whole-ontology petgraph
instance, a general production composition planner/constraint solver and graph embeddings.
SurrealDB was already selected by ADR-0128; a graph-database exclusion is obsolete.

Allowed: text embeddings and derived native search; the existing bounded Boolean/typed condition
kernel with its unknown/refusal limits; optional **offline** finite/theory/optimization evaluation
operations for a named task; optional qualified **non-generative** evidence scoring when a fixed
programmatic comparison warrants the lifecycle burden. Finite/contextual references are the initial
evaluator; solver installation is not a prerequisite. Preserve actual solver statuses/bounds and
independent oracle adequacy. A scorer cannot invent support or bypass context/qualification.

Production behavioral solving still uses declared bounded models/atoms/Merkle nodes and rendering-only
DNF; general production theory solving needs its separate concrete trigger. §B11's no-generative
compilation/query-output guarantees remain. Live optional models/tooling need their actual activation.

> Decision: ADR-0130, ADR-0128, ADR-0106, ADR-0045, ADR-0085

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

### §B12 One semantic graph and derived projections

**Implemented, 2026-10-06; targeted evidence is in the coordinator.** Rust-admitted graph content is realized
in SurrealDB; native queries operate over that content. Attempt-local Arrow segments, prepared
petgraph topology, search indexes and analytical/export views are derived, not parallel semantic
stores. Projection contracts state their universe, identity, multiplicity, roles, lineage and losses.
No PostgreSQL runtime or canonical serving copy remains after its ownership cut.

> Decision: ADR-0128, ADR-0131

<a id="section-b13"></a>

### §B13 FastMCP pins one complete realization

**Implemented / focused Tested, 2026-10-06.** Rust owns operation meaning and typed
requests/responses. Native SurrealQL may be the sole executor of a model-owned operation. Use
indexed native restriction and coarse connected hydration; retain complex kernels where simpler.
Python stays a thin FastMCP adapter with no second classifier. Resources/cursors pin content and
executable realization. Native deadlines/cancellation and response limits retain honest partial
outcomes; no fictitious exact engine-work ceiling is exposed. Activation remains stopped until Q1.

> Decision: ADR-0128, ADR-0114, ADR-0116, ADR-0025, ADR-0073, ADR-0131

<a id="section-b14"></a>

### §B14 Actual embedding dependencies and exact consumed values

**Accepted target, implementation Proposed, 2026-10-06 (ADR-0131).** Current code has the
format2 all-purpose1024 spec; the [combined plan](../plans/evidence-retrieval-and-evaluation-plan_2026-10-06.md)
schedules wholesale replacement, not compatibility with it.

- The typed embedding owner separates actual encoder/input, render/partition, query and projection
  dependencies. Query-only changes reuse unchanged document values; changed projection derives
  without inference. Exact checkpoint/tokenizer/engine/pooling/precision and winning bytes remain.
- Store one normalized full4096 F32 value per actual input. Initial ANN and E1 analytics share an
  explicitly declared normalized1024 prefix projection with F64 norm accumulation and one F32
  rounding. Full values serve candidate rescoring and offline references; full-dimensional analytics
  is a separate policy/experiment, not a service-output side effect.
- Both clients, launch admission, cache/value and consumer receipts, canonical graph/transport,
  native schema/search/reconciliation, analytical results and restore migrate together. Old1024
  values cannot recover missing components. Snapshot reconstruction uses exact canonical winners
  without a mutable cache/provider lookup.
- Current service/checkpoint locks remain; paths do not define values. Pure/fake controls establish
  seams only. Actual native/live usability, precision, quality and Q1 are separate evidence boundaries.

> Decision: ADR-0131, ADR-0128

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

**Implemented dependency policy, 2026-10-05.** Typed records flow through private compiler streams;
`just deps` checks single versions, declared nominal families and the Pyrefly fork.

DataFusion, Arrow/Parquet and object_store resolve to exactly one version each in the core
workspace (§B9). PostgreSQL provider dependencies are retired. Extra families are allowed only when declared in `scripts/check_family.py`
with version and nominal Cargo source scopes. The accepted analyzer target uses one latest
Ruff/ty fork revision in extraction/flow and a separate registry Ruff family only inside the
Pyrefly adapter; salsa's three crates are pinned exactly for the ty graph. M1 owns adoption.
[`docs/pins.md`](../pins.md) is authoritative for every hold, its reason and its dated
verification, and the `pin-check` skill governs changes. Every declared dependency is pinned
exactly (`=x.y.z`, `==x.y.z` or a git `rev`) and the committed lockfiles hold everything beneath;
a version moves only by a deliberate, named change, never a wholesale re-resolve (ADR-0132).

> Decision: ADR-0118, ADR-0132

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
