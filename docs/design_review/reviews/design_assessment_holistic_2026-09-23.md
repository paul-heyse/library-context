# Holistic design assessment: bespoke code, library leverage, principle alignment (2026-09-23)

**The operator's question.** Three parts:
1. How can the implementation be improved in general?
2. Where should it use built-in library functions instead of bespoke code?
3. How can it align more closely with the principles in
   `design_principles/rust_code_intelligence_data_graph_guidelines.md`, the charter (DM-01–60)
   and `ADDENDUM.md`?

The operator also asked for a survey of libraries not yet used.

**Scope.** Everything built since the two earlier leverage reviews, which covered the CPG era
(`design_review_library-leverage_2026-09-23.md` and `design_review_h1-library-leverage_2026-09-23.md`):
- `lctx-analytics`;
- Stage E and F (`analyze.rs`, `synth.rs`, `usage.rs`, `embed.rs`, `bundle.rs`, `diff.rs`);
- the declared relations in `cpg-schema`;
- the corpus extractor's MDX handling;
- `lctx-embed`;
- `python/lctx_mcp`;
- the evaluation scripts.

The code reviewed is `main` at `12bcc46`, plus the uncommitted slice 3.4 work (documented warnings).

**Method.**
- Five investigators ran in parallel on analytics kernels, Stage E/F, schema relations and
  extraction, serving and evaluation, and a library survey.
- Each used the pinned skills (`rust-graphs`, `datafusion`, `fastmcp`) first, then pinned sources
  under `~/.cargo/registry`, Context7 and registry metadata.
- Nine probes ran in the scratchpad against the repository's `Cargo.lock` and `uv.lock`:
  - DataFusion parameter binding and empty lists;
  - leiden-rs renumbering and the no-move case;
  - markdown-rs MDX over the FastMCP docs;
  - serde_arrow against Arrow 59.3;
  - sqlparser literal rendering;
  - pydantic strict parsing;
  - FastMCP error codes;
  - a degraded-embedder scoring run;
  - `symbol_map` promotion on a served generation.
- The author spot-checked the load-bearing claims in the tree:
  - the compiler digest omits direct usage, the selection choices and the variant policies
    (`attempt.rs:86-140`);
  - `extra_digest`'s silent `_` arm (`communities.rs:153`);
  - `usage.rs`'s `unwrap_or(0)`/`unwrap_or(-1)` defaults (`:281-285`, `:322-323`, `:357-358`, `:382-383`).

**Labels** (charter §D): *Tested (probe)*, *Interface-checked* (read in pinned source or skill),
*Proposed*. This document is evidence, not authority. Each adopted item becomes a commit, or an
ADR where it chooses between alternatives.

**Revision (same day), on the operator's instruction: licence is not a criterion.** The project
is not commercialized, and the best use of existing libraries comes first. Every candidate that
had been excluded or marked down for its licence was re-evaluated on merit alone:
odis (AGPL-3.0), rust-igraph (GPL-2.0-or-later) and raphtory (GPL-3.0). The results are in §3.1,
and they change one answer: **odis becomes FCA's test oracle.** No other verdict rested on
licence.

**Incident.** During the assessment `target/` was deleted, probably when a probe cleaned up its
build directory. No repository file or `build/` artifact was affected, and a `cargo build`
restores it. Probes must keep `CARGO_TARGET_DIR` inside the scratchpad, as the briefs said.

## 1. Verdict

**The architecture matches the guidelines where it matters.**
- The division of responsibility follows guideline §1 and §9: relational preparation in
  DataFusion, then narrow Arrow batches, then kernels, then typed provenance-bearing relations.
- Delta is read at pinned versions (§11).
- The graph kernels are bespoke *for stated reasons* that the survey re-checked and upheld:
  - **PageRank**: petgraph's is unweighted and reports no convergence; graphops' is identical but
    unlocked.
  - **NextClosure with the Duquenne–Guigues basis**: fcars enumerates concepts only. odis
    reproduces our output exactly but cannot prune its basis by support or stop at a
    deterministic budget (§3.1). It becomes the test oracle, not the kernel.
  - **Pass A's BFS**: library walkers run newest-first and have no budgets.
  - **Pass B's worklist**: datafrog keeps neither witnesses nor depth.
  - **Exact kNN**: BLAS or ANN would change ties and semantics.

**The debt is in the glue, not the kernels.** It comes in four recurring patterns:

