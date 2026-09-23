# Library leverage review: the codebase after the CPG (2026-09-23)

**Question (operator).** Before slice 4, where can the codebase lean further on DataFusion,
delta-rs, Pyrefly/Ruff and petgraph, and on libraries beyond them? The aim is to align with the
design principles and `rust_code_intelligence_data_graph_guidelines.md`, and to minimize bespoke
code.

**Scope.** Everything on `main` at `2473e6a`: the four crates (about 15.7k lines), `scripts/` and
`justfile`.

**Method.**
- Six parallel reviews, one each for DataFusion/Arrow, delta-rs, Pyrefly/Ruff, petgraph and the
  graph ecosystem, the general Rust ecosystem, and an allocator spike.
- Each used the pinned skill as its primary reference, and Context7, crates.io, docs.rs, GitHub
  and web search for the rest.
- Behaviour was checked by probes: scratch cargo projects on the pinned versions and the repo's
  `Cargo.lock`, run against copies of the pilot store. Probe sources are in the session
  scratchpad; this document records their results and is the durable record.
- The author spot-checked the key claims in the tree: the symlink-following walk
  (`library.rs:554`), `--reinstall` ignored by `compile` (`main.rs:232`), the text-match static
  test (`lexical.rs:153-165`), the `target_partitions = 1` claim with no code
  (`DESIGN.md:1277`), and `TableProviderBuilder::with_adds`/`with_file_paths` at delta-rs
  `58f07cd`.

**Labels** follow the charter's §D:
- *Tested (probe)*: exercised by a probe outside the repo;
- *Interface-checked*: read in the pinned source or docs;
- *Proposed*: not yet checked;
- *Measured*: timed on the pilot.

This is evidence, not authority: each adopted item becomes a commit, and an ADR where it chooses
between alternatives.

## Verdict

The libraries are already used well where it matters most:
- the rules are generated SQL, and the one validator is shared;
- the Arrow kernels (`lexsort_to_indices`, `take`, `RecordBatch::try_new`) are the standard ones;
- Ruff's `SourceOrderVisitor`/`AnyNodeRef` drive the walk;
- Pyrefly's display, class metadata and type traces feed `types.rs`;
- the Delta retention and property verification are right for this revision;
- the `table!`/`codebook!` macros beat every schema-derive crate examined.

Neither Ruff's `SemanticModel` nor Pyrefly's binding layer can replace `lexical.rs` without losing
the conservative candidate sets (§ Rejected).

The review found three groups of change:
- **Nine correctness defects in bespoke code.** Code already in our dependency graph fixes each
  one. None needs a new crate.
- **Six runtime changes measured on pilot data**, three of them large:
  - jemalloc halves the peak (7.1 → 3.64 GB);
  - cached, concurrent validation takes 10 s → about 1 s;
  - reading each snapshot's files from its own commit closes the deferred file-skipping item with
    no migration.
- **Five design corrections for the next slices**, the chief being that petgraph's `page_rank`
  cannot serve §9.5.

Every recommended change stays inside the pinned dependency family. The one real new-crate
candidate, gix, is rejected.

## 1. Correctness: bespoke code with defects (recommended; no new crates)

