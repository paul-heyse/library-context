# Analytics

This owner covers the analyses that run over published facts and derived projections to produce
typed **findings** (ADR-0047): the three structural passes behind briefs (Pass A delegation,
Pass B controls, Pass C handoffs), the whole-surface behavior scan, direct-usage ranking and seed
selection, and the optional statistical and lattice techniques (communities, PageRank, FCA/RCA,
embedding kNN). Its consumers are Stage F synthesis ([§10](synthesis-and-serving.md#section-10)),
the operation tables served by the FastMCP tools ([§11.3](synthesis-and-serving.md#section-11-3))
and, from Stage 4, the capability registry ([§9.9](behavioral-analysis.md#section-9-9)). Inputs are
the invocation projection and declared relations ([§5](storage-and-publication.md#section-5)),
`public_paths`, the official usage corpus and, for kNN, cached vectors
([§11.1](synthesis-and-serving.md#section-11-1)). Dependencies run one way: analytics reads facts
and relations declared in `cpg-schema`; it never writes facts or concept membership. The
kernels live in `crates/lctx-analytics/src/` (`pass_a`, `pass_b`, `pass_c`, `communities`,
`ranking`, `selection`, `concepts`, `neighbours`), their SQL relations in `crates/cpg-schema/src/`
(`public`, `flows`, `communities`, `concepts`, `neighbours`), and orchestration in
`crates/cpg-core/src/analyze.rs`. Transfer summaries and models are the
[behavioral analysis](behavioral-analysis.md) owner's. See the [architecture map](../README.md).

## §9 Analytics

**Label.** The technique rules and algorithm ownership are accepted (ADR-0005, ADR-0021,
ADR-0044). The default analytics (Passes A–C, the behavior scan, direct usage and selection) are
**Implemented** and **Tested** (2026-09-23/24; tests named per section). The optional techniques
are **Implemented** and **Tested** as off-by-default variants (§9.8).

**Rules for every technique:**
- it must have a **named consumer** in the served model: a tool's output or a brief (ADR-0021);
- it must be **deterministic** for fixed inputs and parameters;
- it must record its method, parameters, projection and diagnostics in `analysis_invocations`
  (ADR-0047);
- it runs by default only if it passes the keep criterion (§9.8), and its effect must be
  measurable as a mechanical diff.

**Algorithm ownership** (ADR-0044; DESIGN [§B4](../DESIGN.md#section-b4)). Traversal is an
explicit BFS over immutable petgraph projections; communities use leiden-rs with RBER; PageRank
and FCA (NextClosure) are our own code, with independent oracles (`leiden_rs::compute_flow`,
`fcars`). Transfer summaries use petgraph's iterative `kosaraju_scc` for component discovery;
`lctx-analytics` owns sorted members and canonical callee-first condensation order
([ADR-0052](../../adr/0052-iterative-scc-schedule.md)). A 30,000-edge chain and real
component-order control passed in focused tests (2026-09-26); pilot cost remains open
([plan W13](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)).
The engine for recursive summary composition (bounded native worklist, Ascent or datafrog) is an
**open comparison** at plan order 6, not an adopted dependency
([plan W12](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)).

**Recursive relational walks.** The type-term walks behind the type layer (§9.4) and FCA's
type attributes (§9.6) are DataFusion recursive CTEs over `type_term_args`. They use `UNION`
distinct at each iteration. Unknown ancestry has one row per term id; the type layer has one row
per function and term id. Both downstream consumers use set membership, so repeat paths carry no
separate evidence or modality. A cyclic edge now terminates and produces the same attributes and
type-layer membership as its acyclic counterpart in focused tests (2026-09-26), rather than
relying on extraction's depth cap. The next integrated all-techniques digest must be repinned
after this SQL change ([plan W13](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)).

**The public paths** (**Implemented** and **Tested**, 2026-09-24). One relation,
`cpg_schema::public::public_paths`, names every public spelling of every public function and
class. It is persisted as the analysis table of the same name, written before Stage E, and every
consumer (seeds, communities, selection, FCA scopes, retrieval's `symbol_map`) reads it. Its roots
are bound as `$roots`.
- **Exports:** each declaration exported under a root by a path with no private segment; where a
  path names several, an implementation is preferred over a stub.
- **Members:** each public member, `__init__` or `__call__` an exported class declares or inherits
  along its MRO. The nearest definer wins, with the seed rank among its definitions. The walk
  **fails closed**: nothing is inherited past an unresolved base or an ancestor outside the
  release that precedes the definition, and nothing where a class up to the definer binds the
  name by an assignment or import.
- **Domain limits.** Members are **one level** below an exported class: a nested class is a row,
  but its own members are not, so a seed or gold operation at depth 2 is refused or unresolved.
  An **undefined** base (a name Pyrefly cannot resolve) leaves no ancestry row, so members are
  inherited past it: the refusal fires only for a base that is named but unmapped. Neither case
  occurs on the pilot (Measured, 2026-09-24).
- **`own`** marks a path whose export declares the node. **`preferred`** marks one path per node:
  a path through the class that declares the member first (so a classmethod is never named
  through a subclass it would bind differently), then the fewest segments, then the least. One
  path names one node (`semantic:public-path-one-node`). Every display of a public callable
  (community members, Related, selected seeds, kNN texts) uses the preferred path
  (`cpg_schema::public::preferred_callables`).
- **Tests.** Rules `semantic:public-path-exported`, `-preferred` and `-own`, each with an injected
  case; `public_paths_agree_with_the_seed_resolution` on `analysis_shapes` (it refuses
  `pkg.Shadowed.run` and `pkg.Aliased.tool`); `public_shapes` covers inheritance, rebinding, an
  outside ancestor, overloads, nested classes and private names; a hand-built session covers an
  unresolved base, which no validated snapshot holds.

**Parameters.** The subsystem declaration ([§1.4](../DESIGN.md#section-1-4)), the seeds and the
pass budgets live in one versioned, pre-registered analytics config
(`libraries/fastmcp/analytics.toml`). Its digest is the compiler run's config digest and part of
`content_digest`. The community, PageRank and selection parameters are pre-registered code
(`communities::Params`, `ranking::Params`, `selection::Params`): each invocation records them,
the compiler digest includes them, and `eval/gold/analytics-freeze.json` pins their digest
beside the config's. The config file and these parameters are frozen; an edit is an ADR-0021
amendment. Defaults below are starting budgets, not measured optima.

**Per public callable, not per seed** (ADR-0021; **Implemented** and **Tested**, 2026-09-24).
- **Persistence.** The passes' relations are persisted as the `behavior` tables
  ([§3.2](facts-and-identity.md#section-3-2)).
- **The behavior scan** (`pass_b_surface`, one invocation) runs Pass B's worklist from **every**
  public callable and follows parameters into **any release function**, not only subsystem
  callees. Test: `behaviors_cover_public_callables_outside_the_subsystem`, where
  `pkg.relay.relay` forwards `path` to `pkg.Catalog.load` from outside the fixture's prefixes.
- **An operation's `behavior_status`** is `established` only when its scan met no boundary **in
  its region** (the callables it reached). Otherwise it is `unknown`, with the first boundary in
  `boundary_reason` and every one in `status_reason`. The boundaries, in that order:
  - `budget_reached`: the scan stopped at the depth bound, or a formal it reached is read at the
    frontier;
  - `override_dispatch` or `ambiguous_binding`: a path, or the operation itself, crosses a
    candidate or potential arc;
  - `unresolved_target`: the operation has a call site resolution leaves open, or a tracked value
    reaches one in a callee (`open_site_reads`);
  - `outside_provider_model`: a tracked value is read in a form the scan does not follow.

  Tests: `one_arc_has_one_verdict_and_the_region_decides_the_status` on `behavior_shapes`, and the
  rule `semantic:established-needs-definite-path` with its injected case.
- **Seed findings remain the input to briefs, and differ in mask.** The seed passes follow
  subsystem callees only, and briefs state that stop as a limit, so a seed's brief and its
  behavior rows can differ past the subsystem edge.

> Decision: ADR-0005, ADR-0047, ADR-0021, ADR-0044, ADR-0045


### §9.1 Pass A — public entry point and delegation

**Implemented** and **Tested** (2026-09-23): `lctx_analytics::pass_a` with its hand-worked
projection (`pass_a_finds_delegations_boundaries_and_gaps_with_witnesses`,
`budgets_truncate_and_say_so`), and on `fixtures/python/analysis_shapes` through the whole
attempt, identical across module order and location
(`pass_a_is_identical_across_module_order_and_location`).

A seed is its **`public_paths` row** (§9), and its aliases are the rows naming its node through
the same container (the same exported class, or the module level for a direct export). The
relation's member rule and its two domain limits (§9) therefore apply to seeds. A seed with no
row is refused, naming it (`a_seed_that_could_name_another_method_is_refused`).

- **Question.** Which public API exposes the mechanism, and what does it already coordinate?
- **Method.**
  1. Map public access paths → declarations (`exports`).
  2. From each seed (the declaration node, [§3.4.1](facts-and-identity.md#section-3-4-1)), run an
     **explicit BFS with parent pointers** over the invocation projection (§5).
     - **Arcs** are calls and definitions (§5). A definition arc reaches a nested callable, so a
       decorator factory's behaviour is in its neighbourhood.
     - **Witnesses.** The first witness to a target is the BFS path under sorted adjacency. Up to
       two alternatives are the next **shortest** paths that differ in their final arc, taken in
       canonical arc order. Parallel call sites are distinct arcs, so each can be a witness. A
       longer route to a target already reached is neither shown nor flagged.
     - **Kind.** It is decided over every shortest final arc, never the kept witnesses: a
       `direct_delegation` needs a definite call arc at depth 1, and its first witness is that
       call (`semantic:direct-delegation-is-definite`).
     - **Truncation.** `witnesses_omitted = true` when more shortest final arcs existed than the
       witness budget kept (presentation only; `stop_reason` `witness_limit` is reserved). A
       vertex or arc budget is operational truncation: the invocation is `partial`, and Stage F
       reports it as a limit. The depth bound and the boundaries are the stated model. The depth
       bound is recorded (`depth_limit`) when a frontier vertex has an arc or an unresolved site
       that was not followed.
     - Default budgets: depth ≤ 2; ≤ 128 vertices and ≤ 512 edges per seed; ≤ 3 witness paths per
       target.
     - The traversal stays inside the subsystem and never crosses `potential`, `synthetic` or
       native boundaries.
- **Output.**
  - Findings: `public_alias`, `direct_delegation`, `bounded_delegation_path`,
    `implementation_boundary`, `incomplete_resolution`, and `traversal_stop`. The last is
    emitted when the depth bound or a budget left something unfollowed, so the Limits entry that
    states it cites a finding.
  - Each carries ordered witness call sites, depth, stop reason and an `omitted_paths` flag.
- **Interpretation boundary.** A call edge never means "always reached" or "recommended". That
  needs `documented` evidence.


### §9.2 Pass B — controls and local restrictions

**Implemented** and **Tested** (2026-09-23; tests below). The worklist also drives the
whole-surface behavior scan (§9).

- **Question.** Which controls expose the behavior, and which local conditions constrain them?
- **Recognizers.** Four:

| Recognizer | Accepted pattern | Output |
|---|---|---|
| direct forwarding | a call argument resolves to a source parameter binding, mapped per candidate signature (including implicit receivers) | `forwarding` |
| identity alias | one unambiguous local assignment in straight-line code | `forwarding` (via alias) |
| defaulted or transformed argument | a literal, default or expression supplied downstream | `transformed_argument` |
| local restriction | a supported predicate over known parameters leading to a local `raise` | `conditional_raise` |

- **Supported predicate.** An expression of the function's own parameters (each bound once),
  builtin names and literals, joined by comparisons, `not`/`and`/`or`, arithmetic, tuples, lists
  and sets, where every call's callee is a builtin's name. An attribute, subscript, other call,
  lambda, comprehension or walrus reads state the analysis does not model, so it is not a guard.
- **Never guessed:** `*args`/`**kwargs` passthrough; ambiguous overloads; multiple writes;
  property or subscript access; any [§4.2](acquisition-and-extraction.md#section-4-2)
  `ambiguous_binding` site.
- **Visited key.** `(callable, formal, source parameter)`: the mapping context is the source
  parameter, since the mapping itself is per arc. **Known gap:** whether a raise is suppressed
  depends on the path that reached the state, but the key omits that property, so a raise is
  judged by the first BFS path to each state; a clean path found second is lost. The intended
  key includes a clean-path bit, and Pass B and Pass C builders should refuse unsorted input as
  the graph builder does
  ([plan W14](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)).
- **Promotion.** A `conditional_raise` is reported as "the implementation raises in this
  branch". It becomes a public precondition only with supporting evidence, and never when an
  enclosing handler may catch it.
- **What a path must say.** Every Pass B template shares one path-qualifier rule: each
  override-open hop is named, and each call its caller makes only on some paths is said. A raise
  is not reported when a call on the path may be absorbed (inside a `try` or a `with`, whose
  context manager may suppress it) or sits in a construct of its caller that also tests the
  flowing value (the caller may pass only values the callee accepts).
- **What is declined leaves a trace** ([§4.2.4](acquisition-and-extraction.md#section-4-2-4)). A
  reached parameter read at a call into the subsystem in a form not followed (rebound in its
  scope, inside an expression, unpacked, or taken by no single formal) is an
  `unfollowed_argument` finding, stated in Limits, so "not listed as passed on" is never read as
  "not passed on".

**Implementation.**
- **Relations.** Declared SQL relations (`cpg_schema::flows`), whose digest joins the compiler
  digest. Their arcs take the invocation projection's accepted evidence, and one mapping fragment
  (`maps_formal`) serves Pass B and Pass C.
  - *Argument flows:* each argument of a `call` or `init` arc mapped to one formal of its
    target, the implicit receiver counted. A positional argument before any `*` argument maps by
    index; a keyword argument maps by name to a positional-or-keyword or keyword-only formal,
    also after a `*` argument. Each value is classed as a parameter bound once, one identity
    alias assigned directly in the caller's body, a literal as written, or other. Each row says
    whether its call site sits in a `try` or `with` of the caller, in a conditional construct
    (`if`, loop, `match`, conditional expression, boolean operator, comprehension, lambda,
    `except` clause), and whether such a construct also reads the flowing value.
  - *Guards:* an `if` directly in a function's body whose test is a supported predicate reading
    at least one of its parameters, with a `raise` directly in its branch.
  - *Parameter reads:* each argument of an arc whose value reads a name the caller binds under
    one of its parameters' names: rebound or not, bare or not, unpacked or not.
  - *Receivers:* a method's first positional parameter unless Pysa says it is static, by the
    declaration's kind and never by the parameter's name (the seed parameters and the brief's
    parameter list both use it).
- **Worklist.** `lctx_analytics::pass_b`, keyed as above and bounded by Pass A's depth. For
  briefs (the seed passes) it follows parameters and aliases into subsystem callees only; the
  behavior scan follows them into any release function (§9).
  - It reports `forwarding`, `transformed_argument` (a literal the seed itself supplies),
    `conditional_raise` and `unfollowed_argument` (reason `rebound`, `computed` or `unmapped`).
  - A call its caller makes only on some paths is a `conditional_call` member of each finding
    whose path crosses it.
  - A raise is declined when a call on the way may be absorbed or is tested by its caller, or
    when the tested parameter is rebound.
  - Never mapped: starred arguments and any positional after one, `**` arguments, catch-all
    formals, property or subscript values, and literals below the seed.
- **Stage F.** Pass B's findings become assertions, all `structurally_observed` (the kind policy
  permits nothing more). Documented text reaches Controls and Limits through
  [§10.3](synthesis-and-serving.md#section-10-3)'s parameter descriptions and documented
  warnings, not through Pass B:
  - `control`: per seed parameter, where it is passed on, with the path qualifiers;
  - `transformed_control`, with the path qualifiers;
  - `restriction`: "`callee` raises (`raise E`) when `test`; its `formal` receives `p`", with the
    path qualifiers, citing the test's and the raise's syntax facts;
  - `unfollowed_control` (Limits): per seed parameter, the callees it reaches in forms the
    analysis does not follow, and why. Like `analysis_boundary`, it stays out of the brief
    document.
- **Tests.** `pass_b_finds_the_known_answers_on_analysis_shapes` (`pkg.configure`) covers
  forwarding directly, through an alias, over a bound receiver, over a class receiver
  (`Registry.create`) and into a constructor (`Widget(name)`); two mappings of one parameter; a
  literal; a depth-2 chain and the depth bound's stop; raises reached by each. It shows no raise
  for the `try`-guarded call, the `contextlib.suppress` call, the call under `if name is not
  None`, the attribute guard, the rebound guard or `**options`, and an `unfollowed_argument` for
  each of a rebound value, a computed one, a positional after `*extra`, `*extra` itself and
  `**options`. `templates_say_what_the_findings_show` checks the overridable and conditional
  qualifiers in the published text.


### §9.3 Pass C — direct handoff

**Implemented** and **Tested** (2026-09-23; tests below).

- **Question.** Which public APIs already connect without an adapter?
- **Method.** Over the CPGs of **official examples, tests and doc blocks**, recognize
  `x = producer(...); consumer(x, ...)` and direct nesting, in straight-line regions only.
- **Rejected as a handoff:** reassignment; additional consumers; escapes; resource boundaries.
  Type compatibility alone is a `candidate`, never a published pattern.
- **Output.** A `handoff` finding per (other callable, consumer formal), with its status, its
  occurrence count as score, and its first occurrences' producer and consumer sites (the region
  and binding are the sites' statements).

**Implementation.**
- **Relation.** `cpg_schema::flows::handoffs_sql`. Producer and consumer are both callables the
  release declares, never a helper the usage code defines itself. It holds each occurrence of:
  - `x = producer(...)`, the statement's only target, then `consumer(..., x)`, in a later
    statement of the same block, with the call as that statement's value (awaited or not). `x`
    must be bound once and read once, except as a receiver;
  - or `consumer(..., producer(...))`.
  - The argument maps to a formal by the mapping fragment Pass B shares (`maps_formal`).
- **Setup is not a consumer.** A read of `x` as the object of a called attribute or a decorator
  (`x.method()`, `@x.tool`) configures the produced object. A second argument use, any other
  attribute read, a return, a store or a reassignment rejects the named occurrence, and so does
  a `with` item. The nested form is counted wherever it occurs.
- **Paths.** A doc block is named by its document and fence number (`docs/x.mdx, code block 3`),
  never by the module the compiler materialized it as (`flows::usage_files_sql`, checked by
  `semantic:no-materialized-block-path`).
- **Kernel.** `lctx_analytics::pass_c` groups occurrences per `(other callable, formal)` into one
  `handoff` finding. Its score is the count. Its members are the formal and three occurrences:
  examples first, then doc blocks, then tests. Like Pass B, its builder does not yet refuse
  unsorted input (W14, §9.2).
- **Tests.** `handoffs_and_usage_patterns_come_from_official_code` covers a named handoff after a
  receiver use and a nested one (counted), and two consumers, a reassignment, a usage-defined
  helper, a chained assignment and a passed-on attribute (not counted);
  `pass_c_and_usage_patterns_are_identical_across_location_and_module_order`.


### §9.4 Community detection

**Variant, off by default** (`+communities`; ADR-0020, §9.8). **Implemented** and **Tested**
(2026-09-23). No tool consumes communities today (§9.8).

- **Consumers when enabled:**
  - **seed selection**, where a community caps how many briefs one group receives (below);
  - the brief's **Related** field ([§10.3](synthesis-and-serving.md#section-10-3)).

  A community is not a brief boundary (a brief is one outcome at one public operation) and never
  an FCA scope.
- **Library.** leiden-rs 0.8.1 with `default-features = false` and no features, so it runs
  sequentially, with the **RBER** quality function: CPM with γ relative to the graph's density,
  so γ is scale-free on unit-normalized layers. Not used: the `petgraph` adapter
  (`from_petgraph` would read §5's `u32` arc-row weight as the edge weight), `run_multiplex` (it
  ignores `layer_weights` after the first level; our weighted sum of layers is the objective),
  and `resolution_scan`/`resolution_profile` (they change seeds between points). The graph is
  built with `GraphDataBuilder` from the kernel's own dense index.
- **Input normal form.** The undirected builder does not normalize orientation, and shuffled
  input changes a partition (Tested). So each layer is **integer counts** per `(min, max)` pair,
  counted in Rust over the declared relations' rows in a `BTreeMap`; normalization, hub
  down-weighting and layer weighting follow in canonical order. Each pair keeps its least
  contributing site as lineage, and a community cites the sites behind its strongest pairs.
- **Determinism.** The seed is always set and recorded; `track_quality_history` stands in for the
  missing converged flag; rand is pinned on its 0.9 line; crate versions are in each invocation's
  `library_versions`.
- **Relations** (`cpg_schema::communities`, digested with the invocation projection's digest into
  every community invocation's `projection_digest`): the co-use occurrences (each usage-code call
  to a function with its enclosing scope, over the flows' call targets) and the public callables
  (the function and async-function rows of `public_paths`).
- **Layers**, each a named policy in `Params`. *Invocation:* every invocation-projection arc
  between two distinct subsystem functions (calls, property accesses and definitions; definite
  and candidate), 1 per arc. *Co-use:* every official-usage scope calling two distinct subsystem
  functions, 1 per scope.
- **Weights.** Hubs: an end whose strength (its summed counts) exceeds the layer's
  95th-percentile strength scales the count by threshold / strength. Each layer is normalized to
  unit total, and the layers are summed with weight 0.5 each. The vertices are the subsystem
  functions some pair touches; an isolated function is in no community and not in RBER's density
  term.
- **Resolution.** RBER at **γ = 1**, its own density scale, over seeds 0–9. The profile
  γ ∈ {0.5, 1, 2, 4} also runs and its stability is recorded (mean and SD of pairwise ARI, mean and
  min NMI, community count, largest community), but γ is never chosen from it: a grid choice
  proved to depend on the seed block, not the data. γ = 1 is degenerate when its seed-0 partition
  puts more than half the vertices in one community or has no community of three; then nothing
  is reported and the diagnostics say so.
- **Reported.** Each community of the seed-0 partition with at least two public members whose
  **co-assignment** (the mean, over seeds 1–9, of the share of its public-member pairs sharing a
  community) is at least 0.5, so the score measures the published grouping. A `community`
  finding lists its public APIs (`community_member`, with access path and strength), cites up to
  three supporting sites (`supporting_site`), and has the co-assignment as score.
- **Records.** One `leiden` invocation per run (γ, seed, iterations, convergence, quality
  history) and one `community_consensus` invocation, whose `diagnostics` hold the profile, the
  choice and what was reported.
- **Tests.** `communities::tests`: LFR planted partitions (n = 250, μ = 0.1 and 0.3) recovered at
  γ = 1 with NMI ≥ 0.9; shuffled and flipped edges give the identical consensus; hub
  down-weighting; a trivially small graph is degenerate; the co-assignment score; the parameters
  against their gold freeze. `the_digest_follows_the_invocation_projection` (cpg-schema);
  `communities_are_stable_and_projected_onto_public_apis` on `analysis_shapes`; the co-use layer
  on `docs_shapes`; the module-order and location tests cover the rows.

**Extra layers** (variants `+type-layer`, `+mention-layer`, `+knn-layer`; **Implemented** and
**Tested**, 2026-09-23/24):
- **`+type-layer`** (`cpg_schema::communities::shared_types_sql`): two subsystem functions that
  name one release class anywhere in their declared parameter types (a recursive walk of
  `type_term_args`, the receiver aside; see §9's note on recursive walks) are a pair, 1 per class.
- **`+mention-layer`** (`co_mention_sql`): two subsystem functions that one doc passage's exact
  mentions name, 1 per passage.
- **`+knn-layer`**: each subsystem public API with its three nearest other APIs by embedding
  cosine at or above the kNN floor (§9.7), 1 per direction found. It needs an embedder.
- **Combination.** With extra layers, every layer weighs 1 / (number of layers)
  (`EXTRA_WEIGHT_RULE`); each layer keeps its own hub down-weighting and normalization. Without
  them, `Params`' 0.5/0.5 applies. The consensus records the layers, their policies and the rule,
  and its diagnostics give each extra layer's pairs and hub threshold. A layer is a `LayerSpec`
  (type, mention, or kNN with its embedding spec hash, `k` and similarity floor) matched
  exhaustively; its SQL and lineage join the community relations' digest (`extra_digest`) and the
  consensus parameters, so two embedding specs never share an invocation id
  (`the_knn_layer_digest_follows_its_lineage`). A supporting site names its layer.
- **Tests:** `variants_add_relational_attributes_and_layers` (type and mention layers on
  `analysis_shapes`; the default's diagnostics unchanged) and
  `mention_and_knn_layers_come_from_the_corpus` (`docs_shapes` with the words embedder; the kNN
  layer without an embedder is refused).

**Seed selection and naming** (part of the default; **Implemented** and **Tested**, 2026-09-23):
- **Seed selection** (`lctx_analytics::selection`). Ranking (§9.5), and communities when enabled,
  run first, before any per-seed pass. The configured seeds come first. While the brief budget
  allows, the **eligible** public APIs follow by rank: the most direct official-usage calls first
  (PageRank in the `+pagerank` variant), ties to the smaller id. Eligible means official usage
  calls it at least once and its docstring has a summary, the Outcome its brief will state
  (`synth::summary_span`). With `+communities`, communities **cap** rather than choose: an API is
  skipped once its community holds ⌈budget / 3⌉ seeds, configured ones included; an API in no
  community is capped by nothing. Without communities there is no diversity device, and direct
  usage alone orders selection. Each selected API is named by its preferred path; one whose path
  resolves to another declaration is dropped and recorded. A `seed_selection` invocation records
  the rule, the technique set and its choices (`selection::Params`, frozen) and names the
  configured, selected and dropped seeds and the eligible count. Each selected seed gets Passes
  A–C and a brief (and FCA of its scope when `+fca` is on). Tested by `select`'s unit test,
  `selection_needs_official_usage` (`analysis_shapes` has no usage code: nothing is eligible) and
  `selection_takes_what_usage_calls_within_the_budget` (`docs_shapes`, budget 4:
  `pkg.Server.run` is chosen).
- **Related** (with `+communities`; §10.3): the seed's community co-members, at most five, the
  most called in official usage first (by PageRank in its variant; by name, and saying so, when
  usage calls none of them), citing the community and each listed member's `direct_usage`
  finding, so its status derives `statistically_derived`. The kind policy permits
  `statistically_derived` only for `related`, and a control citing a community finding or stated
  statistically is rejected (two injected cases in `the_analysis_rules_reject_their_violations`).

> Decision: ADR-0020


### §9.5 Centrality

**Direct usage is the default** (**Implemented** and **Tested**, 2026-09-23); delegation
PageRank is the `+pagerank` variant (**Implemented** and **Tested**, off by default).

- **Consumer.** Which operations get briefs (seed selection, §9.4) and the order of a Related
  line. The question both ask is **which operations official usage calls**.
- **Method (default): direct usage** (`lctx_analytics::ranking::usage_counts`). Each call arc
  (definite or candidate, any phase) from an official-usage caller (a function or module of an
  example, test or doc block) into a subsystem function; each call site counts once, split evenly
  among its targets (`USAGE_POLICY`, digested with the invocation projection). A method call on an
  instance is a `candidate` arc (Pysa's override marking), almost always with one target, so a
  definite-only count would see constructors and module functions only. One `usage_count`
  invocation; each public API usage calls is a `direct_usage` finding (`structurally_observed`: a
  count of observed calls) whose score is its count. It orders, and never states behaviour.
- **Why not PageRank by default.** PageRank over the usage projection ranks what usage reaches
  **through the library's own delegation**, so implementation sinks rank high. On the pilot
  (Measured, 2026-09-23) its top entries were helpers the whole surface delegates to, while
  `FastMCP.call_tool`, with 320 direct usage sites, ranked 16th. That answers a different question
  from the consumer's, so it is kept only as a variant for the keep criterion (§9.8).
- **Method (`+pagerank`).** Our own weighted power iteration (ADR-0044) over the **usage
  projection**: the invocation projection restricted and weighted by a named policy
  (`WEIGHT_POLICY`, digested into the compiler digest and the invocation's `projection_digest`).
  Vertices: every subsystem function, and every official-usage caller with an arc into one.
  Arcs: each invocation arc (call or definition, any accepted modality) into a subsystem function
  from a subsystem function or a usage caller, weighted by the arc count per ordered pair.
  - **Iteration.** `r'ⱼ = (1−d)/n + d·(Σᵢ rᵢ·wᵢⱼ/Wᵢ + D/n)`, from the uniform start, over the arcs
    in canonical order, where `D` is the dangling mass (uniform target); damping 0.85, L1
    tolerance 1e-10, 100 iterations (pre-registered code, frozen by digest in
    `eval/gold/analytics-freeze.json`).
  - **Records.** The `pagerank` invocation records iterations, the final L1 residual, `converged`
    and diagnostics; each public API is a `centrality` finding (`statistically_derived`) whose
    score is its rank.
  - **Oracle.** `leiden_rs::compute_flow` (weighted, directed, uniform teleport and dangling mass)
    is valid while the dangling target is uniform.
- **Tests** (`ranking::tests`): the hand-computed 3-node fixed point; parallel arcs counted with
  their weights; a 2-iteration budget reported as not converged; a permuted projection giving an
  identical usage graph, scores and counts; `compute_flow` agreeing to 1e-9 on one hand-made graph
  and on 30 random weighted digraphs of 5–44 vertices, a quarter dangling; direct usage ranking
  what usage calls above the sink its delegation reaches, which PageRank ranks first. The usage
  branch is covered by the unit test's usage caller and by
  `selection_takes_what_usage_calls_within_the_budget` on `docs_shapes`; on `analysis_shapes`,
  with no usage code, `public_apis_are_ranked_over_the_usage_projection` runs the `+pagerank`
  variant.


### §9.6 Formal and relational concept analysis

**Variants, off by default** (`+fca`, `+rca`; ADR-0020, §9.8). **Implemented** and **Tested** for
brief-era structural attributes (2026-09-23). FCA over **behavioral** attributes as candidate
facets is the Stage 4 target consumer (**Proposed**; plan Stage 4 item 7).

- **Consumer.** Shared signatures and implication-style assertions in briefs (e.g. "every writer
  accepting `filesystem` also accepts `format`"); from Stage 4, candidate facets. FCA nominates;
  it never writes concept membership ([§9.9](behavioral-analysis.md#section-9-9)).
- **Kernel** (`lctx_analytics::concepts`; ADR-0044). Our own NextClosure (Ganter) over
  `fixedbitset`, enumerating every set closed under the implications found so far, so one pass
  gives the frequent concepts and the frequent Duquenne–Guigues basis. Pruning infrequent
  candidates is exact: a larger set never has a larger extent. FCbO replaces it only if the
  concept count exceeds the budget. There is no stability index (it is #P-hard). `fcars =0.2.2`
  is a dev-dependency **oracle** for concept sets; no cover relation is computed, so none is
  checked. Parameters (pre-registered code, frozen): support 2 APIs, budget 20,000 closed sets (a
  budget stop is `concept_budget`, completion `partial`).
- **Scopes are nodes.** Objects are the public APIs of one **structurally defined scope**: the
  public namespace a seed's access path names (the exported class or module `container`, e.g.
  `fastmcp.FastMCP` for `fastmcp.FastMCP.tool`: its public methods, declared or inherited; a
  package's own module when no export names it). Each container resolves to its class or module
  node; container strings naming one node are one scope and one invocation, labelled by the
  fewest-segment, then least, of them. One FCA invocation runs per distinct scope with at least
  two APIs. Each seed records its scope (`AnalysisRows.seed_scopes`), and Stage F reads concepts
  and implications of that scope only, so a method inherited into two scopes is never described
  through the other one. Communities are never an FCA scope: their membership is statistical.
- **Attributes** (`cpg_schema::concepts::attributes_sql`, digested): `parameter NAME` (`*`/`**`
  for the catch-alls; the receiver aside by the method's kind), `parameter type T` (declared),
  `returns T` (declared), `raises E` (the typed `raise`s directly in the body; a bare re-raise
  has no type) and `decorator D`. A type Pyrefly could not determine is no attribute: a term
  that is, or holds anywhere in its structure (a recursive walk of `type_term_args`; §9's note on
  recursive walks), an `Any` of style `error` or `implicit`. A raised class is one attribute
  whether raised as the class or an instance; only class-typed raises count. The rule
  `semantic:concept-attribute-known` is a tripwire over labels.
- **RCA** (`+rca`, requires `+fca`). One relational-scaling step: for each object of a scope,
  `calls X` for each call arc (definite or candidate) into a subsystem function, and
  `hands off to X` / `takes from X` for each handoff the Pass C relation holds, where X is the
  partner's preferred path, else its qualified name. The pairs come from the in-memory projection
  and Pass C's relation; `RCA_POLICY` joins the FCA invocations' parameters and relation digest.
- **Known gap: attributes are presentation labels.** The intended contract is typed attribute
  keys carrying endpoint identity, modality and evidence, mapped to bit positions and rendered
  only after analysis. Today attributes are English strings: RCA admits candidate call arcs but
  labels them `calls X` like definite ones, Stage F recovers meaning by parsing label prefixes
  and ignores unknown forms, and a concept-set oracle over the flattened context cannot see the
  lost modality. Candidate-derived `+rca` output is therefore not a behavioral claim, and FCA/RCA
  are not extended before this is fixed
  ([plan W10](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)).
- **Findings.** Each frequent concept with a non-empty intent is an `applicable_case` finding
  (`extent_member` APIs, `intent_attribute`s; score = extent size); each basis implication an
  `implication` finding (`premise`, `conclusion`; score = support). The subject is the scope. Both
  are `structurally_observed`: they are exact over the extracted attributes, with the scope
  stated. (The kind name `applicable_case` is historical: a brief states the concept as a shared
  signature under Related, not as the Applicable case.)
- **Stage F.** A seed's `shared_signature` assertion (section Related) is the concept of **its
  own** scope that holds it with another API, shares at least two attributes, and has the most
  (other API, shared attribute) pairs, |intent| · (|extent| − 1), then the larger intent: "Like
  `A`, `B` and `C` (4 public APIs of `S` in all), `X` declares … and raises `E` directly in its
  body." Up to three `implication` assertions (section Important controls) are the
  best-supported implications of its own scope whose premise the seed meets. The scope is named
  in the text, and neither enters the brief document's header. With `+rca` the templates say
  "calls `X`" and "has its result passed to `X` in official usage". The Applicable-case slot stays
  **absent** until an input-or-mode source exists, so the [§B11](../DESIGN.md#section-b11) gap
  metric sees it. The floor, caps and counts are `selection::Params`, frozen.
- **Tests** (`concepts::tests`): a hand-computed context (seven concepts; the basis `b → a`,
  `c → a`, `{a,d} → {b,c}`); fcars 0.2.2 agreeing on random contexts at supports 0, 2 and 4; the
  basis sound and complete (closing every subset under it gives its Galois closure); the budget
  stop; the frequent basis sound, complete over every frequent set and non-redundant at supports
  1–4 on 29 random contexts. `concepts_come_from_each_seeds_structural_scope` on `analysis_shapes`
  (a registration family, two loaders, an unrelated method; an unresolvable return type yields no
  `Unknown` attribute and `raises KeyError` once);
  `fca_scopes_are_nodes_and_state_only_their_own_apis` (two paths to one class compile as one
  scope; a subclass adds a family without leaking into the parent's lines);
  `relations_become_attributes_of_the_objects_in_scope` and
  `variants_add_relational_attributes_and_layers` for RCA.

> Decision: ADR-0020


### §9.7 Embeddings in analytics

**Variant, off by default** (`+knn`; ADR-0020, §9.8). **Implemented** and **Tested**
(2026-09-23). The operation vectors that `search_operations` ranks by are
`operation_documents` embedded through the cache ([§11.1](synthesis-and-serving.md#section-11-1)),
not this technique's output.

- **Consumers when enabled:** linking doc passages to APIs, supplementing explicit mentions;
  labelling communities by their nearest doc heading; kNN as an optional community layer (§9.4).
- **E0** (`cpg-core::embed::embed_texts`), in Stage E before the communities, whenever kNN or the
  kNN layer reads vectors and an embedder is configured. It embeds the corpus passages
  (`cpg_schema::neighbours::passages_sql`) and the subsystem's public APIs (`path(parameters)` and
  the docstring; `api_texts_sql`, text version 1) as documents under the spec. Each text is cut
  into windows of at most 4,096 bytes at line ends, never dropping text. Vectors are read from
  the cache or embedded and merged; E0's keys join the snapshot's key set in `content_digest`.
  The byte window is a preparation proxy, not token admission, and E0 uses the cache-fill path
  that returns locally computed vectors rather than the committed ones (§11.1;
  [plan W9](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)).
- **kNN** (`lctx_analytics::neighbours`): exact, the best cosine over window pairs. Each API's
  three nearest passages at or above 0.5 are `doc_link` findings; each reported community's
  centroid takes its nearest passage's heading as a `community_label`. Both are
  `statistically_derived`, and one `knn` invocation records the candidate set (APIs × passages)
  and the counts. The parameters (k 3, floor 0.5, 4,096-byte windows) are pre-registered code,
  frozen with the others.
- **Stage F.** A `doc_link` assertion (section Related) lists the operation's nearest
  documentation with its cosines; the Related line names its community's label. Neither feeds the
  Outcome.
- **Tests.** `neighbours::tests`: windows never drop text; exact search, the floor and node-order
  ties; a community labelled by its centroid's nearest heading.
  `doc_links_come_from_embedding_similarity` runs `docs_shapes` end to end with a bag-of-words
  test embedder, and the brief carries the link as `statistically_derived`.

> Decision: ADR-0020


### §9.8 Determinism and ablation

- **Determinism oracles** (**Tested** per technique, above): shuffled input row order gives
  byte-identical findings, communities and concepts; reruns with the same `content_digest` give
  identical output.
- **Ablation is mechanical.** Disable one technique and diff the published findings,
  assertions, evidence, briefs and documents (`lctx diff`). This is a join on content ids
  ([§3.4.1](facts-and-identity.md#section-3-4-1)).
- **The keep criterion** (ADR-0021; **accepted**, not yet applied). A variant is turned on by
  default only if the structured evaluation of the stage that introduces its **tool** consumer
  shows three things:
  - it supplies at least one target item, rated present or partial, that the default lacks;
  - it introduces no item rated incorrect or misleading;
  - it does not push a target operation or item out of a tool's first page.

  A variant with no tool consumer by the end of increment 5, or that fails the criterion at two
  consecutive stages, is deleted by ADR with its tests and frozen parameters. Brief retrieval
  (§12(b)) no longer decides.
- **Named consumers today.** FCA over behavioral attributes (candidate facets, Stage 4) is the
  only planned tool consumer among the variants. **Communities, kNN and PageRank have none:** no
  tool takes or returns communities, and `search_operations` ranks by operation views, not kNN
  output. Whether to retain or delete the variants is decided once Stage 4 gives them consumers
  (ADR-0020 stays proposed until then; [plan §7](../../plans/behavioral-model-forward-plan_2026-09-24.md#7-deferred-each-with-a-trigger)).
- **The current default** (ADR-0020, **Proposed** as a decision; **Implemented**): Passes A–C,
  the behavior scan, direct usage, seed selection and Stage F. Every other technique is off. It
  rests on the brief-era keep rule, applied once and **Measured** 2026-09-23: every variant
  compiled at a 20-brief budget with live vectors, scored over all 44 gold aliases and 157 spans
  (§12(a)–(c)) and diffed. Communities traded one hit@5 for one recalled span; FCA and kNN changed
  briefs but not hit@5; no off-by-default variant improved hit@5. Every difference was one or two
  units, so the rule, which has no noise margin, removed communities, FCA and kNN from the default
  and adopted nothing. ADR-0020 holds the table. The increment-5 held-out evaluation is the
  unbiased check; the gold used here is a development set.
- **Variants** (`cpg_core::analyze::Techniques`; **Implemented** and **Tested**). `lctx compile
  --analytics <variant>` takes `default`, or changes to it by name, `+name`/`-name`:
  `communities`, `fca`, `knn`, `pagerank`, `rca`, `type-layer`, `mention-layer` and `knn-layer`,
  all off by default. A community layer needs `communities`, the kNN layer needs an embedder, and
  `rca` needs `fca` (`Techniques::parse` refuses each otherwise). Fixture tests exercise the
  kernels through explicit variants.
- **Variant identity.** The whole technique set (its JSON, every flag) joins the compiler run's
  config digest, so no two sets share a run id and a variant snapshot never shares a content
  digest with another set's. The label names the techniques that are on (`none` when none are).
  The selection invocation records the set, so its id differs per set; every other invocation
  records only what its technique changed (`rca` in the FCA parameters; `extra_layers` and the
  weight rule in the consensus's). A finding the variant does not touch keeps its id, so an
  ablation diff is a join (`variants_add_relational_attributes_and_layers`).

> Decision: ADR-0020, ADR-0021
