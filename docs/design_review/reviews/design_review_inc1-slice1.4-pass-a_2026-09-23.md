# Design review: increment 1, slice 1.4 — the compiler run, the invocation projection and Pass A (compact)

**Date:** 2026-09-23
**Reviewer:** `design-reviewer` subagent (fresh context), running the `design-review` skill. The
author of slice 1.4 is not the reviewer.
**Target:** commit `ea97ee9` on `main`, at these paths:
- `crates/cpg-schema/src/projection.rs` (the declared invocation projection);
- `crates/lctx-analytics/src/{graph,pass_a,config}.rs` (the §5 adapter, Pass A, the config);
- `crates/cpg-core/src/analyze.rs` and the Stage E part of `attempt.rs`;
- the analysis tables and rules in `crates/cpg-schema/src/{findings,rules}.rs`;
- `fixtures/python/analysis_shapes`, `crates/cpg-core/tests/analysis.rs` and
  `crates/lctx-analytics/tests/pass_a.rs`;
- `libraries/fastmcp/analytics.toml`.

It was reviewed against DESIGN §5 and §9.1 and the operator's graph guidelines §3, §4, §7, §8,
§10 and §12.
**Depth:** compact. This is the slice-end review ADR-0001 owes for a new projection and analytic.
**Prior review:** `design_review_adr-0019-analysis-results_2026-09-23.md` and its Disposition
covered the analysis-table contracts: identity, the completion mapping, the status policy and
provenance. Those items are not re-raised here.

---

## 1. Decision and scope

**Proposal.** Stage E runs Pass A inside the compile attempt. It has five parts:
- A declared projection: vertices, arcs and unresolved sites as generated SQL, with output
  schemas and a digest.
- An adapter that turns the three sorted Arrow results into an immutable petgraph `Graph`, with
  dense index = sorted `node_id`.
- A hand-written BFS per seed, with parent pointers, sorted adjacency, witnesses, candidate arcs,
  boundaries, unresolved sites and budgets.
- Seed resolution through `exports` and the MRO.
- An `lctx-compiler` run whose config digest is the analytics config's.

**Status.** Implemented. Tested by the named tests (§9). Measured on the pilot only by the
author's `just pilot` figures, which I did not re-run.

**Affected revisions.** Code at `ea97ee9`. Pilot snapshot `e9220288c6e63f18c6dc2998ed2f1a14`: 4
invocations, 42 findings (commit message; the counts were re-read by Q1 below).

**Observable outcome.** For each pre-registered seed, the published findings say which public
paths name it, what it reaches within depth 2 inside the subsystem, where the search stops, and
which of the call sites it expands are unresolved. Each finding has witness steps that cite
`edge_id`s in the same snapshot.

**Supported scope.** The v1 invocation projection (DESIGN §5) and Pass A (§9.1). Stage F
(assertions, briefs) is slice 1.5 and out of scope, except where it consumes what Pass A
publishes.

### Method and coverage

- **Read at the commit.** Every cited line is from `git show ea97ee9:<path>`. The working tree
  holds uncommitted slice-1.5 changes to `analyze.rs`, `attempt.rs`, `findings.rs`, `rules.rs`,
  `tests/analysis.rs` and the fixture's `helpers.py`. `crates/lctx-analytics/**` and
  `projection.rs` are identical to the commit.
- **Checks run (2026-09-23):**

  | Check | Command | Outcome |
  |---|---|---|
  | Slice tests at the commit | `git archive ea97ee9` into the scratchpad, then `INSTA_UPDATE=no cargo nextest run --locked --offline -p lctx-analytics -p cpg-core -E 'package(lctx-analytics) or binary(analysis)'` (separate `CARGO_TARGET_DIR`) | **passed** (9 tests: the two `config::tests`, the four in `tests/pass_a.rs`, `pass_a_finds_the_known_answers_on_analysis_shapes`, `pass_a_is_identical_across_module_order_and_location`, `the_analysis_rules_reject_their_violations`) |
  | Same filter on the working tree | same command in the repo | **failed** on one test outside this target: the uncommitted `briefs_are_synthesized_from_findings_and_verbatim_evidence` (the insta snapshot `briefs` differs). The nine slice-1.4 tests passed there too |
  | Probes P1–P4 | a scratch binary, built against the export's `lctx-analytics` and `cpg-schema` (`cargo +1.98.1 run --offline`), that runs `pass_a::run` on hand-built projections | ran. Results in F3 and O1–O3. These are review evidence, not repo tests |
  | Pilot queries Q1–Q7 | `target/release/lctx query --store build/store --snapshot e9220288c6e63f18c6dc2998ed2f1a14 "…"` (read-only) | ran. Results cited inline |
  | `just test-all`, `just pilot` | — | **not_run** by me. The commit message reports both passed on 2026-09-23. That is asserted, not re-verified |

