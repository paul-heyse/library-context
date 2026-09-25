# library-context — design

**This file plus `docs/design/sections/` is the current architectural authority.** It distinguishes implemented state from accepted
targets through the labels below; an accepted target is not an implementation claim.
`docs/adr/` says *why*, and what was rejected. Change a governed section only in the same commit as the ADR that decides it, and
end the section with `> Decision: ADR-NNNN`. Sections are never renumbered: insert `§3.2.1` rather
than shifting `§3.3`. There is no line budget: the detail the design needs comes before length
(operator, 2026-09-22; ADR-0004 amendment). Column-level contracts live in
`cpg-schema` and are snapshot-tested; they are not repeated here.

**Labels.** Every claim carries a principles §D label (`Proposed`, `Interface-checked`,
`Implemented`, `Tested`, `Measured`, `Formally established`). A section's label applies unless a
line says otherwise. `Interface-checked` means the named library surface was read at the pinned
version (skill brief, probe or pinned source), not that our code uses it yet.

**Sources.** `Source: IP Lxxxx–Lyyyy` cites `docs/initial_plan/Initial_plan.md`, the research
input, by line, because its top-level section numbers repeat. `docs/initial_plan/DISPOSITION.md`
maps every one of its sections to where it landed here, or why it did not.

---

<a id="section-1"></a>

## §1 Scope

<a id="section-1-1"></a>

### §1.1 Objective and the v1 promise

**Proposed** (ADR-0021, 2026-09-24; the plan `docs/plans/behavioral-model-forward-plan_2026-09-24.md`, which supersedes `behavioral-model-pivot-plan_2026-09-24.md`).
Source: the behavioral-model review (`design_review_behavioral-model-pivot_2026-09-24.md`).

The long-term aim is deep, evidence-backed insight into Python libraries for coding agents. A
pinned library compiles into an **evidence-carrying behavioral model of its whole public
surface**:
- which operations accept a value, pass it where, under which configuration, raising what, and
  needing which lifecycle;
- each claim with its evidence and derivation;
- an exhaustive answer where the analysis is complete, a ranked candidate where only discovery
  applies, and a named `unknown` where the analysis stopped (§3.9).

**Briefs remain as one rendering,** for a curated subset (§10).

