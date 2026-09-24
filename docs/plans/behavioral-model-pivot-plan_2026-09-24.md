# Plan: the behavioral-model pivot

**Source:** [`design_review_behavioral-model-pivot_2026-09-24.md`](../design_review/reviews/design_review_behavioral-model-pivot_2026-09-24.md)
(deep review, decision **Revise**) of [`docs/behavioral_model_pivot.md`](../behavioral_model_pivot.md).
**Date:** 2026-09-24. **Status:** Proposed. It is blocked on the operator decisions D-1 to D-9 (§4).

**Authority.** This plan is not authority. `docs/design/DESIGN.md` and the ADRs are. Nothing here
changes the design until Stage 0's ADR set lands and a `standard` review accepts it. Section
references such as "review §8.2" point into the source review.

---

## 1. Purpose

Turn library-context from a capability compiler that publishes about 20 briefs into an
**evidence-carrying behavioral model of a pinned library's whole public surface**. An agent can
ask which operations accept X, pass it to Y, under which configuration, raising what, needing
which lifecycle. It gets:
- **exhaustive answers** where the analysis is complete;
- **ranked candidates** where only discovery applies;
- **a named unknown** where the analysis stopped.

Each claim carries evidence and a derivation. Briefs stay, as one rendering of the model.

**Kept from today:**
- programmatic synthesis; no generative model in the pipeline or the query path (§B11);
- immutable, byte-identical generations (§B7, §B12);
- one pinned dependency family per workspace, now extended to declared extra lines (D-9);
- structured, qualitative evaluation with pre-registered targets; no API-agent evaluation;
- the gold is never a compiler input;
- licence is never a criterion.

## 2. Baseline

Measured 2026-09-24 on pilot snapshot `ec03626f63304c3c156dd87c727a9ead` (review §1).

| Quantity | Value |
|---|---|
| Public declaration nodes | 1,534: 740 functions, 431 async functions, 363 classes. 4,763 public paths |
| Briefs; subjects of any Pass A–C finding | 20; 20. Analysis loops over seeds (`crates/cpg-core/src/analyze.rs:1117`) |
| Arguments mapped by `argument_flows` (in session only; never persisted) | 32,075 of 74,712: parameter 3,746, alias 1, literal 12,964, **other 15,364** |
| Guards | 79 rows, against 516 release `if`s that raise directly |
| Pilot compile | 34.7 s, peak 3,877 MiB. "analyze: Pass B" takes 1.03 s for 20 seeds |

**FastMCP 4.0.5 behavior channels** (prevalence script, re-run):
- 324 `self.x = <param>` assignments in `__init__` across 153 classes; 273 are read in another
  method.
- 95 pydantic classes and 52 dataclasses.
- 39 `fastmcp.settings.<x>` reads.
- 15 ContextVars.
- 11 function-object attribute writes, 6 of them `__fastmcp__` metadata.
- 29 registry writes.
- 85 of 462 exception handlers raise a different type.
- 101 `getattr`, 46 `hasattr`, 9 `importlib.import_module`, 6 module-level `__getattr__`.
- 41.9% of public functions and methods are async.

## 3. Principles

Each principle names what breaks without it (review §8.2).

| # | Principle | Without it |
|---|---|---|
| T1 | **The whole public surface.** The universe is `public_paths`; analysis runs per callable, bottom-up; seed traversal becomes a query | Silence about 99% of operations (F2) |
| T2 | **Relations first, sentences last.** Every analysis output is a typed, persisted relation with in-row provenance (ADR-0019); briefs are one rendering | Knowledge locked in templates |
| T3 | **Our abstraction, stated (§B5).** The flow IR is our declared **runtime** model, whichever provider builds it | Type-checker views relabelled as runtime flow (F5) |
| T4 | **Meaning from models, propagation from summaries.** Domain meaning comes from a versioned models catalog; summaries carry it with argument substitution, conditions and exceptions, never as "calls X, so can X" | F4, and the pivot's own prohibition |
| T5 | **Conditions are data** in a closed language with three-valued compatibility. **No solver** (§B10) | F6 |
| T6 | **Five verdicts, never a null.** A negative verdict only inside complete coverage | F3 |
| T7 | **Materialize at compile time; serve bounded selections.** All predicate and membership evaluation runs in DataFusion before publication; Python never re-implements predicate semantics | F11, F12 |
| T8 | **Discovery nominates, definitions decide.** FCA, RCA, communities and embeddings never write membership | F7 |
| T9 | **Pre-registered behavioral question sets** (structured, qualitative) plus mechanical known-answer fixtures | F10, F14 |

## 4. Decisions required before Stage 0's ADRs