- **Pilot queries.**
  - **Q1:** findings per seed, kind and stop reason, with labels.
  - **Q2:** `call_target`, `site_target`, `higher_order_target` and `potential_target` edges by
    modality, origin, fidelity and phase.
  - **Q3:** anti-joins for rows the projection's inner joins could drop.
  - **Q4:** invocations.
  - **Q5:** nested declarations of the seeds, and their arcs.
  - **Q6:** MRO shapes, class-scope bindings and shadowing.
  - **Q7:** `context_definitions` rows per external node.
- **Not attacked (asserted):**
  - an interruption between the four analysis-table commits (the attempt model from slice 2 is
    unchanged);
  - whether a Pass A code change without a `COMPILER_OUTPUT_VERSION` bump is caught (ADR-0019
    O4, owed by 1.5);
  - the pilot's 0.35 s Pass A and 3,643 MiB peak (the author's figures).
- **A lead corrected.** The brief said "two boundary findings share the label
  `builtins.BaseException.__init__` as distinct external_symbol nodes". Q1 and Q7 show otherwise:
  - It is **one** node, `607e2b4d…`, with one finding per seed. There are 42 findings and 42
    distinct ids.
  - `context_definitions` holds one row per run (the library run `11a953ff…` and the corpus run
    `4b159418…`) for 1,109 external nodes, so a label join fans out. This matters to Stage F
    (O5), not to Pass A.

## 2–4. Authority and derivation (compressed)

**Authority.** Each fact has one owner:

| Fact | Owner |
|---|---|
| The projection (kinds, modalities, origins, fidelities, policies, SQL, output schemas) | `cpg_schema::projection::invocation()` |
| The projection's digest | Joins both the invocation id (`analyze.rs:387-393`) and the compiler digest (`attempt.rs`, `compiler_digest`) |
| The subsystem, seeds and budgets | `libraries/fastmcp/analytics.toml`, whose digest is the compiler run's `config_digest` |
| Each finding kind's evidence status | `FINDING_STATUS`, read by both the kernel (`pass_a.rs:294-298`) and `semantic:finding-status-policy` |
| The subsystem predicate (release Function, Class or Module under a prefix) | Code in `analyze.rs:346-354`. Its inputs are in `parameters`, the predicate is not (O4 of ADR-0019 covers this) |

One rule is written twice: the owner of a site (O7).

**Stage path.**
1. Derived tables are written and a session opens at pinned versions.
2. `project` runs the three queries and casts each strictly to its declared schema
   (`analyze.rs:328-333`, `delta.rs` `to_schema`).
3. `Projection::build` refuses:
   - unsorted vertices or arcs;
   - an arc or unresolved caller that is not a vertex;
   - a wrong column type or a code outside its codebook (`lctx-analytics/src/graph.rs:114-213`).
4. `pass_a::run` runs per seed.
5. The rows are written, one commit per table, and registered in the session.
6. Validation runs, then the `snapshots` append.

A Stage E error aborts the attempt before publication.

## 5. Journey: a decorator-factory seed, through the real code

`fastmcp.FastMCP.custom_route` is one of the four pre-registered seeds.
1. **Resolution.** `resolve_seeds` finds the exported prefix `fastmcp.FastMCP`. `member` walks
   FastMCP's MRO, whose entries are AggregateProvider, Provider, LifespanMixin, MCPOperationsMixin
   and TransportMixin (Q6), and finds `TransportMixin.custom_route`.
2. **Source.** In the installed package (`fastmcp/server/mixins/transport.py:131-179`), the
   method's whole effect lives in a nested `def decorator(fn)`. That function calls
   `self._additional_http_routes.append(Route(...))`, and the method returns it.