1. **Identity by string, where a node or code exists.** Examples: gold scoring and exact-symbol
   promotion by path spelling, FCA attributes keyed by Pyrefly display strings, Pass B reasons
   and community layers as free strings, and a variant label that records only the difference
   from the default.
2. **One declaration re-expressed at every consumer** (DM-02, DM-52). Each relation's SQL, schema
   and digest are paired by hand at each call site, and the compiler digest's list of relations
   has drifted. The same goes for:
   - eight copies of finding emission;
   - ten `AnalysisInvocationsRow` literals;
   - eight `EvidenceRow` blocks;
   - seven Arrow decoder families with three different null policies;
   - six IN-list helpers with three quoting policies.
3. **A second parser.** The uncommitted `synth::warnings` string-searches MDX text that the
   extractor has already parsed with markdown-rs.
4. **Digests and records with gaps.** The run identity omits the direct-usage policy, the
   selection choices, the variant policies and the layer SQL. The kNN layer contributes only its
   name. The scorer hides partial degradation to lexical-only retrieval.

**Most fixes need no new package.** markdown-rs's MDX nodes, `serde_json::Value::sort_all_objects`,
proptest, DataFusion parameter binding, Arrow `AsArray` and pydantic are all in the lock already.
One new crate earns a trial: `serde_arrow`, for reading only.

## 2. Findings, ranked by value

Every finding names its oracle (ADDENDUM §4). "(E/F)", "(A)", "(S)", "(V)" and "(L)" mark the
source investigation: Stage E/F, analytics, schema, serving/evaluation, library survey.

### A. Correctness and validity (fix first)

