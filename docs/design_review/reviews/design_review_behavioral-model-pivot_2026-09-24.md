# Design review — the behavioral-model pivot

**Target:** `docs/behavioral_model_pivot.md` (an external agent's proposal, untracked), read against
`docs/design/DESIGN.md` and the code at `7bebdd4`.
**Depth:** deep. **Date:** 2026-09-24. **Reviewer:** Claude (Opus 5.5), at the operator's request;
not the proposal's author.
**Operator direction for this review:** assess what the target design should be for a rich,
programmatic behavioral basis; propose improvements; propose crates, libraries and packages. Web,
Context7 and library research allowed. Licence is never a criterion. Separate pins are acceptable
when managed: a newest-ruff line, and a Lance/LanceDB dependency family (operator, 2026-09-24).

**Decision: Revise.** The pivot's direction is right, and the repository is a good base for it. But
the document is not yet a design: four gates are unresolved on the behavior it claims (§6). §8.2
gives the target design this review recommends. §10 lists the nine decisions the operator has to
make before the superseding ADR is written.

---

## 1. Decision and scope

**Proposal.** Turn library-context from a capability compiler that publishes about 20 briefs into
an "evidence-carrying semantic query engine":

> code facts → conditional behavioral relations → executable capability concepts → structured
> queries and semantic discovery.

Briefs become one optional rendering. FCA, RCA, Graph-FCA, communities and embeddings organize and
discover; they never decide whether a capability exists.

**Status: Proposed** (a document; no ADR). Its research citations are unresolved placeholders
(`:chatgpt-content-reference{index=N}`), so every research claim in it is unsourced as written.
Appendix C gives verified sources.

**Affected revisions.**
- HEAD `7bebdd4`, plus uncommitted Phase 2 R2 fixes, which this review does not cover.
- Every measurement below is on the pilot snapshot `ec03626f63304c3c156dd87c727a9ead` (compiled
  2026-09-24, no optional techniques), read with `lctx query` (read-only).

**Observable outcome, restated.** An agent can ask behavioral questions about a pinned library's
**whole** public surface: which operations accept X, pass it to Y, under which configuration,
raising what, needing which lifecycle. It gets:
- exhaustive answers where the analysis is complete;
- ranked candidates where only discovery applies;
- a named unknown where the analysis stopped.

Each claim carries its evidence and derivation.

**Baseline (Measured, 2026-09-24).** The bottleneck is in the analysis, not only in presentation.

| Quantity | Pilot value | Source |
|---|---|---|
| Public declaration nodes | **1,534**: 740 functions, 431 async functions, 363 classes, reached by 4,763 public paths | `public_paths` |
| Briefs | **20** | `briefs` |
| Subjects of any Pass A–C finding | **20**, the seeds; analysis loops over seeds (`crates/cpg-core/src/analyze.rs:1117`, `for name in &seeds`) | `findings`, kinds 1–10 |
| Arguments mapped to a formal by the whole-release `argument_flows` relation, never persisted (`flows.rs:214`, collected at `analyze.rs:1171`) | **32,075 of 74,712**: parameter 3,746, alias 1, literal 12,964, **other (computed, never followed) 15,364** | `lctx query` over `argument_flows_sql()` |
| Guards (supported predicate plus a direct `raise`) | **79**, against **516** release `if`s that raise directly in their body | `guards_sql()`; `syntax_nodes` |
| Parameter reads at call arguments | 5,449, of which 1,386 inside an expression | `parameter_reads_sql()` |
| Release statements | 3,805 `if`, 800 `raise`, 490 `try`, 316 `with`, 1,145 `await`, 81 `yield`, 14,590 attribute expressions | `syntax_nodes`, role `release` |

**Supported scope and non-goals.** This review judges three things: whether the pivot's target is
*specifiable and decidable* against the charter; what the target design should be; and which
libraries fit. There is no implementation to judge. These non-goals stand:
- analysis of the caller's own codebase;
- comparison across library versions;
- a generative model in the pipeline or the query path (§B11);
- API-agent evaluation (operator, 2026-09-23).

**Constraints.**
- A single operator (ADR-0001).
- Programmatic only.
- Evaluation is structured and qualitative: representative queries with targets written before
  any output is read.
- Licence is never a criterion.
- One GPU, shared with vLLM.
- The pilot is FastMCP 4.0.5.

**What the pivot would supersede.** The pivot says only that it supersedes "deliberate scope
decisions" (F1). The review's list:

| Section | What changes |
|---|---|
| §1.1 | The product. L38 reads "All interpretation is precomputed at compile time. Nothing is synthesized, traversed or generated during a request." |
| §1.3 | Three non-goals: query-time traversal, a domain-capability ontology, and Python CFG/dataflow/alias analysis |
| §B5 | CFG and dataflow move from deferred to core |
| §B13 | The server would compute over more than briefs. L373 reads "It reads files only" |
| §B14 | Embedding views |
| §9 | The rule "a **named consumer** in the brief" (L1820) |
| §9.8 and ADR-0020 | The keep rule, which is judged on brief retrieval |
| §12 | Evaluation |
| §B10 | The constraint-solver exclusion is put at risk (F6) |

### Method and coverage

**Read in full:**
- the pivot;
- DESIGN §1, §2, §3.1–§3.7, §5, §6.4, §9 (opening, §9.1–§9.3, §9.6–§9.8), §10 (opening), §11,
  §12 and §13;
- ADR-0005, and the front matter of ADR-0004;
- the charter's review framework (gates, §C–§H), the ADDENDUM, REVIEW_REFERENCE, and §6–§8 of the
  operator's graph guidelines.

**Code read myself:**
- `analyze.rs`: the seed loop and flow collection;
- `cpg-schema/src/flows.rs`: the module contract and fragments;
- `concepts.rs`: the attribute SQL;
- `lctx-analytics` `neighbours::api_text` and `selection::RULE`;
- `libraries/fastmcp/analytics.toml`: the brief budget of 20;
- `python/lctx_mcp`: the package layout and dependencies.

**Measured myself.** The baseline table above.

**Probes run myself** (scratchpad only; nothing in the repo):
1. **`ty_python_core` 0.0.14 as a standalone semantic index:** passed (§4, D-2).
2. **LanceDB 0.39.0, the same table written twice:** 1 of 10 files byte-identical (§8.3).
3. **FastMCP prevalence** (a stdlib-`ast` script; nothing imported): re-ran it and read its output.

**Source read myself:**
- `ruff_python_semantic` `cfg/graph.rs` at 0.0.11 and 0.0.14;
- `ty_python_core` 0.0.14: `lib.rs`, `use_def.rs`, `program.rs`, `db.rs`, and `builder.rs`
  (`try`, `if`);
- `ruff_linter` 0.16.8 `lib.rs`;
- Pyrefly's `Key` enum and `Bindings` accessors;
- FastMCP `server/mixins/transport.py` (`run_stdio_async`, the stateless/SSE checks, the SSE
  branch), `server/server.py` (459–464, 1962) and `settings.py` (46–70, 270, 299).

**Delegated.** Six research agents ran on 2026-09-24. Their output is treated as leads; everything
cited below was spot-checked, or is marked as the agent's.

| Agent | What it covered | What I checked |
|---|---|---|
| Pivot claims | The pivot's 14 factual claims | The key citations |
| Pyrefly/Ruff internals | Flow structures in both front ends | Pyrefly's `Key` enum and accessors; the ruff CFG stub |
| Crates | The crate survey, with probes of ascent 0.8.1, DataFusion 55.1 recursive CTEs, DuckDB 1.5.5 and datafusion-python 54.0.0 | Not re-run by me; marked "agent probe" |
| Literature | Research precedents | Not re-read by me; Appendix C keeps its opened/metadata marks |
| FastMCP grounding | 12 behavioral questions with ground truth | Five claims, and the prevalence counts |
| Ontology/NLP tooling | Vocabulary, discovery and NLP options | Its recommendations are marked as such |

**Not done:**
- `just test-all`. This is a document review, and the tree holds uncommitted R2 work whose
  `rules_snapshot` is unaccepted.
- Any spike of summaries or of a query executor.
- Running Graph-FCA.
- Measuring the benefit of any embedding view. Every claim about views is a hypothesis.

**Guarantees not attacked:** the pivot's unquantified performance statements; the adequacy of
Qwen3-Embedding-8B for code views (MTEB(Code) numbers only).

---

## 2. Authority and lifecycle map

Reconstructed from the pivot. **⟂ marks a cell the pivot leaves undecided** (the review had to
invent it); the review's recommendation follows in italics.

| Concept | Semantic type and identity | Authority | Revision boundary | Update path | Derived representations |
|---|---|---|---|---|---|
| Code facts | Existing families; node and fact ids (§3.4.1) | Extractor runs, in Delta | Snapshot | Extraction only | Catalogs, projections |
| Flow IR: def-use, conditions, exceptional exits, access paths | ⟂ *a CPG-constructing family `flow`; ids anchored on syntax and producer-scoped, like syntax ids* | ⟂ *one producer surface: our CFG pass or the `ty` index, never both* | Snapshot | Extraction | Behavior relations |
| Behavior relations: argument→formal, parameter→return, field writes and reads, guards, raises, effects | ⟂ | ⟂ *Stage C/D derivations (declared SQL, as `flows` is today), persisted* | Snapshot | Derivation | Summaries, concept membership, the facts embedding view |
| Transfer summaries | "a formal transfer relation" (pivot §3.2); identity ⟂; condition ⟂ | ⟂ *a Stage E kernel* | ⟂ *snapshot + `compiler_digest` + models digest* | Recomputation | Concept membership |
| External semantic contracts: "small versioned contracts for important external primitives" | Format ⟂ | ⟂ *committed data files, digested* | ⟂ | ⟂ *operator-authored and reviewed; append-only ids, like codebooks* | Summaries |
| Capability concepts (executable definitions) | "typed predicates and compositional concepts"; representation ⟂ | "the ontology registry" (§8); location ⟂ *`cpg-schema`* | ⟂ | ⟂ | Membership, facets, lookup |
| Vocabulary (SKOS-like labels) | Identity ⟂ | ⟂ *the same registry* | ⟂ | ⟂ | Lexical and vector lookup |
| Concept occurrences: `capability_occurrence(operation, concept, configuration_condition, behavior_context, evidence)` | Shape stated; identity ⟂; condition normal form ⟂ | ⟂ *materialized at compile time* | ⟂ | Recomputation | Query answers |
| Discovery projections: FCA/RCA, communities, per-view vectors | Existing (§9.4–§9.7) plus new views ⟂ | Statistical, rebuildable | Snapshot | Recomputation | Ranking only |
| Query requests and results | An illustrative JSON algebra | ⟂ *Rust types → JSON Schema → pydantic* | Per request | — | Explanations |
| Briefs | "an optional presentation format" | Derived from ⟂ | Generation | Rendering | — |
| Evaluation targets | The pivot's §9 query families | ⟂ *pre-registered, committed before any output is read* | — | — | — |

**Deliberately opaque behavior.** The pivot keeps native bodies and unresolved boundaries as
explicit unknowns. That is right, but it frames external contracts as a native-code concern. On the
pilot, the boundaries that decide behavior are ordinary Python dependencies: the MCP SDK, pydantic
and pydantic-settings, anyio, starlette and uvicorn, `uncalled_for` (Appendix B). Which of these
are analyzed and which are modelled is undecided.

**Identity behavior.** A behavior record's identity is undecided (DM-15). If it derives from
anything but the canonical subject, roles and the condition's normal form, reordering a conjunct
renames the record, and `lctx diff` reports churn.

---

## 3. Semantic contracts and invariants

The pivot's load-bearing sentences, sorted by kind:

| Contract (the pivot's words) | Kind | Enforcement boundary | Failure behavior | Evidence |
|---|---|---|---|---|
| "Embeddings may nominate or rank candidates. They must not allow a candidate to bypass an unsatisfied structural requirement." (§6) | Specification | ⟂ | ⟂ | Proposed. The existing analogue is §10's kind policy: statistical support never states a control or limit |
| "An exact structured query should search the full eligible entity universe … must not begin with a vector top-k shortlist and then claim completeness." (§7) | Specification | ⟂ | ⟂ | Proposed |
| "Community membership should neither establish a behavioral property nor limit which operations an exact query can find." (§5) | Specification | Partly exists: ADR-0005's statistical-output rule, enforced by the assertion policy | — | Implemented for briefs |
| "Frequent concepts must not define the searchable universe." (§5) | Specification | The universe is `public_paths` | — | Implemented (`public_paths`, with its rules) |
| "Configuration … part of capability identity"; "it must be the compressed JSON payload that reaches that sink, under one compatible configuration" (§4) | Intention | ⟂ no condition language, ⟂ no compatibility decision | ⟂ | Proposed; F6 |
| "Do not propagate capabilities merely because one function calls another." (§3.2) | A prohibition; the transfer semantics are ⟂ | ⟂ | ⟂ | Proposed; F4 |
| "An absent extracted attribute must not silently become a claim that the runtime property is false." (§5) | Specification | Exists for facts (§3.7 coverage and boundaries); ⟂ for derived behavior | ⟂ | F3 |
| "Every result should distinguish: what was established; under what conditions; how complete the answer is." (§7) | Intention | ⟂ no vocabulary | ⟂ | F3 |
| "Recursive functions then have defined fixed-point behavior." (§3.2) | Assumption: a finite, monotone domain | ⟂ no lattice, no widening | ⟂ | Proposed. Pysa needs "broadening" of access-path trees to terminate (Appendix C, item 3) |
| "This sequence delivers useful intelligence before requiring a comprehensive ontology." (§8) | Benefit assertion | — | — | A hypothesis (DM-39, DM-59) |

**The absence and uncertainty lattice (DM-08).** The pivot imports Semantic Code Browsing's
checked / false / unresolved statuses. That paper's "false" is safe only because its analysis
soundly over-approximates a logic language (Appendix C, item 1). Python analysis here is neither
sound nor complete, so a refutation needs an explicit completeness premise. The review proposes
five verdicts per (subject, predicate), mapped onto existing codebooks and never a null:

| Verdict | When | Existing codebook |
|---|---|---|
| `established` | Derived under the stated model, with no boundary inside the region the predicate reads | — |
| `conditional` | Established only under a stated condition (a condition id) | — |
| `refuted_under_model` | Only when the region is `complete_under_stated_model` **and** holds no boundary of the kinds the predicate names | `coverage_status` |
| `unknown` | A boundary intervenes; its reason is named | `boundary_reason`, e.g. `unresolved_target`, `unsupported_unpacking`, `budget_reached` |
| `not_analyzed` | Out of scope, not requested, or cut by a budget | `coverage_status` `not_requested` or `partial` |

Discovery results carry none of these. They are `statistically_derived` nominations.

**Equivalence.** Two conditions are equal when their normal forms are equal: sorted conjuncts of
normalized atoms. That is syntactic equivalence, deliberately not semantic, and it should be
declared as such.

---

## 4. Derivation and execution design

The target stage graph. New stages are marked ★.

| Stage | Inputs | Output contract | Assumptions | Effects | Provenance and invalidation |
|---|---|---|---|---|---|
| A–D (exists) | Release, corpus | CPG families, catalogs | — | Delta attempt | Run ids |
| ★C′ flow IR | `syntax`, `lexical`, `calls`, `types` (+ the `ty` index if chosen) | `flow_*` tables | The stated abstraction (§B5): a statement-level CFG with exceptional exits; `TYPE_CHECKING` read the **runtime** way | Attempt | A producer surface (`lctx-flow` or `ty-semantic-index`) whose revision is in the run id |
| ★D′ behavior relations | CPG + flow IR | `behavior_*` tables: today's `flows` relations persisted and generalized | — | Attempt | `relational_derivation`; one rule per invariant |
| ★E1 summaries | Behavior relations, call-graph SCCs, models | `summary_*` tables | A finite domain; widening to `unknown` with a named reason | Attempt | Invocation rows; budgets recorded |
| ★E2 concept membership | Summaries, relations, the registry | `concept_members`, `conditions` | Closed world only inside complete coverage (§3) | Attempt | Definition digest |
| E3 discovery (existing variants + views) | The above + embeddings | Statistical findings | — | E0 embedding calls (an existing, declared effect) | Spec hash + view id |
| F rendering (existing, now optional) | The above | Briefs for a curated subset | — | — | — |
| Publish (exists) | — | The `snapshots` append | — | — | — |
| ★Bundle `FORMAT` 3 | The published snapshot | Operations, behaviors, conditions, concepts, members, vocabulary, per-view vectors (Arrow IPC) | — | Files | Byte-identical rebuild |
| ★Serve | The bundle | Typed query results | Bounded execution (F11) | The query-embedding call (existing) | Generation key |

**Relationship structures (DM-34).** These must stay distinct:
- control flow (within a function);
- def-use (value);
- calls (between functions);
- summary transfer (derived);
- concept membership (classification);
- vocabulary `broader`/`related` (terminology);
- community and kNN affinity (statistical).

The pivot's diagram feeds FCA/RCA, graph analysis and embeddings as one tier into one executor.
That is acceptable only if the executor keeps their evidence classes apart (F7).

**Provider selection for the flow IR** (the operator's question). Every row was read or probed on
2026-09-24.

| Candidate | What it gives | Why it does or doesn't fit |
|---|---|---|
| **Pyrefly's binding graph** (pinned fork, in process) | Flow-sensitive `BoundName → Forward → Narrow/Phi → Definition`; per-site `self.x` writes (`FacetAssign`); narrowing ops; all reachable today with no new patch (agent report; `Key` enum and the `Bindings` accessors read myself) | It is an IR for type inference. It "drops statically decided branches" (DESIGN §B1, L193), gives handlers only the pre-`try` flow and no exceptional edges, and discards control-flow state after binding (agent report). **Use it as a parity oracle only** |
| **`ruff_python_semantic`**, 0.0.11 and the newest 0.0.14 | A `cfg` module and `SemanticModel` | **A stub, unchanged at 0.0.14:** `process_stmts` has empty arms for `While`, `For`, `If`, `Match`, `Try`, `With`, `Break` and `Continue` (read at both versions). `SemanticModel` is flow-insensitive (latest binding) and needs `ruff_linter`'s checker to drive it; that module is private (`mod checkers;`, `ruff_linter` 0.16.8 `lib.rs:22`). **Not useful** |
| **`ty_python_core` 0.0.14** (Astral's ty semantic index; same release train as ruff 0.0.14) | A public `UseDefMap`: `bindings_at_use` with each binding's reachability constraint, `range_reachability`, `predicates`, narrowing constraints, and member places (`self.x`) | **Probed standalone this session:** a minimal salsa database, an in-memory module, 175 crates, a 12.6 MB release binary, 0.23 ms on the sample. The use of `y` after a `try` reaches both the `try`-body and the handler assignments. After `with open(x) as fh: data = …`, `data` may be unbound (a suppressed exception). Both `sys.version_info` branches are kept, each under its own symbolic constraint. **But** it decides `TYPE_CHECKING` the type checker's way: the `else` branch is `AlwaysFalse`, the opposite of runtime. A member place read in another method has no binding, so cross-method field flow stays ours. It needs `salsa` pinned **exactly** at 0.28.2: 0.28.3 and 0.28.4 break ruff 0.0.14 within a patch release (both failures hit in the probe) |
| **Our own CFG and SSA over the 0.0.11 AST** | Full control; it reuses `walk.rs` placement and the lexical recognizer | Medium-to-large effort. Python's flow semantics (`try`/`finally`, `with`, `match`, loop `else`, comprehension scopes, walrus, `global`/`nonlocal`, `del`) are exactly what ty's builder already encodes and tests |

**Boundary contracts.**
- Rust compile → Arrow IPC bundle → Python server. §B13's rule (no compiler code in the server)
  survives only if predicate semantics are **never** re-implemented in Python (F12).
- If the `ty` provider is chosen, no ty or ruff 0.0.14 type crosses into `cpg-extract`: only byte
  ranges and our ids.
- Both parsers read the same bytes, so byte offsets agree (§3.4). A parity rule has to prove that
  every range maps to exactly one `syntax_nodes` row, and name the residue.

**Coherent publication.** Unchanged: the `snapshots` append, then the bundle. A Lance index, if
ever adopted, sits outside the byte-identical manifest (§8.3, probe 2).

---

## 5. Representative journeys

**Ordinary extension: add the concept "configurable timeout".**
- Under the pivot, where it is written and how it is checked are undecided.
- Under the target (§8.2), it is one declaration in the registry:
  - an id, labels and a scope note;
  - a rule over `summary_flows`: a public operation's parameter reaches the `delay` role of a
    modelled timeout primitive (`anyio.fail_after`, `anyio.move_on_after`, httpx `Timeout`);
  - one known-answer case in `behavior_shapes` and one injected violation.
- Nothing changes in Python, because the server serves concepts generically. The locality is good.

**Meaningful change: a model contract is edited.** For example, `anyio.fail_after` gains its
`TimeoutError` effect.
1. The models digest enters `compiler_digest` (F8), so the run id changes.
2. Summaries and membership are recomputed.
3. Records whose content is unchanged keep their ids: content ids with no config digest, as
   findings already work (§3.4.1).
4. `lctx diff` reports the concept members that changed.

The pivot says nothing about invalidation.

**Boundary: a condition crosses into Python.** A query asks for "compression = gzip". The member's
condition is `level is not None and mode == "gz"`, and compatibility has to be decided at serve
time.
- If Python re-implements condition semantics freehand, that is a second authority (F12).
- The target shares a known-answer corpus, `specs/serving/conditions.json`, run by both suites, as
  `specs/serving/tokens.json` already is for tokenization.

**Interruption or failure: a summary fixpoint over a 40-method SCC exceeds its budget.**
- The affected summary positions become `unknown`, with `budget_reached` (an existing
  `boundary_reason`), and the invocation is `partial`.
- A `find_operations` result touching those operations returns `complete = false` and lists them.
  It never returns a silent "no".
- The pivot says "explicit unknown results" but gives no mechanism.

**Adversarial journeys (deep), on the pilot's real code. All lines were read myself.**

a. **Accepted but unused.**
   - `run_stdio_async(stateless=…)` (`server/mixins/transport.py:215-255`) uses `stateless` only in
     `mode = " (stateless)" if stateless else ""`, which feeds `logger.info(...)`. It is never
     passed to `_mcp_server.run`, although the docstring promises "no session initialization".
   - Without an effect model that classifies `logger.info` as a logging sink, a value-flow analysis
     reports `stateless` as *forwarded*, which is wrong in the unhelpful direction.
   - **Framework and stdlib models are what make this answerable (F4).**

b. **"Never read" is not refutable.**
   - `server_dependencies` and `mounted_components_raise_on_load_error` are declared in
     `settings.py` (L270 and L299) and read by no attribute load in the package.
   - But `Settings.get_setting(attr)` reads by string (`settings.py:46-58`, `getattr(settings,
     attr)`).
   - Under §3's lattice the verdict is `unknown` (a dynamic-access boundary), not
     `refuted_under_model`. The pivot's status scheme has no rule that yields this (F3).

c. **Incompatible modes.**
   - `stateless_http` together with `transport == "sse"` raises `ValueError`
     (`transport.py:307-309`).
   - The SSE branch of `http_app` passes only `message_path`, `sse_path`, `auth`, `debug` and
     `middleware` (`transport.py:459-467`), so every other HTTP option given to it is dropped.
   - Both need conditions as data **and** a compatibility decision (F6).

d. **A wrapper does not pass a configuration on.**
   - `FastMCP.tool` applies the server's `tasks=` default:
     `task=task if task is not None else self._support_tasks_by_default` (`server.py:1962`).
   - `FastMCP(tools=[fn])` calls `Tool.from_function(tool)` with no task (`server.py:459-464`), so
     the default does not reach those tools.
   - Answering "does `tasks=True` apply to every tool?" needs field-sensitive flow through
     `self._support_tasks_by_default` plus interprocedural summaries (F4).

e. **A rare public API.** 1,514 of the 1,534 public nodes have no behavioral finding today. Under
   the target, every public node has a catalog row and behavior rows, and the universe is
   `public_paths`.

f. **Dynamic boundaries.** Across the package: 101 `getattr`, 46 `hasattr`, 5 `setattr`,
   9 `importlib.import_module`, 6 module-level `__getattr__` and 264 function-local imports
   (prevalence script, re-run). Each is a boundary the verdict lattice must name.

---

## 6. Acceptance gates

| Gate | Result | Evidence or scope rationale | Required action |
|---|---|---|---|
| G1 — Authority | **Unresolved** | The pivot names neither the binding decisions it supersedes (F1) nor the single authority for concepts among definitions, labels and discovered concepts (F7). Predicate semantics would be evaluated in two languages (F12) | The Stage 0 ADR set; one registry; membership materialized at compile time |
| G2 — Semantic fidelity | **Unresolved** | No verdict or absence lattice for behavioral answers (F3). The obvious flow-IR providers each carry a type-checker view that would be relabelled as runtime flow (`ty`: `TYPE_CHECKING`; Pyrefly: pruned branches) (F5) | The five-state lattice; a declared runtime view; parity tests |
| G3 — Validity | **Unresolved** | The query algebra's validation boundary, operator set, caps and error contract are unspecified (F11) | Typed pydantic inputs generated from Rust types; bounded operators |
| G4 — Hidden behavior | **Unresolved** | New inputs (models, vocabulary, the `ty` revision and its vendored typeshed, view templates) are not declared in run identity (F8). Serve-time execution cost is unowned (F11). The gold could become a hidden input to models or concepts (F14) | Digests in `compiler_digest`; a rule keeping skill paths out of models and concepts |
| G5 — Consistency and recovery | **Pass** | The pivot keeps immutable published generations read by the executor (§8 of the pivot), which inherits §B7, §B12 and one generation per process. Derived search indexes (FTS, any Lance index) are keyed by the generation key and rebuilt from its Arrow files, so they do not compete | Keep derived indexes outside the byte-identical manifest |
| G6 — Transformation and reuse | **Unresolved** | Multi-view embeddings against the one-spec rule (F9); summary and membership invalidation keys (F8) | View identity in the input key; data-file digests |
| G7 — Truthful capability claims | **Unresolved** | The flagship query (json + gzip to a caller sink under "compatible_behavior_context") has no implementation route in the design: the channels that carry FastMCP's behavior are absent (F4), and compatibility needs a decision procedure §B10 excludes (F6). Unresolved rather than failed, because the pivot calls its JSON "a proposed interface, not an existing repository API" | Scope each stage's claims to what its layers support (§8.2) |

---

## 7. Principle findings

Ranked: correctness and authority first, then semantic duplication and extension, then cost.

| # | Finding | Principle IDs | Concrete evidence or gap | Consequence | Proposed correction | Verification |
|---|---|---|---|---|---|---|
| F1 | The pivot is a scope change that does not name the binding decisions it supersedes. The in-flight plan would delete techniques the pivot re-purposes | DM-02, DM-13, DM-59 · G1 | Pivot §1: "deliberate scope decisions that your new objective now supersedes", with none listed. DESIGN §1.1 L38; §1.3 L82–86; §B13 L373; §9 L1820 ("a **named consumer** in the brief"); §9.8's keep rule. The holistic plan's Phase 4 exit: "a technique that fails its rule is deleted, together with its tests" | Phase 4 would delete FCA, RCA and kNN code by a brief-retrieval rule, while the pivot needs FCA over behavioral attributes and communities for navigation. Work proceeds against two contradictory product definitions | **Stage 0:** supersede ADR-0004 (product); amend ADR-0005 (§B10), ADR-0010 (§B13, §B14), ADR-0020 (the keep rule becomes per-consumer) and ADR-0011 (§B4: dominators, summaries over SCCs); amend DESIGN in the same commits; a `standard` review. Pause Phase 4's deletion exit and Phase 5's B3/B6 | `just adr lint` for the records. **No mechanical oracle** exists for "the decision precedes the code"; that is prose |
| F2 | Behavioral analysis runs per seed, and the whole-release relations it reads are never persisted, so discoverability is fixed by the brief budget | DM-10, DM-23, DM-58 · G7 | Measured: Pass A–C subjects **20 of 1,534**; the loop over seeds at `analyze.rs:1117`; `argument_flows_sql` computes 32,075 rows and `collect`s them only into the session (`analyze.rs:1171-1189`); `briefs.budget = 20` (`analytics.toml:139`) | "Which public operations forward `timeout`?" can be answered for 20 operations; for the other 1,514 the answer is silence, which reads as "none" | **Stage 1:** persist `argument_flows`, `guards`, `parameter_reads` and `handoffs` as a `behavior` family (declared, validated, digested; they already are declared SQL). Compute per callable rather than per seed; seed traversal becomes a query | **Test + rule:** every public callable has a `coverage` row for `behavior` (`semantic:behavior-covers-public`, with an injected case) |
| F3 | No verdict or absence lattice for behavioral answers; negative claims have no completeness premise | DM-08, DM-42, DM-59 · G2 | Pivot §7 "how complete the answer is" and §9 "honest negative or unknown answers" state intent only. `Settings.get_setting(attr)` (`settings.py:46-58`) makes "never read" unprovable. Pass B already declines some cases without a trace (agent report: `pass_b.rs:439-443`, `:480`, `:490`) | A query for "accepted but unused settings" returns `server_dependencies` as a fact. It is a false negative claim whenever a caller uses `get_setting("server_dependencies")` | The five verdicts of §3, mapped onto `coverage_status` and `boundary_reason`. `refuted_under_model` only inside `complete_under_stated_model` regions with no named boundary | **Rule** `semantic:refuted-needs-complete-region` with an injected case, plus a `behavior_shapes` case for dynamic access |
| F4 | The IR the pivot lists (§3.1) omits the channels that carry the pilot's behavior, so most representative questions stay out of reach | DM-43, DM-44, DM-59 · G7 | Prevalence, re-run: 324 `self.x = <param>` in `__init__` across 153 classes, 273 of them read in another method of the same class; 136 more derived from a parameter; 95 pydantic classes (402 fields) and 52 dataclasses (250 fields); 39 `fastmcp.settings.<x>` reads; 15 ContextVars; 11 function-object attribute writes (6 are `__fastmcp__` metadata); 29 registry writes; 85 of 462 handlers raise a different type. The grounding: §3.1 + §3.2 answer **5 of 12** questions (Appendix B) | Q1 (`tool` options), Q3 (`tasks=` / strict validation), Q4 (settings), Q5 (error masking) and Q12 (dispatch) are unanswerable; the pivot's example queries cannot be met on the pilot | Access paths rooted at parameters, `self` fields (depth k ≤ 2), module globals including settings singletons, ContextVar objects and function-object attributes; a **models** catalog covering stdlib, dependency and framework primitives (§8.2 L4) | **Test:** a `fixtures/python/behavior_shapes` package with a known answer for each channel |
| F5 | The flow-IR substrate is unchosen, and each ready-made provider brings a type-checker view that would be relabelled as runtime flow | DM-24, DM-13 · G2; §B5 | §B5: "Pyrefly's inference graph is never relabelled as runtime dataflow". Pyrefly's IR drops decided branches (§B1, L193). **Probe:** ty's index marks `else` of `if TYPE_CHECKING:` `AlwaysFalse`, so `T` after `if TYPE_CHECKING: T = int / else: T = str` reaches only `T = int`, while at runtime `T` is `str`. The ruff CFG is a stub (§4) | A behavior drawn from type-only imports or annotations under `TYPE_CHECKING` is published as runtime behavior. Or the reverse: runtime-only definitions vanish | **D-2 and D-3:** choose the provider (a ty spike with exit criteria) and declare the runtime view: `TYPE_CHECKING` is false; version and platform follow the analyzed context. Our C3 static-branch marks already identify those branches | **Test:** a `flow_shapes` fixture with a `TYPE_CHECKING` case; a parity test (every reaching definition from the provider is among our flow-insensitive `reference_resolutions` candidates) |
| F6 | Conditional records need a condition language and a compatibility decision; in general that is a solver, which §B10 excludes | DM-06, DM-15, DM-43 · G7 | Pivot §4 `configuration_condition` and §7 `"compatible_behavior_context": true`; §B10 L333 excludes "a general composition planner or constraint solver". Pilot modes: `transport.py:307-309`, `:459-467` | Either a solver slips in unreviewed, or compatibility is decided ad hoc and SSE + stateless is reported as a compatible combination | A **closed** condition language: conjunctions of atoms over access paths (`is None`, `is not None`, `== literal`, `in {literals}`, truthiness, `isinstance(C)`), anything else an opaque atom. A normal form, and three-valued compatibility (compatible / incompatible / unknown) by a small finite-domain evaluator. No solver | **Test:** `specs/serving/conditions.json` known answers run by both suites |
| F7 | Concept authority is split between executable definitions, vocabulary labels and discovered concepts | DM-02, DM-23 · G1 | Pivot §4 ("a modest, versioned catalog of semantic primitives and executable definitions"), §5 (FCA and RCA "discover concepts"), §6 (embedding lookup), SKOS labels. Nothing says which one decides membership | An FCA concept and a defined concept with the same label return different operation sets depending on the query route | One registry in `cpg-schema`. A concept is a typed rule plus labels. Discovered concepts are **candidates** (never members), promoted only by an operator review verdict (ADR-0019 facts) | **Rule:** every `concept_members` row cites a definition digest and a witness. **Test:** FCA output never writes `concept_members` |
| F8 | New semantic inputs are missing from run identity | DM-31, DM-32 · G4, G6 | Pivot §9 lists what to record (intent only). `compiler_digest` hashes every `.rs` file of three crates (§3.4.1, L583), not data files. A `ty` provider adds its revision, vendored typeshed and program settings | Editing a model or vocabulary TOML changes behavior, but not the run or producer id. `lctx diff` then compares snapshots that claim the same compiler; any reuse keyed on the digest is wrong | Hash every models, vocabulary and concept file into `compiler_digest`, and the flow provider's revision and settings into its `producer_id` / `context_id` | **Test**, like `every_compiler_source_is_hashed`: changing any registered data file changes the digest |
| F9 | Multi-view embeddings conflict with the one-spec rule and have no view identity | DM-31, DM-32 · G6; §B14 | §6.4 L1691: "One generation never mixes vector spaces … `semantic:one-embedding-spec`". §11.1: the spec hashes the "document template". Pivot §6 has four views | Either one spec per view (breaking the rule) or one spec whose document template no longer describes the texts, so `spec_hash` stops identifying how a vector was made | The spec is the model (one vector space). A view template id and version go into `input_hash` and are a column of `vectors`; the rule holds per model | **Test:** two views under one spec get distinct keys and view ids; a second model is refused |
| F10 | Seven new layers, including research-grade methods, with no consumer or exit criterion per layer | DM-57, DM-58 | Pivot §2's diagram and its §8 ordering. Graph-FCA: pattern products blow up with arity, and the tool was evaluated on tiny data (Appendix C, item 8). RCA's fixed point must be chosen explicitly (Euzenat 2025). No layer has an exit test | Graph-FCA or on-demand RCA is built before summaries exist, and ADR-0020 repeats: techniques that change output without improving the target | Staging with an exit criterion per stage from the structured evaluation (§8.2). Graph-FCA offline and deferred. RCA only over behavioral contexts, least fixed point, declared | **None mechanical:** the structured evaluation packet per stage (prose). Say so |
| F11 | Serve-time query execution is unbounded and unowned | DM-28, DM-37, DM-39 · G3, G4 | Pivot §7 ("bounded paths", "aggregates and facets") and §8 ("read immutable published generations") name no engine, bounds, recursion policy, parameter binding or result caps. Agent probe: DuckDB 1.5.5 accepts non-linear recursion and returned 8 of 12 closure rows. DataFusion 55.1 supports linear recursion only | A path pattern returns a partial closure presented as complete, or a runaway query stalls the stdio server | No recursion at serve time: paths are precomputed as summaries and witnesses. Typed bounded operators, row caps with `truncated`, cursors, bound parameters. **D-4** settles the engine | **Tests** with adversarial queries (caps, empty sets, unknown concepts). An **ast-grep rule** against f-string or `%` SQL in `python/lctx_mcp` |
| F12 | Predicate semantics would acquire a second implementation in Python | DM-02, DM-53 · G1 | §B13: a files-only Python server. Pivot §7: "ontology terms compile into explicit predicates and joins", at query time | Rust (DataFusion) and Python judge "forwarded under condition" differently on an edge case, so compile-time renderings and serve-time answers disagree | Materialize predicates and membership at compile time. Serve time does only generic selection and joins, plus condition compatibility held to shared known answers | **Test:** the shared corpus (F6) run in both suites |
| F13 | The research framing is unsourced and partly overstated | DM-59 | Placeholder citations. "Semantic Code Browsing" is called the closest match, but its "false" rests on soundness. Conditional transfer relations have no industrial precedent (CodeQL rows, Pysa TITO and Joern semantics have no condition column). SCC worklists need broadening to terminate. Pysa, which runs on our own front end (it now takes its types from Pyrefly), is missing (Appendix C) | Decisions cite unverifiable authority, and F3 inherits a refutation rule without its premise | Appendix C as the source list; label conditional summaries **Proposed**, starting with the narrow condition language of F6 | **None mechanical** (prose) |
| F14 | The evaluation plan does not pre-register, and does not keep the gold out of the new authored inputs | DM-28, DM-59 · G4; ADDENDUM Q15 | Pivot §9 ("keep an independent held-out set") does not say targets come first, nor that models and concepts must not be authored from `.claude/skills/` | Concepts or models shaped to the gold's families make the structured evaluation circular | Targets committed before any output is read (the operator's rule); models and concepts cite library source and docs only; `eval/heldout/` stays sealed | **Test or ast-grep rule:** no models, vocabulary or concept file names a `.claude/skills` path. **Recipe:** the pre-registration commit precedes scoring commits |

**Observations** (not findings):
- **O1.** The pivot's factual claims about the repository are accurate, with three corrections
  (Appendix A):
  - the exclusions it cites are §1.3's stage-1 non-goals, not §B10; CFG and dataflow are
    *deferred* with a trigger (§13);
  - all eight optional techniques are off by default, not "several";
  - Passes A and B run over library code, and only Pass C over official usage.
- **O2.** Lance datasets are not byte-reproducible: 1 of 10 files was identical across two
  identical writes (probe).
- **O3.** The ty and ruff 0.0.14 line needs an exact salsa 0.28.2 pin, enforced by the lockfile
  (probe).

**Applicability.**
- **Groups that carried the findings:**
  - 1: authority and boundary;
  - 2: types, absence and relationships;
  - 5: derivation and preservation;
  - 7: dependencies and reuse;
  - 9: providers and capabilities;
  - 10: provenance;
  - 12: proportionality.
- **Group 3 (identity)** bore on conditions and records (§2, F6).
- **Group 11 (evolution)** bore through the codebook-like models and vocabulary.
- **Groups that did not bear:**
  - Group 6 (mutable workspaces): the attempt and publication protocol is unchanged.
  - Group 8 (layouts, performance): nothing is measured yet; performance is a §9 plan item,
    not a finding.
  - Group 4 (declarative composition): it bears only through F7's registry, not as a separate
    finding.

---

## 8. Alternatives and architectural leverage

### 8.1 The options

| Alternative | Semantic duplication and extension locality | Correctness and operational risks | Implementation / maintenance cost | Performance evidence | Why selected or rejected |
|---|---|---|---|---|---|
| **Current baseline** (the capability compiler) | Good within briefs | Seed-scoped: 1.3% of public nodes carry behavior; sentences are the only served form | Low, already built | Pilot 34.7 s, peak 3,877 MiB; "analyze: Pass B" 1.03 s for 20 seeds, including the relations' collection (Measured, `build/pilot-run.out`, 2026-09-24, fake embedder) | Rejected for the new objective: it cannot answer questions about the other 1,514 operations |
| **The pivot as written** | Undecided: three concept authorities and two predicate implementations (F7, F12) | G1–G4, G6 and G7 unresolved | High: seven layers, several research-grade | None | Rejected as a design; **kept as the direction** |
| **Simpler viable alternative:** Stage 1 only. Persist the existing whole-release relations as a `behavior` family, run the passes per public callable, publish an operation catalog, and add `find_operations` / `get_operation` over fixed facets (parameters, types, raises, decorators, async, delegation, forwarding, handoffs), plus a source-body embedding view | One declaration per facet; nothing new in Python | Small: every relation already exists and is validated | Low: 2–4 slices | Relations already computed whole-release at pilot scale (32k flows) | **Adopt as the first stage.** It removes the brief bottleneck at a fraction of the cost and tests the serving shape before the IR is built |
| **Recommended target** (§8.2): the simpler alternative, then flow IR, summaries with models, a concept registry, discovery as nomination, and a bounded serving algebra, staged with exit criteria | One authority per concept; predicates only in Rust | The gates settle as each stage lands | Medium-high, staged | To be measured per stage (§9) | **Selected** |

**Abstractions justified by current needs:**
- The `behavior` family, persisted and validated (F2).
- A flow IR, needed by 11 of 12 grounding questions (branch predicates) and by all 12 (def-use).
- Summaries and a models catalog (F4).
- A concept registry, because the pivot's value is concepts with membership criteria.

**What remains ordinary code:**
- the CFG, SSA and summary kernels (Rust worklists over petgraph SCC order; charter §F);
- the condition evaluator;
- the FCA/RCA kernels.

A Datalog engine is a spike, not a default: rules are few, and DataFusion already owns the
non-recursive relations.

### 8.2 The recommended target design

**Principles.** Each names what breaks without it.

| # | Principle | Without it |
|---|---|---|
| T1 | **The whole public surface.** The universe is `public_paths`; every analysis runs per callable, bottom-up; seed traversal becomes a query | F2: silence for 99% of operations |
| T2 | **Relations first, sentences last.** Every analysis output is a typed, persisted relation with in-row provenance (the ADR-0019 pattern); briefs are one rendering | Knowledge locked in templates |
| T3 | **Our abstraction, stated (§B5).** The flow IR is our declared runtime model, whichever provider builds it | F5: checker views relabelled as runtime flow |
| T4 | **Meaning from models; propagation from summaries.** Domain meaning is anchored in a versioned models catalog (effects and roles of stdlib, dependency and framework primitives). Summaries carry it with argument substitution, conditions and exception handling, never as "calls X, so can X" | F4, and the pivot's own prohibition |
| T5 | **Conditions are data in a closed language,** with three-valued compatibility and no solver | F6 |
| T6 | **Five verdicts, never a null.** A negative verdict only inside complete coverage | F3 |
| T7 | **Materialize at compile time; serve bounded selections.** All predicate and membership evaluation runs in DataFusion before publication | F11, F12 |
| T8 | **Discovery nominates; definitions decide** | F7 |
| T9 | **Evaluate against pre-registered behavioral question sets** (structured, qualitative) plus mechanical known-answer fixtures | F10, F14 |

**Layers.** Each lists its tables, producer, identity and first consumer.

**L1 — Flow IR.** A new CPG-constructing family `flow`. It builds graph facts, so it is in CPG
scope.
- **Tables:**
  - `flow_defs`: a place defined at a syntax site. A place is a local name, `self.f`, `self.f.g`
    (k ≤ 2), a module global, a ContextVar object, or an attribute of a function object.
  - `flow_uses`.
  - `flow_reaching`: use, reaching definition, condition id.
  - `flow_exits`: return, raise, yield or implicit; the value site; the exception type; the
    condition.
  - `conditions` and `condition_atoms`: normal form, with atoms anchored on syntax ids.
  - `flow_regions`: `try`, `with` and handler regions, with what each can absorb.
- **Provider (D-2).** The `ty_python_core` spike or our own builder. Either way:
  - the runtime view is declared, with `TYPE_CHECKING` false (D-3);
  - a range-parity rule is checked against `syntax_nodes`;
  - reaching definitions are a subset of our flow-insensitive `reference_resolutions`.
- **First consumer:** L2.

**L2 — Behavior relations.** Family `behavior`, derived as declared SQL, persisted.
- **Relations:**
  - `value_flows`: from a parameter, field, global, call result or literal; to a call's formal, a
    return, a field write, a raise or a yield; kind `value`, `transform` or `constant`; with the
    site and condition. This generalizes today's `argument_flows`.
  - `field_writes` and `field_reads`: class, field, method, source or use, condition.
  - `guards`: a condition controlling a `raise`, call or return site, from reachability per range
    or from post-dominators. This generalizes today's 79-row `guards`.
  - `raises`: type, condition, locally handled or not.
  - `handlers`: caught types and the action (re-raise, convert to a type, swallow, convert to a
    value).
  - `callbacks`: parameter, and whether it is stored in a field, invoked, forwarded or
    registered in a container.
  - `resources`: acquire, release, kind.
  - `ambient_reads`: a settings field, environment variable or ContextVar; site; condition.
- **Common columns:** every row has evidence syntax ids, `modality` and a condition id.
- **Coverage:** one row per public callable.
- **First consumer:** Stage 1's `get_operation`.

**L3 — Transfer summaries.** A Stage E kernel, `lctx_analytics::summaries`.
- **Tables:**
  - `summary_flows`: callable, input path, output path, kind, condition, verdict.
  - `summary_effects`: callable, effect kind from the models, role bindings, condition.
  - `summary_boundaries`: callable, reason, site.
- **Access-path grammar:** borrow CodeQL's models-as-data shape without its beta file format:
  `Parameter[name]`, `Parameter[self].Field[f]`, `ReturnValue`, `Argument[formal]@Call[target]`,
  `Global[fastmcp.settings.x]`, `Raise[T]`.
- **Algorithm:**
  - Bottom-up over the call graph's SCCs; `tarjan_scc` returns callees first.
  - Within an SCC, iterate to a fixpoint over a finite domain: path depth ≤ k and condition size
    ≤ c.
  - Widening goes to `unknown` with `budget_reached`.
  - An override-open call joins its candidates and is marked open when the candidate set is
    incomplete (§3.6 already records this).
- **Oracle:** Pysa's inferred TITO models on FastMCP, run offline. Pysa sits on our own front end.
  Differential, not truth.
- **First consumer:** Stage 3's questions (Q1, Q3, Q5, Q9).

**L4 — Models.** Committed data, versioned and digested (F8); origin `synthetic_model` (exists).
- **`models/external.toml`:** summaries and effects for stdlib and dependency callables in the L3
  grammar. Effects include `io.read`, `io.write`, `net`, `log`, `timeout`, `thread_dispatch`,
  `compress(format)`, `serialize(format)`, `validate(schema)`, `register(container)` and
  `invoke(callable)`.
- **`models/frameworks.toml`:**
  - pydantic `BaseModel`/`Field` (a generated `__init__`; a validation effect);
  - pydantic-settings (environment binding, prefix);
  - ContextVar `get`/`set`;
  - `functools.partial` and `functools.wraps`;
  - anyio and asyncio primitives;
  - `contextlib` managers;
  - logging sinks.
- **How it is authored:** by the operator, from library source and docs, never from the gold
  (F14), with append-only ids like codebooks.
- **First consumer:** L3. These are the pivot's "semantic primitives", made concrete as effects on
  access paths.

**L5 — Capability registry.** In `cpg-schema`, TOML, compiled to Arrow.
- **`concepts`:**
  - an append-only id, `prefLabel`, `altLabels` (each with its source), `broader`/`related`, a
    scope note;
  - facets following the pivot's table: operation, data roles, configuration, execution, effects,
    extension;
  - a **definition**: a conjunctive query with shared variables over L2 and L3. It is written as
    a Rust enum AST, compiled to DataFusion SQL and digested, as `flows` is today.
- **`concept_members`:** concept, operation, role bindings (role → node or path), condition,
  verdict, witness. Materialized.
- **`facet_values`:** per operation, for faceted filtering.
- **Checks:** SKOS integrity (one prefLabel per language, no `broader` cycle via a recursive CTE,
  `related` disjoint from the `broader` closure) as DataFusion rules.
- **First consumer:** Stage 4.

**L6 — Discovery.** Existing techniques, given new consumers, all statistical.
- FCA over **behavioral** attributes per structural scope suggests facets and "operations like
  this". RCA is used only over behavioral contexts, at the least fixed point (Euzenat 2025),
  declared.
- Communities serve navigation.
- Multi-view vectors, each view a template id under one spec (F9):
  - signature + docstring (today's `api_text`);
  - the bounded source body;
  - a fact serialization (untested in the literature, so **Proposed**);
  - linked doc passages.
- Graph-FCA stays an offline experiment on a bounded projection.

**L7 — Serving: bundle `FORMAT` 3 and five tools.**
- **The bundle adds:** `operations` (public paths, signature, docstring summary, facets),
  `behaviors`, `conditions`, `concepts`, `concept_members`, `vocabulary`, and per-view `vectors`.
  `evidence` and `witnesses` are already there. Briefs remain for the curated subset (D-6).
- **Tools** (typed pydantic input and output; object outputs; §11.3's error contract):

  | Tool | Kind | Returns | Notes |
  |---|---|---|---|
  | `lookup_concepts(text)` | Ranked | Candidate concepts with scope notes | Ambiguity stays visible ("streaming" returns several) |
  | `search_operations(query, filters)` | Ranked | Operations: hybrid, per-view RRF | Labelled as discovery |
  | `find_operations(where, limit, cursor)` | Exhaustive over materialized rows | Results plus `complete` and the unknown list | Never uses vectors |
  | `get_operation(snapshot, operation)` | Lookup | Paths, signature, each control's fate (forwarded, transformed, consumed, logged only, ignored, with a verdict), behaviors with conditions, raises, callbacks, ambient reads, evidence, boundaries; brief if any | — |
  | `explain(snapshot, claim)` | Lookup | Derivation rule, premises and source spans | Soufflé-style: rule id and proof height per row |

- **Executor (D-4):** pyarrow compute (already pinned) over small tables, with no SQL strings
  built at serve time. Condition compatibility in Python is held to shared known answers (F6).
  Row caps and cursors.

**L8 — Evaluation.**
- **Mechanical:**
  - `fixtures/python/flow_shapes`: `try`/`finally`, `with` suppression, loop `else`, `match`,
    walrus, comprehension scope, `global`/`nonlocal`, `del`, `TYPE_CHECKING`;
  - `fixtures/python/behavior_shapes`: the pivot's six adversarial cases plus the Appendix B
    channels;
  - injected-violation cases for every rule;
  - shuffle and relocate determinism for every new table.
- **Structured, qualitative:** `eval/behavior/fastmcp-4.0.5.toml`, seeded by Appendix B's 12
  questions and grown to 15–20. Each question has target items cited to source lines and docs,
  committed before any output is read. The packet script and the present / partial / absent /
  incorrect / misleading rubric are Phase 4.3's. The operator reviews it. `eval/heldout/` stays
  sealed.

**Stages.** Each ends with a compact review. The ADR comes first.

| Stage | Delivers | Exit criterion |
|---|---|---|
| 0 | The ADR set (F1); decisions D-1 to D-9; commit the R2 work; pause Phase 4's deletion exit and Phase 5's B3/B6. Phase 3's typed relations are the foundation L2 needs, so they continue | A `standard` review accepts the ADRs |
| 1 | The simpler alternative (§8.1): a persisted `behavior` family from today's relations, passes per public callable, `FORMAT` 3's `operations`, `get_operation`, `find_operations` over existing facets, and the source-body view | Structured-eval v0: the controls and handoff questions are answered for operations with no brief |
| 2 | L1 with the D-2 provider; conditions; the verdict lattice; `self` fields; ambient reads | `flow_shapes` and `behavior_shapes` known answers; Q4 and Q10 answered |
| 3 | L3 summaries; L4 models v1 (io, json, gzip, logging, asyncio, anyio, pydantic, pydantic-settings, contextvars, functools); exception conversion | Q1, Q3, Q5 and Q9 answered |
| 4 | L5 registry v1 (20–40 concepts, authored), `concept_members`, `lookup_concepts`; FCA over behavioral attributes as facet suggestions | Structured eval on concept queries |
| 5 | Framework models and protocols: registries dispatched by name, middleware chains, lifespan, client lifecycle as partial orders from official usage (Pass C generalized); Graph-FCA offline | Q6, Q7, Q8, Q11 and Q12 |

### 8.3 Tooling: crates, libraries and packages

Versions and dates were read on 2026-09-24 from crates.io, PyPI or GitHub, by me or the research
agents. **Family** means whether a candidate pulls its own Arrow, DataFusion, object_store or
petgraph. Licences are not a criterion and are not listed.

**Adopt now.** These are already pinned, or small.

| Need | Choice | Why | Family |
|---|---|---|---|
| Dominators, post-dominators, SCC order | **petgraph 0.8.3** (pinned): `algo::dominators::simple_fast`; post-dominators over `Reversed(&g)` from a virtual exit; `tarjan_scc` returns callees first | Covers the CFG algorithms. The ~40-line dominance frontier and control dependence are ours (rustworkx's `src/dominance.rs` is a reference) | none |
| Dataflow sets | **fixedbitset** (pinned) | Already used by FCA | none |
| Bounded closures at compile time | **DataFusion 55.1 `WITH RECURSIVE`** (pinned). Agent probe: linear only, `UNION` gives set semantics, no mutual recursion | The vocabulary's `broader` closure and bounded witness relations. **Never at serve time** | pinned |
| Vocabulary, models, concepts | **toml + serde** (pinned; `deny_unknown_fields`) compiled into Arrow relations | One contract; no new crates | none |
| Query schema, one authority | **schemars 1.2.2** (JSON Schema 2020-12) from the Rust request types → committed schema → pydantic 2.13.5 models via **datamodel-code-generator 0.82.0** (dev-only), plus a golden test running the same requests through serde and pydantic | Rust stays the authority | none |
| Identifier splitting, directive tagging, grounding | **heck, aho-corasick, regex, strsim, unicode-segmentation** (all pinned). Directive codebooks from Monperrus et al. 2012 and Li et al. 2018 | FastMCP docstrings are Google style throughout (agent measure: 137 files with `Args:`, 70 `Raises:` blocks) | none |
| Concept lookup | **bm25s 0.3.11** (pinned) + **PyStemmer 3.1.0** (new; stemming merges `serialize`/`serialization`/`serializer` but not `serializable` or `serialise`, so altLabels remain necessary; agent test) + Qwen3-Embedding-8B (pinned) + RRF | Reuses the retrieval stack | none |
| Serve-time executor | **pyarrow 25.0.1 compute / Acero** (pinned) | Filter, join and aggregate built in code; no SQL injection surface; a hard cost bound | none |
| Vector search | **numpy exact cosine** (pinned). Agent measure: 100k × 1024 f32 in 7 ms per query | Deterministic, no new format. Our 4,096-d vectors are 25 MB per view at 1,534 operations | none |

**Spike.** Each has an exit test.

| Candidate | Version | For | Exit test | Notes |
|---|---|---|---|---|
| **`ty_python_core`** (+ `ruff_python_ast`/`parser`, `ruff_db`, `ty_module_resolver`, `ty_vendored`, all `=0.0.14`; **salsa `=0.28.2`**, with `salsa-macros` and `salsa-macro-rules` pinned `--precise`) | 0.0.14, 2026-09-16 | L1 provider (D-2) | Range parity on the pilot (report the residue); `flow_shapes` known answers; reaching definitions ⊆ our candidates; added time and memory | Probed this session. **A second ruff line** next to Pyrefly's 0.0.11: an ADR amending §B1 ("no second parser") and ADR-0002, plus a scoped exception in `check_family.py`. Weekly releases, so upgrade deliberately. `TYPE_CHECKING` is decided the checker's way; override it (D-3) |
| **ascent** | 0.8.1, 2026-08-29 | L3, if three or more recursive rule families repeat the worklist shape | Same summaries as the hand-written kernel on `behavior_shapes`; timings | Agent probe passed: lattices, `agg`, `ascent_par!`, and petgraph unified at 0.8.3. No built-in provenance, so encode rule-id and premise columns. Rules are compiled in |
| **Pysa** (facebook/Pysa, types from Pyrefly) | pyre-check 0.10.0, 2026-08-06 | An offline TITO oracle for L3 | Disagreements are explained or become fixtures | Its `ModelQuery` DSL is the closest precedent for "typed predicates compiled into joins"; its obscure-callee default must not be copied (Appendix C, item 3) |
| **DuckDB** | 1.5.5, 2026-07-22 | An alternative serve-time executor (D-4) | Adversarial query tests | Agent probe: non-linear recursion is accepted but wrong (8 of 12 rows). Forbid recursion at serve time |
| **Qwen3-Embedding-0.6B**; **F2LLM-v2 4B/8B** | — | A cheap second view; a comparison run | Structured-eval deltas | MTEB(Code), per the agent's computation: 8B 80.69, 4B 80.07, 0.6B 75.42; F2LLM-v2 8B 80.16 |
| **griffe** | 2.3.0, 2026-09-04 | A dev-only parity oracle for a Rust Google-style section parser (`Raises:` with conditions, `Returns:`) | Parser parity on the pilot's docstrings | Parses text without importing (agent test) |
| **spaCy** `DependencyMatcher` + en_core_web_sm | 3.8.16 / 3.8.0 | Extracting conditions and verb–object pairs from prose | Precision on a labelled sample | A learned (not generative) parser inside compile: needs an ADR. Repeatability was checked on one machine only |
| **Graph-FCA** (`gfca`, OCaml) | last commit 2024-12 | Proposing query templates offline | Useful patterns on a bounded projection | Never in the pipeline or query path |

**Defer, each behind a trigger.**

| Candidate | Trigger | Placement when it lands |
|---|---|---|
| **Lance / LanceDB** (12.0.0 / 0.39.0; Arrow 58, DataFusion 54, object_store 0.14.1) | More than ~10⁵ vectors at 4,096-d, or filtered ANN together with managed FTS | Per the operator's allowance: an **isolated workspace** with its own lockfile and its own `just deps` family check, and Arrow IPC files as the only interface. A **derived, non-byte-identical index** keyed by the generation key and rebuilt from the generation's Arrow files, because the probe gave 1 of 10 files identical. Python LanceDB already carries its own Rust core and exchanges pyarrow arrays without copying, so Rust-side Lance pays off only if the executor moves into Rust (D-4) |
| **datafusion-python** 55.x | Its 55 release; wanted if serve-time SQL must match the compiler's dialect | Agent probe: 54.0.0 drifts from 55.1 on recursive CTE column naming |
| **usearch** 2.26.2 | Exact search too slow | A Rust-built index should load in Python (untested) |
| **tantivy** 0.26.2 / tantivy-py 0.26.2 | A prebuilt index must ship | Issue #3058 (no read-only directory) is open. **SQLite FTS5** (stdlib; `mode=ro&immutable=1` tested by an agent) comes first |
| **oxttl** 0.2.4 | An external SKOS consumer | Turtle export only |
| **FCA oracles:** fcars 0.2.2, caspailleur 0.2.2, FCA4J (the only maintained RCA engine; JVM), **odis** 2026.9.1 | FCA/RCA over behavioral contexts lands | Dev-only. odis pulls petgraph 0.6 and network crates |
| **Soufflé** 2.5 / **Nemo** v0.10.1 | Provenance or rule cross-checks | Offline oracles only |
| **Qwen3-Reranker** 4B | Retrieval needs it | §B10 excludes neural reranking: an ADR first |

**Avoid.**

| Candidate | Reason |
|---|---|
| `ruff_python_semantic` (CFG, `SemanticModel`) and `ruff_linter` as flow infrastructure | The CFG is a stub at 0.0.14; the model is flow-insensitive; the checkers are private |
| Pyrefly's binding graph as the IR | Shaped for type inference (§4); use it as an oracle |
| LinkML (and its `gen-rust`) | A second schema contract beside `cpg-schema`; the Rust generator is "under development" and emits `serde_yml` 0.0.12 (RUSTSEC-2025-0068) |
| OWL and RDF stacks in the pipeline: horned-owl, reasonable, whelk-rs, oxigraph, rudof | A second query engine or reasoner; rudof brings petgraph 0.6 |
| oaklib | Depends on `llm` (§B11) |
| Substrait (datafusion-substrait 55.1) | `RecursiveQuery` is `not_impl`; spec versions are skewed (v0.85 / v0.99 / Acero v0.20); needs protoc |
| crepe, DDlog, cozo, differential-dataflow | A second petgraph, archived, dormant, and incremental machinery we don't need, respectively |
| Taxonomy induction (HiExpan, TaxoGen, CoRel), and BERTopic/UMAP in the pipeline | Built for corpora orders of magnitude larger; UMAP is repeatable only single-threaded |
| WordNet or VerbNet synonym expansion | No software senses (agent test) |
| Stack Overflow tag synonyms as identity | They merge opposites (deserialize → serialization). Use them as candidates only |
| rust-bert, ort, fastembed | Duplicate the vLLM service; heavy |
| Polars SQL | No `WITH RECURSIVE` |

---

## 9. Verification and measurement plan

| Claim or risk | Evidence label now | Check | Conditions and expected result | Gap |
|---|---|---|---|---|
| The flow IR's reaching definitions are right | Proposed; the provider is Interface-checked (probe) | `flow_shapes` known answers; parity with our `reference_resolutions` (a subset); parity with Pyrefly `BoundName` in straight-line code | Every case matches; parity residue 0 or named | Fixture to write |
| Range parity between the parse and a second parser | Proposed | A rule over the provider's ranges ⋈ `syntax_nodes` | Every range maps to one node | Rule to write |
| Summaries are right under the stated model | Proposed | `behavior_shapes`; Pysa TITO differential on the pilot | Known answers exact; disagreements explained | Both |
| No negative claim outside complete coverage | Proposed | `semantic:refuted-needs-complete-region`, with an injected case | Rejects the violation | Rule |
| Condition compatibility agrees across languages | Proposed | `specs/serving/conditions.json` run in both suites | Identical verdicts | Corpus |
| Membership comes only from definitions | Proposed | Rule: `concept_members` cites a definition digest | Rejects an FCA-sourced row | Rule |
| New inputs move identity | Proposed | A test that each registered data file changes `compiler_digest` | Changes | Test |
| Determinism | Tested for existing tables | Shuffle and relocate oracles over every new table | Byte-identical | Extend |
| End-to-end cost | Not measured | `just pilot` stage timings (D1's instrumentation) and peak RSS, per stage | Report; **no target is claimed** | Measure per stage |
| Serving latency | Not measured | `find_operations` / `get_operation` on the pilot bundle | Report p50/p95 | Measure |
| Answers improve for agents | Not measured | The structured evaluation packet, pre-registered | Rubric per target item; operator review | Per stage |

**Cost accounting.**
- **Storage:** the `behavior` family adds rows of the order of today's in-session relations
  (about 40k rows on the pilot).
- **Vectors:** 25 MB per 4,096-d view at 1,534 operations, before windowing.
- **Compile time:** flow IR and summaries are the new costs to measure. The `ty` probe indexed the
  sample in 0.23 ms, with no pilot-scale figure.
- **Build:** the `ty` line adds about 175 crates.

---

## 10. Exceptions and unresolved decisions

There are no SHOULD-level exceptions to record. These are the decisions the operator has to make.
Each blocks the ADR set (F1):

| # | Decision | Options | Review's recommendation |
|---|---|---|---|
| D-1 | The product definition | Supersede ADR-0004 (behavior model; briefs as rendering) or keep the compiler and add a behavior layer | Supersede; the amended ADRs are listed in F1 |
| D-2 | The flow-IR provider | `ty_python_core` 0.0.14 (a second Astral line) or our own CFG/SSA over 0.0.11 | A time-boxed `ty` spike with §8.3's exit tests; fall back to our own, with `ty` as an oracle |
| D-3 | Static branches | The runtime view (`TYPE_CHECKING` false; version and platform per the analyzed context) or the checker's view | Runtime; declare it in §B5 |
| D-4 | The serve-time executor | pyarrow; DuckDB; a PyO3 extension compiled from our workspace (one predicate authority, but it amends §B13) | pyarrow over materialized rows; revisit if queries need composition beyond it |
| D-5 | The condition language and verdicts | F6's closed atoms and §3's five verdicts, or narrower | As proposed; opaque atoms are `unknown` |
| D-6 | The future of briefs | Keep a curated subset as rendering, or retire them | Keep a subset. It is the only prose surface and it exercises grounding |
| D-7 | Models catalog v1 | Scope and authorship | Stage 3's list; operator-authored from library source and docs; append-only |
| D-8 | The in-flight holistic plan | Continue, pause or cut per phase | **Continue:** commit R2; Phase 3's typed relations and inventory (the foundation for L2). **Pause:** Phase 4's deletion exit (communities, FCA and kNN get new consumers), Phase 5 B3/B6 (Stage F builders). **Keep:** Phase 4.3's structured-evaluation machinery, retargeted to behavioral questions |
| D-9 | Separate pins | A second ruff/ty line; a Lance workspace | Allowed per the operator. ADR-0002 amended: each extra family is declared in `check_family.py` with its scope (a crate, or a separate workspace), and no type crosses the boundary |

---

## 11. Decision and implementation changes

**Decision: Revise.** Adopt the pivot's direction as the target. Do not adopt the document as
design. The superseding ADR set (Stage 0) carries §8.2's target, settles D-1 to D-9, and takes a
`standard` review.

**Reason.** The pivot is right on three counts:
- The measured baseline confirms its diagnosis: analysis is seed-scoped, and the whole-release
  relations are computed but thrown away.
- Its core safeguards hold up: embeddings never bypass structure; exact queries are exhaustive;
  communities are not truth.
- The repository's provenance, identity and validation machinery fits it well.

It is not yet a design because:
- four gates are unresolved on the behavior it claims;
- it omits the channels that carry the pilot's behavior;
- it would split concept and predicate authority.

| Priority | Change | Principle IDs | Acceptance evidence | Regression protection |
|---|---|---|---|---|
| 1 (authority) | The Stage 0 ADR set and DESIGN amendments; pause Phase 4's deletion exit | DM-02, DM-13, DM-59 | A `standard` review accepts | `just adr lint` |
| 1 | The verdict lattice and the negative-claim rule | DM-08, DM-42 | Injected case rejected | `semantic:refuted-needs-complete-region` |
| 1 | One concept registry; membership materialized; no Python predicate semantics | DM-02, DM-23, DM-53 | Rules plus a shared corpus | Rules; `conditions.json` in both suites |
| 1 | The runtime-view declaration and provider parity | DM-24, §B5 | `flow_shapes` passes | Tests |
| 2 (leverage) | Stage 1: persist `behavior`, per-callable passes, operation catalog, two tools | DM-10, DM-23 | Structured-eval v0 | The `behavior` coverage rule |
| 2 | Models, vocabulary and concepts digested into identity | DM-31, DM-32 | Test | `compiler_digest` test |
| 2 | View identity under one spec | DM-31, DM-32 | Test | `semantic:one-embedding-spec` per model |
| 3 (cost) | Stage timings and serving latency per stage | DM-39 | Measured on the pilot | `just pilot` |

**Deferred, each with its trigger (DM-58).**

| Item | Trigger |
|---|---|
| Graph-FCA in the pipeline | An offline experiment yields templates the structured eval rates useful |
| On-demand RCA at serve time | Materialized membership proves too coarse for exploration queries |
| A Datalog engine (ascent) | Three or more recursive rule families repeat the worklist shape |
| Lance / LanceDB | More than ~10⁵ vectors at 4,096-d, or filtered ANN with managed FTS |
| A PyO3 executor | Serve-time composition beyond pyarrow's operators is needed |
| spaCy in compile | Regex directive tagging misses conditions the structured eval needs |
| A neural reranker | An ADR under §B10 after a measured need |
| Native-extension bodies | A pilot question needs one |

---

## Appendix A — The pivot's factual claims, checked

A research agent checked each claim against the code at HEAD; I spot-checked the key citations.

| # | Claim | Verdict | Evidence |
|---|---|---|---|
| 1 | One parse; structural types | Accurate | §B1 L190–191; `types.rs` Merkle term ids. FCA does read `t.display` (`concepts.rs:54`) |
| 2 | Provenance; projections keep call sites, candidates and unresolved sites | Accurate | `tables.rs` `facts`; §5 |
| 3 | Passes A–C recognize delegation, forwarding and guards, and handoffs "in official usage code" | Partly | Only Pass C reads usage code; all three run per seed |
| 4 | Pass B records declines | Accurate, with gaps | `unfollowed_argument`; some declines leave no trace |
| 5 | The analytics API view is `path(parameters)` + docstring | Accurate | `neighbours::api_text` (read) |
| 6 | FCA attributes are mostly signature attributes; RCA adds `calls X` | Accurate | `concepts.rs` five forms; `relational` |
| 7 | Serving excludes query-time traversal | Accurate | §1.1 L38; §B13 L373 |
| 8 | The design excludes an ontology and CFG/dataflow | Accurate in substance; wrong section | §1.3, not §B10; CFG and dataflow are *deferred* (§13) |
| 9 | `lctx query` exists | Accurate | An operator CLI; agents cannot reach it |
| 10 | "Several" techniques are off by default | Understated | All eight are off (`Techniques` derives `Default`) |
| 11 | Communities are careful | Accurate | §9.4 |
| 12 | Spec and input-hash cache | Accurate | §B14 |
| 13 | Native bodies are out of scope | Accurate | §1.3 |
| 14 | Pyrefly inference is not runtime dataflow | Accurate | §B5 L256 |

## Appendix B — Grounding: 12 behavioral questions on FastMCP 4.0.5

Written and answered from source by a research agent. The lines behind journeys a–d (§5) were
read myself. Capability codes:

| Code | Capability |
|---|---|
| C1 | Local def-use |
| C2 | Argument → formal |
| C3 | `**kwargs` |
| C4 | Interprocedural summaries |
| C5 | Field-sensitive `self` |
| C6 | Pydantic and dataclass declarations |
| C7 | The settings singleton |
| C8 | ContextVars |
| C9 | Registries |
| C10 | Higher-order callbacks |
| C11 | Override dispatch |
| C12 | Exception flow and conversion |
| C13 | Branch predicates |
| C14 | Async execution |
| C15 | Lifecycle protocols |
| C16 | Reflection on user code |
| C17 | Dependency contracts |
| C18 | Dynamic access |

| Q | Question | Needs | Stage (§8.2) that answers it |
|---|---|---|---|
| Q1 | What happens to each `@mcp.tool(...)` option: forwarded, transformed, defaulted, consumed, conflicting? | C1 C2 C4 C5 C6 C10 C13 C18 | 3 |
| Q2 | Which tool-function parameters become schema, are injected, or are rejected? | C16 C17 C13 | 5 (rules parameterized by user code) |
| Q3 | Do `tasks=` and `strict_input_validation` apply to every tool? | C2 C4 C5 C8 C9 C12 C13 | 3 (5 for the ContextVar half) |
| Q4 | Which `FASTMCP_*` settings change behavior, when are they read, and which do nothing? | C7 C2 C13 C6 C18 | 2 |
| Q5 | What does a client see when a tool raises, and what changes it? | C12 C13 C5 C7 C11 C4 C17 | 3 |
| Q6 | Which calls reach middleware, in what order, and what does `call_next` run? | C9 C10 C11 C8 C13 C14 C4 | 5 |
| Q7 | When does `lifespan=` run, and how does its value reach a tool? | C5 C15 C14 C4 C17 C18 | 5 |
| Q8 | What protocol must a `Client` follow? | C15 C14 C5 C13 C12 C4 | 5 |
| Q9 | Which run options conflict, or are silently ignored, per transport? | C3 C10 C13 C2 C7 C12 C4 | 3 |
| Q10 | Which accepted parameters or settings do nothing, or act only conditionally? | C1 C7 C5 C3 C13 C18 | 2 |
| Q11 | Does a sync tool, resource or prompt block the event loop? | C14 C16 C13 C5 C12 C17 C4 | 5 |
| Q12 | For `tools/call` N, which function runs, and what makes a tool "Unknown"? | C9 C11 C10 C8 C12 C13 C4 | 5 |

Ranked by how many questions need them: C13 branch predicates (11), C4 summaries (8), C5 `self`
fields (7), C12 exceptions (6). C1 underlies all 12.

## Appendix C — References

**How to read the marks.** Verification was done by the literature agent on 2026-09-24. "opened"
means it read the full text or the official page; "metadata" means only the bibliographic record
or abstract. I did not re-read these sources myself.

**Semantic search and code browsing**
1. García-Contreras, Morales, Hermenegildo. "Semantic code browsing." *TPLP* 16(5–6), 2016.
   doi:10.1017/S1471068416000417 (opened). Its statuses are checked / false / check, over sound
   abstract interpretation of 63 Ciao modules.
2. Stolee, Elbaum, Dobos. "Solving the Search for Source Code." *TOSEM* 23(3), 2014.
   doi:10.1145/2581377 (metadata).
3. Reiss. "Semantics-based code search." ICSE 2009. doi:10.1109/ICSE.2009.5070525 (opened).
4. Premtoon, Koppel, Solar-Lezama. "Semantic Code Search via Equational Reasoning." PLDI 2020.
   doi:10.1145/3385412.3386001 (abstract page).

**Summary formats and industrial precedent**
5. CodeQL, "Customizing library models for Python" (beta notice),
   https://codeql.github.com/docs/codeql-language-guides/customizing-library-models-for-python/
   (opened). The `summaryModel(type, path, input, output, kind)` rows and the access-path grammar;
   value vs taint.
6. Pysa documentation: pyre-check.org/docs/pysa-basics, -implementation-details, -advanced,
   -model-dsl (opened). pyre-check was archived 2026-06-26; Pysa lives in facebook/Pysa and takes
   its types from Pyrefly. TITO, `ModelQuery`, obscure models, broadening.
7. Joern, "Custom Data-Flow Semantics," https://docs.joern.io/dataflow-semantics/ (opened);
   Yamaguchi et al., IEEE S&P 2014, doi:10.1109/SP.2014.44 (metadata).

**Interprocedural analysis and Datalog**
8. Reps, Horwitz, Sagiv. "Precise interprocedural dataflow analysis via graph reachability." POPL
   1995. doi:10.1145/199448.199462 (opened).
9. Sagiv, Reps, Horwitz. IDE, *TCS* 167, 1996. doi:10.1016/0304-3975(96)00072-2 (metadata).
10. Infer abstract-interpretation framework, https://fbinfer.com/docs/absint-framework/ (opened);
    Schubert, Hermann, Bodden, ModAlyzer, ECOOP 2021, doi:10.4230/LIPIcs.ECOOP.2021.2 (abstract).
11. Zhao, Subotić, Scholz. "Debugging Large-scale Datalog: A Scalable Provenance Evaluation
    Strategy." *TOPLAS* 42(2), 2020. doi:10.1145/3379446 (metadata; the arXiv abstract opened);
    Soufflé provenance docs (opened).
12. Sahebolamri, Gilray, Micinski. "Seamless deductive inference via macros" (Ascent). CC 2022.
    doi:10.1145/3497776.3517779 (metadata).
13. Bravenboer, Smaragdakis. Doop, OOPSLA 2009. doi:10.1145/1640089.1640108 (abstract); Shaikhha,
    Herlihy, Ngo, "Compiling Linear Datalog to SQL for Program Analysis," arXiv:2609.06301
    (abstract).

**Concept analysis**
14. Rouane-Hacene, Huchard, Napoli, Valtchev. RCA, *AMAI* 67(1), 2013.
    doi:10.1007/s10472-012-9329-3 (metadata).
15. Bazin et al. "On-demand Relational Concept Analysis." ICFCA 2019.
    doi:10.1007/978-3-030-21462-3_11 (arXiv abstract).
16. Euzenat. "The Fixed-Point Semantics of Relational Concept Analysis." *JAIR* 83, 2025.
    doi:10.1613/jair.1.17882 (opened).
17. Ferré, Cellier. "Graph-FCA: An extension of formal concept analysis to knowledge graphs."
    *DAM* 273, 2020. doi:10.1016/j.dam.2019.03.003 (metadata); "Graph-FCA in Practice," ICCS 2016
    (opened); Fokou et al., "Theoretical comparison of RCA and Graph-FCA," *IJAR* 186, 2025
    (opened).

**Protocols and API properties**
18. Acharya, Xie, Pei, Xu. "Mining API patterns as partial orders from source code." ESEC/FSE
    2007. doi:10.1145/1287624.1287630 (opened).
19. Pradel, Jaspan, Aldrich, Gross. "Statically checking API protocol conformance with mined
    multi-object specifications." ICSE 2012. doi:10.1109/ICSE.2012.6227127 (opened).
20. Robillard et al. "Automated API Property Inference Techniques." *TSE* 39(5), 2013.
    doi:10.1109/TSE.2012.63 (opened).

**Python analysis**
21. PyCG, ICSE 2021, doi:10.1109/ICSE43902.2021.00146 (abstract): about 99.2% precision and 69.9%
    recall. Jarvis, arXiv:2305.05949 (abstract). Monat, Ouadjaout, Miné, ECOOP 2020 and SAS 2021
    (pages opened). Urban, Müller, "input data usage," ESOP 2018 (metadata), a precedent for
    "accepted but unused".

**Documentation to specifications**
22. Monperrus et al. "What should developers be aware of? … directives of API documentation."
    *EMSE* 17(6), 2012. doi:10.1007/s10664-011-9186-4 (opened).
23. Blasi et al., Jdoctor, ISSTA 2018, doi:10.1145/3213846.3213872 (metadata); Zhong et al.,
    Doc2Spec, ASE 2009 (metadata); Pandita et al., ICSE 2012 (read by the ontology agent); Xie
    et al., DocTer, ISSTA 2022 (abstract).

**Vocabulary and schemas**
24. SKOS Reference, W3C Recommendation, 2009, https://www.w3.org/TR/skos-reference/ (opened).
25. OWL-S, W3C Member Submission, 2004, https://www.w3.org/submissions/OWL-S/ (opened). A
    conditional `Result` (`inCondition`, `withOutput`, `hasEffect`) is the nearest schema to the
    pivot's behavior record.
26. Dragan, Collard, Maletic. Method stereotypes, ICSM 2006 (read by the ontology agent). They can
    be computed from existing facts.

**Code representations**
27. Guo et al., GraphCodeBERT, ICLR 2021, arXiv:2009.08366 (abstract); Zhang et al., Qwen3
    Embedding, arXiv:2506.05176 (the results table read); Li et al., CoIR, ACL 2025,
    arXiv:2407.02883 (abstract). No study found measures embedding **serialized program facts**,
    so the pivot's facts view is Proposed.

---

## Erratum (2026-09-24, while pre-registering `eval/behavior/fastmcp-4.0.5.toml`)

**§5 journey b is corrected.** `mounted_components_raise_on_load_error` **is read** by the release:
- the read is in `fastmcp_tasks/lifespan.py:58`
  (`if fastmcp.settings.mounted_components_raise_on_load_error:`);
- `fastmcp_tasks` is the release's third distribution (§1.4).

The journey searched only the `fastmcp` package. Only `server_dependencies` has no attribute load
anywhere in the release.

**Appendix B's Q4 inherited the error.** The pre-registered item Q04.f states the correct fact.

The journey's point stands, and the correction strengthens it. "Never read" is a negative claim
about the whole release, and it is unsafe in two ways:
- a search scoped too narrowly misses real reads;
- `Settings.get_setting(name)` can read any setting by string.

That is F3's completeness premise at work.