3. **Projection.** A call's caller is its innermost owner (§5 L1451; `encloses_call` is
   `COALESCE(s.owner_node_id, s.module_node_id)`, `cpg-schema/src/graph.rs:415`). So the `Route(...)` and
   `append` arcs belong to `custom_route.decorator`, which has 3 arcs in the snapshot (Q5). No
   arc joins `custom_route` to `decorator`: `declares` is not a projection edge kind
   (`projection.rs:109-113`), and `custom_route` never calls `decorator`.
4. **Pass A.** It pops the seed, finds no out-arcs and no unresolved sites, and ends.
5. **Published (Q4).** `vertices_examined = 1`, `arcs_examined = 0`,
   `completion = complete_under_stated_model`, `stop_reason` null. The only finding is the
   `public_alias`.

`FastMCP.resource` does the same one level down. It delegates, as a candidate arc, to
`ResourceDecoratorMixin.resource`, which returns a nested `decorator` whose body holds
`self.add_resource(fn)` (`resources.py:169-212`). The invocation is complete, with `stop_reason`
null. This journey is F1.

## 6. Acceptance gates

| Gate | Result | Evidence or scope rationale | Required action |
|---|---|---|---|
| **G1** Authority | **pass** | See the table in §2–4: the projection spec, its schemas and its digest have one declaration; the adapter reads columns by name through a strict cast to those schemas; `FINDING_STATUS` feeds both the kernel and the rule; the subsystem comes only from the config. The duplicated owner rule is minor (O7) | — |
| **G2** Semantic fidelity | **fail** | Two decorator-factory seeds (custom_route, resource) publish a complete, empty or near-empty result while their behaviour sits in a nested callable the projection cannot reach. Nothing marks the gap (F1). The rest holds: candidate modality is kept per step; phases are kept; parallel sites are distinct arcs; `potential` and `synthetic_model` are excluded by declared policy (Q2); unresolved sites are surfaced (Q3). The kind depends on the witness cap (F3, narrow) | F1 (F3) |
| **G3** Validity | **pass** | What is refused: <br>• the adapter: disorder, unknown vertices, wrong types and nulls (non-nullable output schemas) <br>• the config: unknown keys, duplicate seeds, zero depth or witness budgets, non-dotted names <br>• `analyze.rs:357-366`: two seeds naming one declaration <br>Each new rule has an injected case. Gaps: witness steps are not checked against their edges (O6); the resolver proceeds past MRO entries it cannot read (F2, under G7) | O6 |
| **G4** Hidden behaviour | **pass** | Stage E has one new input, `analytics.toml`, and its digest goes into the run and `content_digest`. Library versions come from `Cargo.lock` at build time and are recorded per invocation. `lctx-analytics` reads nothing ambient (Arrow in, rows out) | — |
| **G5** Consistency and recovery | **pass** | Stage E rows go through the attempt: written, registered at pinned versions, validated, then published. Seed-resolution or adapter errors abort before the `snapshots` append (`attempt.rs` `finish`) | — |
| **G6** Transformation and reuse | **pass** | Dense indices never leave `lctx-analytics`: rows map through `p.ids` (`pass_a.rs:326-338`). Adjacency is re-sorted into canonical arc order (`lctx-analytics/src/graph.rs:222-229`) and never follows walker order. Parallel arcs are kept. The projection digest is in the invocation id and the compiler digest. `pass_a_is_identical_across_module_order_and_location` passed | — |
| **G7** Truthful capability claims | **fail** (narrow) | DESIGN §9.1 L1737 says seeds resolve "each remaining name as a member of its own body or MRO". The code sees only `def`/`class` declarations and skips external and unresolved MRO entries (F2). Two lines are stale: §9.1's "next paths" (O1) and §5's shuffled-arc test (O4) | F2, O1, O4 |

## 7. Findings

### 7.1 Findings

