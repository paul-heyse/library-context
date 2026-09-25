# Analytics

<!-- owner-intro -->

## §9 Analytics

> Decision: ADR-0005, ADR-0011, ADR-0019

**Proposed** (methods); **Interface-checked** (the libraries named). Source: IP L1735–L1885,
L2233–L2578. Scope extended by ADR-0005 to cover community detection, concept analysis and
embeddings.

**Rules for every technique:**
- it must have a **named consumer** in the served model: a tool's output or a brief (ADR-0021;
  until 2026-09-24, "in the brief");
- it must be **deterministic** for fixed inputs and parameters;
- it must record its method, parameters, projection and diagnostics in `analysis_invocations`
  (ADR-0019);
- its effect must be measurable by ablation (§9.8).

**The public paths** (the holistic assessment's A1; Implemented and Tested 2026-09-24, consumers
moving in Phase 2 of its plan). One relation, `cpg_schema::public::public_paths`, names every
public spelling of every public function and class, and it is persisted as the analysis table of
the same name, written before Stage E. Its roots are bound as `$roots`.
- **Exports:** each declaration exported under a root by a path with no private segment; where a
  path names several, the seed resolution's pick (an implementation before a stub).
- **Members:** each public member, `__init__` or `__call__` an exported class declares or inherits
  along its MRO. The rule is the seed resolution's (§9.1 step 1): the nearest definer wins, with
  the seed rank among its definitions. Nothing is inherited past an unresolved base or an ancestor
  outside the release that precedes the definition, and nothing where a class up to the definer
  binds the name by an assignment or import.
- **Its domain** (R2 F4): members are **one level** below an exported class. A nested class is a
  row, but its own members are not (the seed resolution it replaced walked any depth), so a seed
  or gold operation at depth 2 is refused or unresolved. An **undefined** base (a name Pyrefly
  cannot resolve) leaves no ancestry row, so members are inherited past it: the refusal fires
  only for a base that is named but unmapped. On the pilot, neither case occurs (Measured,
  2026-09-24).
- **`own`** marks a path whose export declares the node. **`preferred`** marks one path per node:
  own first, then the fewest segments, then the least. One path names one node
  (`semantic:public-path-one-node`, R2 F3).
- Rules: `semantic:public-path-exported`, `-preferred` and `-own`, each with an injected case. On
  `analysis_shapes`, every brief's seed is its row, and the relation refuses `pkg.Shadowed.run`
  and `pkg.Aliased.tool`, as the seed resolution does (`public_paths_agree_with_the_seed_resolution`).
  `public_shapes` covers inheritance, rebinding, an outside ancestor, overloads, nested classes and
  private names. A hand-built session covers an unresolved base, which no validated snapshot holds.

**Parameters.** The subsystem declaration (§1.4), the seeds and the pass budgets live in one
versioned, pre-registered analytics config. Its digest is the `lctx-compiler` run's config digest
and part of `content_digest`. The community and PageRank parameters, added after that config was
frozen (D21), are pre-registered code (`communities::Params`, `ranking::Params`; D28, D29): each
invocation records them, the compiler digest includes them, and the gold freeze pins their digest
beside the config's (ADR-0011 review F4). Defaults below are starting budgets, not measured optima.

**Per public callable, not per seed** (ADR-0021; **Implemented** in plan Stage 1, 2026-09-24).
- **Persistence.** The passes' relations are persisted as the `behavior` tables (§3.2).
- **The behavior scan** (`pass_b_surface`, one invocation) runs Pass B's worklist from **every**
  public callable. It follows parameters into **any release function**, not only subsystem callees.
  - On the pilot, 305 operations outside the subsystem have 1,036 forwards into callees outside it
    (the re-review, 2026-09-24).
  - Test: `behaviors_cover_public_callables_outside_the_subsystem`, where `pkg.relay.relay`
    forwards `path` to `pkg.Catalog.load` from outside the fixture's prefixes.