| # | Finding | Evidence | Principles | Fix | Effort | Oracle |
|---|---|---|---|---|---|---|
| **A1** | **Public identity is compared as path spelling instead of the declaration node.** (a) `score_gold.py:37-42,79,91-93` scores by string, so `fastmcp.FastMCP.http_app` misses brief `TransportMixin.http_app`, which is the same node. The ADR-0020 review's F1 showed this changes the keep decisions. (b) `symbol_map` holds only a brief's own export spellings (`bundle.rs:138-140`), so the query `fastmcp.FastMCP.http_app` or `fastmcp.FastMCP.run` is not promoted (probe, generation `438801c4`). (c) `lexical_text` repeats each name token once per spelling and per split (`bundle.rs:283-315`, `retrieval.py:28-30`), so BM25 term frequency tracks the alias count. (V1, V4, V5) | Tested (probe) | DM-11, DM-02, DM-24, DM-40 · G7 | Make `public_callables` (`cpg_schema::communities::public_callables_sql`, own and inherited paths per node) the **one** "public path" relation, used by synthesis (`brief_members` gains inherited paths with an `own` flag), the bundle (`symbol_map`) and the scorer (node-level matching; an operation that resolves to no node is "outside the release"). Emit each distinct lexical token once per brief, with shared tokenizer known answers in `specs/serving/tokens.json`. **Pre-register the matcher and tokenization change as an ADR-0010 amendment before any rescore** (ADDENDUM Q15) | M | test (pytest over the matcher with an inherited member; bundle known answers); `just ranking-check` |
| **A2** | **The digests that define run identity have gaps.** (a) `Techniques::label()` records only differences from the default (`analyze.rs:110-122`), so after the ADR-0020 default change, old-all-on and new-all-off share one run id (ADR-0020 review F3). (b) `compiler_digest` leaves out `ranking::usage_digest`, `selection::Params`, `variant_policies()` and the layer SQL (`attempt.rs:86-140`; spot-checked). (c) `extra_digest` has a silent `_ => &mut h` arm (`cpg-schema/src/communities.rs:153`), so `knn` contributes only its name. The Leiden and consensus invocations of `+knn-layer` record no `spec_hash` or kNN parameters, so two embedding specs share invocation ids (A6, S3). (d) Pass B reasons are `&'static str` values matched in `synth.rs:1542-1546` with a `_` default (A1). | Interface-checked | DM-12, DM-31, DM-48, DM-02, DM-06 · G1, G6 | Digest the **absolute** technique set (`#[derive(Serialize)] Techniques`) and record it in the always-present selection invocation. Add a `Relation { name, sql, schema, deps }` inventory and have `compiler_digest` iterate it, with a test that a used relation is listed. Replace `extra_digest`'s string with an `ExtraLayer` enum (`Knn { spec_hash, k, floor }`) and exhaustive matches. Add `UnfollowedReason` and `CommunityLayer` codebooks. Extend `cpg-core/build.rs` to hash the Stage E/F and analytics sources into `compiler_digest_of`, which moves only run and producer ids, not content ids (E/F7). Keep `TEMPLATE_VERSION` as published lineage. | S–M | test (the technique digest is injective; every relation used is in the inventory); the ledger |
| **A3** | **Stage F's warning helper is a second parser of MDX that markdown-rs already parsed.** `synth::warnings` (uncommitted) uses `str::find("<Warning>")`, which matches inside code fences and inline code, misses `<Warning title=…>`, and reads the 9 pilot warnings nested under `<ParamField>` as warnings about the whole seed. The extractor's `collect` (`docs.rs:291-348`) discards `MdxJsxFlowElement`/`MdxJsxTextElement` nodes. The FastMCP docs hold 703 `VersionBadge`, 580 `ParamField`, 257 `Warning` and 154 `ResponseField` components. (S2, E/F5, L1) | Tested (probe: 93 = 93 warnings on 4.0.3; nesting and attributes on a sample) | DM-02, DM-06, DM-21 · G1, G7 | **Before 3.4 is committed:** add a `doc_components` fact family from the mdast. Columns: document, passage, ordinal, parent ordinal, name, flow/text, literal attributes, span, inner span. Documented warnings then become a SQL selection, scoped to a parameter when nested under `ParamField`, and `ParamField` becomes a second documented Controls source. Drop the string helper | M (a schema addition, a declared migration) | test (fixture: a warning in a code fence, one with a title attribute, one nested under `ParamField`) |
| **A4** | **Ids and names are interpolated into SQL text.** Examples: `hex_list` ×3 with two empty-set policies (`""` against `"NULL"`), `quoted()` without escaping (`analyze.rs:225-232`, used unguarded at `:1440-1442`, `:1462-1464`), and IN lists in `cpg-schema` with a third escaping policy. `IN ()` is a parser error. The relation digests hash the query over an empty list, not the text that runs. (E/F1, S5, L4) | Tested (probe: `array_has($ids, …)` with `with_param_values` plans as a hash-set `IN`; an empty list gives an empty relation) | guideline §2; DM-07, DM-15, DM-41 | Constant SQL with bound `$ids` (`DataFrame::with_param_values`, DataFusion 55.1) through one `sql::query_with` helper. Where an id set is itself a relation, use `read_batch` plus a semi-join. Only string literals that remain in `cpg-schema` need one escaping function | M | test (an empty id set; a name containing `'`); an ast-grep rule against `format!` with `X'` in `cpg-core` |
| **A5** | **FCA attributes use display strings as identity.** In the pilot, `returns Tool` covers 3 distinct classes, `parameter type Request` 2 and `parameter type Tool` 2. FCA can therefore publish a `structurally_observed` shared signature across different classes, against guideline §6 ("matching type names cannot establish interoperability"). `raise E` and `E()` are merged by stripping `type[`, and `Unknown` is caught by a regex over labels. (S1) | Tested (store query, snapshot `1a7244fe`) | DM-06, DM-15, DM-02 · G2; guideline §6 | Key attributes by structure: `(AttributeKind, term node | class node | name)` via `type_terms.node_id` and `type_class_targets`. Add an `AnyStyle` codebook, so that "indeterminate" is one join, and `synth` groups by kind instead of parsing prefixes. **FCA is now a variant (ADR-0020), so this is due only when FCA is revived** | M | test (two same-named classes are two attributes) |
| **A6** | **The scorer hides partial degradation.** `score_gold.py:97` keeps only the last alias's mode, and the label comes from the command-line argument. With every other embedding call failing, 7 of 14 aliases ran lexical-only and the run still reported "live vectors / hybrid". (V2) | Tested (probe) | DM-08, DM-30 · G7 | Record each alias's mode and `degraded_reason`. A `--embedder vllm` run with any degraded alias is `blocked` or lists them | S | pytest (a flaky embedder fixture) |
| **A7** | **Rejection semantics differ across languages, and the served error code is wrong.** Python's `isinstance(x, int)` accepts JSON booleans, which Rust's serde rejects (`embedder.py:117,122`). The resource re-raises `CapabilityError` as `ResourceError`, which reaches the wire as −32603 (internal error) for an unknown id. (V6, V7) | Tested (probe) | DM-53, DM-42, DM-43 | Parse with pydantic 2.13.5 strict models (already locked through fastmcp), with a shared `specs/embedding/responses.json` corpus that both test suites run. Make `CapabilityError` subclass `fastmcp.exceptions.ValidationError` (→ −32602) and delete the three wrappers | S | the shared corpus; a protocol test on the resource's error code |
| **A8** | **Null handling differs between decoders, and some defaults are silent.** `usage.rs:281-285,322-323,357-358,382-383` turns a missing value into `0` or `-1`. `graph.rs` and `communities.rs` decode ids with no null check. `synth.rs` panics through `expect`, and `analyze.rs` returns `Err`. (E/F6, A5) | Interface-checked (spot-checked) | DM-07, DM-08, DM-42 | See B2 (one decoder, nullability as the field type). Until then, an `expect_schema` check at each kernel entry (`RecordBatch::try_new` already rejects nulls in non-nullable fields) | S | test (a null in a non-null column is an error) |