| # | Finding | Principle IDs | Evidence or gap | Consequence | Proposed correction | Verification |
|---|---|---|---|---|---|---|
| **F1** | **Pass A cannot see the behaviour of a callable that a reached vertex defines and returns (a decorator factory), and reports the invocation complete with nothing marking the gap.** 2 of the 4 pilot seeds are affected | DM-08, DM-43 · G2 · guidelines §7 MUST ("prohibit interpreting incomplete non-findings as absence"), §3 (edge selection) | **Where arcs come from.** Only `encloses_call ⋈ call_target` and the property `site_target` arcs (`projection.rs:137-159`). A call's caller is its innermost owner (§5 L1451; `cpg-schema/src/graph.rs:415`). **What Pass A emits.** Findings only from arcs (`into`) and from the unresolved sites of expanded vertices (`pass_a.rs:175-259`). Completion is complete unless a budget stops the search (`pass_a.rs:357-363`). **Pilot (Q4, Q5).** `custom_route`: 1 vertex, 0 arcs, complete, no stop reason; its nested `decorator` has 3 arcs. `resource`: 2 vertices, stop null; `add_resource` sits in the nested `decorator` returned at `resources.py:212`. In the core subsystem modules, 16 nested functions are never called by their parent. **DESIGN.** §9.1 L1749 names "the depth bound and the boundaries" as the stated model. A nested definition is neither. §4.2.3 L1210 drops Pysa's `Define` because `declarations` records the link, but no projection reads that link | Slice 1.5 builds briefs from these rows. `custom_route`'s brief then has no delegation, no boundary and no limit under a `complete_under_stated_model` invocation. An agent reads "coordinates nothing", while the method builds and appends a `Route`. For `FastMCP.resource`, the registration step is absent and no limit replaces it. Raising the depth does not help: the closure is never called, at any depth. FastMCP's public surface is decorators, so later seeds hit this too | Decide in DESIGN §5 and §9.1 (and in an ADR-0019 amendment if a stop reason is appended). **(a) Minimum:** the projection gains a fourth relation, the nested callables of each function (from `declarations` by parent). For each expanded vertex, Pass A emits an `implementation_boundary` for every nested callable no arc reaches, with a newly appended `stop_reason`, `nested_definition`, so Stage F can state the limit. About 20 lines and one codebook append. **(b) Coverage:** a declared, typed "defines" arc class that Pass A may follow, with the step marked on the witness (its own phase or modality), so the closure's calls become bounded paths. (a) keeps call semantics apart from definition (guidelines §3: no untyped union). (b) answers more of §9.1's question. Either way, §9.1's stated model names it | **None exists.** Test: `analysis_shapes` gains a decorator-factory method (`def route(self): def decorator(fn): helper(fn); return fn; return decorator`) as a seed. Assert a finding that names `route.decorator` (a boundary under (a), a path under (b)), and that the invocation is not an empty complete result |
| **F2** | **Seed member resolution skips MRO entries it cannot read.** A name defined by an external base, or by a non-`def` binding in a class body, resolves silently to a later ancestor's `def` | DM-59, DM-07, DM-43 · G7 (G3) | **The code.** `member()` reads the MRO with `t.ancestor_node_id IS NOT NULL` (`analyze.rs:274`). It looks up only `declarations`, whose kinds are `function`, `async_function` and `class`, with a parent in the chain (`analyze.rs:289-296`). It returns the first chain entry that has one (`analyze.rs:311`). External ancestors have no `declarations` rows (807 of the pilot's 1,458 MRO targets are external symbols). Class-body assignments are not declarations (792 class-scope assignments in the pilot). **The claim.** §9.1 L1737 says "each remaining name as a member of its own body or MRO". **Pilot (Q6).** 15 release classes have an external ancestor ahead of a release ancestor; for example `OAuthProxy` → OAuthProvider, AuthProvider, `mcp` TokenVerifier, `mcp` OAuthAuthorizationServerProvider, ConsentMixin. No collision exists today: ConsentMixin declares none of the 11 names those external classes define, and no class-body assignment shadows an ancestor `def`. The four pilot seeds resolve correctly | Take `class Server(ext.Base, LocalMixin)` with `tool` defined on both. At runtime `Server.tool` is `ext.Base.tool`, but the seed resolves to `LocalMixin.tool`. Every finding, alias and brief is then about the wrong function, with no error. `class Server(Mixin): tool = _impl` does the same. A new seed in increment 2, or a second library, meets this (pydantic and starlette bases are common) | Make the walk fail closed. Before accepting entry *i*'s declaration, refuse, naming the entry, when: <br>• an earlier external entry defines `<Class>.<name>` in `context_definitions`; or <br>• the owner, or an earlier release entry, binds the name in its class scope (`bindings` of a class scope) with no `def`. <br>About 20 lines. Or narrow §9.1 to what the code does and add the refusal | **None exists.** Test: an `analysis_shapes` variant with `class Shadowed(extdep.Base, Mixin)` and `class Aliased(Mixin): tool = helper`, each a seed. The compile refuses and names the shadowing entry |
| **F3** | **The finding kind and the first witness depend on the witness budget, which the contract calls presentation only** | DM-59, DM-46 · G2 | **The code.** `direct_delegation` needs `best == 1 && paths.iter().any(definite)` over the **chosen** paths (`pass_a.rs:210-215`). `chosen` is the BFS parent arc plus the next `w − 1` final arcs in canonical order (`pass_a.rs:192-197`). **The claim.** `findings.rs:76-77` ("the witness cap is presentation only") and §9.1 L1747. **Probe P1.** Four parallel sites S→A: three candidate, then one definite. `max_witnesses = 3` gives `bounded_delegation_path` with `witnesses_omitted = true`. `max_witnesses = 4` gives `direct_delegation`, and its path 0 is a **candidate** step | **Under-claim:** a function that calls a helper both through `self.x()` (override-open) and directly is reported as a bounded path whenever the definite site sorts past the cap. **Mis-citation:** when the kind is direct, the first witness, which Stage F quotes, can be the candidate site. §3.6 says a candidate site is not evidence of direct delegation. The trigger is narrow: more than *w* shortest parallel arcs, or a candidate BFS parent next to a definite sibling | Decide the kind over all shortest final arcs (`finals`), not only the chosen ones. For `direct_delegation`, make path 0 a definite arc. A few lines | **Test:** P1 as a unit test in `tests/pass_a.rs`: the kind is direct for *w* = 1, 3 and 4, and path 0 is definite. **Rule:** `semantic:direct-delegation-is-definite`, requiring every `direct_delegation`'s path 0 to be one definite step |