| # | Decision | Recommendation (review §10) | Unblocks |
|---|---|---|---|
| D-1 | The product definition | Supersede ADR-0004: a behavior model, with briefs as a rendering | All stages |
| D-2 | Flow-IR provider | A time-boxed `ty_python_core` 0.0.14 spike with exit tests (Stage 2.1). Fall back to our own CFG/SSA over the 0.0.11 AST, with `ty` as a test oracle | Stage 2 |
| D-3 | Static branches | The runtime view: `TYPE_CHECKING` is false; version and platform follow the analyzed context. Declared in §B5 | Stage 2 |
| D-4 | Serve-time executor | pyarrow compute over materialized rows. Alternatives: DuckDB 1.5.5, or a PyO3 extension module built from our workspace (which amends §B13) | Stage 1 tools |
| D-5 | Condition language and verdicts | The closed atom set (§5, L1) and five verdicts (§5, "The verdict lattice"). Opaque atoms become `unknown` | Stage 2 |
| D-6 | The future of briefs | Keep a curated subset as a rendering | Stage 1 bundle |
| D-7 | Models catalog v1 | Stage 3.1's list: operator-authored from library source and docs, append-only ids | Stage 3 |
| D-8 | The in-flight holistic plan | Continue the R2 commit and Phase 3; pause Phase 4's deletion exit and Phase 5 B3/B6; keep Phase 4.3's machinery, retargeted (§8) | Stage 0 |
| D-9 | Separate dependency pins | Allowed. Amend ADR-0002: each extra family is declared in `scripts/check_family.py` with its scope (one crate, or a separate workspace), and no type crosses the boundary | Stage 2 (a `ty` line), a Lance trigger |

## 5. Target architecture

**One table covers the layers:**

| Layer | What | Producer | Group | Identity | First consumer |
|---|---|---|---|---|---|
| L0 | The CPG (exists) | Extractor runs; Stage C/D | Existing families | §3.4.1 | Everything |
| **L1** | Flow IR | The `flow` family (CPG-constructing) | New family | Syntax-anchored, producer-scoped like syntax ids | L2 |
| **L2** | Behavior relations | The `behavior` family: Stage C/D declared SQL, persisted | New family | Content ids over subject, roles and the condition's normal form | Stage 1 tools, L3 |
| **L3** | Transfer summaries | Stage E kernel `lctx_analytics::summaries` | Analysis group | Content ids; the invocation records budgets | Stage 3 tools, L5 |
| **L4** | Models (external, framework) | Committed data, operator-authored | Compiler input, digested | Append-only model ids | L3 |
| **L5** | Capability registry and membership | `cpg-schema` registry; Stage E membership | Analysis group | Append-only concept ids; definition digest | Stage 4 tools |
| L6 | Discovery (FCA, RCA, communities, views) | Existing variants, new consumers | Analysis group | Existing | Ranking only |
| **L7** | Serving | Bundle `FORMAT` 3 and five MCP tools | Generation | Generation key | Agents |
| L8 | Evaluation | Fixtures, rules, a structured set | Tests, `eval/` | — | Stage exits |

**L1 tables (`flow`)**
- `flow_defs`: a place defined at a syntax site.
- `flow_uses`.
- `flow_reaching`: use → reaching definition, with a condition id.
- `flow_exits`: return, raise, yield or implicit; the value site; the exception type; the
  condition.
- `flow_regions`: `try`, `with` and handler regions, with what each can absorb.
- `conditions` and `condition_atoms`: normal form; atoms anchored on syntax ids.

A **place** is one of:
- a local name;
- `self.f` or `self.f.g` (k ≤ 2);
- a module global, settings singletons included;
- a ContextVar object;
- an attribute of a function object.

**L2 tables (`behavior`)**

| Table | What each row holds | Lands |
|---|---|---|
| `value_flows` | From a parameter, field, global, call result or literal; to a call's formal, a return, a field write, a raise or a yield; kind `value`, `transform` or `constant`; site; condition. It generalizes `argument_flows` | Stage 2 |
| `field_writes`, `field_reads` | Class, field, method, source or use, condition | Stage 2 |
| `guards` | A condition that controls a `raise`, call or return site. It generalizes today's 79-row relation | Stage 2 |
| `raises` | Exception type, condition, whether it is handled locally | Stage 2 |
| `handlers` | Caught types, and the action: re-raise, convert to a type, swallow, or convert to a value | Stage 2 |
| `callbacks` | A parameter, and whether it is stored in a field, invoked, forwarded or registered in a container | Stage 2 |
| `resources` | Acquire site, release site, kind | Stage 2 |
| `ambient_reads` | A settings field, environment variable or ContextVar; site; condition | Stage 2 |
| `argument_flows`, `guards` (v1), `parameter_reads`, `handoffs` | Today's relations, persisted | Stage 1 |
| `delegations` | Depth-1 call arcs with modality | Stage 1 |
| `control_fates` | Per public operation parameter: forwarded (to which formal), transformed, literal, unfollowed with a reason, or no read | Stage 1 |

