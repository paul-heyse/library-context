# A1 — Graph catalog, projections and analytics (Phase-1 reconstruction)

Baseline: commit `35afc09` (crates/ and python/ clean). Read-only reconstruction, 2026-09-29. No
build, test or probe was run.

Labels: **[I] Implemented** = the stated code exists and does this at 35afc09 (read directly).
**[IC] Interface-checked** = a consequence reasoned by composing read code paths/contracts (SQL,
types, call sites); not executed. Nothing here is Tested.

Profile gating that frames every row [I]: `catalog` is the default compile profile; Stage E
(`cpg-core/src/analyze.rs::run`), Stage F (`synth::run`), the flow model, the finite summaries and
the behavior scan run only when `analysis.is_some()` (`--profile behavioral`):
`crates/cpg-core/src/attempt.rs:577` (profile/analysis agreement), `:917-923` (flow model),
`:931`/`:1268-1349` (summaries, behavior scan), `:1504-1518` (Stage E), `:1549-1556` (Stage F).
The **catalog contracts, including contextual evidence (`cpg-core/src/evidence.rs`), run in both
profiles** (`attempt.rs:741` → `catalog.rs:457-468`). Derived tables `nodes`, `edges`,
`graph_gaps`, `edge_kinds` are computed for every profile (`cpg-schema/src/derived.rs:1167-1194`).

---

## 1. CI analysis-record table

Served-path notation: table → bundle query (`crates/cpg-core/src/bundle.rs`) → PG
(`lctx_serving.*`) → MCP tool (`python/lctx_mcp/src/lctx_mcp/server.py`). "Brief path" =
`assertion_support`→`supports`/`support_findings`/`support_witnesses`/`support_members`
(`bundle.rs:137-189, 277-318`) → PG `lctx_serving.supports…` (`migrations/202609270004_projection.sql:48-192`)
→ `get_capability`/`search_capabilities` (`server.py:441-477`), behavioral profile only.