| # | Current code | Replace with | What it fixes | Output change | Confidence |
|---|---|---|---|---|---|
| C1 | `lexical.rs:153-165` `static_test` (text match), `:447-458` (only the `if` test is checked; every `elif`/`else` inherits it with polarity false) | Pyrefly's `SysInfo::evaluate_bool`, `is_type_checking_guard` and `is_not_type_checking_guard` (`pyrefly_python/src/sys_info.rs:794, 849, 870`), per clause, as Pyrefly's own binding pass does (`binding/stmt.rs:1276`), through `Handle::sys_info()` | **Five of 10 probe tests disagree with Pyrefly.** Missed: `not TYPE_CHECKING`, `os.name == "nt"`, and an `elif sys.version_info < (3, 9)` after an ordinary `if`. False positive: `cfg.version_info_cache`. Mislabelled: `elif sys.platform …` marked `VersionInfo`. Pass B's §4.2.4 binding rule reads exactly these marks, and §3.2 says the marks are Pyrefly's decisions | `bindings.static_branch`/`static_polarity` values change where such tests occur; no schema or id change. No fork change. About 12 lines become about 15 | Tested (probe) |
| C2 | `library.rs:511-577` (`glob_match` and a recursive `select`); `config.rs:71-90, 300-322` (two more walks) | `globset` 0.4.20 (`literal_separator(true)`) and `walkdir` 2.5.0 (`follow_links(false)`, `sort_by_file_name()`), both already compiled at one version | Several defects in the hand-written walk and matcher:<br>• **Symlinks are followed** (`p.is_dir()`): the probe read a file outside the tree through a symlinked `docs/linked`. A `docs/loop -> ..` produced 81 paths, 40 levels deep. The pilot tree has 10 symlinks, all in dot-directories so far.<br>• `?`, `[ab]` and `{a,b}` are read as literals, so a brace in `documents_exclude` silently excludes nothing.<br>• Matching backtracks exponentially: 10 stars take 3.3 s, 14 stars over 115 s.<br>• There are 1+N walks where one would do. | Same files, same order on the pilot tree (148, 125 and 427 files). About 110 lines become about 45 | Tested (probe) |
| C3 | `library.rs:98-139, 142-176, 461-509` (`toml::Table` get-chains; a hand check for unknown keys covering only `[tool.lctx.source]`) | `serde` derive + `toml::from_str` + `#[serde(deny_unknown_fields)]` (the `toml` serde feature is already on) | **A misspelled `[tool.lctx.sourse]` silently disables the corpus** (`library.rs:130-133`); unknown keys elsewhere pass silently | None: 108 identical `uv.lock` entries in the probe. About 125 lines become about 60 | Tested (probe) |
| C4 | `main.rs:28-79, 417-447`; `lctx-extract.rs:16-69` (hand-written argument parsing) | `clap` 4.6.7 derive with subcommands (already compiled; avoid `#[arg(env)]`) | **`lctx compile --reinstall` is parsed and silently ignored** (`compile` calls `acquire(..., false)`, `main.rs:232`). Every flag is accepted by every command. Both hex parsers accept `+f+f…` (a `from_str_radix` sign quirk), and `lctx-extract` panics on non-ASCII input | CLI only. About 110 lines become about 50; one `Id::from_hex` value parser replaces both | Interface-checked; hex quirks Tested (probe) |
| C5 | `library.rs:199-215` (`RECORD` lines split on `,`) | `csv` 1.4.0 (already compiled), or refuse a line starting with `"` | uv and Python quote a path that contains `,` or `"`. Such a file is then silently neither verified nor a release module. There are none in the pilot's 103 `RECORD` files | None on the pilot | Tested (probe: the quoting) |
| C6 | `main.rs:185` `git init -q` | `git init -q --template=` | The system template directory is the last ambient git input; a `post-checkout` hook there would run on checkout (C5 review F4's class) | None | Tested (probe) |
| C7 | `config.rs:352` `to_value(cfg).unwrap_or(Value::Null)` | Return the error | A config that fails to serialize would hash `"null"` into `context_id` | None | Interface-checked |
| C8 | `hash.rs:11-75` (fact-id payload encoding), `table.rs:41` (canonical sort) | Known-answer tests per column type; a two-row sort case | cargo-mutants on id/hash/table (cpg-schema's own tests): **all 12 `hash_into → ()` mutants survive**, and `num_rows() < 2` → `<= 2`/`== 2` survive. Other crates' snapshots may catch some; not run | Test code only | Tested (probe) |
| C9 | `lib.rs:540, 1081`, `walk.rs:558` (`ends_with("__init__.py")`); `lib.rs:469` (a local "private name" test); `walk.rs:193-201` (docstring range) | `ModulePath::is_init()` (`module_path.rs:194`); `is_public_name` (already exposed by the patch); `Docstring::range_from_stmts` (`docstring.rs:44`) | Pyrefly is the one authority for "package", "public" (§4.2.3) and "docstring". The suffix test would accept `x__init__.py` | `is_package` changes only for such names; the docstring ranges agree on 7 of 7 | Tested (probe) / Interface-checked |

## 2. Runtime: measured changes (recommended)

| # | Current code | Change | Measured | Cost / risk | Confidence |
|---|---|---|---|---|---|
| P1 | glibc malloc | `#[global_allocator] static GLOBAL: tikv_jemallocator::Jemalloc` in `lctx` (and `lctx-extract`). `tikv-jemallocator` 0.7.0 is already in the lock **and already compiled** as Pyrefly's Linux dependency; Pyrefly's own CLI and Ruff's use it | Pilot, 16 runs, one content digest across every allocator (`9e33e57b…`). **Peak: glibc 6.1–7.3 GB → jemalloc 3.64 GB, within 7 MiB across 4 runs, flat from extraction on.** Wall time: 44.8–48.9 s → 40.3–42.7 s, with the lowest user CPU (56 vs 64–68 s). The time gain is suggestive on a shared host; the peak is solid. mimalloc 0.1.52 is a close second (39.6–44.3 s, 3.76–4.13 GB), but compiles new C code and its peak varies more | Adds nothing new to build, no license change (MIT/Apache; C source BSD-2); `cargo deny` and `check_family.py` passed. It is a dependency decision (pin-check, a `docs/pins.md` row, an ADR). DESIGN §4.3 needs amending: `MALLOC_ARENA_MAX` no longer applies, the "raised the peak most" line will read +0, and jemalloc tuning is `_RJEM_MALLOC_CONF`. Unconditional on Linux; guard it with `cfg` if Windows ever matters | Measured |
| P2 | `validate.rs:37-62` runs 496 rules sequentially, and every rule re-decodes its Delta views (`snapshot.rs:49-64`) | (a) Read the hot tables, or all 57, once from the **pinned Delta views** into `MemTable` (`collect_partitioned`); never from the batches before the write. (b) Run the rules through a bounded `buffer_unordered(8)` on the same session, with violations put back in rule order. (c) Set `target_partitions` to 4–8. Still one query per rule, the same SQL, the same shared validator (§B3) | Validation **12–18 s (probe) / 10 s (C6) → 0.57–0.68 s** with everything cached, plus a 0.45–0.76 s cache build. Concurrency alone gives 3.0–3.5 s. Working set +0.6–0.8 GB (under an arena-limited allocator); the default-allocator peak did not rise, because per-rule decoding is what inflated the arenas | About 50 lines. Per-rule `VmHWM` attribution stops meaning anything; use P5's plan metrics. DESIGN's reopen trigger ("when validation outgrows extraction's wall time") has not fired, so taking it now is the operator's call. Re-measure alongside P1 | Tested (probe) |
| P3 | `snapshot.rs:47-62` (`register`), `delta.rs:204-212` (`read_at`): a `snapshot_id` filter over every file of each table | `LogStore::read_commit_entry(v)` + `logstore::get_actions` → `TableProviderBuilder::with_adds`/`with_file_paths` (`delta_datafusion/table_provider.rs:356-367`). Only the files the snapshot's recorded commit added are read. The provider fails by default if a selected file is not active. Keep the `snapshot_id` filter as the row predicate | Exact per-snapshot pruning, and unpublished attempts' files are never opened. The probe matched all 57 published pilot tables against `snapshots.row_count` (including the empty `graph_gaps`); a 200-snapshot table answered in 4.7 ms against 15.7 ms (3.3×). **Closes the deferred "file skipping on `snapshot_id`" item (§4.3) and ADR-0009's revisit trigger with no schema or store change** | About 25 lines. It relies on two things already true: one commit per table per attempt, and JSON commits kept (log cleanup off, verified at open) | Tested (probe) |
| P4 | `table.rs:40-58` sorts on `[snapshot_id, key…]` | Skip the leading `snapshot_id`, which is constant in every write batch (assert that); optionally an arrow-row `RowConverter` for multi-column keys | Deriving `edges`: the sort goes **0.47–0.64 s → 0.16 s**, with identical order (asserted). It was the largest part of that derivation | About 6 lines; no ordering change, no digest bump | Tested (probe) |
| P5 | `validate.rs:38-49` measures each rule by the change in `VmHWM` | `ExecutionPlan::metrics()` per rule (`elapsed_compute`, `build_mem_used`, `bytes_scanned`, pruning, spills) | Per-rule cost that holds under concurrency; `VmHWM` is lazy and process-wide | About 20 lines. It complements `metrics.rs` (Delta writes, casts, sorts and RSS stay there) | Tested (probe) |
| P6 | `delta.rs:180` writes SNAPPY (the default) | `WriterProperties::builder().set_compression(ZSTD(3))` through `with_writer_properties` | Store **270 → 233 MiB (−13.7%)**; `source_files` −36.5%; scan times unchanged | About 5 lines. Raw writes about +0.4 s, estimated from per-table timings. Content unchanged; old files stay readable | Tested (probe) |
| P7 | `udf.rs:32-136` | `return_field_from_args` with `nullable = false`; reject a non-literal kind at plan time (`ReturnFieldArgs::scalar_arguments`); dispatch per column outside the row loop | It **enforces** §3.4.1's "the kind is a text literal", and lets the optimizer fold `IS NULL`. 0 of 1,449,162 ids differ; 0.455 → 0.407 s per 1.45M rows | About 30 lines; `udf::VERSION` unchanged (same encoding) | Tested (probe) |

**Projected pilot** after P1–P4 (Proposed; to be measured): about 35 s and about 3.6 GB, down from
44.6 s and 7.1 GB. Extraction (about 26–31 s, two Pyrefly checks per run) then dominates.

## 3. Observability and hygiene (recommended; small)

| # | Change | Why | Confidence |
|---|---|---|---|
| O1 | A `tracing-subscriber` fmt layer with `EnvFilter` at `warn`, to stderr, in `lctx` (0.3.23, already compiled) | Pyrefly's library code has 42 `warn!`/`error!` sites, delta-kernel emits `tracing` events, and DataFusion's `log` records arrive through tracing-log. **All are dropped today.** `datafusion-tracing` would plug into the same subscriber later | Interface-checked |
| O2 | `cargo shear` in `just deps` | `[workspace.dependencies]` entries no crate uses pin nothing. `parquet` and `object_store` are listed as pins in `docs/pins.md:14-15, 35-36`, but only `Cargo.lock` holds them. `futures` is unused. In the probe, an unused `itertools = "=0.14.0"` resolved to 0.15.0 with no warning | Tested (probe) |
| O3 | `fs-err` 3.3.1 (`use fs_err as fs`) where IO errors are surfaced (about 20 calls) | `ExtractError::Io` prints "No such file…" without the path | Proposed |
| O4 | `anyhow` in `lctx` (optional; already compiled) | Removes 26 `map_err(|e| e.to_string())` | Proposed |
| O5 | Correct `deny.toml:5-7` and ADR-0002 on why `check_family.py` exists | cargo-deny 0.20.2 *does* catch dev-only duplicates (`multiple-versions-include-dev`). The script stays, because a regex covers the 78 family crates and any sub-crate an upgrade adds | Tested (probe) |

## 4. Design corrections before slice 4 and §9 (recommended)

| # | Finding | Recommendation | Confidence |
|---|---|---|---|
| D1 | **petgraph `page_rank` cannot serve §9.5.** It takes no weights. It counts parallel edges in the out-degree but credits the target once (`page_rank.rs:92`): two parallel 0→1 arcs gave [0.542, 0.229, 0.229] against [0.486, 0.326, 0.188] for textbook PageRank that counts each arc. It differs by 0.0053 even on a simple graph. It is O(iterations·V·E): 254 ms against 125 µs for a sparse power iteration at V=2000, E=6000. It reports no convergence: `parallel_page_rank` returns the last iterate with no flag | About 40 lines of our own weighted power iteration in canonical order, recording iterations, residual and a converged flag (guidelines §8). **Amend ADR-0011** (still `proposed`) and §B4/§9.5. graphops' `pagerank` does report convergence, but only through its own trait; its `petgraph` feature pulls petgraph 0.6.5, which fails `just deps` | Tested (probe) |
| D2 | **leiden-rs 0.8.1: keep, but feed it directly.** `from_petgraph` needs `E: Into<f64>`, so our graph would need a second copy. The undirected builder does not normalize orientation, and on a planted-partition (LFR) graph at μ=0.5, shuffled and flipped edges with the same seed changed the partition. With a (min,max)-plus-sort normal form, every perturbation gave identical output. Across 10 seeds, pairwise agreement (NMI) was at least 0.9785 | `default-features = false` (the defaults bring cli, rayon, gryf). Build `GraphDataBuilder` from the dense index. DataFusion aggregates each (min,max) pair under a named weight policy, then sorts. Always set the seed, since `None` uses OS entropy. Enable `track_quality_history`, because there is no converged flag. Keep the planned seed consensus | Tested (probe) |
| D3 | `petgraph::algo::condensation` merges parallel edges through `update_edge` (`algo/mod.rs:477-512`): three call-site arcs became one edge carrying the last call site | When §13 un-defers condensation, build the condensation edges from `tarjan_scc` membership and keep every arc's evidence (guidelines §6) | Tested (probe) |
| D4 | The slice-4 adapter | `Graph::with_capacity`. The sorted `Vec<[u8;16]>` of domain ids is the dense index (read `FixedSizeBinaryArray` directly; no hex strings). Add nodes in order, so `NodeIndex(i) == i`, and edges in canonical arc order, so `EdgeIndex(k)` is arc row k and the edge weight is the row index. Scope with `EdgeFiltered`/`NodeFiltered` and traverse callers with `Reversed`; these keep the original edge ids. Write our own BFS with parent pointers, then `tarjan_scc` with members sorted. Add a test that shuffled arc rows give identical adjacency order. **Not** `Csr`/`GraphMap`, which drop parallel edges, or `StableGraph`, which is unneeded and blocks 9 algorithms. petgraph's `rayon`/`serde-1` features are not needed | Tested (probe) / Interface-checked |
| D5 | FCA/RCA: no usable Rust concept-analysis crate. odis is AGPL; fcars (MIT, PCbO) only enumerates concepts; dci is abandoned | Keep our own FCA: NextClosure (Ganter, ICFCA 2010), which also gives the Duquenne–Guigues basis; FCbO (Outrata & Vychodil 2012) if the concept count blows the budget. Use `fixedbitset` (already locked). Use `fcars` 0.2.2 as a **dev-dependency oracle**, with Python `concepts` 0.9.2 for the cover relation. RCA's ∃-scaling is a DataFusion join feeding the same FCA | Interface-checked; fcars Tested (probe) |
| D6 | `types.rs:110-122, 806-808, 861-886`: the `Defs` re-walk plus inverting Pyrefly's `Coroutine[Any, Any, X]` wrapping for an annotated `async def` | Read the annotation itself: `KeyAnnotation::ReturnAnnotation` (`binding/binding.rs:1662`) through a borrow-only fork accessor like `get_type_at`, or by making `Answers::get_idx` `pub`. That is a 1–4 line patch within ADR-0012 (30 → about 34 lines), and it removes about 35 lines. The same accessor gives declared variable annotations later | Interface-checked |

DESIGN §5's claim that Bfs and Dfs visit siblings in opposite orders is **confirmed**:
- `Graph` lists the newest edge first (`graph_impl/mod.rs:919, 982`);
- `Bfs` visits siblings in that order (`traversal.rs:294-306`);
- `Dfs` pushes all successors and pops the last, so it takes the oldest first (`:108-121`);
- probe: Bfs [0,3,2,1,4] vs Dfs [0,1,4,2,3].

Cite these lines in §5 instead of the unlocated claim.

## 5. Stale claims to correct in DESIGN

- §4.3 line 1277: "`target_partitions = 1` for float aggregates". No code sets it, and there are
  no float aggregates yet. Replace it with P2(c)'s setting, or drop it.
- §4.3's binary-statistics citation has moved to `writer/stats.rs:212-229` at `58f07cd`. The
  conclusion stands, and P3 makes it moot for published reads.
- §5's Bfs/Dfs order: cite the source (above).
- §4.3's memory triggers read "the peak under `MALLOC_ARENA_MAX=2`". Under P1 the plain peak is the
  working set; restate them.

## 6. Keep as is (confirmed)

- **Rules and derivations as generated SQL text.** They are what `compiler_digest` hashes and
  what the snapshots test. Typed `Expr` plans would still name columns as strings, and DataFusion
  shares no execution between separate queries. Planning is 0.9–1.2 s of 12 s. Plan *inspection*
  through `LogicalPlan::apply_with_subqueries` is worth a small test, e.g. that every table is
  read by at least one rule.
- **The `table!`/`codebook!` macros and `ArrowColumn`:**
  - serde_arrow 0.15.1 would replace only the builder loops, and gates every Arrow bump;
  - typed-arrow has no Arrow 59 feature;
  - arrow_convert drags in chrono, glam and more;
  - narrow is immature;
  - strum and num_enum would split the one-line append-only codebook form.
- **Delta:**
  - log cleanup off plus long retention, set as table properties and compared as exact strings at
    open (the typed accessors fall back to defaults silently);
  - the constraint-normalization replica: delta-rs's `Expression::resolve` is crate-private. If
    the replica ever drifts, normalize by running `add_constraint` on `DeltaTable::new_in_memory()`;
  - eager snapshot loading: `without_files()` makes each scan replay the log;
  - no optimize or vacuum: every published read is pinned at its own version;
  - the "classify an ambiguous append by re-reading `snapshots`" protocol. A repeated
    app-transaction marker does not suppress a sequential append.
- **Pyrefly/Ruff:**
  - the exhaustive `NodeKind` match in `syntax.rs`, so a new variant fails the build;
  - the structural-path syntax ids;
  - the `types.rs` term builder: Pyrefly's visitor loses child roles, ordinals and parameter
    names;
  - the independent `__all__` detector (§4.2.3);
  - the textual `is_overload` beside Pyrefly's flag (two assertions, §B6);
  - `absolute_module`, which already calls Pyrefly.
- **Stage A utilities:**
  - `IdHasher`: its encoding is a contract; arrow-row bytes are not stable across releases;
  - `sha2`/`base64`, the encoding `RECORD` dictates;
  - PEP 503 normalization;
  - the environment-scrubbed `uv` call (uv has no library API);
  - `sort_all_objects()`, needed because serde_json's `preserve_order` is on in the graph;
  - the `VmHWM` read: memory-stats reports current RSS, not peak.
- **Repo scripts:** `check_family.py` (see O5), `adr.py`, `check_agents.py`, `check_gold.py`,
  `check_pyrefly_fork.py`. They are repo-specific.

## 7. Deferred, with triggers

| Item | Why not now | Trigger |
|---|---|---|
| Pyrefly's Glean collector (`report::glean::convert::glean`, reachable today) | A new family (§13 cross-references). Probe: 42 xref targets, including the typed attribute xref `self.helper()` → `pkg.mod.C.helper`. One target per use and flow-sensitive, so it complements `reference_resolutions`, never replaces it. Cost on FastMCP not measured | A consumer needs attribute cross-references (Pass C handoffs, §10 "see also") |
| Streaming derive (`WriteBuilder::with_input_plan`) | Costs now quantified: unpartitioned writes funnel into one writer; the write path casts leniently (an Int32 plan was accepted into Int64), so the exact-schema guard must move to the plan; row counts come from commitInfo `operationMetrics`; the canonical sort stays a blocking `SortExec`. Saves at most about 0.5 GB | §4.3's trigger (derivation dominates the working set) |
| `FairSpillPool` + spill directories | **Not a memory bound:** `HashJoinInput` cannot spill, and `edges` fails at ≤512 MiB. Useful only as a generous guard (≥1 GiB) that turns an OOM kill into `ResourcesExhausted` | A library whose working set approaches the host |
| `datafusion-tracing` 55.0.0 | Compatible with DataFusion 55, but brings the tracing/opentelemetry 0.31 family, a new pin family | Per-operator spans are needed (O1's subscriber is the socket) |
| rustworkx-core 0.18.1 | Apache-2.0, petgraph 0.8, adds 7 crates, nothing re-versioned. Has **no pagerank or community detection**. `connected_components` returns a randomly seeded `HashSet`; betweenness goes parallel unless `parallel_threshold = usize::MAX` | A named consumer of eigenvector/Katz centrality, articulation points, core numbers or a keyed topological sort |
| datafrog 2.0.1 | Suits monotone fixed points (the probe's closure was correct); last release 2019 | Pass B needs a Datalog-style fixed point |
| App-transaction marker `lctx.publish/<hex>` on the `snapshots` append | It closes only a concurrent-duplicate window that one operator doesn't have (ADR-0009 option 1) | Concurrent compiles |
| Pyrefly `parse_parameter_documentation` (Sphinx/Google docstring parameters) | Duplicates nothing we have | §10 needs parameter text |
| CinderX `collect_module_types` (1-line visibility change) | §13 trigger | Located, narrowed types are needed |

## 8. Rejected

- **Ruff `SemanticModel` port.**
  - `ruff_python_semantic` 0.0.11 is published and co-resolves: it adds 4 crates, all at 0.0.11.
  - But the model is filled only by ruff_linter's crate-private `Checker`, so a port would be
    larger than `lexical.rs`.
  - It still keeps one binding per name, so our candidate sets would stay ours.
- **Pyrefly `Bindings`/`find_definition`/`Definitions` for name resolution.** They prune
  statically decided branches and are flow-sensitive. Probe: 1 of 3 binding events, and
  `DefinitionNotFound` for names bound only in pruned branches.
- **A fork flag to stop Pyrefly pruning.** A logic change: pruning also drives type solving.
- **CinderX structured types instead of `types.rs`.** Lossy: classes keyed by name string,
  variables by name, parameters unnamed. That undoes C4 review F5.
- **`ruff_python_stdlib` lists.** A second authority beside Pyrefly's `builtins` exports.
- **A typed-plan rewrite of the SQL; recursive CTEs** (`RecursiveQueryExec` has no recursion
  limit; guidelines §7); **DataFusion `Constraints`** (informational only); **`INSERT INTO`/
  `write_parquet`** (they bypass the Delta CHECKs).
- **Partitioning Delta tables by `snapshot_id`.**
  - As Binary it silently returns wrong results: 0 of 1,000 rows came back.
  - As a hex column, it is a migration of every contract.
  - P3 gives the same pruning at no cost.
- **Other Delta file-skipping routes:**
  - an Int64 snapshot sequence (ADR-0009 rejected a monotonic allocator);
  - `dataSkippingStatsColumns`: the writer drops stats for all binary columns;
  - bloom filters: +5–6% size for point lookups Delta doesn't serve;
  - liquid clustering: not an accepted writer feature at the pin.
- **Change data feed.** Every attempt writes a full snapshot, so CDF reports every row as an
  insert. A relational diff by content id is the right tool (guidelines §11).
- **gix 0.87.1.**
  - It worked: a byte-identical checkout of the pinned commit in 1.7 s.
  - Against it: it adds 66 crates, breaks its API roughly monthly, and saves only about 40 lines.
  - Its `file://` transport still spawns `git-upload-pack`.
  - Reconsider if a second git operation lands.
- **`ignore`.** It applies gitignore rules and global excludes by default; **wax** isn't in the
  graph.
- **Graph crates:**
  - `Csr`/`GraphMap`/`StableGraph` for the call projection (D4);
  - graphops with its `petgraph` feature (petgraph 0.6.5);
  - graphina (alpha);
  - rust-igraph, raphtory and odis (GPL-2.0+, GPL-3.0 and AGPL, outside `deny.toml`);
  - other Leiden crates (licenses, a hardcoded seed, no users);
  - pathfinding (duplicates petgraph);
  - GraphAr (the Apache Rust crate is a C++ FFI binding, and guidelines §11 calls graph
    serializations disposable);
  - no Arrow → petgraph adapter crate fits.
- **criterion/divan; miette, tracing-chrome, tracing-timing, metrics, sysinfo.** Nothing would
  consume their output; the pilot is the measurement.

## 9. Decisions for the operator

1. **Allocator (P1).** jemalloc is recommended: already compiled, and upstream's choice. It is a
   dependency decision, so it needs an ADR and a pins row.
2. **Symlinks in a fetched tree (C2).** Recommended: don't follow them. Refuse a selected path
   that is a symlink, naming it (fail closed). The alternative is to skip it silently.
3. **Glob syntax (C2).** Recommended: accept globset's full syntax with `literal_separator(true)`
   and document it in §4.0. The alternative is to refuse metacharacters beyond `*` and `**`.
4. **Validation speed-up (P2) now or at its trigger.** Recommended: now. It is cheap and
   measured, and it removes 10 s from every compile and test.
5. **zstd (P6).** Recommended: yes.
6. **PageRank (D1).** Recommended: our own weighted power iteration, amending ADR-0011 before it
   is accepted.

## 10. Proposed sequencing: one hardening slice (H1) before slice 4

1. **H1a, correctness** (small commits; no new crates): C1 (extractor output changes for static
   branches, a compact review per the cadence), C2 + C3 (Stage A), C4, C5–C7, C8 (tests), C9.
2. **H1b, runtime:**
   - P1 (an ADR, pins, §4.3 amended);
   - P3;
   - P4;
   - P2 + P5 (validation), P6, P7;
   - then O1–O5.

   Re-measure the pilot once, at the end, under the new allocator.
3. **H1c, design:** amend ADR-0011 (D1, D2, D5; D3 and D4 as slice-4 guidance), D6 (a fork
   accessor under ADR-0012), and the §5 corrections.

Then slice 4.

## Decisions (operator, 2026-09-23)

Every item above is adopted. On the six decisions of §9: jemalloc is the allocator; a selected
symlink is refused, naming it; globset's full syntax is accepted and documented; validation is
sped up now; Delta files are written with zstd; PageRank is our own weighted power iteration. They
are carried out as the hardening slice H1 (plan: `~/.claude/plans/h1-library-leverage-hardening.md`),
before slice 4. D1, D2, D3 and D5 are recorded now (ADR-0011 amendment, DESIGN) and built by the
slice that consumes them; D4 is slice 4's adapter recipe.