**L3 tables and algorithm**
- **Tables:**
  - `summary_flows`: callable, input path, output path, kind, condition, verdict;
  - `summary_effects`: callable, effect kind, role bindings, condition;
  - `summary_boundaries`: callable, reason, site.
- **Access-path grammar,** CodeQL-shaped but without its beta file format: `Parameter[name]`,
  `Parameter[self].Field[f]`, `ReturnValue`, `Argument[formal]@Call[target]`,
  `Global[<module>.<name>]`, `Raise[T]`.
- **Algorithm:**
  - Bottom-up over the call graph's SCCs; petgraph's `tarjan_scc` returns callees first.
  - Within an SCC, iterate to a fixpoint over a finite domain: path depth ≤ k and condition size
    ≤ c.
  - Widening goes to `unknown` with `budget_reached`.
  - A call open to overrides joins its candidates, and is marked open when the candidate set is
    incomplete (§3.6).

**L4 files**
- `models/external.toml`: stdlib and dependency callables.
- `models/frameworks.toml`: pydantic, pydantic-settings, ContextVar, `functools`, anyio and
  asyncio, `contextlib`, logging.

Both live under `crates/cpg-schema/models/` and are hashed into `compiler_digest`. The effect
kinds form an append-only codebook: `io.read`, `io.write`, `net`, `log`, `timeout`,
`thread_dispatch`, `compress(format)`, `serialize(format)`, `validate(schema)`,
`register(container)`, `invoke(callable)`.

**L5 tables**
- `concepts`: an append-only id, `prefLabel`, `altLabels` with their sources, `broader` and
  `related`, a scope note, facets, and a **definition**. The definition is a conjunctive query
  with shared variables over L2 and L3, written as a Rust enum AST, compiled to DataFusion SQL and
  digested.
- `concept_members`: concept, operation, role bindings, condition, verdict, witness.
- `facet_values`: per operation, for faceted filtering.
- The vocabulary itself.

**L7: bundle `FORMAT` 3** adds `operations`, `behaviors`, `conditions`, `concepts`,
`concept_members`, `vocabulary` and per-view `vectors`, and keeps the `FORMAT` 2 files.

**L7 tools.** All take typed pydantic input, return objects, carry read-only annotations and
follow §11.3's error contract.

| Tool | Kind | Returns | Lands |
|---|---|---|---|
| `get_operation(snapshot_id, operation)` | Lookup | The full record: public paths, signature, each control's fate and verdict, behaviors with conditions, raises, callbacks, ambient reads, evidence, boundaries; the brief if one exists | Stage 1 |
| `find_operations(library, where, limit, cursor)` | Exhaustive over materialized rows | Matches, plus `complete` and the list of operations that are unknown; never uses vectors | Stage 1 |
| `search_operations(library, query, filters, limit)` | Ranked discovery | Operations by hybrid, per-view RRF | Stage 1 |
| `lookup_concepts(text)` | Ranked | Several candidate concepts with scope notes, so ambiguity stays visible | Stage 4 |
| `explain(snapshot_id, claim)` | Lookup | The derivation's rule id, premises and source spans | Stage 4 |

**The verdict lattice** (codebook `verdict`, append-only)

| Verdict | When |
|---|---|
| `established` | Derived under the stated model, with no boundary in the region |
| `conditional` | Established under a stated condition |
| `refuted_under_model` | Only in a `complete_under_stated_model` region with no named boundary |
| `unknown` | A boundary intervenes; its `boundary_reason` is named |
| `not_analyzed` | Out of scope, not requested, or cut by a budget |

## 6. Stages

Each stage ends with `just test-all`, `just pilot` (stage timings and peak RSS reported), a
`compact` review (each stage adds or changes a family, projection or analytic), and a handoff. Any
change to a §B decision gets its ADR and a `standard` review first. Commits are small, name the
stage and ADR, and state the test outcome.

### Stage 0 — Decide and re-baseline

1. **Finish the in-flight R2 work** (the holistic plan, Phase 2):
   1. Read `crates/cpg-schema/tests/snapshots/contracts__rules_snapshot.snap.new`. It should only
      add `semantic:public-path-one-node`. Then run `cargo insta accept`.
   2. Run `just test-all` and commit the R2 fixes (F1–F4, D53, the disposition).
   3. Run `just pilot`.

   `public_paths` and `FORMAT` 2 are Stage 1's foundation.
