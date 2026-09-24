# Design review: increment 2 end (slices 2.4–2.6; earlier fixes checked), compact

## 1. Decision and scope

**Decision: Revise (small), before increment 3 raises the brief budget.**

G2 fails, for three reasons:
- Stage F picks a seed's applicable case and implications from **any** FCA scope that holds the
  seed's node, but the text names the seed's own scope. A legal config publishes a false
  `structurally_observed` sentence (review probe, F1).
- The FCA attribute relation turns Pyrefly's `Unknown` (an `Any` of kind error or implicit) into a
  shared "type". It is in 22 of the pilot's 107 implication findings (F2).
- Related names operations through arbitrary subclasses, in all four published pilot Related
  lines (F4).

The kernels themselves hold:
- NextClosure with support pruning is correct. The review's probe checked the frequent basis for
  soundness, completeness and non-redundancy at supports 1–4.
- The PageRank iteration agrees with `compute_flow` on 30 random graphs.
- Selection is deterministic and stays within the budget.
- Related stays statistical in both text and status.

Increment 3 is where most of this becomes consequential:
- PageRank over delegation ranks implementation sinks above the operations that official usage
  calls.
- Selection takes one API per community per round, so at a budget of 25 it would fill every
  added slot with helpers (F3).
- The Applicable-case slot is now filled by construction, which removes it from the §B11 gap
  metric (F5).

Both F3 and F5 are decisions the author owes before the 3.3 gold scoring. No §B decision is
reopened.

**Proposal:**
- slice 2.4 (`9b9d09e`): `lctx_analytics::ranking` and the usage projection;
- slice 2.5 (`a34613a`):
  - `cpg_schema::concepts` and `lctx_analytics::concepts`;
  - the applicable-case and implication templates;
  - `chunked`;
- slice 2.6 (`68010a7`):
  - `lctx_analytics::selection`;
  - the Stage E reordering;
  - the Related template;
  - the statistical-policy injected cases.

This review stands in for the per-slice compact reviews of 2.4–2.6 (D33). For 2.1 and 2.2
(`30561df`, `ebcf377`) and 2.3/ADR-0011 (`db36d9a`), it checks only that their fixes hold together
with the later slices.

**Status:** Implemented. The Tested scope is audited in §9.
**Reviewer / author:** design-reviewer subagent (fresh context) / the increment-2 session.
**Affected revisions:** `main` at `68010a7`. Pilot snapshot `8e1e1f190268db53149fc3ed08d6018f`
(generation `99a282da8ce3b738`).

