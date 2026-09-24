# Design review — end of increment 3 (deep): the behavioral model's Stages 0–1, the whole public surface

**Depth:** deep (ADR-0021: one review at the increment's depth covers the plan stage and the
increment; plan §15). **Date:** 2026-09-24. **Reviewer:** the `design-reviewer` subagent, fresh
context. **Author of the change:** Claude, in a straight-through run under the operator's
instruction (deviation log B1).

**Decision: Revise.** Stage 1 delivers what the plan asked for in shape. It persists the relations,
covers all 1,534 public nodes, serves byte-identical `FORMAT` 3 generations, adds three tools with
no string SQL, and pre-registers its evaluation honestly. But the core new semantics do not yet
mean what DESIGN and the tools' notes say, and the pilot measurements below show it:

- A forward through override-open dispatch is published as `established`, while the delegation over
  the same arc is `unknown` (F1).
- `behavior_status = established` does not mean "the scan met no boundary" (F2).
- `find_operations` answers `complete: true`, or "no operation has …", over facet rows that were
  never materialized (F3).

All three are small, local corrections. They should land before Stage 2 builds its refutation
premise on them.

---

## 1. Decision and scope

**Proposal.** Plan Stages 0–1 as committed in `34219bc`, `a84d07b` and `8b59d69`:
- nine analysis tables (`cpg_schema::behavior`);
- five codebooks, and `analytic_method` `pass_b_surface`;
- three semantic rules;
- the Stage 1 scan (`cpg_core::behavior`);
- bundle `FORMAT` 3;
- `get_operation`, `find_operations` and `search_operations`;
- the `no-string-sql-in-lctx-mcp` rule;
- the structured evaluation v0.

It also covers DESIGN as amended (§1.1–§1.5, §B5, §B13, §B14, §3.2, §3.9, §6.4, §9, §9.8, §9.9,
§11.1, §11.3, §12, §13), ADR-0021 (accepted), ADR-0022 (proposed) and the ADR amendments of
2026-09-24.

**Status (charter §D, per claim; §9 below):**
- Implemented, with fixture tests, for the tables, rules, bundle and tools.
- Measured on the pilot for time and memory.
- The verdict and completeness semantics are violated in code, and Measured below.

**Affected revisions:**
- HEAD `8b59d69`;
- pilot snapshot `4dcb40b00a9550ccb88a53a60f9a82a4`;
- generation `0ed0c3b183080b50`;
- the eval set `eval/behavior/fastmcp-4.0.5.toml`, pre-registered in `2fb10b8`.

**Observable outcome.** Any of the 1,534 public nodes has a record:
- its paths;
- facets;
- per-parameter fates, each with a verdict and a source line;
- delegations;
- official-usage handoffs.

Agents can also ask for every operation matching typed facets, or rank operations for a task.

**Baseline.** 20 briefs. Pass A–C findings existed for 20 of 1,534 public nodes. The whole-release
relations were computed each compile and discarded.