The universe is `public_paths` (§9's opening). Every analysis runs per public callable, and a
seed traversal becomes a query.

Agents reach the model through these operations. `get_operation`, `find_operations` and
`search_operations` are **Implemented** (Stage 1; §11.3); `explain` and `lookup_concepts` are
**Proposed** and land by plan stage:

| Operation | Returns | Does not promise |
|---|---|---|
| `search_capabilities(library, query, limit)` (**Implemented**) | Published briefs relevant to a task, including ones whose API names differ from its wording | Exhaustive discovery |
| `get_capability(snapshot_id, capability_id)` (**Implemented**) | The full brief | A synthesized solution for arbitrary requirements |
| `get_operation(snapshot_id, operation)` (Stage 1) | One public operation's record: paths, signature, each control's fate and verdict, behaviors with conditions, raises, callbacks, ambient reads, evidence, boundaries; its brief if one exists | Behavior the analysis did not reach, which is stated as `unknown` or `not_analyzed` |
| `find_operations(library, where, limit, cursor)` (Stage 1; semantic filters Stage 3) | **Exhaustive** matches over the pinned generation, with `complete` and the operations whose answer is unknown; never vectors | Completeness where a boundary intervenes, which it names |
| `search_operations(library, query, filters, limit)` (Stage 1) | **Ranked** discovery over operations, labelled as such | Exhaustiveness |
| `lookup_concepts(text)` (Stage 4) | Candidate capability concepts with scope notes | A single interpretation of ambiguous wording |
| `explain(snapshot_id, claim)` (Stage 4) | The derivation: rule, premises, source spans | Proof of runtime behavior beyond the stated model |

Compile time publishes facts, summaries and lossless condition nodes. A request can select
materialized rows or run a bounded Rust semantic query over that immutable generation (§11.3,
ADR-0025, Proposed). No generative model runs in the request path.

*Superseded (ADR-0004, 2026-09-22):* stage 1 was a capability compiler whose product was a small
set of searchable briefs for one subsystem, reached by the first two operations only.

> Decision: ADR-0021, ADR-0025

<a id="section-1-2"></a>

### §1.2 Increments

**Proposed deliverables; Implemented review policy (ADR-0040, 2026-09-25).** Source:
IP L3073–3085, reshaped by ADR-0004 and ADR-0021. Each increment is a working vertical slice.
The repository binding's **Reviews in this repository** section owns cadence, tier and purpose:
architectural choices and assembled stages are reviewed through expected change scenarios;
bounded implementation retains a scoped conformance review. A coincident stage/increment end
needs one review. Historical compact/standard/deep labels do not impose the old odd/even schedule.

| # | Deliverable |
|---|---|
| 1 | **One complete path.** Real FastMCP 4.0.5 (`libraries/fastmcp`, §4.0), with one hand-registered seed, `fastmcp.FastMCP.tool` (analytics config, §1.4), plus 2–4 distractor briefs for other public entry points. Families provenance, exports, signatures, calls, coverage, findings, embedding_cache. Pass A → increment-1 assertion kinds (§10.2) → brief → bundle → embeddings → hybrid search (BM25, exact cosine, RRF, exact-symbol promotion, degraded lexical-only mode; moved up from increment 4, ADR-0004 amendment) → hydration → both MCP tools |
| 2 | **Analytics families on synthetic fixtures.** Passes B and C with the `syntax` and `lexical` families; single-layer community detection with seed-consensus stability; `page_rank`; FCA within one structural scope (a community's membership is statistical, §9.6) |
| 3 | **Pilot corpus and analytics** (done: slices 3.1–3.3, the holistic assessment's Phases 0–2), then **plan Stages 0–1: the whole public surface** (ADR-0021): the `behavior` family persisted, passes per public callable, the operation catalog, bundle `FORMAT` 3, `get_operation` / `find_operations` / `search_operations`, the source-body view, structured evaluation v0 |
| 4 | **Plan Stages 2–3: the flow IR and summaries** (ADR-0022): `flow` family, conditions, verdicts, `self` fields and ambient reads; transfer summaries and the models catalog |
| 5 | **Plan Stages 4–5: concepts and frameworks**: the capability registry, `lookup_concepts` and `explain`; framework models and protocols; then the held-out structured evaluation and the decision on the LLM trigger (§B11) |

*Superseded rows (ADR-0004):* increment 3 was "~15–25 reviewed briefs"; increment 4 "reliable
serving" (its generation lifecycle is built into §6.4 and §11.3; `claude mcp add` returns with the
tools); increment 5 "agent evaluation" (replaced by the structured evaluation, operator
2026-09-23).

**CPG first** (operator decision, 2026-09-22; ADR-0004 amendment, ADR-0014). Increment 1's
slices 1–3 built the extraction, derivation and publication path. Before its analytic path
(analytics config, Pass A, briefs), the CPG is completed in slices C1–C6 (§3.8):

| Slice | Delivers |
|---|---|
| C1 | The node and edge catalogs over the increment-1 families; typed external and synthetic endpoints; dependency definitions; retention; per-stage metrics |
| C2 | `syntax` |
| C3 | `lexical`, with our recognizer's full name resolution |
| C4 | `types`: observations, type structure, record fields |
| C5 | The source corpus: `docs`, and examples and tests as a usage run |
| C6 | The whole CPG on the pilot, measured |

So `syntax` and `lexical` move up from increment 2, and `types` and `docs` from increment 3. The
analytics that read them stay in their increments, and every family names its consumer.

**Implemented** (ADR-0026, 2026-09-24): the ordinary development loop uses the pinned stable
Rust 1.98.1, 16 Cargo jobs, the default single rustc frontend thread, sccache, and disabled
incremental compilation so workspace compilations can be cached. Agents work on `main` in the
current tree; a separate worktree is reserved for truly concurrent production-code edits.
Clang invokes mold for Linux links. Performance of the exact 16-job stable configuration and
cache recovery remains **Proposed** until measured.

> Decision: ADR-0021, ADR-0014, ADR-0013, ADR-0026, ADR-0040

<a id="section-1-3"></a>

### §1.3 Non-goals for stage 1

**Proposed.** Source: IP L2073–2087, L1878–1882; revised by ADR-0021.

- Analysis of the caller's own codebase.
- Comparison across library versions.
- Arbitrary composition planning.
- Constraint solving over requirements such as "under 500 MB". Conditions are a closed language
  with three-valued compatibility (§3.9), not a solver.
- **Query-time graph traversal.** Paths are precomputed as summaries and witnesses. A request
  selects and joins materialized rows under bounded operators (§11.3).
- Native-extension bodies.
- **A general ontology, an RDF store or a reasoner.** The capability registry is a closed,
  executable vocabulary in `cpg-schema` (§9.9).
- **General alias analysis (points-to).** Flow is intraprocedural over bounded places (§3.9), and
  summaries are interprocedural (§9.9).

*Retired (ADR-0021):* "a domain-capability ontology beyond the briefs" and "Python CFG, dataflow
and alias analysis". Both are now in scope, as stated above.

> Decision: ADR-0021

<a id="section-1-4"></a>

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
  are pre-registered. The gold was the development metric for keeping or removing techniques
  under ADR-0020's rule. Since ADR-0021, its scores (§12(a)–(c)) are a record of brief
  retrieval, and a technique is judged by the structured evaluation (§9.8). The gold is never
  used to tune parameters.
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
- **Unchanged by ADR-0021:** the pilot, the analytics config and the evaluation-only gold.
- **The subsystem scopes briefs only** (ADR-0021, as revised by its standard review's F1): seed
  selection and the seed passes behind briefs.
  - The behavior model's universe is `public_paths` under the config's public roots: 1,534 nodes
    on the pilot, 1,107 of them outside the subsystem's prefixes (Measured, 2026-09-24).
  - Its scan follows parameters into any release function (§9's opening).
- **Every freeze ADR-0004 held is ADR-0021's**, with the disclosure clause: an edit names the gold
  its author has seen. That covers the config, the code parameters, the selection parameters and
  the variant policies (`eval/gold/analytics-freeze.json`), and an edit to any of them is an
  ADR-0021 amendment.

> Decision: ADR-0021, ADR-0013

<a id="section-1-5"></a>

### §1.5 Definition of done

**Proposed** (ADR-0021).

**The behavioral model is done** when four things hold:
- each plan stage's exit criterion has passed on the pilot:

  | Stage | Exit criterion |
  |---|---|
  | 1 | Controls and handoffs answered for operations with no brief |
  | 2 | `flow_shapes` and `behavior_shapes` known answers; eval Q4 and Q10 |
  | 3 | Q1, Q3, Q5, Q9 |
  | 4 | Concept queries |
  | 5 | Q6, Q7, Q8, Q11, Q12 |

- every stage's structured evaluation (`eval/behavior/`, targets pre-registered) has been assessed
  and reviewed by the operator;
- the held-out structured evaluation has run;
- no answer states `refuted_under_model` outside complete coverage (§3.9).

*Record, the capability compiler's definition of done (ADR-0004), kept as history:*

Stage 1 was done when all three derivation families (delegation, configuration/restriction,
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

**Still in force for briefs** (ADR-0021 keeps them as a rendering; D-6):
- the §12 evaluation must have run;
- no brief may reach publication by bypassing the analytics. A brief that is honestly explained
  by documentation alone is allowed, and is labelled that way.
  - "Analysis-backed" means citing a derivation family's positive finding about behaviour
    (`findings::ANALYSIS_BACKED`: delegations and implementation boundaries; Pass B and C kinds
    join as they land). A `public_alias`, an unresolved site or a traversal stop does not count.
  - `semantic:brief-cites-analysis` holds the label both ways. `semantic:documentation-only-has-outcome`
    requires that a documentation-only brief has a `documented` Outcome (slice 1.5 review F4).

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
- **No second parser or type checker**, with one declared exception: `cpg-flow`'s flow facts come
  from ty's semantic index over a second parse (ruff 0.0.14), joined to ours by byte range under
  two-way parity rules (ADR-0012 amendment, ADR-0022 §The flow provider). Every other fact comes
  from the one parse. Otherwise, gaps are closed with adapters, normalization and our own analyses.

> Decision: ADR-0012

<a id="section-b2"></a>

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

<a id="section-b3"></a>

### §B3 DataFusion constructs and validates relations

**Implemented** and **Tested** (C6 review, 2026-09-23): the derivations and the 496 rules run in
DataFusion; `validate` is the one validator publication (`attempt.rs`) and the tests call; what a
rule can prove is stated in §8 (its edit guards counted apart).

- Most nodes and edges are joins, projections and unions over extracted facts.
- Cross-table invariants are DataFusion queries, one per rule (§8).
- The same validators run in tests and before publication.

<a id="section-b4"></a>

### §B4 Graph algorithms have named owners

**Interface-checked.** Source: IP L1335–1504, L2233–2362.

| Owner | Algorithms |
|---|---|
| petgraph 0.8.3 | Traversal and SCCs, over immutable, explicitly declared projections (§5) |
| leiden-rs | Community detection (§9.4), fed a normalized, sorted edge list |
| Our own code | Weighted PageRank with convergence diagnostics (§9.5; petgraph's `page_rank` takes no weights, miscounts parallel arcs and reports no convergence: the library-leverage review, D1); formal and relational concept analysis (§9.6); condensation from `kosaraju_scc` membership (iterative; petgraph's `condensation` merges parallel edges) |

- **Each algorithm has a named consumer in the served model**, a tool's output or a brief (§9;
  ADR-0021).
- **A relationship does not need a graph algorithm** just because it has two endpoints.
- **What runs by default is decided by the §9.8 keep rule** (ADR-0020, Measured 2026-09-23):
  Passes A–C, direct usage and selection. Communities (leiden-rs), FCA, kNN, PageRank, RCA and
  the extra layers are variants, off by default; each is reachable with `lctx compile
  --analytics`.
- **The flow and summary algorithms** (ADR-0011 amendment, 2026-09-24; **Proposed**):
  - dominators by petgraph's `algo::dominators::simple_fast`;
  - post-dominators by the same function over `Reversed`, from a virtual exit;
  - dominance frontier and control dependence in our own code;
  - transfer summaries bottom-up in `tarjan_scc` order (callees first), each SCC iterated to a
    fixpoint over a finite domain, with widening to `unknown` (§9.9).

  A Datalog engine (ascent) is a spike behind a trigger. The existing techniques gain consumers in
  the served model (§9.9), and none of them writes concept membership.

> Decision: ADR-0011, ADR-0020

<a id="section-b5"></a>

### §B5 Python semantics are custom Rust passes

**Proposed** (ADR-0022, 2026-09-24).

- Python-specific semantics are our own Rust code with stated abstractions: v1 recognizers
  (§9.1–§9.3), then the **flow IR** (§3.9), our stated **runtime** abstraction.
  - It is statement-level control flow with exceptional exits, and reaching definitions over
    bounded places.
  - `TYPE_CHECKING` is false; `sys.version_info` and `sys.platform` tests follow the analyzed
    context.
- **The flow-IR provider** is `ty_python_core` 0.0.14 in `cpg-flow` (ADR-0022 §The flow provider,
  decided by the Stage 2.1 spike), behind two-way parity rules and our runtime override.
- **No provider's IR is the model.** Pyrefly's inference graph, and ty's use-def map as ty decides
  it, are never relabelled as runtime dataflow. Pyrefly's binding graph is a parity oracle only.
- **Meaning comes from models, propagation from summaries** (§9.9). A call alone never propagates a
  capability.
- **Implemented and Tested in focused cases (ADR-0033, 2026-09-25):** total normal completion
  is an explicit pinned model assertion, independent of a value-transfer rule and exception
  silence. Only an exact function target with complete exception coverage and no authored
  exception rule may carry it; a source call still needs its own endpoint, target, condition and
  enclosing-exit proof before it can yield a positive summary.
- **Implemented and Tested in focused cases (ADR-0029, 2026-09-25):** an applicable authored
  exception class binds to one pinned context class fact before its candidate source call is
  published. Class spelling is display, not handler-match authority. An absent or ambiguous
  class fails the model compile; a dormant model remains dormant.
- **Implemented and Tested in focused cases (ADR-0030, 2026-09-25):** a different pinned handler
  class can be a positive ancestor candidate only when the pinned Pyrefly context MRO cites it.
  MRO nonmembership is unknown for a model class that may denote a subclass family; no handler
  completion or exception fate follows from this relationship.

**Implemented and Tested in focused cases (ADR-0028, 2026-09-25):** the flow producer retains
each value use's ordered, outer-to-inner call path with callee/argument roles and exact byte spans.
The raw `flow_value_calls` relation cites its parent `flow_values` fact; `through_call` is derived
from path presence. Shared publication validation checks dense ordinals, nested containment,
use attribution and the boolean/path equivalence. The extractor output version is 28. A nested
fixture and path tamper passed focused checks. **Implemented and Tested in focused cases
(2026-09-25):** DataFusion derives `flow_value_call_links` with one row per step, a unique Ruff
call/argument citation or a typed missing/ambiguous status. Shared validation recomputes the
relation, including withheld and absent rows. The compiler output version is 37. This source
bridge cannot by itself identify a completed modeled transfer.

**Implemented and Tested in focused cases (2026-09-25):** `value_flow_contributions`
retains each raw `flow_values.fact_id`, use, source origin and condition before
`value_flows` merges paths. Its `local_through_call` distinguishes a call crossed by
that raw fact from call uncertainty inherited through a reaching definition. L3
joins a local call only through that fact's `flow_value_call_links`; inherited
calls require following the earlier definition instead. Publication reconstructs
the entire relation and rejects missing contributions. This is provenance for a
future summary, not a discharged modeled transfer. **Implemented and Tested in
focused cases (2026-09-25):** `sink_function_node_id` names the callable
containing the raw use independently of a captured parameter's defining
callable. The direct-call fixture verifies the sink owner is populated;
different owners remain a separately testable case. The compiler output
version is 43. **Implemented and Tested in focused cases (2026-09-25):**
`upstream_identity` and `upstream_through_call` preserve the transfer reaching
that use before its local raw value fact. A direct `open(path)` return has an
identity upstream input; returning a prior `open(path)` result carries upstream
call uncertainty and cannot borrow the return fact's call links. The compiler
output version is 44. These flags still do not cite the predecessor fact chain.
**Implemented and Tested in the release Rust suite (2026-09-25):** output version 74
migrates the `value_flow_contributions` key to include local and upstream transfer flags,
so two semantically distinct raw paths cannot collide. Shared reconstruction sorts by
that full identity. The validator selects the distinct snapshot across a library and its
corpus releases, and the pinned context MRO shape rule validates distinct ancestor facts:
both releases may cite the same ancestry without creating a false duplicate, while
conflicting ancestors at an ordinal still fail. The accepted contract and rule snapshots
record the migration. The full cross-language gate was not observed to completion.

**Implemented and Tested in focused cases (2026-09-25, first ADR-0028 source seam):**
`arguments` persists the argument expression's value span separately from its authored
role span. A keyword value excludes the `name=` prefix; a direct positional argument has
equal role and value spans. The schema enforces value-span containment and the extractor
output version is 27. This alone identifies no flow-call path or completed transfer.

**Implemented and Tested in focused cases (ADR-0034, 2026-09-25):** a finite summary is
identified by its callable, input/output paths, transfer kind, condition, exit and ordered
typed proof steps, not by one
raw return fact. The initial step kind cites a raw identity fact. Later call/model/reaching
step kinds require their own checked producer before they can establish a positive flow.

> Decision: ADR-0022, ADR-0028, ADR-0029, ADR-0030, ADR-0033, ADR-0034

<a id="section-b6"></a>

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
- **Finite summary proof paths** (ADR-0034; **Implemented and Tested in focused cases**, 2026-09-25)
  have a canonical `summary_id` derived from the ordered typed witness sequence. The
  `summary_flow_steps` relation retains source evidence and condition ids per step in the same
  snapshot; the shared validator reconstructs it. A new proof variant is appended only with its
  cited source relation and validation rule.

> Decision: ADR-0014, ADR-0019, ADR-0034

<a id="section-b7"></a>

### §B7 Delta canonical store, published by a `snapshots` append

**Implemented** and **Tested** (`an_attempt_publishes_every_table_and_readers_see_only_published_rows`,
`reads_pin_the_version_and_filter_the_snapshot`, `a_failed_snapshots_append_is_classified_by_rereading`,
`retention_keeps_old_versions_loadable`; C6 review, 2026-09-23).

- Fact tables are append-only Delta tables, one per fact family.
- A snapshot becomes visible only through one append to the `snapshots` table, made after
  validation passes.
- Readers resolve table versions through that row and filter by `snapshot_id` (§6).

> Decision: ADR-0017 (superseding ADR-0009)

<a id="section-b8"></a>

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

<a id="section-b9"></a>

### §B9 One pinned Rust dependency family

**Tested** (§7).

DataFusion, Arrow/Parquet, object_store and delta-rs each resolve to exactly one version, from
the set in §7.

> Decision: ADR-0002

<a id="section-b10"></a>

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

**Allowed:** text embeddings, and a *rebuildable* search projection of the published generation
(§B12): briefs and, from ADR-0021, operations.

**Proposed Stage 3 amendment (ADR-0024).** A bounded Boolean decision-diagram kernel and a
narrow typed theory for proven stable primitive places are allowed. They decide compatibility
and implication over evaluation atoms (§3.9), returning `unknown` at node, stability or type
boundaries. A general constraint solver remains excluded. Stage 2 still uses bounded DNF.

> Decision: ADR-0005, ADR-0022, ADR-0024

<a id="section-b11"></a>

### §B11 Insight synthesis is programmatic; no LLM in the query path

**Proposed.**

- Assertions come from typed findings through deterministic templates and extractive text
  selection (§10).
- There is no generative model in v1.
- **Adding a generative model** (a local vLLM model) needs an ADR, triggered by the §12 gap
  metric. Even then it runs only at compile time, under mechanical grounding.
- **No generative model ever runs in the query path.**

> Decision: ADR-0005

<a id="section-b12"></a>

### §B12 Canonical store vs serving projections

**Proposed.**

- **Authority:**
  - the Delta fact store is authoritative for facts, findings and assertions;
  - the serving bundle (§6.4) is a derived, immutable projection, built only from a published
    snapshot and rebuildable byte-for-byte from Delta;
  - any search index built from the bundle is derived from it in turn.
- **No cross-store transactions:** the bundle manifest names the snapshot it came from.

> Decision: ADR-0017 (superseding ADR-0009)

<a id="section-b13"></a>

### §B13 The agent interface is a FastMCP server over file-based generations

**Interface-checked.**

- The Python package `lctx_mcp` serves the operations of §1.1 (§11.3) from one pinned serving
  generation per process: the two brief tools now, the behavioral tools as their stages land
  (ADR-0010 amendment, 2026-09-24; **Proposed**).
- It reads one immutable generation: no Delta, DataFusion, compiler code or network access in
  the semantic executor. Direct lookup and ranked retrieval may use the current pyarrow-loaded
  dictionaries and vectors.
- **Proposed Stage 3 amendment (ADR-0025).** A pinned, in-process Rust/PyO3 extension executes
  bounded semantic queries over that generation. Condition compatibility and implication,
  effect/role filters and witness traversal use the Rust authority. Inputs are typed, never SQL
  strings or executable expressions. Row, node, pair-work and depth budgets yield `unknown` and
  `truncated`, never a negative or `complete` claim. Results cite same-generation row/node ids;
  cursors bind generation, query and deterministic order. No semantic decision is duplicated in
  Python. The current materialized lookup route stays available for direct retrieval.
  The extension is `lctx_semantics._native`, built by a pinned maturin uv workspace member
  for CPython 3.14.7, and startup checks its kernel format and lossless node closure against the
  generation. Editable `uv run` loading is the design-phase gate; a clean wheel installation
  and generation-pinned FastMCP/native query are required at Stage 3.6 product acceptance and
  again at release, after the generation format and query API settle. A semantic filter partitions
  candidates into matched, proven-excluded,
  source-open and unexamined; `complete` requires the latter two to be empty.
  During design, focused probes settle individual contracts; full `just test-all` and `just pilot`
  acceptance runs occur at the integrated Stage 3 end (operator direction, 2026-09-24).
- **Implemented and Tested (2026-09-25, partial FORMAT 7).** The generation now transports
  validated provider and analysis BDD roots, public operation/formal ids, finite value summaries,
  ordered proof steps and explicit summary boundaries. One immutable PyO3 index checks condition
  closure, proof order, callee-summary references and depth before answering an internal
  operation/formal value-path lookup. This is a cited positive-path inspection boundary, not a
  served compatibility, effect or role filter. Source spans, bounded cursors, primitive origins
  and the user-facing semantic tool still need the rest of Stage 3.6.
- **Implemented and Tested (2026-09-25, shared primitive kernel).** The bounded exact-input
  refutation routine and its slim, typed value-link/leaf projection live in `cpg-schema`, next
  to the BDD kernel. Its caller supplies the entry-value effect-rule digest; the compiler's
  focused exact-origin case and the pure kernel tests exercise the same routine. A native
  consumer and served compatibility verdict remain open.
- Which facet rows are complete remains served data (`operation_facet_status`); facet names are
  held to the codebook by `specs/serving/facets.json`.

> Decision: ADR-0025 (superseding ADR-0010)

<a id="section-b14"></a>

### §B14 One embedding spec, cached vectors

**Interface-checked** (the vLLM and Qwen behaviour); **Proposed** (our spec).

- One hashed embedding spec (§11.1) governs every vector.
- Compile-time vectors (Rust client) and query-time vectors (Python client) must both match it,
  checked against shared conformance vectors.
- Vectors are cached by `spec_hash + input_hash` in the canonical `embedding_cache` Delta table.
  Snapshots record the cache version they read, and bundles copy vectors from it. A cached vector
  is a run input.
- **Views** (ADR-0010 amendment, 2026-09-24; **Proposed**):
  - A spec is one model, one vector space.
  - A view (signature and docstring, source body, and later others) is a **column**
    (`operation_documents.embedding_view`, `operation_vectors.embedding_view`). It is **not** part of
    the cache key, which stays the request text's hash (the re-review R4).
  - Two views with the same text share one vector, correctly, because one text has one vector per
    spec. `semantic:one-embedding-spec` holds as written: one spec per snapshot.

> Decision: ADR-0025 (carrying forward ADR-0010)

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

Each item returns by ADR when a consumer needs it.

| Deferred | Where it is described | Trigger |
|---|---|---|
| Full ontology tables beyond the CPG families (the native type graph and `record_fields` are built in C4, §3.2). *The capability registry (§9.9) is not this: it is a closed, executable vocabulary* | IP L469–L717, L334–L409 | an analytic or brief needs the detail |
| Ruff semantic-model port | IP L99–L166 | binding kinds or typing-only context are needed |
| Cross-references (Pyrefly's Glean collector, `report::glean::convert::glean(&Transaction, &Handle)`, reachable in-process today). Probe (2026-09-23): 42 xref targets on a fixture, including the typed attribute xref `self.helper()` → `pkg.mod.C.helper`, in `declarations.qualified_name` form. One target per use, flow-sensitive and pruned, so it complements `reference_resolutions` and never replaces it | IP L209–L231 | a consumer needs attribute cross-references (Pass C handoffs, §10 "see also") |
| CinderX located types (narrowed, unnarrowed, contextual), TSP query surfaces | IP L290–L332, L410–L425 | narrowing or contextual types are needed |
| General alias analysis (points-to). *CFG, dominance, flow over bounded places and summaries left this table on 2026-09-24: ADR-0022, §3.9, §9.9* | IP L821–L884, L1505–L1563 | a question needs aliasing beyond bounded places |
| SCC condensation, dominators on projections: condensation from `kosaraju_scc` membership keeping every arc's evidence, never petgraph's `condensation` (it merges parallel edges; ADR-0011) | IP L1416–L1503 | a consumer beyond recursion labelling |
| LanceDB / Lance, ANN indexes. When adopted: an isolated workspace with its own lockfile and family check (ADR-0002 amendment), Arrow IPC as the only interface, and a derived index keyed by the generation key outside the byte-identical manifest. Lance writes are not byte-reproducible: 1 of 10 files identical across identical writes (review probe, 2026-09-24) | IP L2766–L2965 | more than ~10⁵ vectors at 4,096-d, or filtered ANN with managed FTS |
| A Datalog engine (ascent 0.8.1) for recursive rules | ADR-0011 amendment | three or more recursive rule families repeating the worklist shape |
| Graph-FCA in the pipeline; on-demand RCA at serve time | the behavioral-model review, §8.3 | an offline experiment yields templates the structured evaluation rates useful; materialized membership proves too coarse |
| LLM interpretation | IP L1886–L1966, L2580–L2637 | §12 gap metric (§B11) |
| Graph embeddings, neural reranking, composition planning | IP L2073–L2087 | an ADR after increment 5 |

> Decision: ADR-0012, ADR-0021

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
| 2026-09-24 | The behavioral-model pivot, Stage 0. The product becomes a behavioral model of the whole public surface, with briefs as one rendering: §1.1–§1.5, §9 rules, §12, §13 (ADR-0021, superseding ADR-0004). The model's semantics: places, the runtime view, closed conditions, five verdicts, models as data (§B5, §B10, §3.2, new §3.9, new §9.9; ADR-0022). Serving tools, the executor, `FORMAT` 3 and embedding views (§B13, §B14, §6.4, §11.3; ADR-0010 amendment). Declared extra dependency families (ADR-0002 amendment). Flow and summary algorithms (§B4; ADR-0011 amendment). The keep rule under the new consumers, and the deletion exit paused (§9.8; ADR-0020 amendment). Source: `design_review_behavioral-model-pivot_2026-09-24.md` and the plan `docs/plans/behavioral-model-pivot-plan_2026-09-24.md` | ADR-0021, ADR-0022; deviation log B1 |
| 2026-09-24 | The review standard: §2's pointer moves from `ADDENDUM.md` to the library-context binding; charter and graph-guideline citations re-keyed to DP and CI IDs | ADR-0023 |
| 2026-09-24 | Proposed Stage 3 kernel and serving design: bounded BDD conditions, typed proof links and same-generation native queries (§B10, §B13, §B14, §3.9, §9.9, §11.3). During design, editable native import and focused probes are the fast gate; clean wheel installation with a generation-pinned tool call waits for Stage 3.6 product acceptance, then repeats at release. | ADR-0024, ADR-0025 (proposed) |
| 2026-09-24 | Development loop uses pinned stable Rust, 16 Cargo jobs, sccache with incremental off, Clang/mold, and the main working tree except for concurrent production-code edits (§1.2) | ADR-0026 (supersedes ADR-0001) |
| 2026-09-25 | Stage 3 narrows raise escape to explicit unframed source sites; an unresolved `try` or `with` withholds definite escape until L2 proves the frame action (§3.9, §9.9) | ADR-0027 (supersedes ADR-0022's raise-escape shortcut; retains its other decisions) |
| 2026-09-25 | Proposed call-result provenance for Stage 3 summaries: ordered nested call steps and argument value spans preserve the source-to-model join (§B5, §3.9, §9.9) | ADR-0028 (proposed) |
| 2026-09-25 | Model-authored exception classes bind to pinned context definitions before source application; the extractor producer identity includes model catalog bytes (§B5, §3.2, §4.0, §9.9) | ADR-0029 |
| 2026-09-25 | Positive modeled exception ancestor relationships use pinned Pyrefly context MRO facts; nonmembership stays unknown (§B5, §3.2, §9.9) | ADR-0030 |
| 2026-09-25 | Stage 3 source provenance retains raw value-flow contributions and distinguishes local from inherited call crossing; candidate handlers apply within-frame clause order only, conditional on raise reaching the frame (§B5, §9.9) | ADR-0028 (proposed), ADR-0030 boundary |
| 2026-09-25 | Added the pinned `typing.assert_type` identity model, with exact source argument binding and an exhausted narrow CrossHair oracle (§9.9) | — |
| 2026-09-25 | Added pinned `json.dumps` and `json.dump` transform, serialization and stream-write model candidates with exact `obj`/`fp` binding; custom encoders remain open (§9.9) | — |
| 2026-09-25 | Raw value-flow contributions now distinguish the sink callable from a captured parameter's source callable for L3 summary ownership (§B5, §9.9) | ADR-0028 (proposed) |
| 2026-09-25 | Raw value-flow contributions retain the upstream transfer before the local fact; a later one-call summary may refuse inherited call uncertainty (§B5, §9.9) | ADR-0028 (proposed) |
| 2026-09-25 | A handler with one direct `return None` gets a source/region-cited, pre-finally witness; ty region approximation remains explicit and no catch/completion fate follows (§9.9) | — |
| 2026-09-25 | A bounded direct `try` and sole `return None` handler yields only a candidate-local conditional modeled-exception path, with nested frames and finalizers withheld (§9.9) | ADR-0031 |
| 2026-09-25 | The first source-to-model direct-return bridge requires one exact call step and matching argument/result nodes, with inherited and nested call paths withheld (§B5, §9.9) | ADR-0028 (proposed) |
| 2026-09-25 | Reaching definitions now cite raw predecessor value facts for inherited call paths, while independent conditions and approximation flags remain unresolved pending structural BDD persistence (§B5, §9.9) | ADR-0028 (proposed) |
| 2026-09-25 | Recomposed flow-analysis conditions now persist in a separate structural BDD catalog with shared hydration validation, giving L3 an authority for condition composition (§3.9, §9.9) | ADR-0032 |
| 2026-09-25 | A direct modeled return now requires the call to occupy the entire return-value expression; an outer fallback cannot inherit an identity model (§9.9) | ADR-0028 (proposed) |
| 2026-09-25 | L3 begins with bounded BDD compatibility over cited predecessor, reaching and successor roots; loops and condition boundaries stay unknown (§9.9) | ADR-0024, ADR-0028 (proposed) |
| 2026-09-25 | The exact one-call model step now covers whole definition values as well as whole returns, enabling a cited assignment predecessor without promoting it to a summary (§9.9) | ADR-0028 (proposed) |
| 2026-09-25 | Finite summary paths gain canonical ids from ordered typed proof steps; the initial raw identity step is cited and source-equality validated, while model-call variants remain pending (§B5, §B6, §9.9) | ADR-0034 |
| 2026-09-25 | Pinned function models can assert total normal return independently of transfer modality and exception silence; the first assertions cover `typing.cast` and `typing.assert_type`, with source-call composition still conditional (§B5, §9.9) | ADR-0033 |
| 2026-09-25 | Exact modeled value paths now account for every call argument in source order, citing the selected operand or a direct literal and retaining dynamic siblings as unknown (§9.9) | — |
| 2026-09-25 | A Pysa object receiver plus a direct Ruff attribute callee shifts pinned model positional formals past `self`; class receivers and unpacking remain unknown. `logging.Logger.warning` adds a potential subject-bound log candidate (§9.9) | ADR-0035 |
| 2026-09-25 | One literal `finally: pass` frame now preserves a pending return with cited pass syntax evidence; nested and effectful frames remain unresolved. `return_exit_statuses` gains two nullable proof columns and compiler output version 66 (§9.9) | ADR-0036 |
| 2026-09-25 | A verified Pysa source-to-sink rule with explicit Pyrefly pin and a byte-identical compiler fixture agree on two positive value paths and a constant control; the pinned FastMCP differential remains open (§9.9) | — |
| 2026-09-25 | Nested pass-only finalizers now preserve a pending return with an inner-to-outer sequence of cited source steps; effectful finalizers and `with` remain open, and derivation output version is 70 (§9.9) | ADR-0037 (supersedes ADR-0036) |
| 2026-09-25 | Recursive SCC membership now withholds direct and modeled finite value summaries until a bounded worklist proves completion; explicit unknown boundaries remain (§9.9) | — |
| 2026-09-25 | Direct-return admission now screens earlier attributed source calls, allowing a terminating base return before recursion while retaining post-call unknowns (§9.9) | ADR-0038 |
| 2026-09-25 | Direct-return admission can ignore an earlier source call only when its ty region is BDD-incompatible with the return condition; missing or bounded evidence withholds (§9.9) | ADR-0039 (supersedes ADR-0038) |
| 2026-09-25 | Raw value-flow keys retain local/upstream transfer distinctions; shared validation accepts library-plus-corpus releases under one snapshot and compares distinct MRO ancestry assertions. Output version 74, reviewed schema/rule snapshots (§3.9) | Stage 3 preliminary gate repair |
| 2026-09-25 | Core 3.0: six foundations, scenario-based architectural judgments, repository ownership, risk-based review cadence and single finding disposition owner (§1.2, §2) | ADR-0040 |
| 2026-09-25 | Architectural collection, stable section owners, derived mdBook/Pagefind publication and isolated documentation qualification (§2) | ADR-0041 |