### 7.2 Observations (none moves a gate alone; each has its fix and oracle)

| # | Observation | Evidence | Fix · oracle |
|---|---|---|---|
| O1 | **Witnesses are shortest paths only, but DESIGN says "next paths".** `witnesses_omitted` counts only extra final arcs at the shortest depth. `stop_reason` 3, `witness_limit`, is never emitted: the cap is the flag | §9.1 L1745, L1747; `pass_a.rs:180-198`. **Probe P2:** T is reached directly and also through A. Pass A gives one witness with `witnesses_omitted = false`, and the longer route is neither shown nor flagged | Say "the next *shortest* paths" and "more shortest-path final arcs existed" in §9.1 and the `findings.rs` doc, and mark `witness_limit` reserved · test: P2 as a unit test |
| O2 | **The depth-limit signal ignores unresolved sites at the frontier.** A frontier vertex is never in `expanded`, so its unresolved sites produce nothing, and `depth_limited` counts arcs only | `pass_a.rs:117-119`. **Probe P4:** B, at depth 2, has only an unresolved site; `stop_reason` is null. That site is beyond the stated model, but the invocation then says the bound cut nothing | Set the flag when a frontier vertex has unresolved sites · test: P4 as a unit test |
| O3 | **No test has a self-loop or a cycle back to the seed**, which guidelines §12 MUST lists | Neither `tests/pass_a.rs` nor `analysis_shapes` has one. **Probe P3:** S→S, S→A, A→S, A→A gives one `direct_delegation` (A), no finding for the seed and no crash, which is correct | Add P3 to `tests/pass_a.rs` · test |
| O4 | **§5 describes a test the code does not have.** It says "Test: shuffled arc rows give an identical adjacency order". The adapter instead refuses unsorted arcs, and determinism rests on the arcs query's total `ORDER BY` | L1479; `lctx-analytics/src/graph.rs:169-173`; `the_adapter_keeps_parallel_arcs_isolates_and_refuses_disorder`. The refusal is stricter, and correct | Amend the line · the existing test |
| O5 | **Labelling external boundaries fans out,** and builtins dominate the boundaries. This is for Stage F. 26 of the 42 pilot findings are external boundaries into `builtins`, `functools` or `inspect` | Q7: `context_definitions` has one row per run for 1,109 external nodes. The test's `LABEL` join (`tests/analysis.rs:111-121`) has the same fan-out, and the fixture has no corpus to expose it. Q1 counts the builtin boundaries | Stage F labels externals through `DISTINCT symbol_node_id, module_name, qualified_name`, and does not make each builtin boundary its own Limits line · test in 1.5: a fixture with a corpus run, asserting one entry per boundary node |
| O6 | **Witness rows are not checked against the arcs they cite.** `semantic:witness-chain` checks connectivity, and `ref:witnesses.edge_id` checks existence. Nothing checks that: <br>• the edge's `(src, dst)` is the step's `(call_site, callee)`; <br>• the caller owns the site; <br>• a delegation or boundary finding's last step ends at `related_node_id`; <br>• `depth` equals the path length | `rules.rs:218`. Today these hold by construction: one `Arc` feeds every column (`pass_a.rs:324-339`) | Add one `semantic:witness-step` rule (join `edges`, and `encloses_call` or the property owner) and one `semantic:finding-path` rule · rules with injected cases in `the_analysis_rules_reject_their_violations` |
| O7 | **The owner of a site is written twice:** over `call_syntax` for `encloses_call`, and over `syntax_nodes` for property arcs | `cpg-schema/src/graph.rs:415`; `projection.rs:150`. If one changes (for example, decorator calls owned by the class), call arcs and property arcs disagree about who calls | One shared SQL fragment in `cpg-schema`, or a cross-reference comment · O6's rule covers both |