**Observable outcome:**
- Each brief gains:
  - an Applicable case (`structurally_observed`, entering the brief document's header);
  - up to three implications (Important controls);
  - a Related line (`statistically_derived`).
- With room in the budget, the communities and centrality choose extra seeds.
- On the pilot the budget equals the five seeds, so nothing is selected (`seed_selection`
  diagnostics: `"selected":[]`).

**Supported scope and non-goals:**
- FCA covers one scope per distinct container, over parameter names, declared parameter and return
  types, direct typed raises, and decorators.
- RCA, kNN and extra community layers are not in scope (increment 3).

### Method and coverage

**Read at `68010a7`** (a `git archive` export; the working tree carried another session's
uncommitted kNN work, and none of it is reviewed):
- the whole of `lctx-analytics/src/{ranking,concepts,selection}.rs`;
- the whole of `cpg-schema/src/concepts.rs`;
- `cpg-schema/src/communities.rs` `public_callables_sql`;
- `lctx-analytics/src/communities.rs:215-238`;
- `cpg-core/src/analyze.rs`:
  - 180-400 (`resolve_seeds`, `member`);
  - 526-790 (the reordered Stage E and selection);
  - 1034-1168 (FCA scopes);
- `synth.rs`:
  - 375-435 (`chunked`, `attributes_text`);
  - 1500-1676 (applicable case, implications, Related);
  - 2035-2111 (the brief document);
- `attempt.rs:428-470`;
- `findings.rs` (`FINDING_STATUS`, `ASSERTION_POLICY`, `ANALYSIS_BACKED`);
- `codebook.rs:800-1014`;
- the slice tests and snapshots:
  - `analysis.rs:504-700` and its 2.6 diff;
  - the `bundle.rs` diff;
  - the `centrality_top`, `pagerank`, `seed_selection`, `fca_assertions` and `briefs` snaps;
- the design authority:
  - DESIGN §1.2, §1.5, §9 intro, §9.4 (2.6 part), §9.5, §9.6, §10.2–§10.4 and §11.1 (document
    text);
  - D28–D34;
  - `eval/gold/analytics-freeze.json`;
  - ADR-0011 (Options, Decision);
  - ADR-0005 (statistical output);
  - the ADR-0011, C2 and C4 reviews' deferred rows.

**Ran:**

| Check | Command | Outcome |
|---|---|---|
| Workspace tests at `68010a7` | `INSTA_UPDATE=no cargo nextest run --workspace --no-tests=pass` in the export, `CARGO_TARGET_DIR` inside it | **passed**: 183/183 |
| Probe 1: the frequent DG basis at supports 1–4 on 117 random contexts (sound; complete over every frequent set; no implication follows from the others) | an ad-hoc test in the export's `concepts.rs` | **passed** |
| Probe 2: `pagerank` against `compute_flow` on 30 random weighted digraphs (5–44 vertices, about 25% dangling) | an ad-hoc test in the export's `ranking.rs` | **passed** (agreement within 1e-9) |
| Probe 3: seeds `pkg.Catalog.add_tool` and `pkg.registry.Catalog.add_prompt` (two paths to one class) | an ad-hoc `compile_config` test | the compile is **refused**: `key:findings (7 rows), key:finding_members (43 rows)` |
| Probe 4: the fixture plus `class Widgets(Catalog)` with two registration methods, and seed `pkg.Widgets.remove` | an ad-hoc `compile_config` test | compiles, and publishes the false text quoted in F1 |
| pytest, rule tests, `adr lint`, `just pilot` | — | **not_run**. The pilot is read-only evidence (`lctx query` on `8e1e1f19`) |

The probes existed only in the scratch export, which has been deleted.

**Measured by the review** (SQL on `8e1e1f19`; the method is stated wherever a number is used):
- the ranking and its direct official-usage counts (call sites in example, test and doc-block
  modules, from `call_targets`; an approximation, not the usage projection's arcs);
- a simulation of `select`'s first round from the published communities, centrality findings
  and docstrings;
- the FCA findings carrying sentinel attributes.

**Not inspected, or asserted only:**
- Leiden internals (reviewed at 2.3).
- Pass B and Pass C internals beyond the coherence of their fixes.
- FCA time on large scopes: the budget counts closed sets, not time, and `implication_closure` is
  quadratic in the basis. Unmeasured beyond the pilot's 205 closed sets.
- The rebinding branch of ADR-0011 F6 on the pilot. Only the outside or unresolved-ancestor
  branch was checked (0 instances).

## 2–4. Authority, contracts and derivation (compressed)

| Concept | Authority | Where | Notes |
|---|---|---|---|
| Usage projection | `WEIGHT_POLICY` string + `UsageGraph::build` | `ranking.rs:28-29`, `78-118` | Digest = invocation spec ⊕ policy string. Takes every arc into a subsystem function from a subsystem function or an Example/Test/DocBlock caller, **whatever its `arc_kind` or modality** (`:88-93`). The string names neither |
| PageRank parameters | `ranking::Params` | `:48-64` | 0.85, 1e-10, 100; frozen (`pagerank_parameters`) |
| FCA attributes | `concepts::attributes_sql` | `cpg-schema/src/concepts.rs:41,46,51` | `'… ' \|\| t.display`: Display strings, not C4's term structure |
| FCA scope | the container **string** of each seed's access path | `analyze.rs:1037-1104` | One invocation per string, with the subject the class node the string resolves to |
| Applicable case / implications chosen | Stage F | `synth.rs:1506-1626` | Rule D32; not in the gold freeze |
| Seed selection | `selection::select` + `analyze.rs:700-790` | `selection.rs:25-70` | `RULE` recorded in the invocation; not frozen |
| A node's displayed path | "least access path", re-derived three times | `communities.rs:231-234`, `ranking.rs:219-222`, `analyze.rs:756-759` | First row of `ORDER BY node_id, access_path` |
| Related | Stage F | `synth.rs:1628-1675` | Cites the community and each member's `centrality`; derives `statistically_derived` |

**Stage order.** Stage E runs communities, then PageRank, then selection, then Passes A–C per
seed, then FCA per scope. All of it is composed in memory (`AnalysisRows`). Each analysis table is
written once (`attempt.rs:432-450`), and Stage F reads `found` in memory (`:453-454`). ADR-0017's
one-commit invariant therefore holds (ADR-0019 review O6: the code complies, but the statement is
still owed; see Deferred).

**Reordering Stage E changed no earlier output.**
- In `68010a7` the only snapshots that changed were `briefs.snap` (+3 Related lines), the new
  `seed_selection.snap`, and the codebook and rule value sets.
- The Pass A/B/C, community and FCA snapshots are unchanged.
- The one intended change is that the default fixture config (budget 6, five seeds) now selects
  `pkg.helpers.finish` (bundle test: 5 → 6 briefs; pytest invocations 59 → 64).

## 5. Journey: increment 3 raises the budget (an ADR-0004 amendment), for example to 25

1. **Selection** (`selection.rs:34-68`, with documented members only):
   - 29 communities are reported, 27 of them with a documented member.
   - 26 hold no configured seed, so round 1 visits them first, and a room of 20 is filled
     before the 78-member server community is reached.
   - The review's simulation of round 1 picks, in order:
     - `fastmcp.tools.FunctionTool.from_function`
     - `fastmcp.server.providers.AggregateProvider.add_provider`
     - `fastmcp.Context.request_context` (a property)
     - `fastmcp.prompts.FunctionPrompt.make_key` (this is `FastMCPComponent.make_key`)
     - `fastmcp.prompts.FunctionPrompt.key`
     - `fastmcp.server.dependencies.get_http_request`
     - `fastmcp.resources.ResourceContent.__init__`
     - `…match_uri_template`
     - … then middleware `__init__`s and `ProgressLike.increment`.
   - `FastMCP.call_tool`, `list_tools` and `add_transform` (320, 250 and 180 direct
     official-usage sites, all documented, all in the server community) are never selected
     (**F3**).
2. **Naming.** Each selected node takes its least access path as its seed path, brief title and
   FCA container (`analyze.rs:756-781`, `1037-1041`). `make_key`'s brief would be titled through
   `FunctionPrompt`, and its FCA scope would be `FunctionPrompt`'s namespace (**F4**).
3. **FCA.** New scopes appear, among them `fastmcp.FastMCPApp` (from `get_app_tool`). Like
   `FastMCP`, it is a `Provider` subclass, so the two scopes share inherited `Provider` methods.
   Any seed whose node is in both scopes can then publish a concept of the other scope under its
   own scope's name. A configured seed's `structurally_observed` Applicable case now depends on
   which scopes statistical selection created (**F1**, and ADR-0005's spirit).
4. **Resolution.** `resolve_seeds` walks `member()` over each least path. ADR-0011 F6's mismatch
   would now refuse the compile, where before it only mislabelled a community member (0 pilot
   instances today; Deferred).

## 6. Acceptance gates

| Gate | Result | Evidence or scope rationale | Required action |
|---|---|---|---|
| G1 Authority | **pass** | `FINDING_STATUS` and `ASSERTION_POLICY` generate the rules. One attribute relation, digested. The DESIGN spine is stale in five places (F8). "A node's displayed path" is re-derived in three places, all identical today (F4) | F4, F8 |
| G2 Semantic fidelity | **fail** | A structurally observed Applicable case lists another scope's APIs under the seed's scope (F1, probe 4). `Unknown` is a shared FCA attribute in 4 concept and 22 implication findings on the pilot, and `raise E` vs `raise E()` are two attributes (F2). All four published pilot Related lines name operations through unrelated subclasses (F4). The Applicable-case slot's meaning is ambiguous between §10.3 and §9.6 (F5) | F1, F2, F4; U2 |
| G3 Validity | **pass** | Every new failure path is fail-closed: an over-cap part (`chunked`), duplicate seeds, key violations. Probe 3 shows a *valid* config refused with an unhelpful error, which is part of F1's cause | F1(b) |
| G4 Hidden behavior | **pass** | No ambient input. FCA and PageRank parameters are frozen. Stage F's selection knobs are declared and versioned, but not frozen (F7) | F7 |
| G5 Consistency and recovery | **pass** | In-memory composition and one write per table (§2–4). Probe 3's refusal published nothing ("validation failed, nothing published") | — |
| G6 Transformation and reuse | **pass** | `pass_a_is_identical_across_module_order_and_location` compares every Stage E and F table, centrality, FCA and the selected seed included. Parameters and digests are recorded. The usage projection's arc policy is unstated (F3(c)) | F3(c) |
| G7 Truthful capability claims | **pass (label corrections)** | Claimed routes exist. §9.5's "Tested" covers the kernel and delegation, but not the usage-caller branch (the fixture has `usage_callers: 0`). §9.6's completeness test runs only at support 0 (F6) | F6 |

## 7. Findings

Ordered by severity: correctness first (F1, F2, F4), then the consumer semantics that decide
increment 3's briefs (F3, F5), then oracles, freeze and spine (F6–F8).

| # | Finding | Principle IDs | Concrete evidence or gap | Consequence | Proposed correction | Verification |
|---|---|---|---|---|---|---|
| **F1** | **An FCA scope's identity is its container string, and Stage F does not restrict a seed's concepts to its own scope** | DM-11, DM-24, DM-59 · G2 (G3 for (b)) | (a) `synth.rs:1522-1531`: `holding` is every `applicable_case` finding whose extent contains the seed, from any scope. `:1510-1512`: `scope_label` comes from the seed's access-path string. `:1580`: the implications' scope is `holding.first()`'s subject. **Probe 4** publishes, `structurally_observed`: "`pkg.Catalog.add_tool` belongs with `pkg.Widgets.add_prompt`, `pkg.Widgets.add_gadget`, `pkg.Widgets.add_widget` and `pkg.Widgets.add_resource` (5 public APIs of `pkg.Catalog`)". `pkg.Catalog` has no `add_gadget` or `add_widget`. (b) `analyze.rs:1037-1104` builds one scope per container string. Two strings naming one class give identical implication keys (members carry only attribute labels), against the comment at `:1165`. **Probe 3**: refused with `key:findings (7 rows)`. The pilot is latent (one scope, `fastmcp.FastMCP`) | (a) A false structural claim reaches the brief and its retrieval header whenever a seed's node is inherited into another seed's scope. In increment 3 that is FastMCP and FastMCPApp sharing `Provider` methods (§5). A configured seed's structural text then changes with statistical selection. (b) A config that names one class through two exports cannot compile, and the error names no cause | Key scopes by the class or module node: one invocation per node, labelled by its preferred path (F4). Record each seed's scope node in Stage E and filter `holding` and the implications by it. Take `scope_label` from the scope, not the seed path. About 20 lines | **None exists.** Probes 3 and 4 as tests: a fixture subclass inheriting a seed's method with both scopes present, asserting that every API an applicable-case text names is in the seed's scope; and two export paths to one class compiling |
| **F2** | **The FCA attribute relation reduces C4's typed terms to Display strings, so sentinels become shared attributes and one raise becomes two** | DM-08, DM-42, DM-06 · G2 | `cpg-schema/src/concepts.rs:41,46,51` concatenate `t.display`. On the pilot, `Unknown` is `type_terms.kind = 16` (Any) with `detail` `error` or `implicit`, or a union of them. **Pilot findings (published):** `returns Unknown` is in 4 of 97 concept and 22 of 107 implication findings. Example: the implication `returns Unknown → parameter tags, parameter type set[str] \| None` (support 4: `enable`, `disable`, `from_openapi`, `from_fastapi`). **Raises:** 27 raise observations are `ClassObject` (`raise E`), displayed `type[E]`, e.g. 14 `type[NotImplementedError]` against 12 `NotImplementedError`. `synth.rs:415` renders "the raised type `E`", without §9.6's promised attribute scope (typed, direct raises only; C4 O4's re-raises excluded). No published pilot *assertion* uses these yet | If `FastMCP.enable` becomes a seed, its Important controls say "every one that has the return type `Unknown` also has the parameter `tags` …" as `structurally_observed`. Two APIs whose returns failed to type are grouped as sharing a return type. An API raising `E` both ways splits across two attributes, so implications over `raises E` under-count. "Every one that has the raised type `E`" reads as "every API that can raise `E`" | Build attributes from term structure: no attribute for `Any` of kind error or implicit (or for unions containing one); map a `ClassObject` raise to its class (C2 O4's identity question). Phrase attributes with their scope ("declares …", "raises `E` in its body"). Bump the relation digest | **None exists.** A fixture function with an unresolvable return annotation, and a `raise E` / `raise E()` pair, asserting no `Unknown` attribute and one `raises E`. Plus a cheap `semantic:` rule: no `intent_attribute`, `premise` or `conclusion` label contains `Unknown`, with an injected case |
| **F3** | **Centrality answers "what does delegation reach", while its consumer asks "which operation should get a brief". The selection rule then spends the budget one per community** | DM-22, DM-59; guidelines §3, §4 · — (decision U1) | (a) `ranking.rs:88-93` with uniform teleport: a callee's rank sums its callers'. **Pilot, reviewer-measured direct usage sites** (`call_targets` from example, test and doc-block modules): PageRank's 6th, `AggregateProvider.add_provider`, has 0; the 8th, `Provider.wrap_transform`, has 0; the 4th, `FunctionTool.from_function`, has 15; the 5th, `ToolDecoratorMixin.add_tool`, has 8. Meanwhile `FastMCP.call_tool` (320) is 16th, `list_tools` (250) 27th and `mount` (198) 43rd. The fixture shows the same shape: its top rank is the sink `pkg.helpers.finish`, which the bundle test's selection picks ("which calls nothing"). (b) The round rule (`selection.rs:44-68`) visits all 26 unseeded communities before the server surface (§5). (c) The usage projection counts definition and candidate arcs at 1 each, and neither `WEIGHT_POLICY` nor §9.5 says so (ADR-0011 review F3 fixed the same gap for communities) | Increment 3's 15–25 briefs would be chosen for plumbing (key helpers, properties, middleware constructors), named through subclasses (F4). The Related line's "by usage centrality" leads with delegation targets. The §9.8 ablation compares against ADR-0011's Option 2, which is also PageRank-based, so it cannot expose this. Changing the rule after 3.3 would be tuning on the gold | **U1, the author's decision before the budget rises and before 3.3:** state what the centrality consumer asks. The simplest candidate ranks documented public APIs by direct official-usage in-weight (usage caller → API arcs only, or PageRank personalized to usage callers with no onward flow), with communities as a diversity cap rather than strict rounds. Keep delegation PageRank only if the ablation shows it helps. Name the arc kinds in the policy. Record the decision as an ADR-0011 amendment plus a D-row, and freeze its digest (F7) | **None exists.** A `docs_shapes` (usage-bearing) test asserting that an API usage calls directly outranks its delegation sink. The §9.8 ablation for selection should use a direct-usage baseline |
| **F4** | **A node is named by its lexicographically least access path, which is often an unrelated subclass. The rule lives in three places** | DM-02, DM-24 · G2, G1 | `communities.rs:231-234`, `ranking.rs:219-222`, `analyze.rs:756-759` each keep the first row of `ORDER BY node_id, access_path`. **Pilot Related (all four lines):** `fastmcp.tools.base.Tool.from_function` (exported as `fastmcp.tools.Tool`) appears as `fastmcp.server.providers.fastmcp_provider.FastMCPProviderTool.from_function`, and `ToolDecoratorMixin.add_tool` as `fastmcp.server.providers.FileSystemProvider.add_tool`. The server community also lists `Provider`'s inherited methods as `fastmcp.FastMCPApp.add_transform` and `fastmcp.FastMCPApp.list_tools`. 63 community members' least paths run through a class other than the declaring one | `from_function` is a classmethod, so calling it on `FastMCPProviderTool` binds a different `cls`: the published path names a different call. Agents read `FileSystemProvider.add_tool` as filesystem-specific. Selected seeds inherit these titles and FCA scopes (§5). Fixing one site and not the others makes Related labels and seed paths disagree | One preferred-path rule in `cpg_schema::communities` (for example: fewest segments, then the declaring class's own export, then least), returned as a column of `public_callables_sql` and used by all three consumers | **None exists.** A fixture subclass that sorts before its base (e.g. `class Alpha(Catalog)`), asserting that Related and selection name `pkg.Catalog.add_tool` |
| **F5** | **The Applicable-case slot is filled by construction, so the gap metric stops seeing it, and the text puts other APIs' names in the retrieval header** | DM-08, DM-59 · G2 (decision U2) | Rule `synth.rs:1532-1545`: any concept with ≥ 2 shared attributes. The floor was set after a pilot run (D32, disclosed). **Pilot:** `mount` "belongs with `http_app`, `run_http_async`, `__init__` and `resource` … each has a parameter typed `str \| None` and the raised type `ValueError`". `custom_route` "belongs with `render_prompt`, `get_tool`, `call_tool`, `resource`, `get_prompt` and 1 more … the parameter `name` and a parameter typed `str`". Both lines lead the brief document, as its header in every chunk (`synth.rs:2046-2049`). §10.3 calls the section "Applicable input or mode"; §9.6 says "concepts become `applicable_case` findings". "Belongs with" says more than "shares" | Applicable-case `absent_slots` went from 5 to 0 on the pilot, though 2 of the 5 are coincidental overlaps. §B11's LLM-trigger decision (increment 5) reads a smaller gap than exists. `custom_route`'s retrieval text now carries `get_tool` and `call_tool` (unmeasured; fake embedder) | **U2, the author's decision:** either (a) publish the concept as a non-slot line ("shares its signature with …"), kept out of `SLOT_SECTIONS` and the document header until an input or mode source exists; or (b) keep the slot, with a structural floor fixed before looking at output and frozen, so that a seed below it keeps the slot absent. Replace "belongs with" with "shares" | **None exists.** A fixture seed below the floor with `applicable_case` in `sections_absent`. The 3.3 ranking check with and without the case text |
| F6 | **Labels and oracles are narrower than the claims** | DM-53, DM-59 · G7 | (a) §9.5 claims the usage projection **Tested**, but the fixture's diagnostics show `usage_callers: 0` (`analysis__pagerank.snap`), so no test exercises the usage branch (`ranking.rs:81-86`). (b) `shuffled_rows_give_identical_scores` cannot fail: both inputs go through a `BTreeMap` before `pagerank` (`ranking.rs:298-304`, `362-378`). (c) Basis completeness is tested at support 0 only (`concepts.rs:547-586`), while production runs at 2. (d) `compute_flow_agrees` uses one 6-vertex graph | A regression in which arcs count as usage, or in pruned-basis completeness, passes every test | Relabel the usage branch **Measured (pilot)** until tested. Replace (b) with a permuted `Projection` into `UsageGraph::build`. Adopt probes 1 and 2 | The probes as tests; a `docs_shapes` ranking test (shared with F3) |
| F7 | **The gold freeze covers the analytics parameters but not Stage F's selection knobs** | DM-28, DM-59 · G4 (ADDENDUM Q15) | The freeze pins `communities`, `pagerank` and `fca` parameter digests (`analytics-freeze.json`). The applicable-case rule and floor (`synth.rs:1532-1545`), the implication count 3 (`:1611`), Related's cap 5 (`:1654`), selection's `RULE` and the documented filter change only `TEMPLATE_VERSION` or `COMPILER_OUTPUT_VERSION` | After 3.3, any of these can be retuned against gold scores with no ADR-0004 amendment. D32's floor already shows the pattern: it was adjusted on pilot output | Digest a `StageFParams` (the rule strings and constants) into the freeze beside the others, after U1 and U2 settle | `the_parameters_match_their_gold_freeze`, extended |
| F8 | **The spine contradicts the code in five places** | DM-02, DM-59 · G1 | §9.5 L2090-2092 "Owed … recorded in the analytics-config digest" (D29 says code). §10.2's policy table (L2287-2302) lacks `applicable_case` and `implication`. §10.4 L2384-2385 and §11.1 L2449-2450 "split by applicable case", where §10.3 L2367-2371 and `synth.rs:2066-2081` chunk at whole parts. §9.6 L2134-2135 names Python `concepts` as the cover-relation oracle, but no cover relation is computed. The codebook doc `codebook.rs:986-987` says "most specific … share the most attributes", against D32's pair count | An implementer following §10.4 or §11.1 would split briefs by case, and one reading §10.2 cannot find the FCA kinds' permitted statuses | One DESIGN edit, plus the codebook doc comment (a doc edit, not a code change) | **None exists** (prose). `adr lint`'s status check does not cover this |

**Observations** (recorded so their silence is not read as clean):
- **O1. Selection drops without a record.** A selected node whose least path resolves to another
  node is dropped, is not recorded, and is not replaced (`analyze.rs:776-781`). The
  `docstring IS NOT NULL` filter stands in for "has a documented Outcome". A documented,
  call-free seed whose docstring opens with a section header would refuse the compile under
  `semantic:documentation-only-has-outcome`. No pilot instance: nothing is selected.
- **O2. Implications are scope trivia.** The same three appear in the `tool`, `resource` and
  `prompt` briefs ("every one that has the parameter `tags` also has a parameter typed
  `set[str] | None`"), under Important controls, and they say nothing specific to the seed. They
  are the implication kind's candidate for the §9.8 ablation, not a defect.
- **O3. The fixture was shaped to the rule.** 2.6 added docstrings to `pkg.helpers.finish` and
  `prepare` so that selection would choose them. The test therefore pins the rule's current
  output, not its intent (see F3).

**Applicability.**
- **Applied:** groups 2, 3 and 5 (typing, identity and derivation: F1, F2, F4); 10 (lineage of
  names and scopes); 11 and 12 (oracles, proportionality: F6, O2); and 6 for publication (G5,
  checked).
- **Not applied:**
  - group 4 (no declarative-composition change);
  - group 8 beyond DM-39's note on FCA time;
  - group 9 (no new adapter or provider);
  - group 7's concurrency (Stage E is sequential and in memory).

## 8. Alternatives (brief)

| Alternative | Consumer fit | Cost | Why |
|---|---|---|---|
| Current: delegation PageRank + community rounds; FCA concept as the Applicable case | Ranks sinks (F3); the slot filled by construction (F5) | Built | — |
| ADR-0011 Option 2 (no communities; PageRank + exports) | Same sink bias | Less | It is the §9.8 baseline, so it cannot reveal F3 |
| **Simpler: rank documented public APIs by direct official-usage in-weight; communities only cap per-group picks and fill Related** | The pilot's direct top 15, before the documented filter, is `FastMCP.__init__`, `tool`, `resource`, `call_tool`, `list_tools`, `mount`, `add_transform`, `add_provider`, `Tool.from_function`, `add_middleware`, `add_extension`, `Tool.from_tool`, `read_resource`, `Provider.disable`, `run` (reviewer SQL). `__init__`, `add_middleware` and `from_tool` have no docstring | One aggregate over the usage projection's own arcs; no new machinery | Recommended as U1's baseline, and as the ablation comparison for selection |
| Applicable case: no FCA fill; publish the concept as a non-slot "shares its signature with" line | An honest gap metric | Smaller than today | Recommended as U2's option (a) |

## 9. Verification: increment 2, label audit and top gaps

| Claim | Label deserved | Evidence | Gap |
|---|---|---|---|
| §1.2 row 2: Passes B and C, single-layer communities with seed consensus, `page_rank`, FCA within a structural scope | **Implemented, Tested** on fixtures; pilot **Measured** | 2.1–2.6 tests; nextest 183/183 (this review) | FCA "within one structural scope" is violated in text by F1 |
| §1.5, derivation families end to end on the pilot | **Measured** for facts, method, finding, assertion, evidence and brief: every brief has `coordinates`; `control` in tool, resource and prompt; `restriction` in four; `handoff` in four (query on `8e1e1f19`) | assertion counts per brief | Retrieval by task wording is not shown for any family. It failed at 1.9 and is deferred to 3.3 by design. Nothing in increment 2 claims it |
| §9.4 selection and Related | **Implemented, Tested** (fixture) | `select` unit test; `communities_select_seeds_within_the_budget`; the two injected policy cases | Never exercised on the pilot (budget = seeds) |
| §9.5 usage projection | **Tested** (kernel, delegation); **Measured** (usage branch, pilot only) | F6(a) | a usage-bearing fixture |
| §9.6 NextClosure and the DG basis | **Tested** at support 0; frequent basis at supports 1–4 **Tested by the review's probe only** | F6(c) | adopt probe 1 |
| §9.6 "Every seed has an applicable case" | **Measured**, but the slot semantics are undecided | F5 | U2 |
| ADR-0005: statistics never state a control or a limit | **Tested** | policy (`related` only statistical); the two injected cases in `the_analysis_rules_reject_their_violations`; pilot Related status 2 in all four | — |

## 10. Deviations assessed

| D | Assessment |
|---|---|
| D29 (PageRank parameters as code; the usage projection in Rust) | Sound as provenance: recorded, digested, frozen. The policy's *meaning* is F3, and its arc kinds are unstated (F3(c)) |
| D32 (FCA scope; applicable-case rule) | The scope rule is structural, but keyed by string (F1). The floor, adjusted after a pilot run, is disclosed. It has no freeze (F7), and the slot's meaning is open (F5) |
| D33 (batched reviews) | Proportionate: the three slices share one consumer path. This review covers them at slice depth |
| D34 (documented-only selection) | Right in direction. It is a proxy for "has a documented Outcome" (O1), and it does not stop the sink bias (F3) |

## 11. Decision and implementation changes

**Decision: Revise (small).** G2 fails on published findings and Related text (F2, F4), and on text
a legal config publishes (F1). The kernels, determinism, publication and the ADR-0005 policy hold.
F3 and F5 are decisions (U1, U2) that must precede increment 3's budget increase and the 3.3 gold
scoring.

| Priority | Change | Principle IDs | Acceptance evidence | Regression protection |
|---|---|---|---|---|
| **P1** | F1: scopes keyed by node; Stage F restricted to the seed's scope | DM-11, DM-24, DM-59 | Probe 4 lists only `pkg.Catalog` APIs, and probe 3 compiles | Probes 3 and 4 as tests |
| **P1** | F2: attributes from term structure; no `Unknown`; `ClassObject` raises normalized; attribute scope phrased | DM-08, DM-42 | 0 pilot FCA findings carrying `Unknown` | fixture case; `semantic:` label rule with an injected case |
| **P1** | F4: one preferred-path rule, used by all three consumers | DM-02, DM-24 | Pilot Related names `fastmcp.tools.Tool.from_function` (or the declaring path) | subclass fixture test |
| **P2** | U1 (F3): decide the centrality or selection objective; name the arc kinds; ADR-0011 amendment + D-row | DM-22, DM-59 | The §5 simulation re-run under the new rule, recorded in the D-row | `docs_shapes` ranking test; ablation baseline |
| **P2** | U2 (F5): decide the Applicable-case slot's meaning; "shares" wording | DM-08, DM-59 | Pilot `absent_slots` reflects the decision | fixture `sections_absent` case |
| P3 | F7: freeze Stage F's knobs after U1 and U2 | DM-28, DM-59 | freeze file entry | extended freeze test |
| P3 | F6: relabel; adopt probes 1 and 2; a real shuffle test | DM-53, DM-59 | — | the tests |
| P3 | F8: spine and codebook doc edits | DM-02 | — | prose |

### Deferred

| Item | Status | Why not now | Reopen when |
|---|---|---|---|
| C4 O4 (re-raises) | **Fired** (2.5) | Excluded by construction and stated in §9.6, but not in the published text | **Closed into F2** (attribute scope phrasing) |
| C2 O4 (raised-type identity) | **Fired** (2.5) | `type[E]` against `E` is the identity question, now concrete (27 observations) | **Closed into F2** |
| ADR-0011 review F6 (public callables vs `member()`) | Consumers now live: Related prints the labels, and selection feeds the paths into `member()`, where a mismatch refuses the compile | 0 pilot instances. By SQL, no inherited least path of a community member passes an outside or unresolved ancestor before its declaring class. Rebinding not checked | A second library, **or** increment 3 raising the budget (run the SQL again first) |
| ADR-0011 review: property getters as callables; several nodes per path | **Fired** (the Related template) | No pilot Related line repeats a path. Selection could pick a property (`Context.request_context` in round 1), which the Public access template handles. A getter and a setter both documented would give two seeds naming one path, and the duplicate-seed check would refuse | Selection active on the pilot |
| ADR-0011 review: a rule tying a finding kind to its invocation method, and valid-JSON `diagnostics` | **Fired** (2.4 added a second statistical method, then 2.5 and 2.6 added more) | One producer per kind today | A `FINDING_METHOD` table beside `FINDING_STATUS`, generating one rule with an injected case, when any kernel is next edited |
| ADR-0019 review O6 (in-memory Stage E composition, stated) | **Fired** (2.6 reads other methods' findings); **the code complies** (`attempt.rs:432-454`) | Only the sentence is owed | One line in ADR-0019's Amendments or DESIGN §9, with F8 |
| Increment-1 deep review O6 (`semantic:one-outcome-per-brief`) | Not fired: 2.5 chunks documents, not briefs | Exactly one Outcome is still synthesized | Briefs are split by case (F8's stale sentences say they are) |
| O1 (selection drops; the documented proxy) | New | Nothing is selected on the pilot | Selection active, or a refusal from `documentation-only-has-outcome` |
| FCA time on large scopes | New | 205 closed sets on the pilot | A scope with more than 1,000 attributes or a budget stop (`concept_budget`) |

**Final check.**
- **Claims against evidence.** They match, except:
  - FCA's "structurally defined scope" in published text (F1);
  - the attribute forms' honesty (F2);
  - the usage projection's Tested label (F6);
  - the slot semantics (F5).
- **Scope against guarantees.** Right for the five pilot seeds. Too wide for increment 3's
  selection until F1, F4 and U1 land.
- **The extension path.** Increment 3 raises the budget, and that is exactly where F1, F3 and F4
  fire. Settle U1 and U2, and freeze them (F7), before the 3.3 gold scoring.

## 12. Disposition (author, 2026-09-23)

Every finding is fixed or decided in one change. Outcomes: `just test-all` in the commit message;
the pilot numbers are in DESIGN §9.4–§9.6 once the pilot has run.

| # | Disposition | Where |
|---|---|---|
| F1 | Fixed. Scopes are keyed by node, labelled by the fewest-segment, then least, container. Each seed records its scope (`AnalysisRows.seed_scopes`), and Stage F reads concepts and implications of that scope only. Probes 3 and 4 are now the test `fca_scopes_are_nodes_and_state_only_their_own_apis`: two paths to `Catalog` compile as one scope, and `Catalog.add_tool`'s lines name only `pkg.Catalog` APIs | `analyze.rs`, `synth.rs`, DESIGN §9.6 |
| F2 | Fixed. Attributes are built from term structure: a term that is, or holds anywhere through `type_term_args`, an `Any` of style error or implicit is no attribute; a raised `ClassObject` is its class; only class-typed raises count. The text states each attribute's scope ("declares …", "raises `E` directly in its body"). The tripwire `semantic:concept-attribute-known` has an injected case, and the fixture pair `Catalog.load`/`reload` checks it | `cpg_schema::concepts`, `synth::attributes_text`, `rules.rs` |
| F3 / U1 | Decided (D37; ADR-0011 amendment). Direct usage is the default rank: each usage call site once, split among its definite or candidate targets. The first definite-only draft was revised on a modality measurement, before any output existed under it (D37). Selection takes eligible APIs (a docstring summary, ≥ 1 usage call) by rank, with a community cap of ⌈budget / 3⌉. PageRank is the `+pagerank` variant, off by default, for the 3.3 ablation. The policy names its arc kinds (F3(c)); so does `WEIGHT_POLICY` | `ranking.rs`, `selection.rs`, DESIGN §9.4, §9.5 |
| F4 | Fixed. There is one preferred path per public callable: through the declaring class first, then fewest segments, then least. It is a column of `public_callables_sql`, read by `preferred_paths`, and every consumer uses it (communities, direct usage, PageRank, selection, kNN). The fixture subclass `pkg.Alpha(Server)` sorts before its base, and no label names it | `cpg_schema::communities`, `lctx_analytics::communities` |
| F5 / U2 | Decided (D38): option (a). The concept is a `shared_signature` assertion under Related ("Like … declares …"). The Applicable-case slot stays absent and is out of the document header | `findings.rs`, `synth.rs`, DESIGN §9.6, §10.2, §10.3 |
| F6 | Fixed. The usage branch is Tested (a unit test's usage caller; `selection_takes_what_usage_calls_within_the_budget` on `docs_shapes`). The permuted-projection test replaces the vacuous one. Probes 1 and 2 are tests (`the_frequent_basis_is_sound_complete_and_non_redundant`; `compute_flow_agrees_on_random_graphs`) | `ranking.rs`, `concepts.rs` |
| F7 | Fixed. `selection::Params` holds the rule, the usage policy, the community share, the shared-signature floor and name cap, and the implication and Related caps. It is frozen by digest (`selection_parameters`), and `the_parameters_match_their_gold_freeze` checks it | `eval/gold/analytics-freeze.json` |
| F8 | Fixed: §9.5's "Owed" sentence, §10.2's table (`applicable_case`, `implication`, `doc_link`, `shared_signature`), §10.3/§10.4/§11.1's "split by applicable case", §9.6's cover-relation oracle, and the codebook doc | DESIGN, `codebook.rs` |
| O1 | Fixed. Eligibility uses `summary_span` (the Outcome's own source). A dropped selection is recorded in the invocation's diagnostics (`dropped`), and so is a second seed for one path | `analyze.rs` |
| O2 | Open for the 3.3 ablation: implications are the candidate. `-fca` removes them together with shared signatures | — |
| O3 | Resolved by U1. `analysis_shapes` has no usage code, so nothing is selected there (`selection_needs_official_usage`); the `finish`/`prepare` docstrings no longer steer a test | — |

**Deferred rows:**
- **FINDING_METHOD rule and O6 statement:** done (D40).
- **ADR-0011 F6 and the property row:** still deferred. A dropped selection is now recorded instead of refusing the compile. Rerun the SQL when increment 3 raises the budget (3.4).
- **FCA time on large scopes:** still deferred.