### B. One declaration, generated once (DM-52, DM-17, DM-56)

| # | Finding | Evidence | Fix | Effort | Oracle |
|---|---|---|---|---|---|
| **B1** | **Finding, invocation and evidence rows are built by hand in each module.** Finding emission is copied in eight modules, and each builds `MemberKey`/`StepKey` for the id in one loop and the rows in another, with nothing tying the two together (A2). Each of the ten `AnalysisInvocationsRow` literals repeats serialize → digest → id (E/F4). Each of the eight `EvidenceRow` blocks feeds the same values to `recipe::evidence` and to the row. Seven sites look up `FINDING_STATUS.iter().find()`, each with a runtime error (S7). | Interface-checked | `FindingDraft → (FindingsRow, members, witnesses)`, which derives the id from the rows it emits. `AnalysisInvocationsRow::new(…, &impl Serialize, …)` and `EvidenceRow::new(…)` compute their own ids. Exhaustive `const fn finding_status(kind)`/`finding_method(kind)` replace the lookups. `CoreError::Analytics(#[from] AnalyticsError)` removes 25 `map_err`s. About 650 lines go | M | the existing id snapshot tests and ledger; add "the invocation id recomputes from its row" |
| **B2** | **Row decoding is hand-written and every column type is declared twice.** Each query lists `(name, DataType)`, then picks a typed helper per column. There are seven decoder families. (E/F6, A5, L5, L6) | Tested (serde_arrow probe) / Interface-checked | Step 1: shared `AsArray` (`as_fixed_size_binary_opt`, `as_string_opt`, `as_primitive_opt`; arrow-array 59.3, already used in `bundle.rs`) plus one `Id`-column reader in `cpg-schema`. Step 2: a `query_row!` macro, a sibling of `table!`, that declares a result struct once and generates its schema and `from_batches`, with nullability as the field type. **Or** trial `serde_arrow` 0.15.1 (`arrow-59` feature; adds `serde_arrow`, `marrow`, `bytemuck_derive`; MIT) on Stage F's readers only, and adopt it if the tuple lists disappear. Keep the `table!` write side as it is | S (step 1) / M | tests over null and type mismatches |
| **B3** | **Stage E feeds Stage F through side channels.** `AnalysisRows.seed_scopes` and `.seed_attributes` decide which statements a brief makes, yet no table records them. Stage F rescans members and witnesses once per finding (O(F·M)), round-trips ids through hex lists, and infers "PageRank ran" from the presence of findings, when the selection invocation already records `ranked_by`. (E/F3) | Interface-checked | (a) One `FindingIndex` (members by finding, witnesses by path) and one handoff decoder. (b) Stage F joins `findings`/`finding_members`/`witnesses` in the session, so it can re-run from a published snapshot (DM-26, guideline §9). (c) Publish the seed's scope and attributes as members when FCA is revived | S / M / S–M | the ledger (byte-identical output) |
| **B4** | **Derived projections are prose.** `ProjectionSpec` covers only the invocation projection. The usage projection's `WEIGHT_POLICY` omits its self-arc exclusion (`ranking.rs:225`). The layer policies omit the `(min,max)` symmetrization (`communities.rs:110-117`). `UsageGraph` keeps counts, not the `edge_id`s behind them. (A7, S4) | Interface-checked | **Floor (S):** complete the policy strings and add self-loop fixtures. **Full (M–L):** a small typed `Derived { base, scope, aggregation, self_loops, weight }` whose enums drive the builders and generate the prose. Adopt only if variants keep adding layers (DM-58) | S / M–L | fixtures with self-loops and parallel arcs |
| **B5** | **SQL fragments are duplicated.** The `single` and `valued` CTEs are retyped (`flows.rs:407,482`), the accepted-evidence predicate is written twice, the usage roles are listed three times (two in SQL, one in Rust), and `public_callables_sql` narrows "public" with an unnamed string policy (`__init__`/`__call__`). (S5) | Interface-checked | A `sql` fragment module, `SourceRole::USAGE`, and a named, tested "public callable" policy | S | the snapshot of generated SQL |
| **B6** | **The two long functions** are `synth::run` (about 1,750 lines) and `analyze::run` (about 1,030). (E/F10) | Proposed | One builder per brief section and one function per technique, done together with B1 and B3, not alone | M | the ledger |