**Checked and clean.** Each item says what would break without it.
- **Sorted adjacency.** `out_arcs` sorts rows, and rows are canonical `(src, dst, call_site,
  edge_id)`. Without the sort, petgraph's newest-first `edges_directed` would pick BFS parents
  and first witnesses by insertion order.
- **Budget edges.** Both budget checks run before an arc or vertex is consumed
  (`pass_a.rs:122`, `:133`). A neighbourhood exactly at budget stays complete rather than being
  falsely marked `partial`. Boundaries use the arc budget, not the vertex budget.
- **No silent drops in the projection joins (Q3).** On the pilot there are no `call_target`
  without `encloses_call`, none without a resolution, no resolution without `encloses_call`, and
  no duplicate per site. A duplicate would be refused by the adapter's strict order check, not
  published twice.
- **Exclusions follow the declared policy (Q2).** Excluded: 1,893 `higher_order_target`, 18,605
  `potential_target` and 29,446 `synthetic_model` `site_target` edges. Included: 1,591 property
  arcs.
- **Unresolved sites of both kinds are surfaced.** This covers partial sites (130, with known
  targets plus a remainder) and not-attempted ones (285, `outside_provider_model`). The reason
  can be recovered by joining `resolutions` on `related_node_id`. Both pilot sites are partial
  with 2 targets.
- **The primary seed stops honestly.** `FastMCP.tool` → `ToolDecoratorMixin.tool` (candidate) →
  `decorate_and_register` stops at depth 2. That is one hop before `add_tool`, and it is recorded
  as `depth_limit`. Candidate arcs are followed, so the candidate modality is not what limits
  reach. Every pilot delegation is override-open, so the pilot has no `direct_delegation`
  (§3.6). A different depth must come from a pre-registered rule, not from scoring (ADR-0019 F3).
- **Aliases.** Each seed's aliases are the container's three exported access paths, extended by
  the member name, for the inherited `custom_route` too.
- **Provenance.** `parameters` holds seed, budgets, prefixes and roots, and each invocation
  carries the projection digest and the locked library versions.

### 7.3 Applicability and verdicts