2. **Record the operator's answers to D-1 to D-9** in the plan and in the deviation log.
3. **The ADR set** (`just adr new` / `just adr supersede`), each amending DESIGN in the same
   commit:

   | ADR | What it decides | DESIGN sections |
   |---|---|---|
   | Supersede ADR-0004 | The behavior-model product, reshaped increments, revised non-goals, definition of done | §1.1, §1.2, §1.3, §1.5 |
   | A new behavior-model ADR | The flow IR as our stated abstraction, the runtime view (D-3), the verdict lattice, the condition language, models as data | §B5, §3, §9, §10.2 |
   | ADR-0010 amendment | The serving tools, the executor (D-4), `FORMAT` 3, embedding views under one spec | §B13, §B14, §6.4, §11 |
   | ADR-0012 amendment (only if D-2 chooses `ty`) | A second parser line, scoped, with range parity | §B1, §B8 |
   | ADR-0002 amendment | Declared extra dependency families | §B9, §7 |
   | ADR-0011 amendment | Dominators, post-dominators, bottom-up summaries over SCCs | §B4 |
   | ADR-0020 revision (still proposed) | The keep rule per consumer; the deletion exit paused | §9.8, §12 |
   | ADR-0005 amendment line | "A named consumer in the brief" becomes "a named consumer in the served model"; the §B10 list is unchanged, and the closed condition language is recorded as not being a solver | §9, §B10 |

   Then a `standard` review of the set (the `design-reviewer` subagent).
4. **Holistic plan (D-8):**
   - **Continue:** Phase 3 (typed relations, the relation inventory, and A2(b)'s digest
     completeness, which F8 extends).
   - **Pause:** Phase 4's deletion exit, and Phase 5 B3/B6 (the Stage F builders).
   - **Retarget:** Phase 4.3's `scripts/structured_eval.py` and rubric, to behavioral questions.
5. **Pre-register evaluation set v0.** Write `eval/behavior/fastmcp-4.0.5.toml`:
   - review Appendix B's 12 questions plus 3–8 more;
   - each question's target items cited to FastMCP source lines and docs;
   - nothing from `.claude/skills/`.

   Commit it before any Stage 1 output is read (F14).
6. Run the `handoff` skill (`STATUS.md`).

**Exit:** the `standard` review accepts the ADR set, and eval v0 is committed.
**Closes:** F1; F14 (pre-registration part).

### Stage 1 — The whole public surface (the simpler viable alternative)

1. **Declare the `behavior` family** in `cpg-schema`: `table!` contracts for `argument_flows`,
   `guards`, `parameter_reads`, `handoffs` and `delegations`, with keys and generated
   key/ref/fact rules. Write them through a Stage D derivation (`relational_derivation`) from
   today's declared SQL (`cpg_schema::flows`). Schema snapshots move, which is a declared
   migration.
2. **`control_fates`.** Run the Pass B worklist per public callable, not per seed. The table holds
   one row per (public operation, parameter, target formal or none) with the fate and a reason.
   Measure the cost at 1,534 callables. Seed findings stay for briefs.
3. **Coverage.** Write a `coverage` row per (public callable, `behavior`). Add the rule
   `semantic:behavior-covers-public`, with an injected case (F2).
4. **The `operations` catalog** (analysis group): public paths from `public_paths`, the signature,
   the docstring summary, and facets from existing facts:
   - parameters (name, kind, default);
   - declared types;
   - typed direct raises;
   - decorators;
   - async;
   - delegation count;
   - handoff partners.
5. **Bundle `FORMAT` 3.** Add `operations` and `behaviors`. Regenerate
   `specs/serving/schema_digests.json`. Update Python's `FORMAT` and `expected_schemas`. Keep the
   `FORMAT` 2 files.
6. **Tools:** `get_operation`, `find_operations` (a fixed facet algebra with bounded operators, row
   caps, a `truncated` flag and cursors) and `search_operations`, executed with pyarrow (D-4).
   - The facet vocabulary is served data, with `cpg-schema` as its authority.
   - The request envelope is pydantic.
   - No SQL strings are built at serve time. Add an ast-grep rule `no-string-sql-in-lctx-mcp` in
     `rules/` with fixtures in `rule-tests/` (F11).
7. **The source-body view.** A view template id and version go into `input_hash` and a `view`
   column of `vectors`. `semantic:one-embedding-spec` then holds per model (F9). Tests: two views
   under one spec get distinct keys; a second model is refused.
8. **Tests:**
   - known answers for per-callable relations on `analysis_shapes`;
   - shuffle and relocate determinism for every new table;
   - `fastmcp.Client(mcp)` in both protocol eras for the new tools;
   - adversarial queries (caps, empty sets, an unknown facet).