### C. The graph layer (guideline §4, §5, §8)

| # | Finding | Evidence | Fix | Effort | Oracle |
|---|---|---|---|---|---|
| **C1** | **The petgraph `Graph` is a vestigial second adjacency.** No petgraph algorithm runs on it. `out_arcs` collects `edges_directed` (newest-first), then allocates and re-sorts on every call. `arc()` has no caller. `NodeFiltered` and SCCs are claimed in DESIGN §5 and §B4 but not built. Pass A rebuilds a `HashMap` of unresolved sites for every seed. (A3) | Interface-checked | Keep the canonical arc order and add `first_out: Vec<u32>` (length n+1), which gives contiguous, already-sorted out-arc ranges with no allocation, plus the same offsets for unresolved sites. Build a petgraph graph only when SCCs get a consumer. petgraph's `Csr` is **not** a substitute: it drops parallel edges and trailing isolates. Amend DESIGN §5 and §B4 | S–M | `the_adapter_keeps_parallel_arcs_isolates_and_refuses_disorder` |
| **C2** | **Leiden's diagnostics are partly re-implemented and partly dropped.** `canonical()` is the identity, since leiden-rs already renumbers (probe: 120/120 runs). `LeidenOutput.quality` is discarded. A run in which nothing moves records `iterations: 0, converged: true` and an empty history. `sizes()` duplicates `Partition::community_sizes()`. (A8) | Tested (probe) | Use the `Partition` methods, or keep `canonical()` as a documented upgrade guard. Record `quality`, set `converged = None` when untracked, and document "iterations = levels that moved" | S | test (the no-move case) |
| **C3** | **kNN lineage and the hub cap.** The metric, window aggregation and centroid rule are outside `neighbours::Params`. `dot` truncates on mismatched dimensions through `zip`. The hub cap at 0.95 is never exercised (the only test uses 0.5), and at n < 20 it caps nothing. (A6, A9) | Interface-checked | Add policy strings for these and freeze them (an ADR-0004 amendment); assert equal dimensions. Add a test at 0.95. Consider `RBConfiguration` only if communities are revived | S | tests |

### D. Observability and cost (guideline §7, §12; DM-39)

| # | Finding | Fix | Effort |
|---|---|---|---|
| **D1** | All of Stage E, including 40 Leiden runs and exact kNN, is one stage mark with a stale label, "analyze (Pass A)" (`attempt.rs:453-457`). No cost below can be attributed until this changes. (A4, E/F8) | Pass `Stages` into `analyze::run`; mark the projection, adapter, each pass, each variant kernel and row conversion | S |
| **D2** | `resolve_seeds`/`member` run about 200 small Delta-backed queries at budget 20 (one per symbol, three times over). The passes redo per-seed preparation (`pass_b` maps, `pass_c` scans). (E/F8, A10) | Resolve each name once, batched through A4's binding. Index once in `Flows::build`/`Handoffs::build`. **Measure first (D1)** | S |
| **D3** | Embedding. `cached_vectors` reads the whole global cache (61 MB, growing with every variant) and then filters. `/tokenize` runs before the cache lookup, so a fully cached `--embedder vllm` compile still needs the GPU. A batch failure discards the batches before it. `reqwest::Client::new()` has no timeout. The request body's key order depends on `preserve_order`, which DataFusion enables. (E/F9, V8, V9) | Filter by key. Count tokens only for missing keys. Merge completed batches before returning an error (the insert-only MERGE is idempotent). Set `ClientBuilder::timeout`. Use a typed `EmbeddingsRequest` struct | S each |
| **D4** | Evaluation keeps only hit counts. A matcher fix therefore needs vLLM again, and the ablation was ad-hoc shell. (V3) | One small evaluation module in three steps. **Retrieve**: rankings, node ids, modes, and query vectors cached by `(spec_hash, text)`. **Judge**: (a), (b), (c) and §1.5 as pure functions. **Ablation**: rounds or bundles, the gold extract's sha, the unit convention, then `just score`/`just ablation` | M |