**Supported scope, as the change states it:**
- Positive fates only; Stage 1 makes no negative claim (§3.2:620).
- Behavioral facets and `raises` are never complete (§11.3:3236–3242).
- Declared facets are complete under the stated model (§11.3:3233–3235).
- `refuted_under_model` is rejected everywhere (the re-review's R2).

**Constraints and uncertainty.** During this review a second session began Stage 2.1 in the
working tree (uncommitted):
- ADR-0022 and ADR-0012;
- DESIGN;
- `codebook.rs`, which appends the boundary reasons `dynamic_access` and `override_dispatch`;
- `condition.rs` and `specs/serving/conditions.json`;
- deviation B11.

This review judges HEAD `8b59d69`. Every DESIGN and ADR line number is from `git show HEAD:`. The
cited code files are unchanged from HEAD (checked with `git diff --quiet HEAD`).

### Method and coverage

**Read in full:**
- the charter, directive, template, ADDENDUM and REVIEW_REFERENCE;
- the plan, including §15;
- ADR-0021 and ADR-0022 (HEAD), and the ADR-0010 and ADR-0011 amendment lines;
- the DESIGN sections in scope (HEAD);
- `crates/cpg-schema/src/behavior.rs`, `crates/cpg-core/src/behavior.rs`,
  `crates/lctx-analytics/src/pass_b.rs`, `python/lctx_mcp/src/lctx_mcp/{operations,generation}.py`;
- the diffs of `bundle.rs`, `attempt.rs`, `analyze.rs` and `server.py`;
- the three new rules (`rules.rs:626–658`), `concepts::attributes_sql`, and `flows::arcs`;
- the new tests (`behaviors_cover_public_callables_outside_the_subsystem`, the injected cases,
  `test_operations.py`, the `FORMAT` 3 part of `a_generation_rebuilds_to_the_same_bytes`);
- the structured evaluation, questions Q13–Q20 of the TOML, and the deviation log B1–B10;
- the three prior reviews.

**Checks run in this session:**

| Check | Command | Outcome |
|---|---|---|
| The default loop | `just check` (HEAD `8b59d69`, clean tree, 2026-09-24 06:48–06:50) | **passed**: fmt-check, clippy `-D warnings`, nextest **220/220**, py-fixture 1/1, pytest **73**, pyrefly, `rules-scan` (no match), rule tests **7/7**, `lint-agents` ok, `adr lint` ok (22 records); 2 min 23 s |
| `FORMAT` 3 rebuild at pilot scale | `target/release/lctx bundle --store build/store --snapshot 4dcb40b0… --out <scratchpad>/gens` | **passed**: key `0ed0c3b183080b50`, all 16 files' sha256 identical to `build/generations/0ed0c3b183080b50`, `MANIFEST.json` included |
| The Stage 1 packet | `uv run python scripts/structured_eval.py build/generations/0ed0c3b183080b50 eval/behavior/fastmcp-4.0.5.toml --stage 1 --out <scratchpad>/stage1.md` | **passed** (written; I spot-checked Q14, Q16 and Q19 against it) |
| Pilot data | About 30 read-only queries (`target/release/lctx query --store build/store --snapshot 4dcb40b0… "SQL"`), and in-process calls to `lctx_mcp.operations` on the pilot generation | — (results cited per finding) |
| `just pilot`, `just test-all` (deps, gold, fixtures-check) | — | **not_run**. `just pilot` writes `build/store`, and this review may change nothing else. After 06:57 the working tree held the other session's uncommitted edits, so a later run would not describe HEAD |

**Not inspected:**
- `search_operations`' ranking quality: the pilot generation has fake vectors, and no evaluation
  item uses it;
- the live embedding path for views;
- the bodies of `argument_flows_sql` and `handoffs_sql`. They are unchanged since slices 2.1 and
  2.2, and are persisted here only by a wrapper;
- `lctx diff`.

**Asserted, not attacked:**
- the shuffle and relocate determinism of the six **unserved** persisted tables at pilot scale
  (O6);
- `operation_documents` windows against the live tokenizer;
- the correctness of individual `handoffs` rows beyond the evaluation items.

---

## 2. Authority and lifecycle map

| Concept or fact | Semantic type and identity | Authority / owner | Revision boundary | Update path | Derived representations |
|---|---|---|---|---|---|
| Persisted Pass B/C relations (`argument_flows`, `guards`, `parameter_reads`, `handoffs`) | Rows keyed by snapshot, edge, argument and formal (etc.) | `cpg_schema::flows` SQL, wrapped only to add `snapshot_id` (`behavior.rs:282–315`) | Snapshot; the compiler digest includes every source file | A compiler change | `behaviors` (Pass B rows, handoff rows) |
| `delegations` | Depth-1 call arc of a public callable, with `modality` | `behavior::delegations` (`arcs()` ⋈ `public_paths`) | Snapshot | Compiler | `behaviors` (`delegates`), facet `delegates_to` |
| `operations.behavior_status` | Verdict codebook value | Rust, `cpg_core::behavior` (`behavior.rs:540–576`): depth stop ∪ own open sites | Snapshot | Compiler | Served `operations`, every `OperationRef` |
| `behaviors` verdicts | `verdict` codebook | Rust, **two policies**: Pass B rows by `conditional` only (`:255–297`); delegations by `modality` (`:372–376`) | Snapshot | Compiler | Served `behaviors`, facets, the `unknown` list |
| `behavior_id` | Content id: operation, kind, parameter, callee, target, value, site (`cpg-schema behavior.rs:260–278`) | Rust; **not in DESIGN §3.4.1** | Stable across reruns; excludes verdict, depth and `conditional` | Compiler | Served key |
| Declared facets (`parameter`, `parameter_type`, `returns`, `raises`, `decorator`) | Strings per node | `concepts::attributes_sql`, FCA's relation, over **functions only** (`concepts.rs:44–45`; deviation B9) | Snapshot | Compiler | Served `operation_facets` |
| The facet vocabulary, its completeness class and its "hiding" kinds | Facet names; declared or behavioral; facet → behavior kind | **Python only**: `operations.py:29–59`. Plus the codebook's names in Rust, and DESIGN §11.3's prose | The server's release | Hand edit, no test linking them | `complete`, `unknown`, input validation |
| `operation_documents` and vectors | View and chunk; cache key `(spec_hash, input_hash)` | Rust, plus the global `embedding_cache` | Snapshot, and the cache version | Compiler, and the embedder | Served `operation_vectors` |
| Eval targets | Items with polarity | `eval/behavior/…toml`, append-only, pre-registered | `2fb10b8` | A new item supersedes an old one | Packet, assessment |

**Deliberately opaque:**
- Pysa's dispatch set, which gives `modality` and `candidate_set_complete_under_model`;
- the FCA attribute relation's domain.

Both are inputs whose limits must show in the served answers (F1, F3).

**Identity behavior:**
- **Reruns and relocation.** `behavior_id` is content-derived from node ids and strings. The served
  file is identical across relocation and reversed module order (Tested) and on a pilot rebuild
  (checked above).
- **Choice of site.** A forward's id includes the **first** witness site the BFS finds, so a second
  site reaching the same `(callee, formal, parameter)` is not represented.
- **Re-grading.** The verdict is outside the id. A policy change re-grades rows under unchanged ids,
  and `lctx diff` does not compare `behaviors` (F8).

---

## 3. Semantic contracts and invariants

| Contract or invariant | Representation | Enforcement boundary | Failure behavior | Verification evidence |
|---|---|---|---|---|
| Every public node is an operation; a class, and only a class, is `not_analyzed` | `operations` | `semantic:behavior-covers-public` (`rules.rs:629–637`) | The snapshot is rejected | Injected case; **Tested** (`just check` passed) |
| Behaviors and facets describe operations | FK-like | `semantic:behavior-of-an-operation` | Rejected | Injected case; **Tested** |
| No `refuted_under_model` in Stage 1 | `verdict` | `semantic:refuted-needs-complete-region` rejects every such row | Rejected | Injected case; **Tested** |
| `established` = "no boundary in the region the predicate reads" (§3.9:1142) | `verdict` | **None** | Published | **Violated** (F1) |
| `behavior_status = established` = "its scan met no boundary" (§9:2054) | `operations` | **None** | Published | **Violated** (F2) |
| Declared facets are complete under the stated model (§11.3:3233–3235) | `operation_facets` plus `complete` | Python constant `DECLARED` | `complete: true` | **Violated for classes** (F3) |
| Keys are unique; collisions are validator failures, never merged (§3.4.1:719) | `key:behaviors` | Generated rule, but `behavior.rs:472–473` deduplicates first | A silent merge | Latent (F8) |
| The served generation is rebuilt byte-for-byte from Delta (§6.4) | Manifest | `bundle::verify`; the load-time checks | Refused | **Tested** (fixture: relocation and reversed module order); pilot rebuild **passed** |
| No SQL is built from strings at serve time | Code shape | `rules/no-string-sql-in-lctx-mcp.yml` | Scan error | Rule tests **passed**; scan clean |
| No "unused" claim for a parameter with no fate | The Python note | `operations.py:257–266` | — | `test_a_parameter_without_a_fate_is_not_called_unused`, **Tested** |

**Absence and uncertainty.** Stage 1 distinguishes:
- `unknown` (a named reason);
- `not_analyzed` (classes);
- "no fate", which is not analyzed for the other channels.

It then collapses two further states into one message. Its invalid-params error, "no operation has
`facet` = `value` in this generation", is raised both for a value that exists only on unanalyzed or
override-open rows and for one that truly does not occur (F3). The error text states the absence.

**Equivalence.**
- Generation files: byte equality.
- `behavior_id`: content equality under an undocumented recipe (F8).
- Condition equivalence: Stage 2 (ADR-0022 open list).

---

## 4. Derivation and execution design

| Stage | Inputs | Output contract | Preconditions / assumptions | Effects | Provenance / invalidation |
|---|---|---|---|---|---|
| Relations (1.60 s) | Session tables | Four persisted relations, plus `delegations` | One snapshot per session (`with_snapshot`) | None | Compiler digest (source digest) |
| The surface scan (0.92 s) | Flows, guards, reads; parameters | Pass B rows → `behaviors`; one `pass_b_surface` invocation | `inside = |_| true`; `max_depth` 2 | None | The invocation records its parameters and digest. **The findings and witnesses are discarded** (F6) |
| Operations and facets (0.22 s) | `operation_sources`, `open_sites`, FCA attributes | `operations`, `operation_facets` | FCA's function-only domain (F3) | None | Compiler |
| Documents (0.14 s, fake embedder) | Declarations, module text | `operation_documents` (2,454) | 4,096-byte windows | A global cache MERGE (declared) | Keys in `content_digest` |
| Validate, publish | All tables | The `snapshots` append | Rules pass | Delta commits | §B7, unchanged |
| Bundle (0.62 s) | The published snapshot | 16 files, `FORMAT` 3 | — | Writes `generations/<key>` | Manifest names the snapshot; rebuild **passed** |
| Serve | One generation | Three tools | pydantic input; cursor bound to the generation and request | Reads files; the query embedder only for `search_operations` | — |

**Relationship structures.**
- A **delegation** is a call arc with modality.
- A **forward** is a parameter's value carried along call arcs. Here the arcs' modality is
  **dropped** (F1).
- A **handoff** is an observed usage pattern.
- A **facet** is an attribute.

All four are kept as distinct kinds (`behavior_kind`), but the forward loses the dispatch
qualifier that §9.2:2143–2147 requires Pass B's paths to name.

**Coherent publication.** Unchanged: the nine tables are written in the attempt before the
`snapshots` append, and the bundle is built only after it.

---

## 5. Representative journeys

### Adversarial 1: "Where does `render_prompt`'s `arguments` go?" (attack 1)

`get_operation("fastmcp.FastMCP.render_prompt")` on the pilot generation returns:
- `arguments` → `fastmcp.prompts.base.Prompt._render.arguments`, depth 1, **established**, at
  `server/server.py:1802` (`prompt._render(arguments)`);
- `arguments` → `fastmcp.prompts.Prompt.render.arguments`, depth 2, **established**, at
  `prompts/base.py:361` (`self.render(arguments)`).

The same record lists the delegation at 1802 to `Prompt._render` as **unknown**, value
`candidate`. `override_targets` shows three release overrides of `Prompt.render`:
- `FunctionPrompt.render`;
- `FastMCPProviderPrompt.render`;
- `ProxyPrompt.render`.

For a decorator-registered prompt the base `render` never runs. DESIGN §3.6:850–853 says an
override-open target "is never a single callee". The served forward names one, as established.

### Adversarial 2: "Which operations delegate to `Client.complete_mcp`?" (attack 2)

`find_operations(where={facets:[{delegates_to: "fastmcp.Client.complete_mcp"}]})` raises
invalid-params: **"no operation has delegates_to = 'fastmcp.Client.complete_mcp' in this
generation"**. Callers exist, through `self.complete_mcp(...)`, an override-open arc. The facet is
written only for established delegations (`behavior.rs:613`). The value check
(`operations.py:347–354`) runs before the `unknown` list could be computed. 363 of the 939
delegated-to callees are reached only through such arcs.

### Adversarial 3: "Which classes are dataclasses?" and "Which classes take `name`?" (attack 2)

- `decorator = "dataclass"` → **"no operation has decorator = 'dataclass'"**, but 41 public classes
  carry `@dataclass`.
- `kind = class, parameter = name` → **`complete: true`, total 0**, but 115 public record classes
  have no public `__init__`. For example, `fastmcp.prompts.PromptArgument` has the init field
  `name`.

The declared facets come from FCA's attribute relation, whose `wanted` set holds functions only
(`concepts.rs:44–45`).

### Adversarial 4: "Is `Tool.from_function`'s scan complete?" (attack 1)

The status is **established**, and there is no reason. Yet its `fn` reaches
`ParsedFunction.from_function.fn` at depth 2. There `fn` is rebound and read at calls to:
- `parse_docstring`;
- `transform_context_annotations`;
- `without_injected_parameters`.

None of these is reported: `pass_b.rs:479–482` skips reads at the frontier. None counts as a depth
stop: `:375–378` sets `depth_limited` only for a followable flow.

### Ordinary extension: a Stage 2 facet (for example, a settings read)

The edits:
- append to the `operation_facet` codebook;
- emit the facet in `behavior.rs`;
- extend Python's `FacetName` literal;
- classify the facet in `DECLARED` or `BEHAVIORAL`, and choose its "hiding" kind
  (`operations.py:29–59`);
- update DESIGN §11.3.

The Python classification is an independent semantic decision with no test linking it to
`cpg-schema` (F4). If it is forgotten, queries on the new facet are refused. If it is misfiled as
`DECLARED`, a behavioral facet reports `complete: true`.

### Meaningful change: Stage 2 re-grades override dispatch

The uncommitted codebook adds `override_dispatch`. If Stage 2 makes such forwards `unknown`, rows
keep their `behavior_id` and change verdict. `lctx diff` compares findings, assertions, evidence,
briefs and documents, not `behaviors`, so the semantic change has no diff (F8).

### Boundary: from the store to the bundle to Python

- The bytes are preserved: rebuild **passed**.
- The served `behaviors` drop `parameter_node_id`, `target_node_id`, `site_node_id` and
  `invocation_id`.
- The Pass B witness path never existed in the store (F6).

### Interruption

The publication protocol is unchanged. The one effect before publication is the global
`embedding_cache` MERGE, which is declared global and disposable (§3.2, §B14).

---

## 6. Acceptance gates

| Gate | Result | Evidence or scope rationale | Required action |
|---|---|---|---|
| G1 — Authority | **fail** (narrow) | The facet vocabulary, its completeness class and the facet → hiding-kind map are defined in Python (`operations.py:29–59`) apart from the `operation_facet` codebook, and no test links them. The map already disagrees with DESIGN §11.3:3240–3242 ("an `unknown` row of the facet's kind"): `forwards_to` maps to `unfollowed`, and the handoff facets map to nothing. The store → bundle authority **passes**: pilot rebuild byte-identical | F4 |
| G2 — Semantic fidelity | **fail** | F1: 352 established forwards and 28 established literal supplies cross an override-open arc; 33 of the forwards name a callee the release itself overrides. F2: `established` status for operations whose scan met open sites, frontier reads or override-open calls. F3: `complete: true` and "no operation has …" over rows never materialized | F1, F2, F3 |
| G3 — Validity | **pass** | The three rules reject their injected cases (`just check` **passed**). Input is validated by pydantic. The cursor is bound. The generation is verified at load. A latent bypass of the key rule is F8 | F8 (latent) |
| G4 — Hidden behavior | **pass** | The tools only read. No SQL (rule, scan clean). The query embedder is called only by `search_operations`, and the degradation is reported. The gold is not an input. B10's post-read change is disclosed | — |
| G5 — Consistency and recovery | **pass** | Tables are written in the attempt before the `snapshots` append. The manifest names the snapshot. One generation per process. Pilot rebuild **passed** | — |
| G6 — Transformation and reuse | **pass** | Kernel order comes from content-keyed sorted batches. The compiler digest hashes every source file (`every_compiler_source_is_hashed`). Cache keys are `(spec_hash, input_hash)`. Served tables are identical across relocation and reversed module order (Tested). Unserved tables: O6 | O6 |
| G7 — Truthful capability claims | **fail** | `find_operations` is advertised as exhaustive, with `complete` true for declared facets (§11.3:3233–3235; the server instructions). That is unbacked for 363 classes, whose declared facets are not materialized, and for 363 override-only callees. §9:2054's "met no boundary" is unbacked (F2) | F2, F3 |

An unresolved gate is not a pass. None is marked unresolved here: each result rests on code read at
HEAD and on pilot measurements.

---

## 7. Principle findings

Severity order: correctness and authority, then duplication and extension, then cost.

| # | Finding | Principle IDs | Concrete evidence or gap | Consequence | Proposed correction | Verification |
|---|---|---|---|---|---|---|
| **F1** | **One arc, two verdicts. A forward, a supplied literal or a guarded raise reached through override-open dispatch is `established` (or `conditional` on control flow only), while the delegation over the same arc is `unknown`.** The dispatch qualifier §9.2 requires is dropped from the behavior model | DM-08, DM-24, DM-42, DM-02 · G2 | `pass_b.rs:368–372`: the worklist follows flows whatever their `modality`. `behavior.rs:256–281`: the verdict depends only on `ConditionalCall` members. `behavior.rs:373–376`: delegations map non-definite to `unknown`. The served `Fate` has no modality field (`operations.py:93–107`). §3.6:850–853: "Override dispatch is never a single callee… any override of `f` can be reached". §3.9:1142: `established` = "no boundary in the region the predicate reads". **Measured** (pilot, last hop only, the only hop recoverable, see F6): 352 of 1,148 established `forwards` (in 188 operations) and 28 of 141 established `supplies_literal` cross a `candidate` arc; 33 of those forwards name a callee that a release function overrides (`override_targets`), for example `FastMCP.render_prompt` → `Prompt.render`, which 3 release classes override. `TransportMixin.run_http_async` → `http_app` at `transport.py:328`: four forwards established, delegation `unknown` | An agent reading `get_operation`, or `find_operations(forwards_to=X)`, is told unconditionally that a parameter reaches a base implementation that subclasses, the release's own included, replace. The structured evaluation credits four such forwards in Q14.b, and its precision note calls the `unknown` delegation "correct under the model" without seeing the conflict. If Stage 2 re-grades these rows (the uncommitted `override_dispatch` reason points that way), published meaning changes under unchanged ids (F8) | Decide ADR-0022's open item "verdicts ↔ `modality`" now, for Stage 1, because Stage 1 already publishes both policies. Minimal: any `candidate` hop on a row's witness path makes it `unknown`, with reason `candidate` (or `override_dispatch`), as for delegations. Alternative: keep the positive verdict, but add and serve an `override_open` flag naming the hop, as §9.2 already does for briefs. One place in `behavior.rs`, reading `result.witnesses` (modality is on every step). Record the choice in DESIGN §3.9 | **None exists.** Rule `semantic:established-needs-definite-path`: no `forwards` or `supplies_literal` row with verdict `established` whose last-hop `argument_flows` modality ≠ `definite` (it can be written today; make it whole-path once F6 persists hops). Test: an `analysis_shapes` override fixture (a base method called as `self.m(p)` and overridden in a subclass) yields a non-`established` forward |
| **F2** | **`behavior_status = established` does not mean "its scan met no boundary"** (§9:2054; the `operations` doc, `cpg-schema behavior.rs:170–174`). The status sees only a depth stop with a followable flow, and the operation's **own** unresolved or partial sites | DM-08, DM-59, DM-07 · G2, G7 | `behavior.rs:540–563`, with `open_sites` (`cpg-schema behavior.rs:350–360`), which tests `status <> resolved OR has_unresolved_remainder` only. **Measured** (pilot): (a) 738 own call sites of public operations are override-open (`resolved`, `candidate_set_complete_under_model = false`) and never count; 320 of 991 `established` operations hold such delegations. (b) 23 `established` operations forward a parameter into a callee whose unresolved or partial site takes that value (28 pairs). Examples: `FastMCP.add_prompt` → `PromptDecoratorMixin.add_prompt` (partial, remainder); `FastMCP.from_openapi` → `FastMCP.__init__` (unresolved). (c) 7 `established` operations have 16 frontier reads at depth 2 that are neither followed nor recorded as a stop (`pass_b.rs:479–482` against `:375–378`); journey 4. (d) 152 `established` operations carry `unfollowed` (`unknown`) rows | Every `OperationRef` and record shows `established` while boundaries lie inside the region. `find_operations`' `unknown` list omits (a)–(c) for `forwards_to`. Deviation B7 names this status as the **refutation premise**, so Stage 2's first negative claims would rest on it | Either define the status over the scan's **region**: `unknown` when any visited state encloses an open site (unresolved, partial, remainder, or an incomplete candidate set) receiving a tracked value, or has a frontier read (Pass A's own rule: "the depth bound is recorded when a frontier vertex has an arc … not followed", §9.1:2098–2100). Or narrow §9's sentence and the doc comment to what is measured, rename it (for example `scan_cut`), and withdraw it as B7's premise. Both are about 30 lines | **None exists** for (a)–(c). Tests on `analysis_shapes`: an operation → callee with an unresolved call taking the forwarded parameter gives `unknown`; a depth-2 frontier read gives `unknown`; an own override-open site gives `unknown` (if that option is taken). The re-review's R1 asked for the depth-3 test; it was not added |
| **F3** | **`find_operations` claims completeness, and states absence, over facet rows that were never materialized** | DM-08, DM-43, DM-42 · G2, G7 | (a) Declared facets exist only for functions: `concepts.rs:44–45` (`wanted` … `kind IN (function, async_function)`) and `behavior.rs:592` (classes excluded). Public classes carry only `kind` and `module`, plus 4 class-body `delegates_to` rows. **Measured:** 41 `@dataclass`, 6 `@runtime_checkable` and 1 `@total_ordering` public classes; 115 public record classes with no public `__init__`. `decorator=dataclass` gives "no operation has decorator = 'dataclass'". `kind=class, parameter=name` gives `complete: true`, total 0 (`PromptArgument` has `name`). (b) `delegates_to` is written only for established delegations (`behavior.rs:613`), and the value check raises before `unknown` is computed (`operations.py:347–354`): 363 of 939 delegated-to callees give "no operation has delegates_to = …" (journey 2). (c) §11.3:3214 says the set lists operations whose answer is `unknown` **or `not_analyzed`**; the code lists `unknown` only (`operations.py:372`) | Agents receive a completeness flag, or an error that asserts absence, exactly where the model has no rows. This is the silence-read-as-"none" that ADR-0021 was written to remove (review F2), and the re-review's R1 "Simpler" row assumed declared facets were safe | (a) Materialize class facets: decorators from `declarations`; constructor parameters from the synthesized `__init__` (§3.2 `types`: a `synthetic_callable` with `parameter_semantics`) or from `record_fields`. Or scope `complete` to functions and methods, and make it false when classes are in the universe. (b) Serve behavioral facet rows for non-established behaviors too, with their verdict, or check a value against facets ∪ `unknown`-row values. Never say "no operation has" for a behavioral facet; say "no established row". (c) Align §11.3's table with the Semantics paragraph, or list `not_analyzed` | **None exists.** pytest over the fixture generation: a decorated class is found by `decorator`; a callee reached only by an override-open call gives `complete: false` with the caller in `unknown`, not an error. The fixture already has `Registry.create`-style class receivers |
| **F4** | **The facet vocabulary, its completeness class and the "hiding" map are a second authority in the server** | DM-02, DM-41, DM-52 · G1 | `operations.py:29–59`: `DECLARED`, `BEHAVIORAL` (`forwards_to`→`unfollowed`, `delegates_to`→`delegates`, handoffs and `raises`→`None`) and `FacetName`. The codebook is `codebook.rs:1187–1212`. Nothing is served and nothing is tested across them. The plan's Stage 1.6 says "the facet vocabulary is served data, with `cpg-schema` as its authority". For `hands_off_to`/`takes_from`, `unknown` is decided by the **parameter scan's** status, which says nothing about Pass C's usage coverage | The meaning of `complete` is decided in Python (§B13:469: "No semantic decision is re-implemented in Python"). A Stage 2 facet needs an independent Python edit. A mis-filed facet silently reports `complete: true`. The handoff `unknown` list is evidence of nothing | Declare each facet's class and hiding kind next to the codebook (for example a `const` table in `cpg_schema::behavior`), and serve it (a small bundle file, or a known-answer JSON under `specs/serving/`, as `tokens.json` and `schema_digests.json` already are). Python reads it. Decide what hides a handoff match (Pass C's scope), or state that nothing does | **None exists.** Shared known answers: Rust writes, and Python asserts equality of names, classes and hiding kinds |
| **F5** | **Stage 1's exit is reported as "met in part", a state neither the exit criterion nor ADR-0021's triggers can decide. The trigger that was checked cannot fire on this set, and two of the three tools were never evaluated** | DM-59, DM-60 | Plan §6 Stage 1 exit and DESIGN §1.5:190 give no threshold. The evaluation says "met in part" (3 present, 16 partial, 25 absent of 44) and "the revisit trigger … did not fire". B3 chose Q13–Q20 **because** they have no brief, so briefs answer none of them by construction. The packet shows only `get_operation` (`structured_eval.py:51–56, 128–129`). ADR-0021's other trigger ("a stage's exit criterion fails twice") needs a fail. The assessor is the author; the operator's review is outstanding (B10) | Stage 2 starts on an exit that neither passed nor failed. The revisit triggers cannot fire. `find_operations`' completeness (F3) and `search_operations` have no evidence on the pilot. The report itself is honest: I confirmed Q14.b, Q16.a–b and Q19.a against the served data, and none of the evaluation's own ratings looks inflated. Only F1's verdicts behind Q14.b were missed | Pre-register a pass/fail rule per stage exit (for example: at least k items present or partial per question kind, and none incorrect or misleading), and record Stage 1's result under it with the operator. Add pre-registered `find_operations` and `search_operations` items to the Stage 2 set. Restate the "no gain over briefs" trigger against a set that includes brief-covered operations | Prose. **None mechanical** (a tally from a ratings file could be scripted) |
| **F6** | **The scan's lineage is discarded.** Behavior rows keep only the last site; delegation and handoff rows carry no invocation | DM-46, DM-23 | `behavior.rs:228–331`: `result.findings`, `members` and `witnesses` are read locally and never written. Only the path's last step survives (`last_step`, `:232–242`). `invocation_id: None` for delegation and handoff rows (`:406`, `:468`). §11.3:3250 promises that `explain` returns "a behavior row's site, its witness steps" | A depth-2 row cannot be audited for an override-open **first** hop, so F1's oracle stops at the last hop. Stage 4's `explain` has no stored steps to return | Persist the scan's witness steps (the `witnesses` shape already exists), or at least a per-row hop summary (hop count, any non-definite hop). Give every row its producing invocation | **None exists.** Test: every `depth > 1` row's path is reconstructable from stored rows |
| **F7** | **The source-body view and operation vectors are default machinery that nothing judges** | DM-58, DM-39 | `operation_vectors.arrow` is 41.6 MB of the pilot generation's 45.7 MB (91%), and the vectors are fake. No evaluation item uses `search_operations`. There is no ablation switch. The re-review's R6 ruled that views are not a variant, so ADR-0021's keep criterion never applies. Live embedding cost for 2,454 documents is not measured | The generation grew from about 1.1 MB of `FORMAT` 2 files to 45.7 MB for a consumer with no evidence, and nothing will ever decide whether the source-body view earns its GPU time | Put the views under the keep criterion (a `-source-view` switch), or pre-register `search_operations` items in Stage 2's set. Measure one live embed of the pilot's documents | None mechanical. The §9.8 ablation diff once a switch exists |
| **F8** | **The `behavior_id` recipe is undeclared, and collisions are merged silently** | DM-15, DM-07, DM-49 | `behavior.rs:472–473` sorts and deduplicates by `behavior_id` before `key:behaviors` can see a duplicate, against §3.4.1:719 ("never silently merged"). The recipe (`cpg-schema behavior.rs:255–278`) is not in §3.4.1's table and excludes the verdict, depth and `conditional`. No collision occurs today: the kernel's visited keys make each `(parameter, formal)`, `(formal, literal)`, `(raise, test, parameter)` and `(site, target, parameter, reason)` unique per operation (`pass_b.rs:379–381, 416, 452, 494`) | A future change that yields two rows under one id keeps whichever was pushed first, with its verdict, and validation passes. Re-grades under stable ids are invisible to `lctx diff` | Replace the dedup with an error; add the recipe row to §3.4.1 (what it covers, and that the verdict is a grade of the claim, not part of it); add `behaviors` to `lctx diff`, joining on the id and comparing the verdict | The existing `key:behaviors`, once the dedup is removed; a unit test pushing a duplicate |
| **F9** | **The spine is stale in several places about what Stage 1 is** | DM-02, DM-59 | §3.2:620: "Derived: Stage C/D declared SQL, persisted, one coverage row per public callable", against B6 (analysis tables, no coverage rows). §1.1:42: "The five behavioral tools are **Proposed**", while three are Implemented (§11.3:3206–3209). §B13:467: "Its executor is pyarrow compute"; `operations.py` filters Python dicts and sets and never calls `pyarrow.compute`. `cpg-schema behavior.rs:215–216`: `value` holds "the test's source text (`raises_when`)", but it is null in all 78 pilot rows; the text is in `site_text`. §1.2:66–67 still says every stage ends with a `compact` review (the re-review's O2). The Revision history has no row for the 2026-09-24 ADR-0021/0022 amendments. `server.py`'s `find_operations` docstring says `complete` is false "when an operation's behavior is unknown", though it is always false for behavioral facets | A reader of DESIGN or of the tool description builds the wrong expectations; B6's table and §3.2 disagree | Wording, in the commit that fixes F1–F3 | Prose |

### Observations (not findings)

- **O1. Concurrent Stage 2 work.** Stage 2.1 decisions were being written into ADR-0022, DESIGN and
  the codebook while this review was open, before its disposition. F1 and F2 bear directly on
  ADR-0022's open items: verdicts ↔ modality, the budget cut, and the refutation premise. Settle
  them together.
- **O2.** 38 annotated non-receiver parameters of 35 operations, and 18 declared returns, carry a
  type containing `Unknown`. FCA's filter (`concepts.rs:36–42`) drops them from `parameter_type`
  and `returns`. With exact-display equality those values cannot be queried anyway. But
  `get_operation`'s `facets` then show an annotated parameter as having no type (for example
  `Client.__init__`'s `sampling_handler`).
- **O3.** Literal supplies are deduplicated per `(formal, literal)` (`pass_b.rs:416`). A second
  site supplying the same literal is dropped, and `occurrences` stays 1.
- **O4.** `search_operations` turns an unknown facet value into an empty filter silently
  (`operations.py:438–440`), while `find_operations` refuses it.
- **O5.** 69 of 170 handoff groups have only `candidate` consumers, all `established`. The usage
  code does make that call, so this is less serious than F1, but the rule behind it is unstated.
- **O6.** The shuffle and relocate oracle covers the served `FORMAT` 3 files
  (`a_generation_rebuilds_to_the_same_bytes`, relocation and reversed module order). The six
  unserved persisted tables have none. They are deterministic by construction (sorted batches,
  content-keyed order); asserted here.
- **O7.** The new tools' calls are asserted in the `auto` protocol era only
  (`test_operations.py:111–139`). `legacy` only lists them (`test_server.py`), and plan 1.8 asked
  for both.
- **O8.** `STATUS.md` is dated 2026-09-23 (increment 2's close-out, "next: slice 3.2"). Plan §6
  owes a handoff at each stage's end, and AGENTS.md makes `STATUS.md` the first thing a session
  reads.
- **O9.** `let _ = public;` (`behavior.rs:172`) is a dead parameter.

### Applicability and principle verdicts

| Group | Bore on scope? | Verdicts |
|---|---|---|
| 1 Authority and boundary | Yes | DM-02 **violated** (F4; F1's two policies). DM-04 satisfied (Pysa's dispatch and FCA are named inputs), but their limits are not surfaced (F1, F3) |
| 2 Types and invariants | Yes | DM-07 **violated** for §9 and §11.3's invariants (no enforcement; F2, F3). DM-08 **violated** (F1–F3). DM-06, DM-09 and DM-10 satisfied (codebooks, keys, typed columns) |
| 3 Identity | Yes | DM-11, DM-12 and DM-14 satisfied (content ids, snapshot-qualified keys, manifest). DM-15 **unresolved** (F8: the recipe is undeclared, and the verdict lies outside it by an unstated choice) |
| 5 Derivation | Yes | DM-22 satisfied (declared relations, invocation). DM-23 satisfied for the bundle (rebuild **passed**), and **unresolved** for behavior lineage (F6). DM-24 **violated** (F1) |
| 6 Execution | Barely | DM-28 and DM-29 satisfied (the cache MERGE is declared). DM-30 unchanged |
| 7 Dependencies | Yes | DM-31 and DM-32 satisfied (source digest, cache keys). DM-34 satisfied (distinct behavior kinds) |
| 8 Performance | Yes | DM-39 satisfied for time and memory (Measured: 37.7 s, peak 4,050 MiB, behavior stages 2.9 s, `build/pilot.log`); live embedding unmeasured (F7). DM-40 satisfied (Tested) |
| 9 Boundaries | Yes | DM-41 **violated** (F4). DM-42 **violated** (F1: the qualifier is lost between briefs' paths and behaviors). DM-43 **violated** (F3). DM-45 satisfied (pydantic, no SQL, read-only) |
| 10 Provenance | Yes | DM-46 **unresolved** (F6). DM-47 satisfied (structured errors with near values). DM-49 **unresolved** (F8) |
| 11 Verification | Yes | DM-51 satisfied (declared migrations, schema snapshots). DM-53 satisfied for bytes and **unresolved** for verdict invariants. DM-54 partly (injected cases exist; adversarial tool cases do not). DM-60 **violated** for F1–F3 (no oracle) |
| 12 Leverage | Yes | DM-58 SHOULD deviation (F7). DM-59 **violated** (F5; §9's status claim) |
| 4 Composition | No | No templates or bindings changed (§E is judged in journey 5) |

No dimension scores are given: at this depth the gates and findings carry the decision, and a
score would only restate them.

---

## 8. Alternatives and architectural leverage

| Alternative | Duplication and extension locality | Correctness and operational risks | Cost | Performance evidence | Why selected or rejected |
|---|---|---|---|---|---|
| **Baseline** (before Stage 1): 20 briefs; seed-scoped Pass A–C; relations discarded | One derivation, for briefs | Silence about 1,514 operations, read as "none" (the pivot review's F2) | — | 32.2 s (B2) | Rejected by ADR-0021 |
| **As built**: nine tables, five codebooks, three rules, `FORMAT` 3 (five files), three tools, two embedded views | One verdict per row, but **two verdict policies** (F1). Facet class in Python (F4) | F1–F3 publish overconfident or false-negative answers | About 3,500 inserted lines across the three commits, snapshots and documents included | 37.7 s, peak 4,050 MiB. Behavior stages 2.9 s. Generation 45.7 MB, 91% of it vectors | The plan's shape, met |
| **Simpler viable (the counter-design)**: persist the relations and `delegations`; one `behaviors` table carrying **raw qualifiers** (hop modalities, `conditional`, unfollowed reason) and the path, with the verdict computed once by one rule in Rust; `get_operation`; `find_operations` over declared facets materialized for **functions and classes** (behavioral facets always `complete: false`, served as positive nominations with their verdict); `search_operations` lexical-only until a pre-registered item needs vectors | The verdict policy lives in one place, over the full path. The facet class is served. No views | F1 cannot arise: the qualifier is data and the rule reads it. F3(a) is closed by materializing class facets. F6 is closed by construction | Less than as built: `operation_documents`, `operation_vectors` and the per-view fusion are deferred | Generation about 4 MB. No embedding of 2,454 documents | **Recommended as the target shape of the Revise.** It keeps everything the Stage 1 evaluation used (`get_operation` only) and removes F1, F3(a) and F7 as classes. It needs no ADR change beyond deciding ADR-0022's verdict ↔ modality item, which is already open |

**Abstractions justified by current needs:**
- the persisted relations (Stage 2's inputs; 1.6 s);
- `operations` and `behaviors` (the evaluation's only consumer);
- the three rules (cheap, with injected cases);
- the no-SQL rule.

**Not yet justified:**
- the source-body view and the per-view vector legs (F7);
- the `unknown` list for handoff facets (F4).

**What remains ordinary code:** Pass B's worklist (a specialized kernel behind a declared
relation), and `find_operations`' conjunction (set intersections over served rows). Neither should
become a DSL.

---

## 9. Verification and measurement plan

| Claim or risk | Evidence label (now) | Test / analysis | Conditions and expected result | Current result or gap |
|---|---|---|---|---|
| Every public node is an operation; classes are `not_analyzed` | **Tested** | `semantic:behavior-covers-public` + injected case | Fixture: violation rejected | `just check` **passed** |
| `established` means no boundary in the region (§3.9) | **Violated** (Measured on the pilot) | F1's rule; override fixture | No established row over a non-definite hop | Gap: no oracle |
| `behavior_status` means the scan met no boundary (§9) | **Violated** (Measured) | F2's three fixture cases | `unknown` for callee open sites, frontier reads, (own override-open sites) | Gap; the re-review's R1 test was never added |
| Declared facets are complete | **Violated for classes** (Measured) | F3's pytest cases | A class decorator found; a candidate-only callee gives `complete: false` | Gap |
| The facet vocabulary has one authority | Proposed | F4's shared known answers | Python equals the served vocabulary | Gap |
| `FORMAT` 3 rebuilds byte-identically | **Tested** (fixture: relocation, reversed module order) | `a_generation_rebuilds_to_the_same_bytes`; pilot rebuild | Identical sha256 | **passed** (2026-09-24, 16/16 files) |
| No string SQL at serve time | **Tested** | Rule and rule tests; scan | Fixtures match; the tree is clean | **passed** |
| Determinism of the unserved tables | Implemented (asserted) | Shuffle and relocate on `analysis_shapes` over the six tables | Byte-identical | Gap (O6) |
| Stage 1 exit | Undecidable as written | A pre-registered pass/fail rule (F5) | Pass or fail | "met in part" |
| End-to-end cost | **Measured** (author's `build/pilot.log`, 2026-09-24 06:40, fake embedder) | `just pilot` | Reported, no target | 37.7 s; peak 4,050 MiB (validate); behavior stages 2.9 s; bundle 0.62 s |
| Serving latency (plan §10) | Proposed | p50/p95 of `find_operations` and `get_operation` | Reported | Gap: not measured |

**Cost accounting.**
- Construction: +2.9 s. Validation of nine tables is within validate's 1.31 s.
- Storage: served generation 45.7 MB, of which 41.6 MB (91%) are operation vectors.
- Live embedding cost is unknown (F7).

---

## 10. Exceptions and unresolved decisions

- **SHOULD deviation (DM-58), F7.** The source-body view as default machinery.
  - Scope: operation views and vectors.
  - Compensating control: none today.
  - It is acceptable as a scoped exception **only** if a revisit trigger is recorded: the Stage 2
    structured evaluation includes pre-registered `search_operations` items, or a
    `-source-view` switch lands.
  - Owner: the operator.
- **Unresolved decision for the author.** ADR-0022's open item "the relation of verdicts to
  `modality`" is no longer open in practice: Stage 1 publishes two answers (F1). Choose one before
  Stage 2's `standard` review. Choose F2's status definition in the same place, because B7 names
  that status as the refutation premise.

**Deferred** (each with the trigger that reopens it):

| Item | Why not now | Trigger |
|---|---|---|
| O2: `Unknown`-bearing declared types absent from facets | Not queryable under exact-display equality | A question needs callback-typed parameters |
| O5: candidate-consumer handoffs | The usage call is observed; F1's rule may cover it | Stage 5 (protocols from usage) |
| O6: unserved-table determinism oracle | Deterministic by construction | Stage 2 adds a consumer that reads the persisted relations |
| O7: legacy-era calls of the new tools | The `auto` era is covered | The next change to a tool signature |
| Serving latency p50/p95 | No latency claim is made | Stage 2's `find_operations` operators |

---

## 11. Decision and implementation changes

**Decision: Revise.**

**Reason.** G1, G2 and G7 fail on behavior the design claims to support. That behavior is the
verdicts and the completeness of `find_operations`. The rest is sound and verified in this session:
- the publication path;
- byte-identical generations;
- the rules with injected cases;
- the no-SQL boundary;
- the honest, pre-registered evaluation.

The corrections are local. None needs a new table or a new layer. The counter-design (§8) shows
the smallest shape that closes F1, F3(a), F6 and F7 together.

| Priority | Change | Principle IDs | Acceptance evidence | Regression protection |
|---|---|---|---|---|
| 1 (correctness) | F1: one verdict policy per arc, from the witness path's modalities (`unknown` or a served `override_open` flag); DESIGN §3.9 and ADR-0022 record it | DM-08, DM-24, DM-42 | The override fixture; the pilot count of established rows over a candidate hop is 0 (or all flagged) | `semantic:established-needs-definite-path` + injected case |
| 1 | F2: `behavior_status` over the scan's region (callee open sites, frontier reads, and own override-open sites if chosen), or narrow and rename it and withdraw it as B7's premise | DM-08, DM-59 | Three fixture cases; the pilot's `established` count re-measured | Those tests |
| 1 | F3: class facets materialized, or `complete` scoped; behavioral values present in `unknown` rows answer `complete: false` rather than an error; §11.3's table aligned | DM-08, DM-43 | pytest: `decorator` on a class; a candidate-only callee | Those tests |
| 2 (authority) | F4: the facet class and hiding kinds declared in `cpg-schema` and served; Python reads them | DM-02, DM-41 | Shared known answers | A Rust/Python equality test |
| 2 | F6: persist the surface scan's witness steps (or a hop summary), and an invocation on every row | DM-46 | Every `depth > 1` row reconstructable | Test |
| 2 | F5: a pass/fail rule per stage exit; Stage 1's result recorded with the operator; `find_operations` and `search_operations` items pre-registered for Stage 2 | DM-59 | The operator's review of the Stage 1 assessment (B10) | Prose |
| 3 (cost and hygiene) | F7: views under the keep criterion, or deferred; one live embedding measurement | DM-58, DM-39 | A switch, and the ablation diff | §9.8 |
| 3 | F8: dedup → error; the §3.4.1 recipe row; `behaviors` in `lctx diff` | DM-15, DM-07, DM-49 | `key:behaviors` sees duplicates | Unit test |
| 3 | F9 and O8: spine wording; `STATUS.md` handoff | DM-02 | — | Prose |

**Final check.**
- The design's claims do not yet match the evidence for verdicts and completeness (F1–F3).
- The scope matches the implemented guarantees everywhere else (verified above).
- Later extensions have a clear path, once the facet class has one authority (F4) and the verdict
  policy has one place (F1).

---

## 12. Disposition (author, 2026-09-24)

The Revise is taken in full, with the review's counter-design (§8) as the target shape where it
applies. Measured on the pilot after the fix (snapshot `eb444ac5`, generation `0af33db8`,
2026-09-24, fake embedder): `just pilot` **passed** (36.8 s, smoke 20/20). `just test-all`
**passed** (nextest 250/250, pytest 79, rule tests 7/7, lint-agents, adr lint, fixtures, deps,
gold).

| # | Disposition | Where |
|---|---|---|
| F1 | **Fixed.** One verdict policy, in one place (`cpg-core/src/behavior.rs`, `hop_reason`): a behavior whose witness path crosses a `candidate` arc is `unknown` with `override_dispatch` (a `potential` one, `ambiguous_binding`), exactly as the delegation over it. ADR-0022 §Verdicts and DESIGN §3.9 record it. **Pilot:** 502 forwards, 42 literal supplies and 2 guarded raises that were `established` or `conditional` are now `unknown`; `render_prompt`'s `arguments` → `Prompt.render` is `unknown` (`override_dispatch`) | rule `semantic:established-needs-definite-path` + injected case; `one_arc_has_one_verdict_and_the_region_decides_the_status` (`behavior_shapes`: `Base.handle` → `self.render`) |
| F2 | **Fixed**, with the stricter option: the status is over the scan's region and counts the operation's own override-open calls. Boundaries in order: the depth cut **and a formal read at the frontier**; an override-open or potential call on a path or of its own; an open site of its own **or one in a callee taking a tracked value** (new relation `open_site_reads`); a read the scan does not follow. `boundary_reason` names the first and `status_reason` all. **Pilot:** `established` 991 → 570; `unknown` 601 (override 383, open site 79, budget 66, unfollowed 73); `not_analyzed` 363 classes | `behavior_shapes` (`deep.top` for the frontier, `open.forward_open` for a callee's open site, `own_open`, `dispatch.Base.handle`); DESIGN §9's opening |
| F3 | **Fixed.** (a) Class facets: decorators from the declaration (41 `dataclass` classes on the pilot); parameters from the class's public `__init__` path, own or inherited (187 classes); without one, `not_analyzed` (176). (b) Behavioral facet rows are written for every verdict and carry it; only `established` and `conditional` rows match; a value on `unknown` rows only answers `complete: false`, never "no operation has". (c) `unknown` lists every operation that could still match: incomplete rows (`not_analyzed` included) or an `unknown` row | `operation_facet_status` (served); `class_facets_and_their_completeness_are_data`; pytest `test_a_class_without_a_public_constructor_could_match_any_parameter`, `test_a_value_on_unknown_rows_only_is_open_not_absent`; DESIGN §11.3 |
| F4 | **Fixed.** Completeness is served data (`operation_facet_status`, `FORMAT` 4); `DECLARED` and `BEHAVIORAL` are gone from Python. The facet names are held to the codebook by `specs/serving/facets.json`, which Rust writes and both languages assert. The handoff facets are declared never complete (two usage shapes) | `the_facet_names_are_the_shared_known_answers`; `test_the_facet_names_are_the_codebook_s` |
| F5 | **Fixed.** Pass/fail exit rules for Stages 2, 3 and 5 are pre-registered in the question set, before any Stage 2 output existed; the Stage 2 set gains two `find_operations` items (Q21 settings, Q22 `raise_on_error`) and two `search_operations` items (Q23, Q24). Stage 1's result stays "met in part": no rule was registered for it, and applying one now would not be pre-registration. Its assessment still awaits the operator (B10) | `eval/behavior/fastmcp-4.0.5.toml`; `scripts/structured_eval.py` renders requests and exit rules |
| F6 | **Fixed.** `behavior_steps` persists every behavior's path hop by hop with modality and `conditional` (4,465 steps for 4,033 behaviors on the pilot); relation rows (delegations) carry their one step; handoffs have none. Invocations stay on Pass B rows; a relation row's producer is its declared relation | `every_forward_carries_its_path_hop_by_hop` |
| F7 | **Kept as a scoped deviation** (DM-58), with its trigger now recorded: the pre-registered `search_operations` items Q23 and Q24 are judged at Stage 2's evaluation; if neither view earns a present rating there, the source-body view is removed. One live embedding measurement is owed when the GPU is free | deviation log B13 |
| F8 | **Fixed.** A duplicate `behavior_id` is an error of the scan; §3.4.1 has the `behavior_id` and `condition_id` rows (the verdict is a grade, outside the id); `lctx diff` keys behaviors by id **and** verdict, and operations by node **and** status | `a_diff_is_a_join_on_content_ids` (snapshot updated: behaviors and operations unchanged across the FCA variant) |
| F9 | **Fixed.** §3.2's behavior row (analysis tables, no coverage rows), §1.1 (three tools Implemented), §B13 (Python dictionaries over pyarrow-loaded rows), `behaviors.value`'s doc, §1.2 (stage and increment reviews merged), the `find_operations` docstring. The revision history row lands with the handoff | DESIGN; `cpg_schema::behavior`; `server.py` |
| O6 | **Deferred** (trigger: Stage 2 reads the persisted relations; it does in 2.6) | — |
| O8 | **Fixed** at this batch's handoff | `STATUS.md` |
| O9 | **Fixed** (`public` now drives class constructors) | — |
| O2, O5, O7 | **Deferred** with the review's triggers | §10 of this review |