9. **Structured eval v0.** Build the packet (targets, the tools' answers, the brief where one
   exists, mechanical marks), then the assessment
   `docs/design_review/reviews/structured_eval_<date>.md`. The operator reviews it.

**Exit:** eval v0 shows the controls and handoff questions answered for operations with no brief.
Pilot time and memory are reported.
**Closes:** F2; parts of F9 and F11.

### Stage 2 — Flow IR, conditions and verdicts

1. **The D-2 spike** (in a worktree, time-boxed):
   - A crate `cpg-flow` pinning `ruff_python_ast`, `ruff_python_parser`, `ruff_db`,
     `ty_module_resolver`, `ty_vendored` and `ty_python_core` at `=0.0.14`, and salsa, salsa-macros
     and salsa-macro-rules at **0.28.2 `--precise`**. 0.28.3 and 0.28.4 break ruff 0.0.14; the
     review's probe hit both failures.
   - A salsa database built from our explicit context: Python version and platform from the run
     contract; no ambient configuration discovery.
   - Run it over the pilot.

   **Exit tests:**
   - range parity against `syntax_nodes`, with the residue reported;
   - `flow_shapes` known answers;
   - reaching definitions ⊆ our `reference_resolutions` candidates;
   - added time and memory.

   Decide `ty` or our own CFG/SSA, and write the ADR-0012 amendment if `ty`.
2. **`fixtures/python/flow_shapes`:** `try`/`except`/`finally`, `with` suppression, loop `else`,
   `break`/`continue`, `match`, walrus, comprehension scope, `global`/`nonlocal`, `del`,
   `if TYPE_CHECKING:` (runtime `else`), and `sys.version_info` branches.
3. **The `flow` family** (L1):
   - Override the runtime view for `TYPE_CHECKING` using C3's static-branch marks.
   - Add a range-parity rule with an injected case.
   - Put the provider's revision and settings into `producer_id` and `context_id`, with a test
     (F8).
4. **Condition language (D-5).**
   - **Atoms:** `is None`, `is not None`, `== literal`, `in {literals}`, truthiness,
     `isinstance(C)`, over places; everything else is opaque.
   - **Normal form:** sorted conjuncts, with a canonical encoding for ids (DM-15).
   - **Compatibility** (compatible / incompatible / unknown): a Rust evaluator plus a Python twin
     in `lctx_mcp`, both held to `specs/serving/conditions.json` (F6, F12).
5. **Verdict codebook** and the rule `semantic:refuted-needs-complete-region`, with an injected
   case (F3).
6. **L2 v2 from the flow IR:** `value_flows`, `field_writes`/`field_reads`, `guards` (from
   reachability per range or post-dominators; petgraph `simple_fast` over `Reversed`, plus our own
   ~40-line frontier), `raises`, `handlers`, `callbacks`, `resources` and `ambient_reads`.
7. **`fixtures/python/behavior_shapes`, part 1:**
   - a parameter never read;
   - a setting never read statically but read by string, which must be `unknown`;
   - incompatible modes;
   - a wrapper that does not pass a default on;
   - a rare API with no usage;
   - a dynamic boundary.

   The "used only for logging" case waits for Stage 3's models.
8. **Tools:** `get_operation` shows control fates with verdicts, conditions and ambient reads.

**Exit:** `flow_shapes` and `behavior_shapes` part 1 pass, and eval Q4 and Q10 are answered.
**Closes:** F3, F5, F6; F8 (the provider part).

### Stage 3 — Summaries and models

1. **Models catalog v1 (D-7).** `crates/cpg-schema/models/{external,frameworks}.toml`, in the
   access-path grammar, with effects from the effect codebook and `origin = synthetic_model`. v1
   covers:
   - `open`/`io`/`pathlib`, `json`, `gzip`/`zlib`, logging;
   - asyncio and anyio: `fail_after`, `move_on_after`, `to_thread`, task groups;
   - contextvars; `functools.partial` and `wraps`; `contextlib`;
   - pydantic `BaseModel`, `Field` and `TypeAdapter.validate_python`;
   - pydantic-settings `BaseSettings` (environment prefix);
   - httpx timeouts; the starlette and uvicorn entry points.

   Guards on the catalog:
   - its digest joins `compiler_digest`, with a test like `every_compiler_source_is_hashed` (F8);
   - a rule or test that no model cites a `.claude/skills` path (F14).
2. **The summary kernel** `lctx_analytics::summaries` (L3):
   - SCC order over the invocation projection;
   - fixpoint per SCC with k and c bounds and widening to `unknown`;
   - joins over override-open candidates;
   - exception conversion through `handlers`;
   - invocation rows that record budgets.