| # | Question answered | Projection (universe · selector · relations · direction · multiplicity · weights · scope) | Method & settings | Exact / conservative / heuristic + model | Budgets & partial behaviour | Output & evidence linkage | Consumers → served? |
|---|---|---|---|---|---|---|---|
| (a) adapter `Projection::build` `lctx-analytics/src/graph.rs:90-195` [I] | none itself: materializes the invocation projection | Universe: `nodes` of kinds Module, Class, Function, SyntheticCallable, ExternalSymbol + module name/role (`cpg-schema/src/projection.rs:107-113,134-139`). Arcs: (1) `call_target ⋈ encloses_call` (caller = `ec.src_node_id` = `owner_of`), accepted `modality∈{definite,candidate} ∧ origin=analyzer_assertion ∧ fidelity=report_projection`, **any phase, any callable dst** (`projection.rs:148-159`); (2) `site_target` with phase ∈ {property_get, property_set}, caller `graph::owner_of("sn")` (`:161-169`); (3) `declares` Function→Function definition arcs, modality hard-coded definite, **bypassing the accepted filter** (`:171-177`). Directed caller→callee; parallel sites kept (edge weight = arc row, `graph.rs:176-185`); unweighted; whole snapshot (subsystem mask applied by callers, `analyze.rs:568-576`). Unresolved relation: `resolutions` with `target_count=0 OR has_unresolved_remainder` (`projection.rs:180-187`). | Arrow → sorted-id dense index (`binary_search`, `graph.rs:115-122`); strict canonical-order check on arcs (`:151-155`); `Graph<(),u32,Directed,u32>`; `out_arcs` collects + sorts per call (`:204-212`). No petgraph algorithm is applied to this graph. | Exact representation of the declared spec. | None; refuses unordered vertices/arcs and unknown endpoints (`AnalyticsError::Order/UnknownVertex`). | In-memory `ids/kinds/modules/roles/arcs/unresolved`; each arc keeps `call_site`, `edge_id`, `phase`, `modality`, `arc_kind`, `has_unresolved_remainder` (property arcs hard-code `false`, `projection.rs:163`). Spec digest → invocations' `projection_digest` (`analyze.rs:566,1176-1193`). | Pass A, direct usage, PageRank, communities invocation layer, RCA calls (`analyze.rs:1445-1464`), kNN API selection (`analyze.rs:627-635`). Served only through those. |
| (b) Pass A BFS `pass_a.rs:91-395` [I] | Which public API exposes/coordinates what: direct & bounded delegation, boundaries, open sites | Adapter arcs from one seed; may enter only subsystem vertices; ExternalSymbol, SyntheticCallable and outside-subsystem targets are `implementation_boundary` (`pass_a.rs:70-77`). Candidate and definition arcs followed; `potential`/`synthetic_model` absent from projection. | BFS with parent pointers, sorted adjacency (`:123-156`); witnesses = BFS path + next-shortest distinct final arcs (`:184-243`); `direct` only if a definite **call** is among shortest final arcs (`:204-208`, `graph.rs:41-43`). Settings from `analytics.toml` `pass_a.{max_depth,max_vertices,max_edges,max_witnesses}` (`config.rs:40-47`). | Exact reachability under projection+depth model (not dataflow). | Depth cut → `complete_under_stated_model` + `traversal_stop(depth_limit)`; vertex/edge budget → `partial` + stop reason (`:267-280, 382-387`). | Findings PublicAlias/DirectDelegation/BoundedDelegationPath/ImplementationBoundary/IncompleteResolution/TraversalStop; witnesses carry `edge_id`, modality, arc kind, phase (`:348-364`); invocation `projection_digest` = invocation spec digest, subject = seed (`analyze.rs:1175-1207`). | Stage F delegation/limits assertions; rules `semantic:witness-edge` (`rules.rs:3706-3713`), direct-delegation definite rule (`rules.rs:3692-3702`). **Served** via brief path (`support_witnesses` resolves spans, `bundle.rs:163-189`). |
| (c) Pass B worklist `pass_b.rs:352-675`; surface rerun `cpg-core/src/behavior.rs:323-404` [I] | Controls/limits: seed parameter forwarded, literal supplied, conditional raise, unfollowed argument | Relations `argument_flows_sql`/`guards_sql`/`parameter_reads_sql` (`cpg-schema/src/flows.rs:214-445`). Arcs = `edges` CallTarget with **accepted lists copied from the projection** (`flows.rs:68-76`) + `dst_kind=Function` + phase ∈ {call, init} (`:80-100`), caller via `encloses_call` (`:103-112`). State key `(callable, formal, source param, suppressed)`. Stage E: `inside` = subsystem; surface scan: `|_| true` over every public callable (`behavior.rs:389-404`). | BFS worklist over `by_caller` index (`pass_b.rs:388-439`); depth bound = `pass_a.max_depth` (`analyze.rs:1262`, `behavior.rs:345`). Surface: flows rebuilt from flow-IR `value_flows` via `v2_flows` (`behavior.rs:165,315-322`). | Conservative (syntactic identity flow through a parameter or one alias; candidate arcs followed). Surface verdict `unknown` on any non-definite hop (`behavior.rs:516-521`). | Depth cut sets `stop_reason=depth_limit` but completion is **always** `complete_under_stated_model` (`pass_b.rs:665-667`); no state budget. Surface marks partial ops with `budget_reached` boundary (`behavior.rs:405-415`, `partial.insert` at `:409`). | Stage E: findings + witnesses (`edge_id`, modality, phase) + members; `projection_digest` = `flows::digest()` (`analyze.rs:1215,1248-1277`). Surface: findings transient → `behaviors` rows under one `pass_b_surface` invocation (`behavior.rs:339-345`). | Stage E → Controls/Limits assertions (**served**, brief path). Surface → `behaviors` (`bundle.rs:373`) → PG `lctx_serving.behaviors` → `get_operation` relationships/sections (`lctx-postgres/src/packet.rs:211-240`). |
| (d) Pass C group-by `pass_c.rs:141-301` [I] | Official usage hands one release callable's result to another | `handoffs_sql` (`flows.rs:495-592`): usage modules only (`usage_files_sql`), targets = `flows::call_targets` restricted to release declarations (`:509-514`); named (`x = p(); c(x)`) or nested (`c(p())`) occurrences; formal mapping `maps_formal`. | Group by `(other, formal, producer/consumer modality+phase)`; members sorted example→doc→test, path, byte (`:183-204`); `max_occurrences = pass_a.max_witnesses`. | Exact over the declared syntactic pattern. | Truncation only of members (`witnesses_omitted`); completion always complete (`:293-298`). | Handoff finding: formal, producer/consumer site members, `HandoffAttribute` member (attribute id; incidences carry producer/consumer `edge_id`, `concepts.rs:340-357`); `projection_digest = flows_digest` (`analyze.rs:1314-1336`). | Stage F handoff & usage-pattern preference (`synth.rs:878-903`); RCA. **Served** via brief path. |
| (e) Communities `communities.rs:254-716`; driver `analyze.rs:681-846` [I] | Which subsystem functions group together (diversity cap, Related) | Dense index = sorted ids of subsystem functions touched by some pair (**isolates omitted**, `:291-300`). Layers: invocation = every projection arc between distinct subsystem functions incl. definition & property arcs, 1/arc (`:265-272`); co-use = usage scope (`encloses_call` src, module-level included) × `flows::call_targets` (`cpg-schema/src/communities.rs:18-31`); optional type/mention/kNN layers. Undirected pair counts (least site kept); hub down-weight at 95th percentile; unit-normalized; 0.5/0.5 (or 1/L with extra layers). | Leiden RBER γ=1 reported; profile γ∈{0.5,1,2,4}; seeds 0..10; max_iter 100; ε 1e-10 (`:72-95`); consensus = seed-0 partition; co-assignment score (`:521-535`). | Heuristic (statistically_derived). Degenerate γ → no communities. | Per-run `converged = iterations < max_iterations` → per-run `partial` (`analyze.rs:811-815`); consensus always `complete_under_stated_model` (`communities.rs:724-726`). | Community findings: public members (weight = strength) + `SupportingSite` members = call-site or scope **node ids with layer label**, no fact/edge (`:636-645`); `projection_digest` = `communities::digest()` (+extra) (`cpg-schema/src/communities.rs:124-137`). | Seed selection cap, Related, kNN labels. **Served** only in Related (`+communities`, off by default). |
| (f) Ranking: direct usage `ranking.rs:69-168`; PageRank `:217-408` [I] | Which operations official usage calls (seed eligibility/order, Related order) | Direct usage: projection arcs with `arc_kind=Call` (any modality the projection accepts, any phase, **incl. property get/set site arcs**) from vertices whose module role ∈ {example,test,doc_block}; per call site, share `1/|targets|` to subsystem functions; filtered to preferred public paths (`:69-90,123-127`). PageRank: `UsageGraph` = subsystem functions + usage callers with an arc into one; call **and definition** arcs; self-arcs dropped; weight = arc count per ordered pair (`:217-251`). | Direct: sum of shares. PageRank: own power iteration d=0.85, tol 1e-10, 100 iters, uniform teleport & dangling (`:266-309`); oracle `leiden_rs::infomap::compute_flow` in tests. | Direct: exact count under `USAGE_POLICY` (ordering only). PageRank: heuristic. | Direct: always complete. PageRank: non-convergence → `partial` (`:397-401`). | DirectUsage / Centrality findings (score only, no members/witnesses); `projection_digest` = `usage_digest(spec)` / `projection_digest(spec)` (`ranking.rs:48-53,171-176`; `analyze.rs:850-944`); parameters JSON carries `USAGE_POLICY`/`WEIGHT_POLICY`. | Selection eligibility & rank (`analyze.rs:1035-1076`); Related order & citation (`synth.rs:1672-1745`). **Served** only as cited Related support (`+communities`). |
| (g) FCA/RCA `concepts.rs:126-573`; driver `analyze.rs:1357-1558` [I] | Shared signatures / implications among one exported scope's public APIs | Objects = public APIs of an exported class/module namespace (≥2) (`analyze.rs:1361-1412`); attributes = `concepts::attributes_sql` (params, param types, returns, raises, decorators; unknown type terms excluded via recursive CTE, `cpg-schema/src/concepts.rs:37-40`); `+rca`: projection Call arcs (phase required) into subsystem functions + Pass C handoffs (`analyze.rs:1443-1483`, `concepts.rs:286-361`). | NextClosure over `fixedbitset`, min_support 2, budget 20 000 closed sets (`concepts.rs:194-199`). | Exact over the formal context. | Budget → `partial` + `concept_budget` (`:564-569`). | ApplicableCase/Implication findings; members = extent node ids + intent attribute ids; incidences carry `source_fact_id` or `(site, edge_id)` (`concepts.rs:305-323, 340-357, 395-406`). `projection_digest` = concepts digest (+RCA policy) (`analyze.rs:1440,1479-1482`). | SharedSignature/Implication assertions; bundle `support_attribute_incidences` joins **`edges`** for the evidence fact (`bundle.rs:304-318`). **Served** (`+fca`, off by default). |
| (h) kNN `neighbours.rs:125-345`; driver `analyze.rs:592-680, 947-1010` [I] | Documentation near an API; community label | Items: preferred public Functions in subsystem (API text windows) × corpus passages; community member lists read from **already-emitted community findings** (`analyze.rs:950-966`). | Exact brute-force best-window cosine; k=3, min 0.5, window 4096 B, text v1 (`neighbours.rs:42-49`); centroid per community. | Heuristic similarity (statistically_derived); exact search. | None; `COMPLETION` constant complete (`:344-345`). | DocLink/CommunityLabel findings; only a `Label` member (text), no node/fact (`:233-259`); `projection_digest` = `neighbours::digest()` (texts SQL only) (`cpg-schema/src/neighbours.rs:46-51`). | DocLink/Related-label assertions. **Served** (`+knn`, off by default). |
| (i) SCC schedule `lctx-analytics/src/summaries.rs:33-124`; inputs `cpg-core/src/summaries.rs:116-132, 222-261` [I] | Callee-first order for finite summary composition | Vertices: every Function/AsyncFunction declaration (all roles). Arcs: `DISTINCT (call_syntax.owner_node_id, call_targets.target_node_id)` where both are Function/AsyncFunction declarations, `argument_node_id IS NULL`, `NOT in_annotation`; **no modality/origin/phase filter; site targets absent; module/class owners dropped**. Parallel arcs deduped (`summaries.rs:40-42`). | petgraph `kosaraju_scc`; members sorted; own Kahn condensation order keyed by min member id; self-loop ⇒ recursive (`:60-123`). | Exact topology; "confers no behavior verdict" (`cpg-core/src/summaries.rs:220-221`). | None. | `summary_components` rows (`component_id = recipe::summary_component(members)`); **no `analysis_invocations` row, no digest** (`attempt.rs:1268-1269`). | `finite_flows` grouping (`finite.rs:2005-2092`); recomputed by `validate_summary_components` (`validate.rs:539-542`). Not served. |
| (j) Finite worklist `summaries/finite.rs:1466-2450`, `summaries/worklist.rs` [I] | Which parameter→return value flows compose through local calls | Groups of local seeds keyed `(component_order, component_id)` (`finite.rs:2080-2194`); `dependent` map callee formal → caller seeds (`:2201-2206`); `callers` map summary→citing summaries for unexpanded-witness propagation (`:443-461`). | Per component `ComponentQueue` (pair-work limit 1 000 000, `:1466`); frontier of non-dominated (depth ≤ 8, cost ≤ `MAX_SUMMARY_PROOF_STEPS`) witnesses (`worklist.rs:6-7,34-52`); BDD implication checks. | Exact under stated atoms; conservative refusals. Seed missing from components ⇒ `call_transfer` refusal (`finite.rs:2090-2093`). | Named refusals: `summary_pair_work_limit`, `summary_depth_limit`, `summary_proof_limit`, `call_transfer` (`:2434-2450`). | `summary_flows/steps/boundaries` (`attempt.rs:1296-1299`). | Discharges → behaviors; **served** (`bundle.rs:414-436` → PG `summary_flows`… → `inspect_value_paths`/`get_operation`). |
| (k) flow-model use→def fixpoint `cpg-core/src/flow_model.rs:822-1036` [I] | Which parameters/fields reach a use or sink, under which condition | Uses/defs/reaching rows from ty flow facts; `parents` child-use→parent-use map (`:955-976`). Reads `edges` only for ArgumentValue spans and MroEntry ancestry (`:464-474, 586-591`). | BTreeSet worklist to least fixed point; `MAX_REACH_WORK` 1 000 000 (`:769, 978-994`). | Sound over-approximation (loop-carried keeps use side, `:903-908`). | Cap ⇒ every dependent use widened to unknown + `flow_reach_boundaries` rows (`:995-1021, 1040-1054`). | `value_flow_contributions`, etc. | Behavior scan `v2_flows`, summary seeds → **served** indirectly (behaviors, summary_flows). Behavioral only (`attempt.rs:917-923`). |
| (l) syntax child indexes + owner walks [I] | Structural navigation for completion/evaluation/binding/evidence | Six independent `children` maps: `context_protocol.rs:110-119` (key (snapshot,parent); sort (field, ordinal, fact_id)); `source_call.rs:100` (own `grouped`, HashMap, input order); `completion.rs:1440-1452` (sort (ordinal, start_byte, fact_id)); `evaluation.rs:922-930` (key adds module; same sort); `call_binding.rs:220` (second, separate `grouped` helper `:62`); `cpg-core/src/evidence.rs:185-191` (BTreeMap, input order). Owner/ancestor walks: `evidence.rs:225-235` (cycle guard), `completion.rs:651-670, 1477-1495, 2184-2195`, `usage.rs:247-257` (64-step cap), SQL climb `cpg-schema/src/behavior.rs:2044-2061` (depth 128). | Hash/BTree grouping per call. | Exact structural. | Walk caps only. | Internal. | Feed (i)-(k) and catalog evidence. |
| (m) recursive SQL CTEs [I] | Transitive closures in SQL | `cpg-schema/src/concepts.rs:37-40` (unknown type terms upward over `type_term_args`); **duplicated verbatim-by-meaning** in rule `semantic:concept-attribute-known` `rules.rs:3024-3027`; `cpg-schema/src/communities.rs:40-56` (type reach); `behavior.rs:2044-2061` (handler climb, `UNION ALL`, depth ≤ 128). | DataFusion recursive CTE; `UNION` set semantics except the climb. | Exact. | Climb depth bound only. | Relations. | FCA attributes, rules, type layer, modeled handlers. |
| (n) test-only tarjan `cpg-core/tests/graph.rs:280-370` [I] | Known-answer: isolates, parallel arcs, self-loops, cross-file cycle | Its **own** call-edge SQL: `encloses_call ⋈ call_target`, `dst_kind=Function`, no modality/origin/phase filter (`:296-306`). | `petgraph::algo::tarjan_scc`. | Exact. | – | Assertions only. | Test. |