## 3. Libraries

**Adopt (all already locked; no new package):**

| Library / API | Replaces | Evidence |
|---|---|---|
| markdown-rs 1.0.0 `mdast::MdxJsxFlowElement`/`MdxJsxTextElement` | `synth::warnings`; unlocks components as facts (A3) | Tested (probe) |
| DataFusion 55.1 `DataFrame::with_param_values`, `array_has`, `read_batch` + semi-join | `hex_list`/`quoted` interpolation (A4) | Tested (probe) |
| Arrow 59.3 `AsArray` | seven decoder families (B2 step 1) | Interface-checked |
| `serde_json::Value::sort_all_objects` (1.0.151) | `bundle.rs:374-384` `sorted()`; the one canonicalizer for map-built JSON. **Do not** re-canonicalize struct-ordered digests: their order is frozen and mirrored in Python | Interface-checked |
| proptest 1.11.0 in `lctx-analytics` (dev) | the hand-rolled LCG/xorshift generators in the FCA and PageRank oracle tests; it shrinks failures to a minimal case, and `PROPTEST_RNG_SEED` fixes runs | Interface-checked |
| pydantic 2.13.5 strict models (Python) | `embedder.py:_parse`; declare `pydantic` and `mcp-types` in `python/lctx_mcp/pyproject.toml` (both are imported, but come in only through fastmcp) | Tested (probe) |
| `fastmcp.exceptions.ValidationError` | three error wrappers (A7) | Tested (probe) |
| `FixedSizeBinaryArray::try_from_sparse_iter_with_size`, `from_iter` builders | four builder loops in bundle normalization | Interface-checked |

**Consider:** `serde_arrow` 0.15.1, read side, as a trial on Stage F's readers. It is the first
utility crate that is not already in the lock. It adds three packages, all MIT-compatible, and
must follow the Arrow family (`check_family.py` would catch a stale feature). Also run
`cargo-mutants` occasionally by hand, never as a gate.

**Rejected, with reasons:**
- `arrow_convert` and `narrow`: they couple Arrow upgrades to their release cadence.
- JCS crates (`serde_jcs`, olpc): they differ from Python's `json.dumps(sort_keys=True)` on floats
  and on non-BMP key order, and they do not address the real hazard, maps built with `json!`.
- minijinja, askama, tera: they move branching sentence logic into a second language, and a version
  is still needed.
- simsimd, faer, ndarray: a different summation order can flip top-k and floor ties. There is no
  measured need; 527k pairs take under a second.
- hnsw and instant-distance: approximate search would silently replace the declared exact kNN.
- graphops PageRank: identical to ours, unlocked, and a second authority for a variant-only
  45-line kernel.
- rustworkx-core: no algorithm gap. Its BFS runs newest-first with no budgets. ADR-0011's claim
  that it "collapses parallel arcs" is inaccurate, but the walker-order objection stands.
- fcars as the kernel: it enumerates concepts only. For odis as the kernel, see §3.1: it is
  rejected on merit and adopted as the oracle.
- datafrog and ascent for Pass B: no witnesses and no depth. A recursive CTE enumerates paths
  (guideline §7).
- ruff_python_semantic: it does not remove `lexical.rs`.
- roaring: it would be a second bitset type.
- pulldown-cmark: no MDX; it would be a second markdown authority.
- bolero: it duplicates proptest.
- DataFusion `Expr` builders for the derivations: SQL text is declarative, reviewable and hashable.
- Python rank-bm25, orjson, polars, duckdb: stale, not needed, or a second SQL engine.

**Available but not worth it now:** `uv-pep440`, `hex` and `regex` (all transitively locked).
Each saves only lines, not decisions.

### 3.1 The licence-blind re-evaluation

**odis 2026.9.1 (AGPL-3.0-only)**
- **What it provides:** NextClosure, FCbO, Titanic iceberg enumeration by minimum support, the
  Duquenne–Guigues canonical basis (plain and "optimised"), lattices and attribute exploration,
  over a bitset-backed `FormalContext`.