3. **Ascent spike** (only if the trigger fires: three or more recursive rule families repeating the
   worklist shape). ascent 0.8.1 must match the hand kernel on `behavior_shapes`, with timings.
   Provenance is encoded as rule-id and premise columns.
4. **The Pysa oracle.** Run Pysa (facebook/Pysa, which takes its types from Pyrefly) offline on
   the pinned FastMCP with `--save-results-to`. Compare its TITO models with `summary_flows`. Each
   disagreement is explained or becomes a fixture. Never copy Pysa's obscure-callee default.
5. **`behavior_shapes` part 2:** the "used only for logging" case, and a wrapper that disables an
   option of its callee.
6. **Tools:** summarized flows and effects in `get_operation`; effect and role filters in
   `find_operations`.

**Exit:** `behavior_shapes` part 2 passes, and eval Q1, Q3, Q5 and Q9 are answered.
**Closes:** F4 (the models part); F8 (the data-file part).

### Stage 4 — The capability registry

1. **The registry** in `cpg-schema` (TOML with `deny_unknown_fields`, compiled to Arrow):
   - concepts: append-only ids, labels with their sources, `broader`/`related`, scope notes,
     facets;
   - definitions as a Rust enum AST → DataFusion SQL, digested.
2. **SKOS integrity rules:**
   - one `prefLabel` per language;
   - `broader` acyclic, via a DataFusion `WITH RECURSIVE` using `UNION` (set semantics);
   - `related` disjoint from the `broader` closure.
3. **Seed the vocabulary** with a one-off script whose output the operator reviews:
   - altLabel candidates from Stack Overflow tag synonyms, with sources (they merge opposites, so
     they are candidates only);
   - Wikidata and EDAM `closeMatch` IRIs;
   - method stereotypes as behavioral primitives.

   Author 20–40 concepts.
4. **Membership.** `concept_members` materialized with role bindings, condition, verdict and
   witness. Rules:
   - every member cites a definition digest and a witness;
   - a test that FCA output never writes `concept_members` (F7).
5. **`lookup_concepts`:** bm25s plus PyStemmer 3.1.0 plus Qwen3 vectors, fused by RRF. It returns
   several candidates with scope notes.
6. **`explain`:** a witness chain with rule id, premises and spans. Each derived row stores its rule
   id and proof height, in the Soufflé style.
7. **FCA over behavioral attributes** per structural scope, as facet suggestions marked
   *candidate*. RCA over behavioral contexts at the declared least fixed point (Euzenat 2025), only
   if the structured eval shows value.
8. **Query schema authority.** Only if Rust needs the request types: Rust types → schemars 1.2.2
   → a committed JSON Schema → pydantic via datamodel-code-generator 0.82.0 (dev-only), plus a
   golden test through serde and pydantic.

**Exit:** the structured eval of concept queries (new targets pre-registered before this stage's
output is read).
**Closes:** F7; F12 (completed).

### Stage 5 — Framework models and protocols

1. **Framework models:**
   - registries dispatched by name;
   - middleware `call_next` chains;
   - lifespan;
   - state carried by ContextVars;
   - function-object metadata such as `__fastmcp__`.
2. **Protocols as partial orders** mined from official usage by generalizing Pass C (Acharya et
   al. 2007). They are labelled *observed pattern*, never *enforced protocol*.
3. **Rules parameterized by user code** (Q2, Q11): conditional records keyed on predicates about
   the user's annotations, whether it is a coroutine, and whether it is a generator.
4. **Graph-FCA offline** (`gfca`, OCaml) on a bounded projection, to propose query templates. It is
   judged by the structured eval and never enters the pipeline.

**Exit:** eval Q6, Q7, Q8, Q11 and Q12 answered. Then a `deep` review at the end of the increment.
**Closes:** F4 (completed); F10 (by construction).

## 7. Grounding questions by stage

From review Appendix B, FastMCP 4.0.5.

| Stage | Answers |
|---|---|
| 2 | Q4 (settings: effect, read phase, dead settings), Q10 (accepted but inert parameters) |
| 3 | Q1 (`tool` options), Q3 (`tasks=` and strict validation), Q5 (error masking), Q9 (transport option conflicts) |
| 5 | Q2 (schema, injected and rejected parameters), Q6 (middleware), Q7 (lifespan), Q8 (Client protocol), Q11 (event-loop blocking), Q12 (dispatch and "Unknown tool") |

## 8. The in-flight holistic plan (D-8)