- **An operation's `behavior_status`** is `established` only when its scan met no boundary **in
  its region**, the callables it reached (increment 3's deep review, F2). Otherwise it is
  `unknown`, the first boundary in `boundary_reason` and every one in `status_reason`. The
  boundaries, in that order:
  - `budget_reached`: the scan stopped at the depth bound, or a formal it reached is read at the
    frontier;
  - `override_dispatch` or `ambiguous_binding`: a path, or the operation itself, crosses a candidate
    or potential arc;
  - `unresolved_target`: the operation has a call site resolution leaves open, or a tracked value
    reaches one in a callee (`open_site_reads`);
  - `outside_provider_model`: a tracked value is read in a form the scan does not follow.

  Tests: `one_arc_has_one_verdict_and_the_region_decides_the_status` on `behavior_shapes`, and the
  rule `semantic:established-needs-definite-path` with its injected case.
- **Seed findings remain the input to briefs, and differ in mask** (the ADR review's F7, deferred to
  Stage 2.6). The seed passes follow subsystem callees only, and briefs state that stop as a limit.
  So a seed's brief and its behavior rows can differ past the subsystem edge.

> Decision: ADR-0021, ADR-0022


### §9.1 Pass A — public entry point and delegation

**Implemented** and **Tested** (slice 1.4, 2026-09-23): `lctx_analytics::pass_a` with its
hand-worked projection (`pass_a_finds_delegations_boundaries_and_gaps_with_witnesses`,
`budgets_truncate_and_say_so`), and on `fixtures/python/analysis_shapes` through the whole attempt,
identical across module order and location (`pass_a_is_identical_across_module_order_and_location`).
A seed is its **`public_paths` row** (§9's opening; the holistic assessment's A1, 2026-09-24), and
its aliases are the rows naming its node through the same container (the same exported class, or
the module level for a direct export). The relation carries the member rule that `member()` used
to walk here (a whole-MRO walk, external ancestors included, that **fails closed**; slice 1.4
review F2): nothing is inherited past an unresolved ancestor or a non-release class before the
definition, nor where a class in the chain binds the name other than by `def` or `class`. Two
limits of that domain (R2 F4; §9's opening): members are one level below an exported class, and
an undefined base, which Pyrefly drops, fails open. A seed with no row is refused, naming it. Tested: `a_seed_that_could_name_another_method_is_refused`,
`public_paths_agree_with_the_seed_resolution`. On the pilot the lookup replaced about 0.9 s of
seed-resolution queries (Measured: "analyze: seed selection" 0.89 s before, 0.02 s after,
2026-09-24).

- **Question.** Which public API exposes the mechanism, and what does it already coordinate?
- **Method.**
  1. Map public access paths → declarations (`exports`).
  2. From each seed (the declaration node, §3.4.1), run an **explicit BFS with parent pointers**
     over the invocation projection (§5).
     - **Arcs** are calls and definitions (§5). A definition arc reaches a nested callable, so a
       decorator factory's behaviour is in its neighbourhood (slice 1.4 review F1).
     - **Witnesses.** The first witness to a target is the BFS path under sorted adjacency. Up to
       two alternatives are the next **shortest** paths that differ in their final arc, taken in
       canonical arc order. Parallel call sites are distinct arcs, so each can be a witness. A
       longer route to a target already reached is neither shown nor flagged (review O1).
     - **Kind.** It is decided over every shortest final arc, never the kept witnesses: a
       `direct_delegation` needs a definite call arc at depth 1, and its first witness is that
       call (review F3; `semantic:direct-delegation-is-definite`).
     - **Truncation.** `witnesses_omitted = true` when more shortest final arcs existed than the
       witness budget kept (presentation only; `stop_reason` `witness_limit` is reserved). A
       vertex or arc budget is operational truncation: the invocation is `partial`, and Stage F
       reports it as a limit. The depth bound and the boundaries are the stated model (ADR-0019
       review F4). The depth bound is recorded (`depth_limit`) when a frontier vertex has an arc
       or an unresolved site that was not followed (review O2).
     - Default budgets: depth ≤ 2; ≤ 128 vertices and ≤ 512 edges per seed; ≤ 3 witness paths per
       target.
     - The traversal stays inside the subsystem and never crosses `potential`, `synthetic` or
       native boundaries.
- **Output.**
  - Findings: `public_alias`, `direct_delegation`, `bounded_delegation_path`,
    `implementation_boundary`, `incomplete_resolution`, and `traversal_stop`. The last is
    emitted when the depth bound or a budget left something unfollowed, so the Limits entry that
    states it cites a finding (slice 1.5 review F1).
  - Each carries ordered witness call sites, depth, stop reason and an `omitted_paths` flag.
- **Interpretation boundary.** A call edge never means "always reached" or "recommended". That
  needs `documented` evidence.


### §9.2 Pass B — controls and local restrictions

- **Question.** Which controls expose the behavior, and which local conditions constrain them?
- **Recognizers.** Four:

| Recognizer | Accepted pattern | Output |
|---|---|---|
| direct forwarding | a call argument resolves to a source parameter binding, mapped per candidate signature (including implicit receivers) | `forwarding` |
| identity alias | one unambiguous local assignment in straight-line code | `forwarding` (via alias) |
| defaulted or transformed argument | a literal, default or expression supplied downstream | `transformed_argument` |
| local restriction | a supported predicate over known parameters leading to a local `raise` | `conditional_raise` |

- **Supported predicate** (slice 2.1 review F5). An expression of the function's own parameters
  (each bound once), builtin names and literals, joined by comparisons, `not`/`and`/`or`,
  arithmetic, tuples, lists and sets, where every call's callee is a builtin's name. An
  attribute, subscript, other call, lambda, comprehension or walrus reads state the analysis does
  not model, so it is not a guard.

- **Never guessed:**
  - `*args`/`**kwargs` passthrough;
  - ambiguous overloads;
  - multiple writes;
  - property or subscript access;
  - any §4.2 `ambiguous_binding` site.
- **Visited key.** `(callable, formal, source parameter)` (slice 2.1 review O2: the mapping
  context is the source parameter, since the mapping itself is per arc).
- **Promotion.** A `conditional_raise` is reported as "the implementation raises in this
  branch". It becomes a public precondition only with supporting evidence, and never when an
  enclosing handler may catch it.
- **What a path must say** (slice 2.1 review F1). Every Pass B template shares one path-qualifier
  rule: each override-open hop is named, and each call its caller makes only on some paths is
  said. A raise is not reported when a call on the path may be absorbed (inside a `try` or a
  `with`, whose context manager may suppress it) or sits in a construct of its caller that also
  tests the flowing value (the caller may pass only values the callee accepts).
- **What is declined leaves a trace** (slice 2.1 review F4, §4.2.4). A reached parameter read at
  a call into the subsystem in a form not followed (rebound in its scope, inside an expression,
  unpacked, or taken by no single formal) is an `unfollowed_argument` finding, stated in Limits,
  so "not listed as passed on" is never read as "not passed on".

**Implemented** and **Tested** in slice 2.1, revised by its compact review (2026-09-23;
deviation log D17, D18, D26):
- **Relations.** Declared SQL relations (`cpg_schema::flows`), whose digest joins the compiler
  digest. Their arcs take the invocation projection's accepted evidence (review O1), and one
  mapping fragment serves Pass B and Pass C (review O7).
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
    declaration's kind and never by the parameter's name (review F8; the seed parameters and the
    brief's parameter list both use it).
- **Worklist.** `lctx_analytics::pass_b`, keyed by `(callable, formal, source parameter)` and
  bounded by Pass A's depth. For briefs (the seed passes) it follows parameters and aliases into
  subsystem callees only. The behavior scan follows them into any release function (§9's opening).
  - It reports `forwarding`, `transformed_argument` (a literal the seed itself supplies),
    `conditional_raise` and `unfollowed_argument` (reason `rebound`, `computed` or `unmapped`).
  - A call its caller makes only on some paths is a `conditional_call` member of each finding
    whose path crosses it.
  - A raise is declined when a call on the way may be absorbed or is tested by its caller, or
    when the tested parameter is rebound.
  - Never mapped: starred arguments and any positional after one, `**` arguments, catch-all
    formals, property or subscript values, and literals below the seed.
- **Stage F.** Pass B's findings become assertions, all `structurally_observed` (the kind policy
  permits nothing more; review F7). Documented text reaches Controls and Limits through §10.3's
  parameter descriptions and documented warnings, not through Pass B:
  - `control`: per seed parameter, where it is passed on, with the path qualifiers;
  - `transformed_control`, with the path qualifiers;
  - `restriction`: "`callee` raises (`raise E`) when `test`; its `formal` receives `p`", with the
    path qualifiers, citing the test's and the raise's syntax facts;
  - `unfollowed_control` (Limits): per seed parameter, the callees it reaches in forms the
    analysis does not follow, and why. Like `analysis_boundary`, it stays out of the brief
    document (D14).
- **Tests.** `pass_b_finds_the_known_answers_on_analysis_shapes` (`pkg.configure`) covers
  forwarding directly, through an alias, over a bound receiver, over a class receiver
  (`Registry.create`) and into a constructor (`Widget(name)`); two mappings of one parameter; a
  literal; a depth-2 chain and the depth bound's stop; raises reached by each. It shows no raise
  for the `try`-guarded call, the `contextlib.suppress` call, the call under `if name is not
  None`, the attribute guard, the rebound guard or `**options`, and an `unfollowed_argument` for
  each of a rebound value, a computed one, a positional after `*extra`, `*extra` itself and
  `**options`. `templates_say_what_the_findings_show` checks the overridable and conditional
  qualifiers in the published text.
- **Pilot (Measured, 2026-09-23, snapshot `c9309c78`).** 35 `forwarding`, 4 `conditional_raise`
  and 5 `unfollowed_argument` findings. `FastMCP.tool`'s `name_or_fn` reaches
  `ToolDecoratorMixin.tool`'s `isinstance(name_or_fn, classmethod)` guard, now stated with its
  overridable hop; `FastMCP.mount` has `server is self`. The unfollowed ones are `FastMCP.tool`'s
  `meta` (rebound) and `task` (computed), `FastMCP.resource`'s `mime_type` and `meta` (rebound),
  and `FastMCP.mount`'s `namespace` (computed).


### §9.3 Pass C — direct handoff

- **Question.** Which public APIs already connect without an adapter?
- **Method.** Over the CPGs of **official examples and tests**, recognize
  `x = producer(...); consumer(x, ...)` and direct nesting, in straight-line regions only.
- **Rejected as a handoff:**
  - reassignment;
  - additional consumers;
  - escapes;
  - resource boundaries.

  Type compatibility alone is a `candidate`, never a published pattern.
- **Output.** A `handoff` finding per (other callable, consumer formal), with its status, its
  occurrence count as score, and its first occurrences' producer and consumer sites (slice 2.2
  review O2: the region and binding are the sites' statements).

**Implemented** and **Tested** in slice 2.2, revised by its compact review (2026-09-23; deviation
log D24, D25, D30):
- **Relation.** A declared relation (`cpg_schema::flows::handoffs_sql`) over the official examples,
  tests and doc blocks. Producer and consumer are both callables the release declares, never a
  helper the usage code defines itself (review F2). It holds each occurrence of:
  - `x = producer(...)`, the statement's only target, then `consumer(..., x)`, in a later
    statement of the same block, with the call as that statement's value (awaited or not). `x`
    must be bound once and read once, except as a receiver;
  - or `consumer(..., producer(...))`.
  - The argument maps to a formal by the mapping fragment Pass B shares (`maps_formal`).
- **Setup is not a consumer.** A read of `x` as the object of a called attribute or a decorator
  (`x.method()`, `@x.tool`) configures the produced object (D25, narrowed by review F4). A second
  argument use, any other attribute read, a return, a store or a reassignment rejects the named
  occurrence, and so does a `with` item. The nested form is counted wherever it occurs (review O5).
- **Paths.** A doc block is named by its document and fence number (`docs/x.mdx, code block 3`),
  never by the module the compiler materialized it as (review F5; `flows::usage_files_sql`,
  checked by `semantic:no-materialized-block-path`).
- **Kernel.** `lctx_analytics::pass_c` groups occurrences per `(other callable, formal)` into one
  `handoff` finding. Its score is the count. Its members are the formal and three occurrences:
  examples first, then doc blocks, then tests.
- **Tests.** `handoffs_and_usage_patterns_come_from_official_code` covers a named handoff after a
  receiver use and a nested one (counted), and two consumers, a reassignment, a usage-defined
  helper, a chained assignment and a passed-on attribute (not counted).
  `pass_c_and_usage_patterns_are_identical_across_location_and_module_order` (review F6).
- **Pilot (Measured, 2026-09-23, snapshot `8a882a72`, after the review).** 205 seed occurrences
  in 7 handoff findings, all between release callables: `FastMCP(...)` → `FastMCP.mount(server=…)`
  153 (156 before one-target producers and the narrowed receiver rule), `require_scopes(...)` →
  `FastMCP.tool(auth=…)` 24, `create_proxy(...)` → `mount` 13. The three findings on usage-defined
  helpers (`require_tenant`, `require_access_level`, `two_question_server`) are gone. Every seed
  has a pattern; doc blocks are named by document and fence (`docs/servers/authorization.mdx,
  code block 10`), and `custom_route`'s pattern now comes from a test that imports what it reads.


### §9.4 Community detection

> Decision: ADR-0020

**Off by default since ADR-0020** (the §9.8 keep rule, 2026-09-23): the `+communities` variant. Its record below stands as the variant's.

- **Label.** Implemented and Tested (slice 2.3, revised by the ADR-0011 standard review;
  ADR-0011 accepted with it).
- **Consumers:**
  - **seed selection**, which decides which entry points get briefs within the brief budget;
  - the brief's **Related** field (§10.3).

  A community is not a brief boundary: a brief is one outcome at one public operation
  (IP L1697).
- **Library.** leiden-rs 0.8.1 (`default-features = false` and no features, so it runs
  sequentially; not its `petgraph` adapter, whose `from_petgraph` would read §5's `u32` arc-row
  weight as the edge weight), with the **RBER** quality function: CPM with γ relative to the
  graph's density, so γ is scale-free on unit-normalized layers (ADR-0011). leiden-rs's
  `resolution_scan` and `resolution_profile` are not used: they change seeds between points.
  `run_multiplex` is never used: it ignores `layer_weights` after the first level, so our weighted
  sum of layers is the objective. The graph is built with `GraphDataBuilder` from the kernel's own
  dense index.
- **Input normal form** (the library-leverage review, D2; Tested). The undirected builder does not
  normalize orientation, and shuffled input changed an LFR partition at μ=0.5. So each layer is
  **integer counts** per `(min, max)` pair (counted in Rust over the declared relations' rows,
  which is as bit-stable as an integer aggregate), in a `BTreeMap`; normalization, hub
  down-weighting and layer weighting follow in canonical order. Each pair keeps its least
  contributing site as lineage (H1 F9), and a community cites the sites behind its strongest pairs.
- **Determinism.** The seed is always set and recorded; `track_quality_history` stands in for the
  missing converged flag; rand is pinned on its 0.9 line; crate versions are in each invocation's
  `library_versions`.

**Implemented** and **Tested** (slice 2.3 and the ADR-0011 review, 2026-09-23; deviation log D28):
- **Relations** (`cpg_schema::communities`, digested with the invocation projection's digest into
  every community invocation's `projection_digest`). The co-use occurrences (each usage-code call
  to a function with its enclosing scope, over the flows' call targets) and the public callables
  (the function and async-function rows of `public_paths`, §9's opening; before 2026-09-24 a
  relation of their own, which the holistic assessment's A1 folded into it).
- **Layers**, each a named policy in `Params` (review F3). *Invocation:* every
  invocation-projection arc between two distinct subsystem functions (calls, property accesses and
  definitions; definite and candidate), 1 per arc. On the pilot, 370 of its 712 pairs come from
  candidate arcs, whose sites nearly always have that one target, and 33 from definitions.
  *Co-use:* every official-usage scope calling two distinct subsystem functions, 1 per scope.
- **Weights.** Hubs: an end whose strength (its summed counts) exceeds the layer's 95th-percentile
  strength scales the count by threshold / strength. Each layer is normalized to unit total, and
  the layers are summed with weight 0.5 each. The vertices are the subsystem functions some pair
  touches; an isolated function is in no community, and is not in RBER's density term.
- **Resolution.** RBER at **γ = 1**, its own density scale, over seeds 0–9 (review F2: choosing
  the γ with the highest mean ARI from a grid depended on the seed block, not the data). The
  profile γ ∈ {0.5, 1, 2, 4} runs too, and its stability is recorded (mean and SD of pairwise ARI,
  mean and min NMI, community count, largest community), never chosen from. γ = 1 is degenerate
  when its seed-0 partition puts more than half the vertices in one community or has no community
  of three: then nothing is reported and the diagnostics say so.
- **Reported.** Each community of the seed-0 partition with at least two public members whose
  **co-assignment** (the mean, over seeds 1–9, of the share of its public-member pairs sharing a
  community) is at least 0.5 (review F5: the score measures the published grouping). A
  `community` finding lists its public APIs (`community_member`, with access path and strength),
  cites up to three supporting sites (`supporting_site`), and has the co-assignment as score.
- **Records.** One `leiden` invocation per run (γ, seed, iterations, convergence, quality
  history) and one `community_consensus` invocation, whose `diagnostics` (a declared migration)
  hold the profile, the choice and what was reported.
- **Tests.** `communities::tests`: LFR planted partitions (n = 250, μ = 0.1 and 0.3) recovered at
  γ = 1 with NMI ≥ 0.9; shuffled and flipped edges give the identical consensus; hub
  down-weighting; a trivially small graph is degenerate; the co-assignment score; the parameters
  against their gold freeze. `the_digest_follows_the_invocation_projection` (cpg-schema).
  `communities_are_stable_and_projected_onto_public_apis` on `analysis_shapes`; the co-use layer
  on `docs_shapes`; the module-order and location tests cover the rows.
- **Pilot (Measured, 2026-09-23, snapshot `48bf9454`).** 516 vertices and 1,298 combined pairs
  (712 invocation, 598 co-use). All runs converged. At γ = 1: mean pairwise ARI 0.749 (SD 0.082),
  mean NMI 0.909, min NMI 0.866; 45 communities, the largest of 101. 29 are reported (2–78 public
  members; co-assignment 0.681–1.0, mean 0.954); 1 falls below 0.5, and 15 have fewer than two
  public members. Four seeds (`FastMCP.tool`, `mount`, `resource`, `prompt`) share the 78-member
  server surface (co-assignment 0.831); `custom_route` (declared on `TransportMixin`) is in no
  reported community. The profile's mean ARI is 0.700, 0.749, 0.793 and 0.739 at γ = 0.5, 1, 2
  and 4.
- **Closed** (the review's F6; the holistic assessment's A1, 2026-09-24): the public-callables
  relation's MRO walk skipped unresolved or outside ancestors and class-level rebindings, where
  §9.1's `member()` refused. Both now read `public_paths`, which refuses as `member()` did. On the
  pilot the move changed no published output (`lctx diff` `5dc48895` → `3c2be6da`); it dropped one
  unpublished `direct_usage` finding, `FastMCP.instructions`' getter, because a property's getter
  and setter share one path and the seed rank names the setter (deviation log D48).

**Extra layers, Implemented and Tested in slice 3.2** as variants, off by default (2026-09-23;
D41):
- **`+type-layer`** (`cpg_schema::communities::shared_types_sql`): two subsystem functions that
  name one release class anywhere in their declared parameter types (a recursive walk of
  `type_term_args`, the receiver aside) are a pair, 1 per class.
- **`+mention-layer`** (`co_mention_sql`; C5 O1): two subsystem functions that one doc passage's
  exact mentions name, 1 per passage.
- **`+knn-layer`**: each subsystem public API with its three nearest other APIs by embedding cosine
  at or above the kNN floor (§9.7's parameters), 1 per direction found.
- **Combination.** With extra layers, every layer weighs 1 / (number of layers)
  (`EXTRA_WEIGHT_RULE`); each layer keeps its own hub down-weighting and normalization. The
  default's two layers keep `Params`' 0.5/0.5. The consensus records the layers, their policies
  and the rule, and its diagnostics give each extra layer's pairs and hub threshold. The layers'
  SQL joins the community relations' digest (`extra_digest`). A supporting site names its layer.
  Since the holistic assessment's A2(c) (2026-09-24), a layer is a `LayerSpec` (type, mention,
  or kNN with its embedding spec hash, `k` and similarity floor) matched exhaustively: the kNN
  layer's lineage joins `extra_digest` and the consensus's parameters (`knn_layer`), so two
  embedding specs never share an invocation id (`the_knn_layer_digest_follows_its_lineage`).
- **Tests:** `variants_add_relational_attributes_and_layers` (type and mention layers on
  `analysis_shapes`; the default's diagnostics unchanged) and
  `mention_and_knn_layers_come_from_the_corpus` (`docs_shapes` with the words embedder; the kNN
  layer without an embedder is refused).
- **Pilot (Measured, 2026-09-23, snapshot `904d86eb`, fake vectors).** The type layer has 4,068
  pairs (hub threshold 126), the mention layer 7 and the kNN layer 0: fake vectors clear no
  floor. 536 vertices; 27 communities are reported, against 29 by default. The 3.3 ablation
  judges each layer with live vectors.

**The consumers, Implemented and Tested in slice 2.6** (2026-09-23; deviation log D34), **revised
by the increment-2 review** (U1; ADR-0011 amendment; deviation log D37):
- **Seed selection** (`lctx_analytics::selection`). Communities and direct usage (§9.5) run
  first, before any per-seed pass. The configured seeds come first. While the brief budget
  allows, the **eligible** public APIs follow by rank: the most direct official-usage calls first
  (PageRank in the `+pagerank` variant), ties to the smaller id. Eligible means official usage
  calls it at least once and its docstring has a summary, the Outcome its brief will state
  (`synth::summary_span`; the review's O1). Communities **cap** rather than choose: an API is
  skipped once its community holds ⌈budget / 3⌉ seeds, configured ones included; an API in no
  community is capped by nothing. Each selected API is named by its preferred path (below); one
  whose path resolves to another declaration is dropped and recorded. A `seed_selection`
  invocation records the rule and its choices (`selection::Params`, frozen with the analytics
  parameters: the review's F7) and names the configured, selected and dropped seeds and the
  eligible count. Each selected seed gets Passes A–C, FCA of its scope and a brief. Tested by
  `select`'s unit test, `selection_needs_official_usage` (`analysis_shapes` has no usage code:
  nothing is eligible) and `selection_takes_what_usage_calls_within_the_budget` (`docs_shapes`,
  budget 4: `pkg.Server.run` is chosen). The pilot's budget equals its five seeds, so nothing is
  selected there until increment 3 raises it (an ADR-0004 amendment). *Superseded:* 2.6's rule,
  rounds over the communities each taking its most PageRank-central documented member, which the
  review showed would fill a larger budget with helpers (F3).
- **The preferred path** (the review's F4). A public callable is shown by one path wherever it is
  named (community members, Related, selected seeds, kNN texts):
  `public_paths`' `preferred` column, read by `cpg_schema::public::preferred_callables`. It is a path through the class that declares
  the method first, so a classmethod is never named through a subclass it would bind
  differently, then the fewest segments, then the least. *Superseded:* the least path, which
  named `Tool.from_function` as `FastMCPProviderTool.from_function` on the pilot.
- **Related** (§10.3): the seed's community co-members, at most five, the most called in official
  usage first (by PageRank in its variant; by name, and saying so, when usage calls none of them),
  citing the community and each listed member's `direct_usage` finding, so its status derives
  `statistically_derived`.
  The policy case holds by construction and by rule: the kind policy permits
  `statistically_derived` only for `related`, and a control citing a community finding or stated
  statistically is rejected (two injected cases in `the_analysis_rules_reject_their_violations`).
- **Pilot (Measured, 2026-09-23, snapshot `8e1e1f19`).** Nothing is selected (budget five, five
  seeds). Four seeds list Related operations from the 78-member server community (co-assignment
  0.83), led by `FastMCP.__init__` and `FastMCPProviderTool.from_function`; `custom_route`, in no
  reported community, has none.
- **Pilot after the increment-2 review (Measured, 2026-09-23, snapshot `a8591982`, generation
  `4a789baa`, fake vectors).** 196 public APIs are eligible; nothing is selected (budget = seeds).
  The four Related lines name the server community by preferred path, the most called first:
  `fastmcp.FastMCP.__init__`, `tool`, `resource`, `call_tool`, `list_tools` (`mount` for two).
  No line names a method through an unrelated subclass. Stage E takes 3.9 s; the compile 35.1 s.


### §9.5 Centrality

- **Consumer.** Which operations get briefs (seed selection, §9.4) and the order of a Related
  line: the question both ask is **which operations official usage calls** (the increment-2
  review's U1; ADR-0011 amendment).
- **Method (the default): direct usage** (`lctx_analytics::ranking::usage_counts`, the review's
  U1). Each call arc (definite or candidate, any phase) from an official-usage caller (a function
  or module of an example, test or doc block) into a subsystem function; each call site counts
  once, split evenly among its targets (`USAGE_POLICY`, digested with the invocation projection).
  A method call on an instance is a `candidate` arc (Pysa's override marking), almost always with
  one target, so a definite-only count would see constructors and module functions only (pilot:
  11,794 of 11,854 candidate usage sites have one target; D37). One `usage_count` invocation;
  each public API usage calls is a `direct_usage` finding (`structurally_observed`: a count of
  observed calls) whose score is its count. It orders, and never states behaviour.
- **Method (the `+pagerank` variant, kept for the §9.8 ablation).** Our own weighted power
  iteration (about 40 lines) over the usage projection in canonical order. The edge weights are
  the usage counts from examples and tests (a named weight policy), with dangling-mass
  redistribution. It records iterations, the final L1 residual and a converged flag (CI-09). petgraph's `page_rank` is rejected (the library-leverage review, D1). The input projection
  and the damping, tolerance, iteration budget and dangling target are pre-registered code (D29),
  frozen by digest in `eval/gold/analytics-freeze.json`. The dangling target is uniform, so
  `leiden_rs::compute_flow` (weighted, directed, uniform teleport) is the reference oracle.
  PageRank ranks what usage reaches **through the library's own delegation**, so implementation
  sinks rank high (the review's F3: on the pilot, its 6th and 8th public APIs have no direct
  usage call, while `FastMCP.call_tool`, 320 sites, is 16th). It replaces direct usage as the
  order only in its variant, and is kept only if the ablation shows it helps.
- **Tests:**
  - a hand-computed 3-node fixture;
  - two parallel arcs counted with their weights;
  - a budget too small to converge, reported as not converged;
  - shuffled rows giving identical scores.
- **Output.** A `statistically_derived` ranking finding.

**Implemented** and **Tested** in slice 2.4 (2026-09-23; deviation log D29); since the increment-2
review, the `+pagerank` variant's method:
- **The usage projection** (`lctx_analytics::ranking`, H1 F9 answered). The invocation projection
  restricted and weighted by a named policy (`WEIGHT_POLICY`, digested with the invocation
  projection into the compiler digest and the invocation's `projection_digest`). Vertices: every
  subsystem function, and every official-usage caller (a function or module of an example, test
  or doc block) with an arc into one. Arcs: each invocation arc into a subsystem function from a
  subsystem function or a usage caller (call and definition arcs, any accepted modality: the
  review's F3(c), now named in the policy), weighted by the arc count per ordered pair. A function
  ranks by the official usage that reaches it, directly or through the library's own delegation.
- **Iteration.** `r'ⱼ = (1−d)/n + d·(Σᵢ rᵢ·wᵢⱼ/Wᵢ + D/n)`, from the uniform start, over the arcs
  in canonical order, where `D` is the dangling mass; damping 0.85, L1 tolerance 1e-10, 100
  iterations (pre-registered code, D29). The `pagerank` invocation records iterations, the final
  L1 residual, `converged` and diagnostics; each public API (`cpg_schema::communities`'
  public callables) is a `centrality` finding whose score is its rank.
- **Tests** (`ranking::tests`): the hand-computed 3-node fixed point; parallel arcs counted with
  their weights; a 2-iteration budget reported as not converged; a permuted projection giving an
  identical usage graph, scores and counts (the review's F6(b); the earlier reversed-rows test
  could not fail); `compute_flow` agreeing to 1e-9 on one hand-made graph and on 30 random
  weighted digraphs of 5–44 vertices, a quarter dangling (the review's probe 2); direct usage
  ranking what usage calls above the sink its delegation reaches, which PageRank ranks first.
  **The usage branch is Tested** (the review's F6(a)): the unit test's usage caller, and
  `selection_takes_what_usage_calls_within_the_budget` on `docs_shapes` (counts `make_server` 12,
  `Server.tool` 9, `run` 2, `stop` 2). On `analysis_shapes`, with no usage code,
  `public_apis_are_ranked_over_the_usage_projection` runs the `+pagerank` variant.
- **Pilot (Measured, 2026-09-23, snapshot `25e8465c`).** 4,444 vertices (607 subsystem
  functions, 3,837 usage callers), 8,080 weighted arcs (total weight 9,346), 273 dangling;
  converged in 38 iterations. 328 public APIs ranked: `FastMCP.tool` 7th, `resource` 13th,
  `mount` 43rd, `prompt` 65th, `custom_route` 169th. The top of the ranking is helpers the whole surface delegates
  to (`fastmcp.decorators.get_fastmcp_meta`, `fastmcp.server.dependencies.get_http_request`),
  which is what PageRank over delegation measures, and why the review moved the default to direct
  usage.
- **Direct usage on the pilot (Measured, 2026-09-23, snapshot `a8591982`).** 7,030.5 calls reach
  257 subsystem functions; 219 public APIs are `direct_usage` findings. The top: `FastMCP.__init__`
  1,185.5, `tool` 674, `resource` 429, `call_tool` 320, `list_tools` 249, `mount` 198,
  `add_transform` 180, `add_provider` 174, `Tool.from_function` 162, `add_middleware` 142: the
  review's direct-usage list (§8 of that review), which it computed independently.


### §9.6 Formal and relational concept analysis

> Decision: ADR-0020

**Off by default since ADR-0020** (the §9.8 keep rule, 2026-09-23): the `+fca` variant, with `+rca`. The record below stands as the variants'.

- **Consumer.** Applicable cases and modes, shared controls, and implication-style assertions
  (e.g. "every writer accepting `filesystem` also accepts `format`").
- **FCA (increment 2).**
  - Our own NextClosure (Ganter, ICFCA 2010) over `fixedbitset`, which also yields the
    Duquenne–Guigues implication basis; FCbO (Outrata & Vychodil 2012) only if the concept count
    exceeds the budget. No usable crate exists: odis is AGPL, fcars enumerates concepts only. So
    `fcars =0.2.2` is a dev-dependency **oracle** for concept sets. No cover relation is computed,
    so none is checked (the increment-2 review's F8; Python `concepts` 0.9.2 would be its oracle).
  - Objects: the public APIs of one **structurally defined scope**: the subsystem, one module, or
    one class hierarchy. Communities are never an FCA scope, because their membership is
    statistical.
  - Attributes: parameter names, parameter and return types, raised exception types, decorators.
  - A support threshold is applied. There is no stability index: it is #P-hard.
- **RCA (increment 3).** Adds one relational-scaling step (∃-scaling over calls and handoffs), a
  DataFusion join that adds attribute columns to the same FCA.
  It is kept only if the ablation shows it changes published output.
  **Implemented and Tested in slice 3.2** as the `+rca` variant (2026-09-23; D41):
  `lctx_analytics::concepts::relational` adds, for each object of a scope, `calls X` for each
  call arc (definite or candidate) into a subsystem function, and `hands off to X` / `takes from
  X` for each handoff the Pass C relation holds. X is the partner's preferred public path, else its
  qualified name. The pairs come from the in-memory projection and Pass C's relation, so no new
  SQL is needed; `RCA_POLICY` joins the FCA invocations' parameters and relation digest. Stage F
  reads each seed's attributes as the context held them (`AnalysisRows.seed_attributes`), and
  the templates say "calls `X`", "has its result passed to `X` in official usage". Tested by
  `relations_become_attributes_of_the_objects_in_scope` and
  `variants_add_relational_attributes_and_layers`. **Pilot (Measured, 2026-09-23, fake vectors,
  snapshot `904d86eb`, `+rca,+type-layer,+mention-layer,+knn-layer`):** the `fastmcp.FastMCP`
  scope grows from 193 to 363 attributes, 95 to 111 concepts and 104 to 131 implications; 16
  relational attributes appear in concepts or implications.
- **Output.**
  - Concepts become `applicable_case` findings (the kind's name is historical: a brief states one
    as a **shared signature** under Related, not as the Applicable case; the increment-2 review's
    U2).
  - Implications with confidence 1 over the support threshold become `implication` findings.
  - Both are `structurally_observed`, because they are exact over the extracted attributes, with
    the attribute scope stated.

**Implemented** and **Tested** in slice 2.5 (2026-09-23; deviation log D32):
- **Scope.** Each seed's structural scope is the public namespace its access path names: the
  public APIs `container.x` of the exported class or module `container` (`fastmcp.FastMCP` for
  `fastmcp.FastMCP.tool`: its public methods, declared or inherited; a package's own module when
  no export names it). One FCA invocation runs per distinct scope with at least two APIs.
- **Attributes** (`cpg_schema::concepts::attributes_sql`, digested): `parameter NAME` (`*`/`**`
  for the catch-alls; the receiver aside by the method's kind), `parameter type T` (declared),
  `returns T` (declared), `raises E` (the typed `raise`s directly in the body; a bare re-raise has
  no type, C4 review O4) and `decorator D`.
- **Kernel** (`lctx_analytics::concepts`): our own NextClosure over `fixedbitset` enumerating every
  set closed under the implications found so far, so one pass gives the frequent concepts and the
  frequent Duquenne–Guigues basis. Pruning infrequent candidates is exact: a larger set never has
  a larger extent. Parameters (pre-registered code, frozen with the others): support 2 APIs,
  budget 20,000 closed sets (a budget stop is `concept_budget`, completion `partial`).
- **Findings.** Each frequent concept with a non-empty intent is an `applicable_case` finding
  (`extent_member` APIs, `intent_attribute`s; score = extent size); each basis implication an
  `implication` finding (`premise`, `conclusion`; score = support). The subject is the scope.
- **Stage F** (as revised by the increment-2 review, F1 and U2; deviation log D38). A seed's
  `shared_signature` assertion (section Related) is the concept of **its own** scope that holds it
  with another API, shares at least two attributes, and has the most (other API, shared
  attribute) pairs, |intent| · (|extent| − 1), then the larger intent: "Like `A`, `B` and `C` (4
  public APIs of `S` in all), `X` declares … and raises `E` directly in its body." Up to three
  `implication` assertions (section Important controls) are the best-supported implications of
  its own scope whose premise the seed meets. Both are `structurally_observed`, and the scope is
  named in the text. Neither enters the brief document's header. The Applicable-case slot stays
  **absent** until an input-or-mode source exists, so the §B11 gap metric sees it. The floor,
  caps and counts are `selection::Params`, frozen (F7). *Superseded:* 2.5 published the concept
  as the Applicable case ("belongs with"), from any scope holding the seed, in the document
  header.
- **Scopes are nodes** (the review's F1). Each seed's access-path container resolves to its class
  or module node; container strings naming one node are one scope, one invocation, labelled by
  the fewest-segment, then least, of them. Each seed records its scope (`AnalysisRows.
  seed_scopes`); Stage F reads concepts and implications of that scope only, so a method inherited
  into two scopes is never described through the other one. Tested by
  `fca_scopes_are_nodes_and_state_only_their_own_apis` (the review's probes 3 and 4: two paths to
  `Catalog` compile as one scope; `Widgets(Catalog)` adds a family, and `Catalog.add_tool`'s lines
  name only `pkg.Catalog` APIs).
- **Attributes from term structure** (the review's F2). A type Pyrefly could not determine is no
  attribute: a term that is, or holds anywhere in its structure (`type_term_args`, a recursive
  walk), an `Any` of style `error` or `implicit` (displayed `Unknown`). A raised class is one
  attribute whether raised as the class (`raise E`, a `ClassObject` term) or an instance; only
  class-typed raises count. The text says what an API does with each attribute's scope
  ("declares …", "raises `E` directly in its body", "is decorated with …"). The rule
  `semantic:concept-attribute-known` is a tripwire over labels. Tested in
  `concepts_come_from_each_seeds_structural_scope` (`Catalog.load`/`reload` return an unresolvable
  type and raise `KeyError` both ways: one `raises KeyError`, no `Unknown`).
- **Tests** (`concepts::tests`): the hand-computed context of §12 (seven concepts; the basis
  `b → a`, `c → a`, `{a,d} → {b,c}`); fcars 0.2.2 agreeing on random contexts at supports 0, 2
  and 4; the basis sound and complete (closing every subset under it gives its Galois closure);
  the budget stop; the frequent basis sound, complete over every frequent set and non-redundant at
  supports 1–4 on 29 random contexts (the review's probe 1, F6(c)).
  `concepts_come_from_each_seeds_structural_scope` on `analysis_shapes` (`pkg.Catalog`: a
  registration family, two loaders and an unrelated method).
- **Pilot (Measured, 2026-09-23, snapshot `57039be8`; before the increment-2 review).** One scope,
  `fastmcp.FastMCP`: 51 public APIs, 195 attributes, 97 frequent concepts and 107 implications from 205 closed sets, far under
  the budget. `tool`, `resource` and `prompt` share an applicable case of eight parameters
  (`auth`, `description`, `icons`, `meta`, `name`, `tags`, `title`, `version`) and six declared
  parameter types; `mount`'s is five APIs sharing a `str | None` parameter and raising
  `ValueError`. Every seed had an applicable case, which the review showed filled the slot by
  construction (U2), and 1–3 implications, 22 of the 107 implications carrying `Unknown` (F2).
  The longest brief document is 1,429 bytes: none is split on the pilot.
- **Pilot after the review (Measured, 2026-09-23, snapshot `a8591982`).** The one scope,
  `fastmcp.FastMCP`: 51 APIs, 193 attributes, 95 concepts and 104 implications from 200 closed
  sets; no attribute is `Unknown`. `tool`, `resource` and `prompt` each carry the shared signature
  of the eight registration parameters and six declared types; `mount`'s ("Like `http_app`,
  `run_http_async`, `__init__` and `resource` … declares a parameter typed `str | None` and raises
  `ValueError` directly in its body") and `custom_route`'s (`name` and a `str` parameter) are the
  coincidental overlaps the review named, now under Related and out of the retrieval header. The
  Applicable-case slot is absent on all five briefs (`absent_slots`: `applicable_case` 5).


### §9.7 Embeddings in analytics

> Decision: ADR-0020

**Off by default since ADR-0020** (the §9.8 keep rule, 2026-09-23): the `+knn` variant. The record below stands as the variant's.

- **Consumers:**
  - linking doc passages to APIs, supplementing explicit mentions;
  - labelling communities by their nearest doc heading;
  - kNN as an optional community layer.
- **Method.** Vectors come from the cache (§11.1). kNN runs in Rust with a fixed `k` and a
  similarity margin.
- **Output.** `statistically_derived` findings.

**Implemented** and **Tested** in slice 3.1 (2026-09-23; deviation log D35):
- **E0** (`cpg-core::embed::embed_texts`), in Stage E after PageRank, with an embedder configured.
  It embeds the corpus passages (`cpg_schema::neighbours::passages_sql`) and the subsystem's
  public APIs (`path(parameters)` and the docstring; `api_texts_sql`, text version 1) as
  documents under the spec. Each text is cut into windows of at most 4,096 bytes at line ends,
  never dropping text. Vectors are read from the cache or embedded and merged; E0's keys join the
  snapshot's key set in `content_digest`.
- **kNN** (`lctx_analytics::neighbours`): exact, the best cosine over window pairs. Each API's
  three nearest passages at or above 0.5 are `doc_link` findings; each reported community's
  centroid takes its nearest passage's heading as a `community_label`. Both are
  `statistically_derived`, and one `knn` invocation records the candidate set (APIs × passages)
  and the counts. The parameters (k 3, floor 0.5, 4,096-byte windows) are pre-registered code,
  frozen with the others.
- **Stage F.** A `doc_link` assertion (section Related) lists the operation's nearest
  documentation with its cosines; the Related line names its community's label. Neither feeds the
  Outcome.
- **kNN as a community layer:** the `+knn-layer` variant (slice 3.2, §9.4). E0 now runs before
  the communities whenever kNN or the kNN layer reads its vectors.
- **Tests.** `neighbours::tests`: windows never drop text; exact search, the floor and node-order
  ties; a community labelled by its centroid's nearest heading.
  `doc_links_come_from_embedding_similarity` runs `docs_shapes` end to end with a bag-of-words
  test embedder: `Server.tool` links the quickstart's "Install" section (0.77), and its brief
  carries the link, `statistically_derived`.
- **Pilot (Measured, 2026-09-23).** Fake vectors (snapshot `0f8b911f`): E0 covers 2,079 windows
  (328 APIs; 1,751 windows of 1,608 passages), 527,424 candidate pairs, and no link clears the
  floor, as expected of unrelated hash vectors; Stage E takes 3.8 s instead of 2.2. **Live
  vectors** (Qwen3-Embedding-8B via `just pilot-live`, snapshot `89d3d4d0`; the service was
  stopped afterwards): 957 links, 323 of 328 APIs linked, cosines 0.50–0.91 (mean 0.70), and all
  29 communities labelled. Each seed's links are its own documentation (`FastMCP.tool`: "tools.mdx §
  The @tool Decorator" 0.85; `mount`: "composition.mdx § Namespacing" 0.84); Stage E takes 46.6 s,
  embedding included. The §1.5 ranking check on that generation is **failed**: `FastMCP.tool`
  is first for 0 of 2 `fm.register` aliases (ranks 4 and 2; 1 of 2 at slice 1.9). 3.3's
  pre-registered evaluation judges it.


### §9.8 Determinism and ablation

- **Determinism oracles:**
  - shuffled input row order gives byte-identical findings, communities and concepts;
  - reruns with the same `content_digest` give identical output.
- **Ablation is mechanical.** Disable one technique and diff the published assertions, briefs
  and boundaries. This is a join on content IDs (§3.4.1).
- **Keep rule (ADR-0020, for briefs; superseded by the criterion below).** Keep a technique only
  if it **changes published output and** improves the pre-registered development metric
  (§12(b)) without lowering §12(a) or (c).
- **Unbiased check.** Using the gold for this choice makes it a development set. The unbiased
  check is the increment-5 held-out evaluation.
- **Agent-based evaluation** is reserved for the raw-vs-compiled comparison (§12). *Superseded
  2026-09-23 by the operator:* there are no API-agent evaluations; see the structured evaluation
  (§12).
- **Under ADR-0021** (the ADR-0020 amendment, 2026-09-24):
  - A technique is judged by its consumer in the served model, through the pre-registered
    behavioral question sets. Brief retrieval (§12(b)) no longer decides.
  - The holistic plan's deletion exit is paused. Communities, FCA and kNN stay as variants.
  - **The criterion** (ADR-0021, the ADR review's F4). A variant is turned on by default only if
    the structured evaluation of the stage that introduces its tool consumer shows three things:
    - it supplies at least one target item, rated present or partial, that the default lacks;
    - it introduces no item rated incorrect or misleading;
    - it does not push a target operation or item out of a tool's first page.
  - **The exit.** A variant with no tool consumer by the end of increment 5, or that fails the
    criterion at two consecutive stages, is deleted by ADR, with its tests and frozen
    parameters.
  - **Named consumers today:** FCA over behavioral attributes (facet suggestions, Stage 4) only.
    **kNN and communities have none** (the re-review R6). The operation vectors `search_operations`
    ranks by are `operation_documents` embedded through the cache, not the kNN technique's output,
    and no tool takes or returns communities.

**The ablation, Measured in slice 3.3** (2026-09-23; ADR-0020; deviation log D42, D43). Every
variant was compiled at budget 20 with live vectors, scored by `scripts/score_gold.py --embedder
vllm` (deterministic: two rescorings were identical) and diffed by `lctx diff`. Hits are counted
over all 44 gold aliases:

| Variant | hit@5 | hit@1 | (a) | touched | (c) | briefs +/−/~ |
|---|---|---|---|---|---|---|
| default then (communities, FCA, kNN) | 14 | 7 | 0.0444 | 8 | 4 | — |
| `-communities` | 13 | 9 | 0.0430 | 7 | 5 | 12/12/7 |
| `-fca` | 14 | 7 | 0.0444 | 8 | 4 | 0/0/14 |
| `-knn` | 14 | 7 | 0.0444 | 8 | 4 | 0/0/20 |
| `+pagerank` | 13 | 8 | 0.0442 | 8 | 4 | 10/10/7 |
| `+rca` | 14 | 7 | 0.0444 | 8 | 4 | 0/0/10 |
| `+type-layer` | 14 | 8 | 0.0509 | 9 | 6 | 3/3/17 |
| `+mention-layer` | 14 | 7 | 0.0444 | 8 | 4 | 0/0/18 |
| `+knn-layer` | 13 | 7 | 0.0444 | 8 | 4 | 1/1/19 |
| `-communities,-fca,-knn` (the kept set) | 13 | 9 | 0.0430 | 7 | 5 | — |

- **Keep decisions.** Communities improve hit@5 by one and lower (c) by one, so they are not kept.
  FCA and kNN change output but leave hit@5 unchanged, so they are not kept. No off-by-default
  variant improves hit@5, so none is adopted. **The default is now Passes A–C, direct usage and
  selection** (ADR-0020); the rest are variants.
- **Every difference is one or two units.** No margin is added after the fact. The increment-5
  held-out evaluation is the unbiased check, and it is ADR-0020's revisit trigger.
- **The §1.5 ranking check** is **failed** on both the old default (`FastMCP.tool` first for 1 of
  2 `fm.register` aliases) and the kept set (0 of 2).

> Decision: ADR-0020

**Variants, Implemented in slices 3.2 and 3.3 groundwork** (2026-09-23; deviation log D41):
- `lctx compile --analytics <variant>`: `default`, or changes to it by name, `+name`/`-name`
  (`cpg_core::analyze::Techniques`). Since ADR-0020, every technique is off by default:
  `communities`, `fca`, `knn`, `pagerank` (the increment-2 review's U1), `rca`, `type-layer`,
  `mention-layer` and `knn-layer`. A community layer needs `communities`, the kNN layer needs
  an embedder, and `rca` needs `fca` (`Techniques::parse` refuses each otherwise).
- **The whole technique set** (its JSON, every flag) joins the compiler run's config digest (the
  holistic assessment's A2(a), 2026-09-24, closing the ADR-0020 review's F3), so no two sets share
  a run id, and a variant snapshot never shares a content digest with another set's. The label
  names the techniques that are on (`none` when none are). The selection invocation records the
  set, so its parameters, and so its id, differ per set; every other invocation records only what
  its technique changed (`rca` in the FCA parameters; `extra_layers` and the weight rule in the
  consensus's). A finding the variant does not touch keeps its id, so an ablation diff is a join
  (`variants_add_relational_attributes_and_layers`). `lctx diff` compares findings, assertions,
  evidence, briefs and documents, not invocations.

> Decision: ADR-0011, ADR-0019, ADR-0005