- **Its dependencies:** `bit-set` 0.8, plus `rust-sugiyama` 0.3 (a lattice-drawing crate), which
  is not optional and brings **petgraph 0.6.4** into the lock beside our 0.8.3.
- **Correctness probe** (scratchpad `odis-probe`, 2026-09-23; our kernel copied verbatim from
  `lctx-analytics/src/concepts.rs:1-190`). Across 39 random contexts (8–12 objects × 6–9
  attributes) and supports 0–4, 195 cases in all, three odis results each matched ours exactly,
  with **0 mismatches**:
  - `Titanic` iceberg concepts;
  - `NextClosure` concepts filtered by support;
  - the full canonical basis filtered to premises with support ≥ s.

  The last one confirms that our pruned enumeration gives exactly the frequent part of the
  Duquenne–Guigues basis.
- **Timing probe.** Synthetic contexts shaped like the pilot, with skewed attribute frequencies;
  they are denser than the real pilot scope, which enumerated 205 closed sets.

  | Context | Ours: concepts + frequent basis | odis `Titanic`: concepts only | odis full basis (optimised / plain) |
  |---|---|---|---|
  | 51 × 193 | 184 ms (595 concepts, 479 implications) | **2.6 ms** (595) | 1.9 s / 10.8 s (4,899 implications, 479 frequent) |
  | 51 × 363 | 11.0 s (1,501 concepts, 1,596 implications) | **15 ms** (1,501) | 24 s / 114 s (9,156 implications, 1,596 frequent) |

  `Titanic`'s output order was identical across runs.
- **Verdict as the production kernel: reject, on merit.**
  - *No support-pruned basis.* It computes every pseudo-intent, infrequent ones included, so the
    basis costs 10–60× more here and grows without bound.
  - *No deterministic budget.* `SearchBudget` is a millisecond limit that only the drawing
    algorithms take, while guideline §7 and §8 require a stated, reproducible truncation.
  - *A second petgraph.*
  - odis is still much faster at concepts. If FCA is revived (it is a variant since ADR-0020) and
    its time matters, first replace our naive `implication_closure` with LinClosure (analytics
    finding F12). Our cost sits in the basis walk, which Titanic cannot replace.
- **Verdict as the test oracle: adopt** (a dev-dependency, replacing or joining `fcars`).
  - It checks the basis directly. fcars covers concepts only, and our basis checks are
    property-based (soundness, completeness, non-redundancy).
  - Its lattice gives the cover relation that DESIGN §9.6 once assigned to Python `concepts`.
  - Adopting it needs `deny.toml` to allow `AGPL-3.0-only`, since cargo-deny checks
    dev-dependencies. It also needs a `docs/pins.md` row, and amendments to ADR-0011's option
    text and DESIGN §9.6, which both give "odis is AGPL" as the reason.
  - Oracle: `fcars_agrees_on_the_concepts` gains an odis twin, plus
    `odis_agrees_on_the_frequent_basis`.

**rust-igraph 0.7.0 (GPL-2.0-or-later): reject, on merit.** It has no gap to fill.
- `pagerank_weighted` (`algorithms/properties/pagerank_weighted.rs:52`) fixes the damping and the
  iteration cap as constants and returns only the scores: no iterations, residual or convergence
  flag. That fails guideline §8, which ours meets.
- Its Leiden runs with fixed defaults (β = 0.01, two iterations) and offers none of leiden-rs's
  RBER or RBConfiguration objectives or quality history.
- `compare_communities` duplicates leiden-rs's `try_nmi`/`try_ari`.
- It computes on its own `Graph`, which means a full copy.
- It is the fallback toolbox if a future analytic needs flows, cuts, isomorphism or layout.

**raphtory 0.17.0 (GPL-3.0): reject, on merit.**
- No design question is temporal. Revision comparison is relational (`lctx diff`), which is the
  guidelines' §11 default.
- It pins Arrow ^56 and DataFusion ^50, a second Arrow/DataFusion universe beside 59.3/55.1.
- It does not compile as published without a `rand` fix (rust-graphs probe C001).

**Policy.** When a crate with a copyleft licence is adopted, widen `deny.toml`'s `allow` list and
add the pins row; don't reject the crate. The standing instruction is recorded in project memory.

## 4. Correctly bespoke (keep)

- **Our weighted PageRank.** It has L1 diagnostics, and `compute_flow` stays as its oracle.
- **NextClosure with the Duquenne–Guigues basis.** Support pruning and a budget come in the same
  pass.