| Phase | Action | Why |
|---|---|---|
| Phase 2 R2 (uncommitted) | **Finish and commit** (Stage 0.1) | `public_paths` and `FORMAT` 2 underlie the operation catalog |
| Phase 3 (typed relations, relation inventory, A2(b), C1) | **Continue** | L2 is persisted declared relations; F8 extends A2(b)'s digest |
| Phase 4.1–4.2 (re-registration, ablation) | **Pause.** Revise ADR-0020 in Stage 0 | The brief-retrieval keep rule no longer decides techniques whose consumers move |
| Phase 4 deletion exit (F5) | **Pause** | FCA, RCA, communities and kNN get new consumers (L6) |
| Phase 4.3 (structured evaluation) | **Keep, retargeted** | It is the evaluation instrument of every stage |
| Phase 5 B1 (FindingDraft) | Continue when convenient | Useful for new analysis tables |
| Phase 5 B3/B6 (Stage F builders) | **Pause** | Briefs become a rendering (D-6) |
| Phase 6 | Re-plan after Stage 1 | — |

## 9. Tooling by stage

Details and versions are in review §8.3.

| When | Adopt | Spike, with its exit test | Defer (trigger) | Avoid |
|---|---|---|---|---|
| Stage 1 | pyarrow compute (pinned); numpy exact search (pinned); bm25s (pinned); toml + serde (pinned) | DuckDB 1.5.5 as an alternative executor (no serve-time recursion; non-linear recursion is silently wrong); Qwen3-Embedding-0.6B as a second view; F2LLM-v2 as a comparison | tantivy 0.26.2 (issue #3058); SQLite FTS5 first | Substrait; Polars SQL |
| Stage 2 | petgraph 0.8.3 (`dominators::simple_fast`, `Reversed`, `tarjan_scc`); fixedbitset | `ty_python_core` 0.0.14 line (above); griffe 2.3.0 as a docstring parity oracle | — | `ruff_python_semantic` CFG and `SemanticModel`; `ruff_linter` internals; Pyrefly's binding graph as the IR (oracle only) |
| Stage 3 | heck, aho-corasick, regex, strsim (pinned) for identifier splitting and directive tagging (the Monperrus and Li codebooks) | ascent 0.8.1 (on its trigger); Pysa as an offline oracle; spaCy 3.8.16 `DependencyMatcher` (needs an ADR: a learned parser in compile) | Soufflé 2.5 / Nemo as rule and provenance oracles | crepe, DDlog, cozo, differential-dataflow |
| Stage 4 | PyStemmer 3.1.0; DataFusion `WITH RECURSIVE` (compile time only); schemars 1.2.2 (if needed) | — | oxttl 0.2.4 SKOS export (an external consumer appears); FCA oracles fcars 0.2.2, caspailleur, FCA4J, odis (dev-only; petgraph 0.6) | LinkML and its Rust generator; OWL/RDF stacks; rudof; oaklib; WordNet/VerbNet synonyms; taxonomy induction; BERTopic and UMAP in the pipeline |
| Stage 5 | — | Graph-FCA offline | Qwen3-Reranker (needs a §B10 ADR) | rust-bert, ort, fastembed |
| Any | — | — | **Lance / LanceDB** (more than ~10⁵ vectors at 4,096-d, or filtered ANN with managed FTS): an isolated workspace with its own lockfile and `just deps` family check; Arrow IPC as the only interface; a derived, non-byte-identical index keyed by the generation key (probe: 1 of 10 files identical across identical writes). Also usearch 2.26.2 (exact search too slow); datafusion-python 55.x (its release, if dialect parity matters) | — |

## 10. Verification and measurement

| Claim | Check | Expected | Stage |
|---|---|---|---|
| Every public callable has behavior coverage | `semantic:behavior-covers-public` + injected case | Violation rejected | 1 |
| No SQL built from strings at serve time | ast-grep `no-string-sql-in-lctx-mcp` + rule tests | Matches fixtures only | 1 |
| Views keep identity under one spec | Tests on view keys and a second model | Distinct keys; second model refused | 1 |
| The flow IR is right | `flow_shapes` known answers; ⊆ `reference_resolutions`; parity with Pyrefly `BoundName` in straight-line code | All pass; residue 0 or named | 2 |
| Range parity with a second parser | Parity rule + injected case | One node per range | 2 |
| No negative claim outside complete coverage | `semantic:refuted-needs-complete-region` + injected case | Violation rejected | 2 |
| Condition compatibility agrees across languages | `specs/serving/conditions.json` in both suites | Identical verdicts | 2 |
| New inputs move identity | Tests over provider settings and model or registry files | The digest changes | 2–4 |
| Summaries are right under the model | `behavior_shapes`; the Pysa TITO differential | Exact; disagreements explained | 3 |
| Membership comes only from definitions | A rule on `concept_members`; a test that FCA writes none | Pass | 4 |
| Determinism | Shuffle and relocate over every new table | Byte-identical | 1–5 |
| End-to-end cost | `just pilot` stage timings and peak RSS | Reported; **no target claimed** | 1–5 |
| Serving latency | p50/p95 of `find_operations` and `get_operation` on the pilot bundle | Reported | 1–5 |
| Agents get better answers | The structured evaluation packet against pre-registered targets | Rubric per item; operator review | 1–5 |

## 11. Findings traceability

| Finding (review §7) | Closed by | Oracle |
|---|---|---|
| F1 — the scope change names no superseded decisions | Stage 0.3–0.4 | `just adr lint`; no mechanical oracle for sequencing (prose) |
| F2 — analysis is seed-scoped; relations are discarded | Stage 1.1–1.3 | `semantic:behavior-covers-public` |
| F3 — no verdict lattice | Stage 2.5 | `semantic:refuted-needs-complete-region` |
| F4 — the channels that carry the pilot's behavior are missing | Stages 2.6, 3.1, 5.1 | `behavior_shapes` |
| F5 — checker views relabelled as runtime flow | Stages 2.1–2.3 | `flow_shapes` (the `TYPE_CHECKING` case); parity tests |
| F6 — condition compatibility needs a solver | Stage 2.4 | `conditions.json` in both suites |
| F7 — split concept authority | Stage 4.1, 4.4 | Membership rule; FCA test |
| F8 — new inputs missing from identity | Stages 2.3, 3.1, 4.1 | Digest tests |
| F9 — views against the one-spec rule | Stage 1.7 | View-key tests |
| F10 — seven layers with no exit criteria | This plan's staging | Structured eval per stage (prose) |
| F11 — unbounded serving | Stage 1.6 | Adversarial tests; ast-grep rule |
| F12 — a second predicate implementation | Stages 2.4, 4.4; T7 | Shared corpus |
| F13 — unsourced research | Review Appendix C; the ADRs cite verified sources | Prose |
| F14 — pre-registration and the gold boundary | Stages 0.5, 3.1 | Skill-path test; commit order |

## 12. Risks

| Risk | Mitigation |
|---|---|
| `ty` crate churn (weekly releases) and salsa patch skew | Exact pins and a `--precise` lock; upgrades are deliberate commits with the parity tests; the fallback is our own CFG |
| Build size from a second ruff line | Measured by the probe: about 175 crates and a 12.6 MB release binary. It is isolated in one crate |
| Summaries lose precision or fail to terminate | A finite domain with widening to `unknown`; report `unknown` rates per stage |
| Scope creep across seven layers | Exit criteria per stage; the deferred list (§13); nothing starts before its stage |
| Circular evaluation | Pre-registered targets; the skill-path test; `eval/heldout/` sealed |
| Serving complexity or latency | Materialization (T7); pyarrow; caps and cursors; measured p95 |
| Disk space (the shared disk was about 90% full in the last handoff) | Keep one pilot store; prune `target/` incremental; store copies only when needed |
| GPU contention when re-embedding views | The embedding cache; the fake embedder in tests; live legs only with ≥ 30 GB free, and vLLM stopped afterwards |

## 13. Deferred, each with a trigger

| Item | Trigger |
|---|---|
| Graph-FCA in the pipeline | An offline experiment yields templates the structured eval rates useful |
| On-demand RCA at serve time | Materialized membership proves too coarse for exploration |
| A Datalog engine (ascent) | Three or more recursive rule families repeat the worklist shape |
| Lance / LanceDB | More than ~10⁵ vectors at 4,096-d, or filtered ANN with managed FTS |
| A PyO3 executor | Serve-time composition beyond pyarrow's operators is needed |
| spaCy in compile | Regex directive tagging misses conditions the structured eval needs |
| A neural reranker | An ADR under §B10 after a measured need |
| Native-extension bodies | A pilot question needs one |

## 14. Standing conventions

- Small commits to `main`, each naming its stage and ADR and stating the `just test-all` outcome.
  Never push, force-push or `reset --hard`.
- Snapshot changes: read the `.snap.new` first, then `cargo insta accept`. A schema snapshot change
  is a declared migration. Never run `cargo insta review`.
- Codebooks are append-only (`verdict`, the effect kinds, concept ids, model ids).
- Report every check as `passed`, `failed`, `blocked` (naming the prerequisite) or `not_run`, with
  its command. A mocked provider is never a pass. Label every design claim with a charter §D label
  and date it.
- The gold (`.claude/skills/`) is never a compiler input. `eval/heldout/` stays sealed until the
  final evaluation. No API agents evaluate content.
- Licence is never a criterion. Extra dependency families are allowed when declared and isolated
  (D-9).
- `analytics.toml` is frozen; editing it needs an ADR-0004 amendment, which becomes an amendment to
  its successor.