Notes [I]: `out_arcs` is called for every dequeued vertex before the depth check (`pass_a.rs:124-129`), so a
vertex at the bound is still sorted. The only production petgraph algorithm is `kosaraju_scc`
(`summaries.rs:15,60`); the adapter graph is used only through `edges_directed` (`graph.rs:205-209`).

---

## 2. Semantic-interpretation map

"Derived" = mechanically computed from another site (shared helper/const/generated SQL). Checked
first for each question.

### 2.1 Call edge

| Site | Rule | Derived from? |
|---|---|---|
| S1 projection arcs `projection.rs:148-179` | call_target (accepted m/o/f, any phase, any callable) ∪ property site_target ∪ function→function declares | origin of the accepted lists |
| S2 `flows::call_targets`/`arcs` `flows.rs:80-112` | call_target, accepted **lists copied from S1** (`flows.rs:68-76`), `dst_kind=Function`, phase ∈ {call, init} | **Derived** (lists only; its own `encloses_call` join, not S1's SQL) |
| S2a co-use `cpg-schema/src/communities.rs:18-31`, S2b `delegations` `behavior.rs:3417-3425`, S2c handoff targets `flows.rs:509-514` | reuse S2 | **Derived** from S2 |
| S2d `validate_concept_call_sources` `validate.rs:212-215`; S2e RCA Rust filter `analyze.rs:1445-1464` | S1 `arcs_sql` (Call, Function dst, phase non-null) | **Derived** from S1 |
| S3 summaries `call_arcs` `cpg-core/src/summaries.rs:121-132` | raw `call_targets` ⋈ `call_syntax` ⋈ function declarations both ends; no m/o/f/phase filter; not in annotation | independent |
| S4 `model_applications` `cpg-schema/src/behavior.rs:2069-2087` | raw `call_targets` ⋈ `model_targets` (phase = model phase), `reason IS NULL`, not higher-order, not in annotation; modality/origin kept as columns | independent |
| S5 local summary seeds `behavior.rs:2806-2880` | `call_targets` + `resolutions` (target_count=1, complete, no remainder) + `argument_flows` (**definite**, phase call) + Function declarations both ends | partially derived (via `argument_flows` = S2) |
| S6 usage candidates `cpg-core/src/usage.rs:62-82` | `edges` kind ∈ {CallTarget, SiteTarget}, `modality ∈ {definite, candidate}` (literal list), **no origin/fidelity/phase filter** | independent |
| S7 **catalog contextual evidence** `cpg-core/src/evidence.rs:177-183, 772-835` | Rust filter `EdgeKind::CallTarget | SiteTarget` over the full `edges` table; **no modality/origin/fidelity/phase filter**; basis `resolved_target` iff `fact.modality = definite ∧ pysa_calls.unresolved_reason IS NULL` | independent |
| S8 test `cpg-core/tests/graph.rs:296-306` | encloses ⋈ call_target, Function dst | independent |

Concrete divergences [IC]:
- Property read `width = child.size` in a release function (`fixtures/python/pysa_variants/variants/__init__.py:62`): S1 arc (property_get); S2/S3/S4/S5 none (call_targets only); S6, S7 yes.
- `getattr(child, "size", 0)` (`…/variants/__init__.py:88-89`; the extractor test asserts an artificial-attribute row exists, `crates/cpg-extract/tests/variants.rs:87-92`): Pysa `ArtificialAttributeAccess` ⇒ `origin = synthetic_model` (`cpg-extract/src/pysa_map.rs:550-554,702-714`) ⇒ `site_targets` ⇒ SiteTarget edge (`graph.rs:651-690`, only `potential` excluded). S1 excludes it (origin filter); **S6 and S7 admit it**; with one getter target and `is_attribute=false` it is `definite` (`pysa_map.rs:616-622`) ⇒ S7 basis `resolved_target`.
- `obj.run()` with an `Overrides` target (candidate): S1, S2, S3, S6, S7 yes; S5 and pinned targets no (definite only).
- `__new__` call (phase `new`): S1, S3, S4 (if model phase new) yes; S2 no (call/init only).
- Call inside `async def` or into an async target: S3 yes (AsyncFunction admitted); S5 no (`d.kind = Function`, `target.kind = Function`, `behavior.rs:2868-2871`).

### 2.2 Caller / owner

| Site | Rule | Derived? |
|---|---|---|
| `graph::owner_of` `cpg-schema/src/graph.rs:68-72` | `COALESCE(owner_node_id, module_node_id)` | shared helper |
| `encloses_call` edge `graph.rs:410-427` | `owner_of("s")` | **Derived** |
| S1 call arcs, S1 unresolved, S2 `arcs`, co-use, `open_sites`/`open_site_reads` (`behavior.rs:3449-3500`) | `edges.encloses_call.src` | **Derived** (via edge) |
| S1 property arcs `projection.rs:147,161` | `owner_of("sn")` on `syntax_nodes` | **Derived** (helper) |
| S3 `call_arcs` | `call_syntax.owner_node_id` ⋈ Function/AsyncFunction declaration | independent |
| S4 `model_applications.function_node_id` | raw `c.owner_node_id` (nullable; **a Class id when the call is in a class body** despite the column name; table `behavior.rs:120-128`) | independent |
| `pinned_call_targets_sql` `behavior.rs:2019-2041` | INNER JOIN `declarations owner` (drops NULL owner); `caller_function_scope = owner.kind = Function` (**async ⇒ false**) | independent |
| S5 local seeds | `c.owner_node_id = sink function`, `kind = Function` | independent |
| S7 evidence scenario | innermost `StmtFunctionDef`/`StmtClassDef` ancestor span (`evidence.rs:653-661`) | independent |
| extraction `cpg-extract/src/walk.rs:401-409, 704-710` | innermost declaration whose **body** holds the node (decorators/defaults → enclosing scope); lambdas/comprehensions → enclosing def | source |

Divergence [IC]: module-level `helper()` in `pkg/m.py`: S1 caller = module; S3 dropped;
S4 `function_node_id = NULL`; pinned dropped. Class-body `x = helper()`: S1 caller = class; S3
dropped; S4 `function_node_id = <class id>`; pinned kept with `caller_function_scope=false`.
`async def f(): helper()`: S1/S3 caller f; pinned scope false; S5 excluded.

### 2.3 Unresolved target

| Site | Rule | Derived? |
|---|---|---|
| `resolutions` derivation `derived.rs:489-535` | `has_unresolved_remainder = NOT(targets>0 ∧ remainders=0)`; status NotAttempted/Unresolved/Partial/Resolved | source |
| S1 unresolved `projection.rs:180-187` | `target_count = 0 OR has_unresolved_remainder` | independent SQL |
| `open_sites`, `open_site_reads` `behavior.rs:3449-3500` | `status <> resolved OR has_unresolved_remainder` | independent SQL |
| S5 / pinned `behavior.rs:2034-2037, 2852-2855` | closed world: `target_count=1 ∧ candidate_set_complete_under_model ∧ ¬remainder` | independent |
| S7 `evidence.rs:809-811` | per-target-record `pysa_calls.unresolved_reason IS NULL` | independent |
| typed null rules `graph.rs:1831-1864` | null target ⇒ reason required | shared |

Equivalence [IC]: S1-unresolved ≡ open_sites, by the derivation (`status≠resolved ⇔ pysa_rows=0 ∨
targets=0 ∨ remainders>0 ⇔ has_unresolved_remainder`, since `pysa_rows=0 ⇒ targets=0`) — equal
by algebra, not by sharing. Gap [IC]: attribute/artificial sites have no `resolutions` row, so an
unresolved `site_targets` record (reason `unresolved_target`) never appears in S1's unresolved
relation; property arcs hard-code `has_unresolved_remainder = false` (`projection.rs:163`).
S7's check is redundant with `definite` (definite ⇒ single target, no rest, `pysa_map.rs:616-622`).

### 2.4 Modality / "definite"

| Site | Rule | Derived? |
|---|---|---|
| provider `pysa_map.rs:541-549, 609-633` | definite ⇔ one target ∧ no remainder ∧ not attribute-conditional; Overrides ⇒ candidate | source |
| S1 `projection.rs:120` | accept {definite, candidate} | source list |
| S2 `flows.rs:68-76` | S1 list | **Derived** |
| S6 `usage.rs:73,80-81` | literal {definite, candidate} | independent (equal today) |
| `Arc::is_definite_call` `lctx-analytics/src/graph.rs:41-43` | Call ∧ definite | local |
| Pass A direct / rule `rules.rs:3692-3702` | definite call only | local |
| behavior scan `hop_reason` `behavior.rs:117,516` | non-definite hop ⇒ `unknown` | local |
| S5/pinned `behavior.rs:2034,2864` | definite only | independent |
| S7 `evidence.rs:809` | definite ⇒ `resolved_target`, else `candidate_targets` | independent |
| SiteTarget/PotentialTarget edges `graph.rs:665,818` | ≠ potential / = potential | registry |

Divergence [IC]: S6 and S1 accept the same modalities but S6 has no origin/fidelity filter (2.1).

### 2.5 Direct usage

| Site | Text |
|---|---|
| Implemented `ranking.rs:42-45, 69-90` | "call arcs (definite or candidate, any phase) … each call site counts once, split evenly among its targets" |
| Design `docs/design/sections/analytics.md:423-430` | same (definite or candidate, any phase) |
| Stale `codebook.rs:1036-1038` | "definite call arcs … one per call site. The score is the count." |
| Stale `findings.rs:695-696` | "A count of observed definite calls" |
| Stale `analyze.rs:848-849` | "each public API's definite calls" |
| Selection rule `selection.rs:14-17` | "at least one direct official-usage call" (embeds `USAGE_POLICY`, `:43`) |

No site derives from the codebook text; the policy constant is shared by ranking, the usage
digest, the invocation parameters (`analyze.rs:851-856`) and selection params [I].

### 2.6 Dense index construction

Independent constructions, all ascending by `Id` [I]: adapter (`graph.rs:115-122`, error on
unknown); `UsageGraph` (reuses adapter indices through a `BTreeMap<usize,u32>`, `ranking.rs:228-241`
— **derived**); communities (`BTreeSet` of pair ends, `communities.rs:291-300`, `expect` panics on
miss); FCA (objects sorted by id, attribute `BTreeMap`, `concepts.rs:421-434`); SCC
(`sort+dedup`, error on miss, `summaries.rs:37-58`); kNN (positional over `preferred` order,
`analyze.rs:627-677`); test (`BTreeMap<String,NodeIndex>`). Divergence is in **universe**, not order:
communities drop isolates; `UsageGraph` keeps isolated subsystem functions; SCC keeps every
function declaration. Error behaviour differs (panic vs `UnknownVertex`).

### 2.7 Finding emission

Shared [I]: `FINDING_STATUS` (`findings.rs:634-699`) checked by rule `semantic:finding-status-policy`
(`rules.rs:2992`), `FINDING_METHOD` (`findings.rs:740-761`, `rules.rs:3013`), `FindingKey::id`
(`findings.rs:863-911`). Repeated per emitter: status lookup + key + member rows + finding row —
`pass_a.rs:317-379`, `pass_b.rs:602-663`, `pass_c.rs:176-292`, `communities.rs:578-690`,
`ranking.rs:118-156, 344-381`, `neighbours.rs:216-275`, `concepts.rs:439-500`.
Divergence [I]: `MemberKey` is documented as "every column but the weight" (`findings.rs:619-620`),
but Pass C builds keys with `label: None` while its rows carry the path label (`pass_c.rs:211-221`
vs `:267-278`), and FCA extent members likewise (`concepts.rs:455-462` vs `:477-488`); Pass A/B,
communities and kNN hash the label. No validator recomputes finding ids (only
`cpg-schema/tests/contracts.rs:168-250`), so the difference is undetected [IC].

---

## 3. Fidelity triage

### F-a — owner-only caller in summary_components / call_arcs / model_applications

**Verdict: no fidelity loss; precision/naming defects only. Not served as a wrong claim.**

- `call_arcs` does drop module-level (NULL owner) and class-body callers [I]
  (`cpg-core/src/summaries.rs:122-127`). Consequence for `summary_components`: **none** [IC] —
  the vertex set is Function/AsyncFunction declarations only (`:116-120`), so a module/class caller
  can never be an SCC member, and an arc from it cannot order two function components.
- Finite composition seeds only from Function callers into Function targets with a definite,
  single, closed-world local call (`behavior.rs:2847-2871`); every such arc is in `call_arcs`
  (call_targets, not higher-order, both ends Function) [IC], so the callee-first order is sound
  for everything composition follows. A missing arc would only cause a conservative
  `call_transfer`/unexamined refusal, never a wrong verdict (`finite.rs:2090-2093, 2440-2446`) [IC].
- `model_applications` does **not** drop module-level callers [I]: `function_node_id` is nullable
  raw `owner_node_id` (`behavior.rs:2072`, `:128`). Class-body calls store a Class id in a column
  named `function_node_id` (naming defect).
- `pinned_call_targets_sql` drops NULL-owner calls (inner join, `behavior.rs:2028`) and marks
  class **and async-function** callers `caller_function_scope=false` (`:2024`) [I]. In evaluation a
  missing call entry refuses `UNSUPPORTED` and a non-function scope refuses `ScopeBoundary`
  (`evaluation.rs:428-443`) [I] — both are refusals (CI-04-compliant); only the reason precision
  differs. Declared model: synchronous callers only (`behavior.rs:2798`).
- Served reach: `summary_components` is not a served file; `summary_flows/boundaries` are, but no
  path above changes a verdict [IC]. Coverage note: async callers (FastMCP is async-heavy) are
  outside the local-composition and pinned-evaluation model by design.
- Severity: Low. CI-02 (column misnames a class as a function) / CI-05 (the SCC projection is
  undeclared: no `ProjectionSpec`, no invocation row, no digest). Not CI-G1.
- Minimal correction: one declared caller relation in `cpg-schema` (owner + owner kind, from
  `owner_of`) consumed by `call_arcs`, `model_applications` and pinned targets; rename
  `model_applications.function_node_id` → `owner_node_id` (+ `owner_kind`); record the SCC input as a
  declared projection with a digest on an invocation row.

### F-b — synthetic_model site targets admitted outside the invocation projection

**Verdict: two consumers relabel provider-asserted `synthetic_model` relations; the more
important one is on the default (catalog) served path and was not in the prior list.**

1. **Catalog contextual evidence (new finding)** [I]: `cpg-core/src/evidence.rs` loads the whole
   `edges` table (`:20-35, 51-66`), indexes every CallTarget/SiteTarget edge (`:177-183`), and for
   each usage/test/doc-block site emits `catalog_associations` with role `"invokes"` (or
   `"tests_failure"`) and basis `resolved_target` when the evidence fact is definite (`:772-835`).
   `AssociationSupport` carries `edge_id`, `fact_id`, `modality`, `phase` but **no origin**
   (`cpg-schema/src/evidence.rs:172-185`). Exposure [I]: `catalog_associations`
   (`cpg-schema/src/evidence.rs:239-244`) is a catalog serving file (`cpg-schema/src/catalog.rs:162`)
   → PG `lctx_serving.catalog_associations` (`migrations/202609280011_evidence.sql:84-111`) →
   `get_operation` demonstrations filtered `basis='resolved_target'`
   (`lctx-postgres/queries/packet_demonstrations.sql:1`, `packet.rs:190-206`) and the Evidence
   section (`packet.rs:325` → `lctx-postgres/src/evidence.rs:287-330`) → MCP `get_operation`
   (`server.py:487-507`). Runs in both profiles (`attempt.rs:741`, `catalog.rs:465-467`).
2. **Stage F usage patterns** [I]: `usage.rs:62-82` admits SiteTarget edges of any origin and any
   phase; the chosen pattern becomes a `usage_pattern` assertion with `Example` evidence spans,
   `cited_fact_id = None`, status `documented` (`synth.rs:1782-1800`, `findings.rs:444-448`).
   Served via the brief path (behavioral only). The site→seed relation behind the choice is not cited.

Does it state something the provider did not assert (CI-02)? **No invention**: Pysa did assert
the artificial-site target (`pysa_map.rs:550-554`). It is **relabelling**: a relation the
invocation projection deliberately excludes (`projection.rs:102-103`;
`docs/design/sections/storage-and-publication.md:53`) is served as an ordinary
"invokes / resolved_target" association with its origin unrecoverable from the served record
(`facts` is not served). Concrete input [IC]: a test or example containing
`getattr(obj, "size", 0)` where `size` is a catalog member's property yields a
`resolved_target` "invokes" association (and an eligible demonstration) that no Pass A/direct-usage
arc corroborates. Open question worth one known-answer probe: which Pysa record carries a bare
`@mcp.tool` decorator application; if it is an artificial site, FastMCP's dominant idiom reaches
S6/S7 but not direct usage.

Severity: Medium (CI-02 · CI-G1 — served on the default profile; the served statement is likely
true but its origin is erased). Minimal correction: one shared "accepted call evidence" predicate in
`cpg-schema` (the projection's lists, plus a declared site-target phase/origin policy) used by S1,
S6 and S7; add `origin` (and Pysa site kind/origin kind) to `AssociationSupport`; give
`synthetic_model` targets their own basis (e.g. `modelled_protocol_target`) or exclude them from
`resolved_target`; cite the `edge_id`/fact behind a usage pattern's site (CI-11).

### F-c — direct usage "definite" vs "definite or candidate"

**Verdict: implemented and designed = definite or candidate (any phase, per-site share); three
code comments are stale; the served lineage is correct.**

- Implemented [I]: `USAGE_POLICY` (`ranking.rs:42-45`), `usage_counts` has no modality filter and
  counts property get/set arcs as calls (`:72-78`); design agrees (`analytics.md:423-430`).
- Stale [I]: `codebook.rs:1036-1038`, `findings.rs:695-696`, `analyze.rs:848-849` (also "one per
  call site … the score is the count" vs a fractional share).
- Served [I/IC]: DirectUsage findings reach an agent only when cited by a Related assertion
  (`synth.rs:1672-1745`, citing at `:1735-1742`; `+communities`, behavioral); `support_findings` then serves method
  `usage_count` and the invocation `parameters` JSON containing the correct `USAGE_POLICY`
  (`bundle.rs:145-162`). Otherwise the counts decide only seed eligibility/order (which briefs
  exist) (`analyze.rs:1035-1076`). Codebook doc comments are not served (`finding_kind` is a
  string, `cpg-schema/src/wire/responses.rs:447`).
- Severity: Low (documentation drift; CI-09 governed — ordering only). Correction: rewrite the
  three comments to cite `ranking::USAGE_POLICY`.

---

## 4. `nodes` / `edges` / `edge_kinds` / `graph_gaps` readers (exhaustive at 35afc09)

| Table | SQL readers | Rust / typed readers | Rules |
|---|---|---|---|
| `nodes` | `projection.rs:134-139` (vertices); `validate.rs:212-215` (concept call sources); `synth.rs:367-386` (Stage F labels); `graph.rs:733` (`Introduces` edge source joins nodes); `graph.rs:1317-1327` (`edges` endpoint kinds) | none | 85 `ref:<table>.<col>->nodes` rules from `node_columns` (`graph.rs:1431-1611, 1677-1689`); 22 `REFERENCES` to `nodes` incl. catalog-profile tables `catalog_constructors.class_node_id`, `catalog_bindings.declaration_node_id`, `catalog_evidence.subject_node_id` (`rules.rs:36,48-52,103-107,138-142`) and findings/witnesses/evidence/briefs (`rules.rs:183-269, 1769-1795`); `nodes.existence_fact_id` (`rules.rs:2404`) |
| `edges` | projection arcs/unresolved `projection.rs:154-184`; flows `flows.rs:85,108,236,420,535`; co-use `communities.rs:23`; behavior relations `behavior.rs:3391-3425, 3449-3500`; usage candidates `usage.rs:62-74`; flow model `flow_model.rs:464-474, 586-591`; bundle `support_attribute_incidences` `bundle.rs:313-314` (**served query**); validate `validate.rs:212-215` | **`cpg-core/src/evidence.rs:27,65,177-183`** (full-table typed load; catalog profile; stage-cache key via `input_dependencies`, `stage_cache.rs:319-331`) | graph rules per edge kind (endpoint/evidence/one-per-evidence/no-parallel/lineage, `graph.rs:1614-1676`), `support:edges` (`:1825-1830`); `semantic:refuted-not-overridden` (`rules.rs:3583-3594`); `semantic:witness-edge` (`rules.rs:3706-3713`); edge-id refs from `concept_incidences`, `handoffs`, `witnesses` (`rules.rs:254-266`) |
| `edge_kinds` | **none** | none | none (only registered, `derived.rs:1192`). Its stated purpose — "a projection selects by them from the store" (`graph.rs:1368-1370`) — is unimplemented: `ProjectionSpec.edge_kinds` are compile-time constants |
| `graph_gaps` | none outside rules; SQL is empty by construction (`… AND FALSE`, `graph.rs:1352-1364`) | none | `partition:pysa_calls-gaps` (`graph.rs:1752-1758`, an edit guard `rules.rs:3780`); `graph_gaps.gap_fact_id` → facts (`rules.rs:2405`) |

None of the four is a bundle/PG file or read by MCP (`migrations/202609280011_evidence.sql:157`
list; `catalog.rs:128-163`) [I]; they are Delta-published and reachable by `lctx query`.

If they became **views derived from the registry declarations** [IC]: semantics unchanged (the
registry already generates the SQL); cost moves to every reader (35-way union + two `nodes`
joins + `lctx_id` per read, read ≥10 times per behavioral compile and by ~200 rules); stored
`edge_id` values (witnesses, handoffs, concept_incidences, `AssociationSupport.edge_id`) would
reference a virtual relation — fine while `lctx_id` is deterministic; snapshot schema contract
changes (schema migration); `lctx query` must register the views.

If **retired** [IC]: every reader re-expresses encloses/call/site/argument-value/MRO relations
from family tables (today only `EdgeSource` SQL encodes `argument_value`, `graph.rs:618-650`);
CI-03 relationship identity (`edge_id`) needs a new owner for 5 reference columns and the served
evidence support; the kind-typed node universe used by 85+22 reference rules must be replaced by
per-kind existence sources. `edge_kinds` could be retired with no consumer change; `graph_gaps`
likewise (drop one partition rule, one edit guard, one reference rule).

---

## 5. Upstream lineage (analysis consuming another analysis)

| Consumer | Consumes | Lineage recorded | Gap |
|---|---|---|---|
| Direct usage | invocation projection | `projection_digest = usage_digest(spec digest ⊕ USAGE_POLICY)`; parameters JSON (`analyze.rs:850-890`) | — |
| PageRank | projection via `UsageGraph` | `projection_digest(spec ⊕ WEIGHT_POLICY)`, iterations/residual/converged (`analyze.rs:895-943`) | — |
| Communities | projection arcs, co-use (S2), public paths, extra layers | `communities::digest_with(spec digest)` ⊕ co-use SQL ⊕ public-paths SQL; `extra_digest` incl. kNN spec hash/k/min (`cpg-schema/src/communities.rs:103-137`) | Supporting-site members cite node ids, not edge/fact ids |
| kNN community labels | community findings (`analyze.rs:950-966`) | only `neighbours::digest()` (texts SQL) + knn params + spec hash | **No community invocation id or community digest**; CommunityLabel carries only a text label member |
| Seed selection | direct-usage counts or Centrality findings; community findings; docstrings | parameters (budget, choices incl. `USAGE_POLICY`, `ranked_by`, techniques) + diagnostics; `projection_digest = None` (`analyze.rs:1098-1143`) | No invocation ids of the usage/PageRank/community inputs |
| Pass A | projection; seeds (configured + selected) | spec digest; subject = seed | Whether a seed was selected (and by which rank) only via selection diagnostics |
| Pass B (Stage E) | flows relations (S2) | `flows::digest()` = SQL text (`flows.rs:594-602`) | Tracks S1's accepted lists only because they are inlined in the SQL; S1's `arcs_sql`/owner rule not in the digest |
| Pass B surface | flows + flow-model `value_flows` + open-site reads | `behavior::digest()` ⊕ flow-model digest ⊕ relations (`behavior.rs:229-237`) | Per-finding Pass B ids transient |
| Pass C | `handoffs_sql` | `flows_digest` | — |
| FCA +RCA | attributes SQL; **projection arcs**; **Pass C handoff rows** | `concepts::digest()` ⊕ `RCA_POLICY` (`analyze.rs:1479-1482`) | **Neither the projection digest nor `flows::digest()`** |
| Handoff attributes (always) | Pass C rows | incidences carry producer/consumer `edge_id`s (`concepts.rs:340-357`) | — |
| Stage F usage pattern | usage candidates (S6, `edges`) + Pass C handoff sites | evidence spans; cites shown Handoff findings (`synth.rs:1803-1824`) | **Site→seed edge/fact not cited** (CI-11) |
| Related | community + DirectUsage/Centrality + CommunityLabel findings | cites each finding (`synth.rs:1735-1742`) | — |
| SCC schedule | `call_arcs`/`call_functions` | none (no invocation row, no digest) | undeclared projection; validated only by recomputation |
| Finite summaries | components, flow model, conditions | proof steps per summary | — |
| Catalog evidence (S7) | `edges`, facts, pysa_calls, catalog contracts | `AssociationSupport{edge_id, fact_id, support_fact_id, modality, phase}` | **No origin** |
| `validate_concept_call_sources` | S1 `arcs_sql` | recomputation | — |

---

## 6. External-review claim ledger

| Id | Claim | Verdict | Evidence |
|---|---|---|---|
| E1 | "The general projection registry currently returns only the invocation projection." | **Confirmed**, refined | `projection.rs:207-210` returns `vec![invocation()]`. Other graphs are unregistered: usage graph (a policy digest, `ranking.rs:171-176`), community layers (`cpg-schema/src/communities.rs`), SCC call graph (`cpg-core/src/summaries.rs:116-132`, no spec), flows arcs, usage/evidence edge selections. |
| E2 | "Graph-local indices map back to canonical IDs, parallel arcs preserved, metadata separate." | **Confirmed for the adapter; refined** | `graph.rs:4-9, 115-122, 169-185, 197-217`. Arc metadata is copied into a Rust `Vec<Arc>` (not kept in Arrow as `graph.rs:7` says). Parallel arcs are collapsed by `UsageGraph` (`ranking.rs:247-249`), community layers (`communities.rs:110-121`) and the SCC wrapper (`summaries.rs:40-42`) — each by stated policy. |
| E3 | "`out_arcs` collects and sorts outgoing arcs on each call." | **Confirmed** | `graph.rs:204-212`; arcs were inserted in canonical order (`:176-185`), so an offset/CSR slice would make the sort unnecessary; only Pass A calls it, once per dequeued vertex including those at the depth bound (`pass_a.rs:124-129`). |
| E4 | "Existing SCC wrapper: petgraph owns the algorithm, wrapper owns identities and deterministic ordering." | **Confirmed, refined** | `summaries.rs:31-124`: `kosaraju_scc`; members sorted; wrapper also dedups edges, computes self-loop recursion and runs its own Kahn condensation order keyed by min member id (does not rely on kosaraju's output order). Only production petgraph algorithm in the workspace. |
| E5 | "ProjectionSpec declares node universe, accepted evidence and unresolved-target policy; digest includes declared fields and queries." | **Confirmed, refined** | Fields `projection.rs:16-38`; digest over name, code lists, three policy strings and three SQL strings `:42-82`. Refinements: policies are free text; direction/multiplicity are prose only; the SQL is hand-written `format!` using only the IN-lists from the declaration — `edge_kinds` is not used to build SQL, the property-phase filter and the definition-arc bypass of the accepted filter (`:171-177`) are not declared fields; module doc "Its SQL is generated here" overstates. |
| E6 | "The code distinguishes optional communities, FCA/RCA, PageRank and neighbour analyses (analytics config separates them)." | **Refined (config part refuted)** | `AnalyticsConfig` has only subsystem, seeds, pass_a, briefs (`config.rs:14-47`). Separation is by `Techniques` flags + parse rules (`analyze.rs:58-118`), distinct `AnalyticMethod` invocations, and per-module code `Params::preregistered()` digested into the compiler digest (`attempt.rs:199-238`). All of Stage E is behavioral-profile only. |

---

## 7. Line counts of cited files (at 35afc09)

| File | Lines | File | Lines |
|---|---|---|---|
| crates/cpg-schema/src/projection.rs | 255 | crates/lctx-analytics/src/graph.rs | 227 |
| crates/cpg-schema/src/graph.rs | 1866 | crates/lctx-analytics/src/pass_a.rs | 395 |
| crates/cpg-schema/src/flows.rs | 694 | crates/lctx-analytics/src/pass_b.rs | 750 |
| crates/cpg-schema/src/communities.rs | 201 | crates/lctx-analytics/src/pass_c.rs | 301 |
| crates/cpg-schema/src/concepts.rs | 100 | crates/lctx-analytics/src/communities.rs | 911 |
| crates/cpg-schema/src/neighbours.rs | 79 | crates/lctx-analytics/src/ranking.rs | 664 |
| crates/cpg-schema/src/behavior.rs | 3588 | crates/lctx-analytics/src/concepts.rs | 984 |
| crates/cpg-schema/src/derived.rs | 1203 | crates/lctx-analytics/src/neighbours.rs | 462 |
| crates/cpg-schema/src/rules.rs | 4017 | crates/lctx-analytics/src/selection.rs | 158 |
| crates/cpg-schema/src/findings.rs | 1010 | crates/lctx-analytics/src/config.rs | 257 |
| crates/cpg-schema/src/codebook.rs | 1954 | crates/lctx-analytics/src/summaries.rs | 299 |
| crates/cpg-schema/src/catalog.rs | 363 | crates/lctx-analytics/src/summaries/finite.rs | 4618 |
| crates/cpg-schema/src/evidence.rs | 632 | crates/lctx-analytics/src/summaries/worklist.rs | 218 |
| crates/cpg-schema/src/public.rs | 192 | crates/lctx-analytics/src/evaluation.rs | 1799 |
| crates/cpg-core/src/analyze.rs | 1609 | crates/lctx-analytics/src/completion.rs | 3019 |
| crates/cpg-core/src/summaries.rs | 753 | crates/lctx-analytics/src/context_protocol.rs | 543 |
| crates/cpg-core/src/usage.rs | 673 | crates/lctx-analytics/src/source_call.rs | 316 |
| crates/cpg-core/src/behavior.rs | 1505 | crates/lctx-analytics/src/call_binding.rs | 357 |
| crates/cpg-core/src/flow_model.rs | 3181 | crates/cpg-core/tests/graph.rs | 647 |
| crates/cpg-core/src/evidence.rs | 1219 | crates/cpg-extract/src/pysa_map.rs | 846 |
| crates/cpg-core/src/bundle.rs | 1194 | crates/cpg-extract/src/walk.rs | 1017 |
| crates/cpg-core/src/synth.rs | 2360 | crates/lctx-postgres/src/packet.rs | 493 |
| crates/cpg-core/src/validate.rs | 2410 | crates/lctx-postgres/src/evidence.rs | 511 |
| crates/cpg-core/src/attempt.rs | 1793 | crates/lctx-postgres/queries/packet_demonstrations.sql | 1 |
| crates/cpg-core/src/stage_cache.rs | 395 | crates/lctx-postgres/migrations/202609280011_evidence.sql | 180 |
| crates/cpg-core/src/catalog.rs | 1521 | python/lctx_mcp/src/lctx_mcp/server.py | 749 |
| fixtures/python/pysa_variants/variants/__init__.py | 89 | docs/design/sections/analytics.md | 625 |