- **Pass A's frontier BFS and Pass B's bounded worklist.** Guideline §1 explicitly allows them.
- **Exact kNN, text windows, integer layer counts over a (min,max) normal form.** The layer counts
  are shuffle-invariant, and their weights cannot be malformed.
- **RRF, promotion, the tokenizer and the discriminating-word filter.** These are ADR-0010's
  decisions; stemming would be tuning on the gold.
- **`id.rs`** (length-prefixed BLAKE3 recipes mirrored by the `lctx_id` UDF), the **serving digest
  grammar**, and the **own HTTP clients** (vLLM's `/tokenize` is outside the OpenAI API).
- **SQL-text relations with text digests.** They over-invalidate on cosmetic edits but are sound.
- **Templates as Rust code, and `TEMPLATE_VERSION` as published lineage.** The version stays;
  A2 only makes the compiler digest automatic.
- **`lctx diff`.** Two pinned sessions and an in-memory set difference (ADR-0017), which is right.
  Two small fixes: key documents by content, not by brief id, and print codebook names.

## 5. Principle alignment beyond the code

- **The keep rule's development metric cannot see brief content.** §12(b) scores retrieval
  (hit@5), and §12(a) and (c) score membership and spans. FCA and kNN statements enter neither
  the retrieval document nor selection, so under this rule they could never be kept (ADR-0020
  review). Selection techniques win or lose on one-unit differences.
  - Before any rerun, re-register the procedure (ADR-0020 review F2, F5):
    - rounds with dependent techniques as bundles;
    - all metrics over all gold units, with node identity (A1);
    - an explicit statement that content techniques are decided only by the increment-5 agent
      evaluation, which must then run a with/without arm, or else be removed for good.
  - Deciding this is the operator's call: it changes what the pre-registration means (DM-59,
    ADDENDUM Q15).
- **Guideline §12's acceptance record** (for each analytic: the question, projection, crate and
  licence, adapter cost, numerics, termination, output schema, cache key, complexity, measured
  stage metrics). DESIGN §9 covers most of it. It lacks adapter and representation cost, cache
  and invalidation, complexity and per-stage metrics (D1). Add them as a column in each
  §9.x *Implemented* block rather than as a new document type (ADR-0001).
- **DESIGN.md as "current truth".** It holds superseded text in place, for example the 2.6
  selection rule, the pre-ADR-0020 default and the §5 `NodeFiltered`/SCC claims. The ADR-0020
  review found four stale sections. This is not a length problem, and no detail should be
  trimmed for length. The fix is to state current behaviour first, with the history sitting in
  ADR amendments and reviews that DESIGN links to, so a reader never applies a superseded rule.
- **Process weight** (ADR-0001: "light"). There are 43 deviation entries, 24 review files, 20 ADRs
  and three hand-bumped output versions. The reviews have been paying for themselves: each of the
  last three found decision-changing defects. The hand bumps have not paid for themselves. A2's
  automatic source digest removes one kind of bump, and the ledger stays as the alarm.

## 6. Recommended sequence

1. **Before finishing slice 3.4:** A3 (MDX component facts replace `synth::warnings`). A2(a) is
   the absolute technique digest, a one-hour fix for a live identity collision.
2. **Identity and digests:**
   - A1, with its ADR-0010 amendment registered before any rescore;
   - the rest of A2 (the relation inventory, the layer enum and codebooks, the build-script
     source digest);
   - A6, A7, A8.

   Then re-register the keep-rule procedure (§5) and rerun the ablation, from recorded rankings
   (D4) so that later matcher fixes need no GPU.
3. **Glue consolidation:**
   - A4 (bound parameters);
   - B2 step 1 (`AsArray`, with a trial of `serde_arrow`);
   - B1 (the row builders);
   - B3(a)(b), B5 and B6, together.

   The ledger proves the output is byte-identical at each step.
4. **FCA, when it is next touched:** adopt odis as its dev oracle (§3.1), with the `deny.toml`
   allowance and the ADR-0011 and DESIGN §9.6 amendments. Use LinClosure if its time matters.
5. **The graph layer and observability:**
   - C1 (offsets instead of the vestigial graph, with the DESIGN amendment);
   - C2;
   - D1 (per-stage timing), then D2 and D3 only where D1 shows cost;
   - C3 and A5 when their techniques are revived.

Every step keeps `just test-all` and `just pilot` green, and each one names the oracle it adds.