**Groups that bore on this scope:**
- 2 (invariants: F2, O6);
- 5 (derivation contracts: F1, F3);
- 7 (distinct relation structures: F1's choice between a definition and a call);
- 8 (ordering and determinism);
- 10 (lineage: F3, O6);
- 11 (known-answer tests: O3).

**Groups that did not:**
- 3 (identity) was settled by the ADR-0019 review.
- 6 (effects and publication) is unchanged from slice 2.
- 9 (providers): no new provider.
- 12 is covered in §8.

**Verdicts:**

| Verdict | Principles |
|---|---|
| Violated | DM-08 and DM-43 (F1); DM-59 (F2, F3) |
| Satisfied | DM-34 (a separate call projection; definitions are not unioned in); DM-40 (canonical order, refusal of disorder, a determinism test); DM-07 for the adapter and config (the resolver gap is F2); DM-46 except the path-0 mis-citation (F3) and the unchecked step↔edge link (O6) |
| Unresolved | DM-54 for self-loops (O3; the behaviour is correct by probe, but no repo test covers it) |

## 8. Alternatives (compressed)

| Alternative | Duplication and extension | Risks | Cost | Performance evidence | Verdict |
|---|---|---|---|---|---|
| As built: petgraph `Graph` plus a hand-written BFS | One adapter per projection (§5 recipe) | None found in the adapter | Small | Pass A 0.35 s on the pilot (author's figure) | Keep |
| F1 (a): a nested-definition boundary | One relation and one stop reason | Coverage stays call-only, but the gap becomes visible | About 20 lines | — | The minimum that clears G2 |
| **Simpler:** BFS over the sorted arc arrays, with no petgraph | Out-arcs of *u* are a contiguous row range, because arcs sort by `src`. `out_arcs` today calls `edges_directed` and then re-sorts into that same order (`lctx-analytics/src/graph.rs:222-229`). Nothing else in 1.4 calls petgraph; `mask` builds a `FixedBitSet` | None | Removes one layer from Pass A | Nothing measured to gain | **Not recommended now.** §B4 and ADR-0011 name petgraph for SCCs and filtered views in increment 2, and the build is cheap. Revisit if no increment-2 analysis uses the container (DM-58) |

## 9. Top verification gaps

| Claim or risk | Label now | Oracle | Gap |
|---|---|---|---|
| Decorator-factory seeds are not reported as coordinating nothing (F1) | Violated (pilot Q4, Q5) | the `route.decorator` fixture test | none exists |
| Seed resolution matches Python's MRO (F2) | Implemented for `def`/`class` of release classes only | the shadowing fixture test | none exists |
| The witness cap is presentation only (F3) | Violated (probe P1) | P1 unit test plus the direct-is-definite rule | none exists |
| Shortest-only witnesses, frontier unresolved sites, self-loops (O1–O3) | Tested by the review probes only | unit tests | not in the repo |
| Vertex-budget `partial` through the whole attempt with a Limits entry | Tested on the hand-built projection only (`budgets_truncate_and_say_so`) | ADR-0019 F4 disposition: `analysis_shapes` with a vertex budget of 1 | owed by 1.5 |
| Pass A cost | Measured by the author (`just pilot`, 2026-09-23) | `just pilot` | not re-run here |

## 10. Exceptions and unresolved decisions

There are no SHOULD exceptions. One decision is unresolved: F1's choice between (a), a nested
definition as a stated-model boundary, and (b), a typed "defines" arc Pass A may follow. It
changes what §9.1 promises, so it belongs in DESIGN §5 and §9.1, and in ADR-0019 if a stop
reason or arc class is appended.

## 11. Decision

**Decision: Revise (small).** G2 fails on in-scope behaviour. Two of the four pre-registered
seeds publish a complete, empty or near-empty neighbourhood while their behaviour sits in a
closure the projection cannot reach (F1). G7 fails narrowly on the seed-resolution claim (F2).

Everything else holds, which is what makes the fixes small:
- the adapter and traversal are deterministic and order-safe;
- lineage is kept;
- exclusions are declared;
- the projection's joins lose nothing on the pilot.

| Priority | Change | Principle IDs | Acceptance evidence | Regression protection |
|---|---|---|---|---|
| P1: before Stage F builds briefs for `custom_route` and `resource` | F1: decide (a) or (b), amend §5 and §9.1, and implement | DM-08, DM-43 | the pilot's `custom_route` invocation shows the nested `decorator` | the `route.decorator` fixture test |
| P1: same commit | F2: narrow §9.1 L1737 to what the code does, or implement the fail-closed walk | DM-59 | the DESIGN line matches `member()` | the shadowing fixture test (with the code fix) |
| P2: by 1.5, since Stage F quotes path 0 | F3: kind over all `finals`; path 0 definite for a direct delegation | DM-59, DM-46 | P1 as a test | unit test plus `semantic:direct-delegation-is-definite` |
| P3 | O1, O2, O3, O4, O6: the wording, the frontier flag, the self-loop test, the §5 line and the witness-step and finding-path rules | DM-59, DM-54, DM-46 | per §7.2 | per §7.2 |

### Deferred

| Item | Why not now | Reopen when |
|---|---|---|
| F2's code fix (if §9.1 is narrowed instead) | No pilot seed is affected (Q6) | Any seed is added, or a second library is compiled |
| O5 | Stage F's concern | Slice 1.5 labels boundaries or writes Limits |
| O7 | The two copies agree today | Either owner rule changes |
| §8's petgraph-free BFS | ADR-0011 names later consumers | Increment 2 ends with no analysis using the `Graph` container |
