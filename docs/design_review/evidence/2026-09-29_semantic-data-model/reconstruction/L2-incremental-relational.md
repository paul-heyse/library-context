# L2 — Incremental computation, recursive relational closure, persistence/hydration: library capability research

Date: 2026-09-29. Scope: library capability evidence for the design review's decisions (a) artifact
dependency-graph ownership, (b) recursive relational closure, (c) persistence/hydration and a
hydrated-object cache. No repository file was edited and nothing was built in the product workspace.

## Evidence conventions

Registry root below is `R=~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f`; delta-rs is
`D=~/.cargo/git/checkouts/delta-rs-dcb716bfdc369320/58f07cd` (the pinned rev 58f07cd6…).
Evidence kinds:

- **source**: read in the exact-version crate source (path:line).
- **runtime**: an executed probe with a control (skill probe receipts or repository evidence receipts).
- **compile**: a compile probe (skill).
- **repo**: repository code/docs as of the working tree today (not a verification of behaviour).
- **upstream-doc**: crate docs/book, or Context7 excerpts of upstream docs (not version-verified unless stated).
- **inference**: my reasoning from the above; treat as Proposed.

Recommendations are design claims with the principles label **Proposed**.

Pins checked against `docs/pins.md` (2026-09-29) and `Cargo.lock`: datafusion =55.1.0; arrow-* =59.3.0
(arrow-ipc with `lz4`,`zstd` via `crates/lctx-workspace-hack/Cargo.toml:30`); deltalake git 58f07cd
(deltalake-core 1.0.0); sqlx =0.9.0; pgpq =0.12.0; salsa/salsa-macros/salsa-macro-rules =0.28.2
(product features: `default-features = false, ["compact_str","macros","salsa_unstable","inventory","ordermap"]`,
`Cargo.toml:77`, so no `persistence`, no `accumulator`, no `rayon`); moka 0.12.16 (transitive);
PostgreSQL 18.6. Not in `Cargo.lock`: ascent, datafrog, differential-dataflow, timely, quick_cache.

---

## 1. salsa 0.28.2

### 1.1 What the repository has decided and proven

- **ADR-0081 option 3** (`docs/adr/0081-bounded-agent-journeys-and-coarse-rebuilds.md` §Options):
  "Introduce generic incremental computation or a recipe service. Rejected for this scope: neither
  has a consumer that the existing canonical facts, pure derivations and artifacts cannot satisfy.
  A finite stage plan is easier to invalidate and test locally." Reopen trigger (§Consequences):
  "a demonstrated workload requiring finer reuse". (repo)
- **§14.10** (`docs/design/sections/api-and-evidence-product.md:534–594`): "Separate coarse rebuild
  inputs without creating another incremental framework"; stage cache is **Implemented and bounded
  Tested (2026-09-29)**; "Cache admission re-derives its pure output and compares canonical batches …
  This is reuse of admitted stage artifacts … not a measured reduction in pure-stage validation cost."
  "Salsa persistence, if later admitted under §14.11, is a disposable version-qualified cache;
  incompatible cache state is discarded, while canonical/serving validation remains mandatory." (repo)
- **§14.11** (same file, 596–685), conditional table row **Salsa**: consumer only when "Repeated
  fine-grained catalog derivation demonstrably exceeds coarse reuse. Keep a separate catalog
  database and pinned 0.28.2 provider family; no ty handles, ambient SQL/files/embedding effects or
  accumulator-only evidence … Persistence is exercised only by the isolated PR2 probe; no production
  persistence is enabled." Admission compares cold vs reused outputs after doc/signature/member
  insert-delete/missing-evidence/coordinate/policy changes, incl. cache reload, measuring work
  counts/time/peak+retained memory/cache size. Forward plan §7 row (`docs/plans/behavioral-model-forward-plan_2026-09-24.md:1130`)
  repeats this and adds "No automatic pin upgrade, daemon or canonical cache". (repo)
- **Current stage cache** (`crates/cpg-core/src/stage_cache.rs`, `rebuild.rs`): key =
  SHA-256 over IPC-stream bytes of every declared input table after `to_declared` +
  `canonical_sort` + `rebind_snapshot` (stage_cache.rs:14–54), plus parameters and code digest;
  artifacts are Arrow IPC **File** per table under `rebuild-cache/<stage>/` with a JSON
  `CURRENT.json` envelope (format 1, key, per-file SHA-256; ≤32 files, ≤512 MiB total) written by
  temp+rename (stage_cache.rs:55–127). Admission **always recomputes** (`derive_contracts`,
  `PreparedEvidence::derive`, `catalog_domains::derive`) and compares canonical batches
  (stage_cache.rs ~290–340: `is_some_and(|c| output(c, …) == expected_output)`), so reuse saves no
  pure-stage compute today. `rebuild.rs:20–51` declares a closed `Stage` enum (Facts,
  ContractNormalization, ContextualAssociation, BehavioralEnrichment, CatalogFinalization,
  Retrieval) and receipts with `dependencies: Vec<Stage>` hand-written per step (rebuild.rs:184–232,
  395–396). (repo)

### 1.2 PR2 salsa evidence (`docs/design_review/evidence/2026-09-28_pr2/`)

- `salsa_catalog_probe.rs` (Cargo example over a published pinned fact snapshot): one
  `#[salsa::input] struct Inputs { #[no_eq] pinned: Pinned, roots: Vec<String>, snapshot: String }`
  where `Pinned` holds `Arc<PreparedCatalog>` and `Arc<Vec<PublicPathsRow>>` behind
  `#[derive(salsa::SalsaValue)]` with `#[salsa_value(unsafe(prove_safe_to_retain_manually))]`;
  two tracked fns `catalog_bytes(db, input) -> Vec<u8>` (Debug encoding of derived `Contracts`) and
  `output_digest(db, input) -> String`; events captured via `salsa::Storage::new(Some(Box::new(..)))`.
  (repo, source of probe)
- Receipt `raw/salsa-roots.json` (**runtime, Tested and Measured 2026-09-28**, per README): eight root
  cases (initial, unchanged, redundant setter, narrow, expand, restored, absent) all `equal: true`
  between clean and salsa outputs; prepare 31 µs; unchanged read 0–1 µs; changed-root execution
  170–1861 µs; redundant setter re-ran `catalog_bytes` and `output_digest` was only
  `DidValidateMemoizedValue` (backdating). README: "not an end-to-end speed comparison or a scaling
  result"; it varies **roots only** — it does not exercise fact-level changes, per-function keys,
  cycles, or Arrow inputs.
- `persistence-probe/` (isolated workspace; `salsa =0.28.2` with
  `["macros","inventory","salsa_unstable","persistence"]`, macro family held at 0.28.2): a
  `#[salsa::input(persist, singleton)]` plus one `#[salsa::tracked(persist)]` and one transient
  query; serialise with `serde_json::to_string(&<dyn salsa::Database>::as_serialize(&mut db))`,
  wrap in an owner envelope `{"compatibility": "salsa-0.28.2/probe-format-1/source-A", "database": …}`,
  restore with `<dyn salsa::Database>::deserialize(&mut db, &mut serde_json::Deserializer::from_str(..))`.
  Receipt `raw/salsa-persistence.json` (**runtime**): 478 bytes; 0 persisted-query executions after
  restore; 1 transient execution; changed pin or source refused by the envelope before salsa
  deserialisation. README: "Reordering ingredient JSON through a generic map caused a restore panic";
  "a floating macro helper patch did not compile against that family". (repo, runtime)

### 1.3 Exact API at 0.28.2 (and differences from 0.28.4)

**Tracked-fn cycle options** (source: `R/salsa-macros-0.28.2/src/options.rs:66–79,382–427`,
`R/salsa-macros-0.28.2/src/tracked_fn.rs:268–305`, `R/salsa-macro-rules-0.28.2/src/setup_tracked_fn.rs:337–365`):

```rust
#[salsa::tracked(cycle_fn = recover, cycle_initial = initial)]      // Fixpoint
fn f(db: &dyn Db, key: K) -> V { … }
fn initial(db: &dyn Db, id: salsa::Id, key: K) -> V { … }
fn recover(db: &dyn Db, cycle: &salsa::Cycle, last_provisional: &V, value: V, key: K) -> V { … }

#[salsa::tracked(cycle_initial = initial)]                           // Fixpoint, recover = identity
#[salsa::tracked(cycle_result = fallback)]                           // FallbackImmediate, no iteration
fn fallback(db: &dyn Db, id: salsa::Id, key: K) -> V { … }
```

- Mapping (tracked_fn.rs:271–304): `(cycle_fn, cycle_initial)` → `Fixpoint`; `cycle_initial` alone →
  `Fixpoint` with `unexpected_cycle_recovery!` which returns the new value unchanged
  (`R/salsa-macro-rules-0.28.2/src/unexpected_cycle_recovery.rs:5–11`); `cycle_result` →
  `FallbackImmediate`; `cycle_fn` without `cycle_initial` is a compile error; `cycle_result` with
  either is a compile error; `no_eq` with `cycle_fn` is refused (tracked_fn.rs:104–107). (source)
- `salsa::Cycle<'_>` exposes `head_ids()`, `id() -> Id`, `iteration() -> u32`
  (`R/salsa-0.28.2/src/cycle.rs:477–500`). `MAX_ITERATIONS: u8 = 200`, then panic
  "too many cycle iterations" (cycle.rs:55–57; `function/execute.rs:433,751,869`). Convergence =
  value returned by `cycle_fn` equals previous provisional value (cycle.rs:30–32), so V needs `Eq`.
  Nested cycles iterate inside the outer cycle; inner heads transfer lock ownership to the outer
  (cycle.rs:40–44). (source)
- Upstream example `R/salsa-0.28.2/tests/cycle.rs:126–190` uses exactly the signature above. (source)
- **0.28.2 vs 0.28.4**: `src/cycle.rs` is byte-identical (empty `diff`); `options.rs` option set is
  identical; the macro differences are `fn_util::retain_body_attr` (0.28.3 "preserve public
  attributes on tracked queries") and let-chains; `salsa-macro-rules` changes the
  `intern_id(zalsa, zalsa_local, key, |_, data| data)` call to `intern_id(zalsa, zalsa_local, key)`
  (setup_tracked_fn.rs:105,476) — i.e. a runtime-API break between macro-rules and runtime, which is
  why the three must move together. CHANGELOG 0.28.3: "adopt newer Rust language and library
  features", "Remove assemble support from interneds", MSRV → 1.88; 0.28.4: "restore builds with
  optional interning features" (`R/salsa-0.28.4/CHANGELOG.md:10–28`). The salsa skill's cycle brief
  (`.claude/skills/salsa/content/capabilities/salsa.cycles.md`, indexes 0.28.4) matches 0.28.2
  semantics for cycles — **concepts transfer; no cycle-API difference found**. (source; skill =
  upstream-doc + runtime at 0.28.4)

**Accumulators in cycles**: skill probe B029 (0.28.4) "Accumulating inside a fixpoint cycle panics"
(runtime at 0.28.4). At 0.28.2 in this product the `accumulator` feature is **not enabled**
(`Cargo.toml:77`), so accumulators are unavailable anyway; evidence must be returned in values
(consistent with §14.11 "no … accumulator-only evidence"). Accumulated values also do not survive
persistence (skill B026, runtime at 0.28.4). (source + skill)

**Durability** (source `R/salsa-0.28.2/src/durability.rs:84–101`): `Durability::LOW`, `MEDIUM`,
`HIGH`, `NEVER_CHANGE` ("Setting an input field to this durability permanently freezes that field.
Any later attempt to change its value or durability will panic"). Set with
`input.set_field(&mut db).with_durability(Durability::HIGH).to(v)` or builder
`field_durability(d)`. A later plain `.to(v)` keeps the earlier durability (skill B005, runtime
0.28.4). Durability changes verification cost, never the answer (skill salsa.durability brief).

**LRU / memory** (source `R/salsa-0.28.2/src/function.rs:306`, `function/eviction/lru.rs:46`,
`database.rs:45`): `#[salsa::tracked(lru = N)]`; generated `f::set_lru_capacity(db, n)` compiles
only with `lru`; `Database::trigger_lru_eviction(&mut self)`; `lru` excludes `specify`.
Interned GC by slot reuse after `revisions` newer revisions (default 3) (skill B019–B021, runtime
0.28.4). `heap_size = f` is accounting only; `<dyn Database>::memory_usage()` is behind
`salsa_unstable` (database.rs:400–414, enabled in product features).

**Persistence** (source `R/salsa-0.28.2/Cargo.toml [features]`: `persistence = ["dep:serde",
"dep:erased-serde","salsa-macros/persistence","thin-vec/serde"]`; `database.rs:170–195`
`as_serialize(&mut self) -> impl Serialize` / `deserialize(&mut self, D)`): **present at 0.28.2**,
exercised by upstream tests `tests/persistence.rs`, `persistence_never_change.rs`,
`tracked-struct-entries-persistence.rs`. The serialised map is keyed by numeric
`ingredient_index().as_u32()`, sorted by jar kind (database.rs:214–228) — so the format depends on
ingredient registration order, i.e. on the compiled query layout. There is no format-versioning in
salsa itself: compatibility must be an external envelope (as PR2 did). Constraints (skill
salsa.persistence brief, runtime at 0.28.4, source-consistent at 0.28.2): every input a persisted
query reads must be `persist` (else `as_serialize` panics, B027); a persisted fn returning a tracked
struct or keyed on an interned struct needs `persist` on that struct (E0277); restored queries re-run
once an interned dependency is GC'd (B036). **Status**: feature exists, no stability guarantee in the
source (no docs claim one; inference); in this product it is **off** and turning it on changes the
single, feature-unified salsa build that `cpg-flow`/ruff_db 0.0.14 compile against — untested
(inference; the PR2 probe deliberately used an isolated workspace).

**Cancellation** (source `R/salsa-0.28.2/src/database.rs:76–78`, `lib.rs:317`): `db.cancellation_token()
-> CancellationToken`, `.cancel()`; `salsa::Cancelled::catch(|| …)`; a setter on one handle cancels
queries on clones (they unwind at the next fetch). Skill evidence at 0.28.4 (runtime): B024 queries
with `cycle_initial`/`cycle_fn` **ignore** local cancellation while running; B037 a waiter
recomputes rather than being cancelled; B040 an idle `cancel()` cancels the next call; B039 tokens are
per handle. Upstream parallel tests at 0.28.2 include `tests/parallel/cancellation_in_fixpoint.rs`,
`cancellation_token_cycle_nested.rs` (source).

**Determinism of parallel execution**: salsa does not parallelise inside a query; parallelism is
by cloned handles on threads. Two threads requesting the same key: second blocks (`WillBlockOn`)
(skill B013). For acyclic pure queries results are input-determined (inference). For cycles, the
**cycle head is whichever query is re-entered first** (cycle.rs:9–22), so which query iterates
depends on entry order/thread interleaving; the final value is order-independent only if
`cycle_fn` is a monotone lattice step converging to a unique fixpoint and does not use
`cycle.iteration()` or fallbacks (inference from cycle.rs docs; upstream has regression tests
`tests/cycle_dependency_order_different_entry_queries.rs`,
`tests/parallel/cycle_iteration_mismatch.rs`). A `cycle_fn` that clamps on iteration count (as in
upstream `tests/cycle.rs:126–145`) is entry-order sensitive. The repo's own ruff_db parallelism
knobs are output-neutral for the ty index (ty-flow TY15), which says nothing about a catalog DB.

**Pin trap** (docs/pins.md:83; ty-flow skill SKILL.md and probes TY05/TY06, **compile**, verified
2026-09-24): salsa 0.28.4 fails in `ruff_python_ast` (E0407 `HashEqLike`); `salsa = "=0.28.2"`
alone floats `salsa-macro-rules` on a fresh lock (to 0.28.5) and `ruff_db` fails; hold all three with
`cargo update -p <crate> --precise 0.28.2`. Because Cargo unifies one salsa version per lock, **any
catalog salsa DB in this workspace is frozen at 0.28.2 until ruff/ty move** (inference from pins +
single-version lock).

### 1.4 What a per-function/per-SCC tracked fn over Arrow-derived inputs would look like (0.28.2)

Salsa is synchronous and must not read ambient state (skill B003: a plain DB field read is stale;
config must be an input). So DataFusion/Delta reads happen *before* salsa, and Arrow-derived data
enters as inputs. `'static` values need no `SalsaValue` impl (`R/salsa-0.28.2/src/salsa_value.rs:10–30`:
"Ordinary `'static` values are accepted directly"), so `Arc<RecordBatch>` or domain row structs work;
backdating needs `PartialEq` (RecordBatch has it) unless `no_eq`. Sketch (inference, Proposed; not
compiled here):

```rust
#[salsa::input]                       // one per published fact snapshot / scope
struct FactSet { #[returns(ref)] functions: Arc<BTreeMap<Id, FnFacts>>, policy: PolicyDigest }

#[salsa::interned]                    // stable key per Python function (canonical Id, not arena ids)
struct FnKey { id: [u8; 16] }

#[salsa::tracked(cycle_fn = recover, cycle_initial = bottom)]   // mutual recursion ⇒ fixpoint
fn summary(db: &dyn Db, facts: FactSet, f: FnKey) -> Arc<Summary> {
    let me = &facts.functions(db)[&Id(f.id(db))];
    let callees = me.callees.iter().map(|c| summary(db, facts, FnKey::new(db, c.0)));
    Arc::new(Summary::join(me, callees))            // evidence carried in the value
}
fn bottom(_: &dyn Db, _: salsa::Id, _: FactSet, _: FnKey) -> Arc<Summary> { Arc::new(Summary::BOTTOM) }
fn recover(_: &dyn Db, _: &salsa::Cycle, _last: &Arc<Summary>, v: Arc<Summary>, _: FactSet, _: FnKey)
  -> Arc<Summary> { v }                              // monotone lattice; never iteration-based
```

Caveats: (i) a single `functions` map input means **any** fact change invalidates every reader
(coarse); fine-grained reuse needs one input per function (e.g. `#[salsa::input] struct FnFacts`) and a
diff-applier that calls setters — that diff step is new code the repo does not have (inference).
(ii) Per-SCC keys would require computing SCCs outside salsa (petgraph) and interning an `SccKey`;
recomputing SCCs on edit changes keys and discards memos (skill B018: identity = untracked fields).
(iii) Iteration within a cycle recomputes every member each round; there is no semi-naive delta
(inference from cycle.rs). (iv) Parallel handles + cycles: see determinism above.

---

## 2. DataFusion 55.1 recursive CTE, plan construction, MemTable

**Config**: `datafusion.execution.enable_recursive_ctes: bool, default = true`
(`R/datafusion-common-55.1.0/src/config.rs:988–989`; also datafusion skill
`content/catalogs/config-options.md:44`). When false, planning returns `not_impl_err!("Recursive CTEs
are not enabled")` (`R/datafusion-sql-55.1.0/src/cte.rs:88–95`). (source)

**Planning** (`R/datafusion-sql-55.1.0/src/cte.rs:81–205`): body must be `UNION [ALL]`; otherwise planned
as non-recursive; static term planned first; the self-reference becomes a `CteWorkTable` scan whose
schema is the static term's with **every field made nullable** (cte.rs:150–160, `nullable_schema`);
if the recursive term does not reference the CTE it falls back to an ordinary UNION;
`distinct = !is_union_all` → `LogicalPlanBuilder::to_recursive_query(name, recursive_plan, distinct)`.
Recursive and static terms must have equal column counts; recursive term is type-coerced to the
static schema (`R/datafusion-expr-55.1.0/src/logical_plan/builder.rs:176–201`). (source)

**Execution** (`R/datafusion-physical-plan-55.1.0/src/recursive_query.rs`):
- `UNION` (distinct): a `DistinctDeduplicator` (hash `GroupValues`) persists across the static term
  and **all** iterations; each batch is filtered to rows never seen before (lines 336–352, 458–500).
  So `UNION` is semi-naive: only new rows enter the next work table, matching PostgreSQL semantics
  ("discard … rows that duplicate any previous result row", PG18 docs via Context7). Dedup is over
  all output columns, so carrying a `depth` column defeats dedup/termination on cycles.
- `UNION ALL`: no dedup; each iteration's full output feeds the next.
- Termination: stops when an iteration yields 0 rows (359–370). "There won't be any limit or checks
  applied to detect an infinite recursion, so it is up to the planner" (52–66). **No max-iteration
  setting exists**; bounds must be in SQL (`WHERE depth < N`) as the repo does
  (`crates/cpg-schema/src/behavior.rs:2046–2060`, `crates/cpg-core/tests/syntax.rs:393`).
- The recursive term always executes **partition 0 only** (single-threaded per iteration, 380–384)
  and is re-planned each iteration via `reset_plan_states` (`execution_plan.rs:1976`), which for
  `HashJoinExec` clears `left_fut` (`joins/hash_join/exec.rs:407–411,1409`): the join build side is
  rebuilt every iteration, so an edges relation on the build side is re-scanned/re-hashed per
  iteration — cost ≈ iterations × |E| + output (inference from source).
- Only **one** self-reference is allowed: "Multiple recursive references to the same CTE are not
  supported" (390–411) — rules out non-linear (e.g. `path ⋈ path`) recursion.
- Memory: result buffer and dedup table reserve from the session memory pool (298, 316–330, 468–498).
  The repo configures no pool (`crates/cpg-core/src/snapshot.rs:45–62` sets only target partitions);
  DataFusion's default is `UnboundedMemoryPool` (`R/datafusion-execution-55.1.0/src/runtime_env.rs:365,495`),
  so recursive CTE memory is **unbounded** in the product today (inference from source).
- sqlparser 0.62 `Cte` has no SEARCH/CYCLE fields (`R/sqlparser-0.62.0/src/ast/query.rs:805–816`);
  PostgreSQL's `SEARCH … SET` / `CYCLE … SET … USING` are **not available in DataFusion**. (source)
- The datafusion skill has no brief on recursive CTEs (`reference.py find --task 'recursive CTE
  transitive closure'` returned nothing; silence ≠ absence). No built-in graph/transitive-closure
  operator exists besides `RecursiveQuery` (inference: skill silent, no such function in registry).

**Programmatic plans**: `LogicalPlanBuilder::scan(name, provider_as_source(Arc::new(CteWorkTable::new(name, schema))), None)`
(`R/datafusion-catalog-55.1.0/src/cte_worktable.rs:36–52`; how the SQL planner does it:
`R/datafusion-55.1.0/src/execution/session_state.rs:2032–2041`), then
`LogicalPlanBuilder::from(static_plan).to_recursive_query(name, recursive_plan, is_distinct)?.build()`.
Also `MemTable::try_new(schema, partitions: Vec<Vec<RecordBatch>>)` (needs ≥1 partition;
`R/datafusion-catalog-55.1.0/src/memory/table.rs:80–86`), `SessionContext::register_batch`,
`read_batch(es)` (`R/datafusion-55.1.0/src/execution/context/mod.rs:536,1799,1812`). (source)

**How Derived SQL runs today** (repo): `Derived::sql() -> String`
(`crates/cpg-schema/src/derived.rs:23`) → `sql::query(ctx, &T::sql())` = `ctx.sql_with_options(sql,
read_only())` with DDL/DML/statements disallowed (`crates/cpg-core/src/sql.rs:15–25`) → collect →
prepend `snapshot_id` column → `to_declared::<T>` strict cast → `concat_batches` →
`canonical_sort(key)` (`crates/cpg-core/src/derive.rs:22–43`). Values are bound via
`with_param_values` in `sql::fetch` (sql.rs:35–126), but some derived SQL still interpolates
`X'hex'` literals through `format!` (e.g. `crates/cpg-schema/src/concepts.rs:30–37`). Recursive CTEs
already in use: `concepts.rs:37` and `rules.rs:3024` (`UNION`, unknown-propagation closure),
`behavior.rs:2046` and `communities.rs:40` (`UNION ALL` with depth caps); ADR-0044
(`docs/adr/0044-analytics-algorithms.md:140–150`) records "type-term walks run as DataFusion
recursive CTEs with UNION ALL, relying on extraction's depth cap … row identity is an open item under W13".

**Fit (inference, Proposed)**: DataFusion `WITH RECURSIVE … UNION` is adequate for linear,
single-relation reachability/closure with set semantics over a published snapshot, and keeps the
declared-relation/Delta-validation path. It is weak for: multi-relation mutually recursive rules,
non-linear recursion, per-row witness/provenance (a witness column breaks dedup), negation/aggregation
through recursion, cycle detection metadata, and bounded memory. A Rust worklist (petgraph SCC then
iterate) or ascent 0.8.1 (rust-reasoning `reason.fixpoint-choice` / `reason.ascent-rules`: lattice
columns converge per key; stratified negation/aggregates compile-checked; B019–B021 runtime) covers
those. Ascent would add new lock versions (ascent 0.8.1 requires `dashmap 5.5`, `boxcar 0.1`,
`hashbrown 0.14` raw; lock has dashmap 6.2.1, boxcar 0.2.14; `R/ascent-0.8.1/Cargo.toml:128–168`),
default feature `par` pulls rayon (already locked 1.12.0).

---

## 3. arrow-ipc / arrow-ord / arrow-select / arrow-row 59.3

**Writer** (`R/arrow-ipc-59.3.0/src/writer.rs`): `FileWriter::try_new(w, &schema)`,
`try_new_buffered`, `try_new_with_options(w, &schema, IpcWriteOptions)` (1622–1662);
`write(&batch)`, `write_metadata(k, v)`, `finish()` (1684–1726). `IpcWriteOptions::try_new(alignment,
write_legacy_ipc_format, MetadataVersion)` (467), `try_with_compression(Some(CompressionType::ZSTD|LZ4_FRAME))`
(394; features enabled in this workspace), `with_dictionary_handling(DictionaryHandling::{Resend (default), Delta})`
(518, 1330–1341). `DictionaryTracker::new(error_on_replacement)` — **a FileWriter may not replace a
dictionary** mid-file (1360–1380), so a dictionary column whose values change across batches must be
written as one batch or with a unified dictionary. `StreamWriter` / `StreamEncoder` for streams
(1853–1930). The repo writes bundles with `IpcWriteOptions::try_new(64, false, V5)`
(`crates/cpg-core/src/bundle.rs:686–689`) and stage-cache files with default options. (source, repo)

**Reader** (`R/arrow-ipc-59.3.0/src/reader.rs`): `FileReader::try_new(r: Read+Seek, projection)`
(1391), `FileReaderBuilder::new().with_projection().with_max_footer_fb_tables().with_max_footer_fb_depth().build(r)`
(1154–1223), `num_batches`, `set_index`, `custom_metadata`, `unsafe with_skip_validation`. **Zero-copy**:
`FileDecoder::new(schema, footer.version())`, `.with_projection(Vec<usize>)`,
`.with_require_alignment(bool)` (default false: misaligned buffers are copied; aligned stay
zero-copy), `unsafe .with_skip_validation(bool)`, `read_dictionary(&Block, &Buffer)`,
`read_record_batch(&Block, &Buffer) -> Result<Option<RecordBatch>>` (1021–1153), with
`read_footer_length`, `arrow_ipc::root_as_footer`, `convert::fb_to_schema`; the doc example
(960–1018) points to upstream `arrow/examples/zero_copy_ipc.rs` for mmap. For mmap you need a
`Buffer` over the mapping (e.g. `Buffer::from_custom_allocation`) and a mapping crate: `memmap2
0.9.11` is already in `Cargo.lock` (via blake3) but not a direct dependency. `StreamReader` and
`StreamDecoder` are the stream counterparts. The repo's stage cache reads with
`FileReader::try_new(Cursor::new(bytes))` after `std::fs::read` (stage_cache.rs:88–90) — full copy,
not zero-copy. (source, repo)

**Representation caveat for content keys** (inference from writer source): IPC bytes encode
representation, not only logical value — view arrays write their variadic buffers as held
(writer.rs:1035–1074), dictionaries are written whole including unused values, and alignment/
compression options change bytes. The stage key therefore depends on the repo's normalisation
(`to_declared` cast + `canonical_sort` + `rebind_snapshot` before hashing, stage_cache.rs:28–41) to be
meaningful; keep dictionary/view types out of hashed inputs or normalise them first.

**Canonical sorted adjacency**: `arrow_ord::sort::lexsort_to_indices(&[SortColumn{values, options}],
limit) -> UInt32Array` (`R/arrow-ord-59.3.0/src/sort.rs:871–875,940–1010`) is **unstable**
(`sort_unstable_by`, 880; docs 45,137) — deterministic only if the key is total (unique); the repo's
`canonical_sort` relies on declared keys (`crates/cpg-schema/src/table.rs:47–67`).
`arrow_select::take::{take, take_arrays, take_record_batch}` (`R/arrow-select-59.3.0/src/take.rs:88,154,1121`).
CSR-style adjacency = lexsort by (src, dst) → take → offsets from run boundaries (e.g.
`arrow_ord::partition::partition` or a scan) (inference).
`arrow_row::RowConverter::new(Vec<SortField>)`, `convert_columns(&[ArrayRef]) -> Rows`, `append`,
`convert_rows`, `parser()`, `Rows::try_into_binary` / `from_binary` (`R/arrow-row-59.3.0/src/lib.rs:958–1270,1455`):
memcmp-comparable multi-column keys, good for hashing/joins/dedup in a Rust worklist; dictionaries
are flattened (lib.rs:120–143); `from_binary` expects bytes "produced by this RowConverter" and may
panic on malformed data (1200–1216), so row bytes are a process-local encoding, not a persisted
format (inference). (source)

---

## 4. delta-rs 58f07cd: Change Data Feed

**API** (source `D/crates/core/src/operations/load_cdf.rs:111–140,534–560`; skill
`.claude/skills/deltalake/content/capabilities/delta.cdf.md`, reviewed 2026-09-18, runtime-tested at
this rev): `DeltaTable::scan_cdf(self) -> CdfLoadBuilder`;
`.with_starting_version(v)`, `.with_ending_version(v)`, `.with_starting_timestamp(ts)`,
`.with_ending_timestamp(ts)`, `.with_allow_out_of_range()`;
`async build(&self, session: &dyn Session, filters: Option<&Arc<dyn PhysicalExpr>>) -> DeltaResult<Arc<dyn ExecutionPlan>>`,
`build_with_metrics(..)`; or `DeltaCdfTableProvider::try_new(cdf_builder)` for SQL. Output adds
`_change_type` (Utf8), `_commit_version` (UInt64), `_commit_timestamp` (Timestamp ms) (skill claim
delta.cdf.7). Bounds inclusive; end beyond head is **clamped** (delta.cdf.1, runtime); CDF must be
enabled (`delta.enableChangeDataFeed=true`) at the start version or it errors
`ChangeDataNotEnabled`/`ChangeDataNotRecorded` (load_cdf.rs:318–331; delta.cdf.6 runtime); CDF rejects
non-None column mapping (delta.cdf.3, runtime); an `ORDER BY` over the CDF provider hit a planning
failure in the skill probe (unknowns).

**Derivation** (load_cdf.rs:310–378): per version, `cdc` actions if present; otherwise `add` with
`data_change` → `insert`, `remove` with `data_change` → `delete`, same-path add/remove pairs →
deletion-vector-resolved pairs.

**Fit with the repo's model** (`docs/design/sections/storage-and-publication.md` §6.1–§6.3; repo):
tables are append-only (`delta.appendOnly`), **one commit per table per attempt**, every row carries
`snapshot_id`, publication = the later `snapshots` row, unpublished attempts' commits are
interleaved, and "There is no relational read across snapshots; a diff compares two snapshots read
separately" (§6.2 Limit). Consequences (inference):
- For append-only tables, CDF over [v_a+1, v_b] returns exactly the `add`-derived rows of the
  intervening commits as `insert` — the same rows the repo already reads with
  `snapshot::commit_provider` (`read_commit_entry(v)` → adds → `with_adds`). CDF adds no
  information; it would include unpublished attempts' commits that must be filtered out via
  `snapshots`.
- Each snapshot writes its **full** relation under a new `snapshot_id`; "what changed between two
  snapshots" is a keyed set difference of two whole relations (content IDs), not row updates. CDF
  never emits deletes/updates here. The repo already does this in memory (`crates/cpg-core/src/diff.rs:1–5`,
  id-based join across two snapshot-scoped sessions).
- Enabling CDF is a table-property/protocol change → a schema/verify migration under §6.3 (open-time
  verify compares properties exactly) and a fresh store (ADR-0048).
**Verdict**: reject CDF for snapshot diffs (no fit, cost of a migration, zero added information);
keep key-based diff of two snapshot-scoped reads. Conditional trigger: a move to in-place
update/delete (merge) tables with row-level change consumers.

---

## 5. PostgreSQL 18 + SQLx 0.9 + pgpq 0.12

**Target**: PG **18** only — startup assertion `server_version_num / 10000 != 18` →
"PostgreSQL major version 18 is required" (`crates/lctx-postgres/src/lib.rs:211–222`); installed
18.6 (`docs/postgresql.md:11`, `docs/pins.md:149`); test images postgres:18.6 and pgvector 0.8.6-pg18
(pins.md:152,157). Migrations `crates/lctx-postgres/migrations/202609270001…202609280012` (12 files).
No recursive SQL in migrations or `lctx-postgres/src` (rg; repo).

**WITH RECURSIVE / SEARCH / CYCLE** (upstream-doc, PG18 docs via Context7
`/websites/postgresql_18`, pages `queries-with.html`, `sql-select.html`): `UNION` discards rows
duplicating any previous result row; `UNION ALL` does not. `SEARCH {DEPTH|BREADTH} FIRST BY cols SET
seqcol` adds an ordering column; `CYCLE cols SET is_cycle [TO v DEFAULT d] USING path` adds a mark
column and an internal path column and **stops** recursion on a cycle; both only on a two-branch
`UNION [ALL]` recursive `WITH` (available since PG14; present in 18).

**SQLx 0.9 checked queries** (source `R/sqlx-postgres-0.9.0/src/connection/describe.rs:60–200`,
`R/sqlx-macros-core-0.9.0/src/query/output.rs:59–110,352–387`): `query!`/`query_as!` prepare the
statement; column nullability comes from `pg_attribute` via each column's origin table/attnum, then
an `EXPLAIN (VERBOSE, FORMAT JSON) EXECUTE … (NULL, …)` patch that **only** marks outer-join inner
sides nullable. CTE/recursive/computed columns have no table origin → `None` → macro default
`unwrap_or(true)` → `Option<T>`. Override with `AS "col!"` (non-null), `"col?"`, or `"col: Type"`.
Recursive queries therefore compile but yield `Option` columns unless annotated (inference from
source). Selecting the `CYCLE … USING path` column (an array of row values) is likely undecodable by
SQLx's static typing — keep it out of the output list (inference, unverified). Offline metadata
(`.sqlx`) must be refreshed after migration (§14.11 text). Runtime `sqlx::query_as` is typed
decoding, not compile-time checking (§14.11).

**pgpq 0.12** (source `R/pgpq-0.12.0/src/lib.rs:105–289`, `encoders/`): **encode-only**
`ArrowToPostgresBinaryEncoder::try_new(&schema)` / `try_new_with_encoders`, `write_header(&mut BytesMut)`,
`write_batch(&batch, &mut BytesMut)`, `write_footer(&mut BytesMut)`; no decoder (PG→Arrow is the
repo's own "declared Arrow reconstruction"); no `Dictionary` encoder (supported types:
Boolean, Int8–64, UInt8–64, Float16/32/64, Decimal32/64/128, Date32, Time32/64, Timestamp, Duration,
Utf8/LargeUtf8/Utf8View, Binary/LargeBinary/FixedSizeBinary, List/LargeList/FixedSizeList, Struct);
unsupported types error `TypeNotSupported`. Repo usage: `crates/lctx-postgres/src/projection.rs:21–48`
(budgeted encode into `BytesMut`) and `import.rs:630–655` (`tx.copy_in_raw("COPY pg_temp.lctx_stage(..)
FROM STDIN WITH(FORMAT BINARY)")` → `send` → `finish` count check → `INSERT … SELECT` into
`lctx_serving.*`). Arrow IPC generation files are the import source (`import.rs:111–125`
`FileReader`). (source, repo)

**Current persistence/hydration chain** (repo): Delta canonical facts (publication authority) →
Arrow IPC generation bundle (`bundle.rs`, alignment 64, V5) → PG18 serving projection via pgpq COPY →
JSON packet hydration in Rust (`crates/lctx-postgres/src/hydration.rs`, typed decode/encode +
`check_response`). Disposable rebuild stage artifacts are Arrow IPC files (`stage_cache.rs`).

---

## 6. moka 0.12.16

**Why it is in `Cargo.lock`**: only via `hickory-resolver 0.26.3` (`moka = { version = "0.12",
features = ["sync"] }`, `R/hickory-resolver-0.26.3/Cargo.toml:234–236`) ←
`datafusion-table-providers-common 0.13.1` ← `datafusion-table-providers-postgres` (owned fork rev
e6fc4c40) ← `cpg-core` (Cargo.lock reverse walk). Compiled with `sync` only (its lock deps are
crossbeam-*, parking_lot, portable-atomic, smallvec, tagptr, uuid, equivalent — no async-lock /
event-listener / futures-util). (repo lock + source)

**Features** (`R/moka-0.12.16/Cargo.toml [features]`): `default = []`; `sync`; `future = [async-lock,
event-listener, futures-util]`; `logging`; `quanta`; `atomic64`; `unstable-debug-counters`.
rust-version 1.71.1. Adopting `future::Cache` adds three crates' worth of deps (check versions
against lock before claiming none are new).

**API** (source):
- `sync::Cache::builder()` / `CacheBuilder`: `.max_capacity(u64)`, `.initial_capacity`,
  `.eviction_policy(EvictionPolicy::tiny_lfu() | lru())`, `.weigher(Fn(&K,&V)->u32)`,
  `.eviction_listener(Fn(Arc<K>, V, RemovalCause))`, `.time_to_live(Duration)`,
  `.time_to_idle(Duration)`, `.expire_after(impl Expiry<K,V>)`, `.support_invalidation_closures()`,
  `.name()`; `SegmentedCache` via `.segments(n)` (`src/sync/builder.rs:95–513`).
- `sync::Cache`: `get`, `insert`, `get_with(key, FnOnce()->V)`, `get_with_by_ref`,
  `optionally_get_with`, `try_get_with(key, FnOnce()->Result<V,E>) -> Result<V, Arc<E>>`,
  `invalidate`, `remove`, `invalidate_all`, `invalidate_entries_if`, `iter`, `run_pending_tasks`,
  `entry_count`, `weighted_size`, `policy` (`src/sync/cache.rs:626–1764`); entry API
  `entry(k)` / `entry_by_ref(&q)` → `or_insert_with`, `or_try_insert_with`, `or_optionally_insert_with`,
  `and_compute_with`, `and_try_compute_with`, `and_upsert_with` (`src/sync/entry_selector.rs:153–1089`).
- Coalesced init: "concurrent calls on the same not-existing key are coalesced into one evaluation
  of the `init` closure (as long as these closures return the same error type)"; `Err` is **not**
  inserted and is returned as `Arc<E>`; a panicking init panics only its caller and another waiter
  retries (cache.rs:1280–1365). `future::Cache::get_with/try_get_with(key, impl Future)` are
  `async` (`src/future/cache.rs:1049,1305`); if the initialising future is dropped, a
  `WaiterGuard` marks `EnclosingFutureAborted` and waiters retry
  (`src/future/value_initializer.rs:55–110`); future cache has `async_eviction_listener`
  (`future/builder.rs:350`).
- **Bounds are best-effort**: "All cache implementations perform a best-effort bounding of the map"
  (`src/lib.rs:15–17`); `entry_count`/`weighted_size` are estimates until `run_pending_tasks`
  (cache.rs:660–700); eviction happens in housekeeping (Context7 `/moka-rs/moka` MIGRATION-GUIDE:
  maintenance tasks include admission decisions, LRU eviction when max capacity is exceeded,
  expiry, invalidation, listener delivery). TinyLFU (default) may **reject admission** of a new
  entry (lib.rs:65–92; `policy.rs:72–115`).
- **Memory accounting caveats** (inference): the weigher returns a caller-defined `u32` "relative
  size" — it is not measured memory; values shared via `Arc` (e.g. `Arc<RecordBatch>` slices sharing
  buffers) are counted per entry, not per allocation; the capacity can be exceeded transiently between
  housekeeping runs; the LFU sketch and per-entry metadata are outside the weight. So moka weights
  cannot replace the hard request/response budgets §14.9/§14.11 require.
- **quick_cache**: registry has 0.6.24 (not in lock); crates.io latest **0.7.0** (`cargo search`,
  2026-09-29). `sync::Cache::new(items)`, `with_weighter(..)`, `get_or_insert_with` (coalescing)
  (`R/quick_cache-0.6.24/src/sync.rs:52–583`); smaller dependency surface; no TTL/TTI. (source/registry)

**Fit** (inference, Proposed): the serving tier's generations are immutable and addressed by
generation digest; PostgreSQL + typed hydration is the owner; §14.11 already conditions moka on a
measured repeated selection/hydration cost with full request/generation/policy keys. No measurement
exists. Keep as **conditional**; if adopted, prefer `sync::Cache` (already compiled; no new deps) with
`try_get_with` keyed by (generation digest, normalized request, policy digests) and explicit byte
weights, and keep hard budgets outside the cache.

---

## 7. differential-dataflow / timely

**Versions** (crates.io via `cargo search`/`cargo info`, 2026-09-29): `differential-dataflow 0.25.1`
(MIT, rust-version 1.86; deps columnar 0.13, columnation 0.1.1, fnv, paste, serde, smallvec,
timely 0.31) and `timely 0.31.0` (deps bincode 1.3, byteorder, columnar, columnation, itertools 0.14,
serde, smallvec, timely_bytes/communication/container/logging 0.31; default feature `getopts`). None
are in `Cargo.lock`; `columnar`, `bincode` are new (lock query). (registry metadata)

**Model** (upstream-doc, Context7 `/timelydataflow/differential-dataflow`, mdbook ch. 2.7, 3, 5.4):
collections of `(data, time, diff)` updates inside timely `worker.dataflow(..)`; recursion via
`iterate(|x| … .concat(x).distinct())`, arrangements (`arrange_by_key`) reused across iterations via
`enter`; inputs changed with `InputSession::insert/remove/advance_to/flush` and progress driven by
`worker.step()` until a `probe` passes the input time; results observed via `inspect`/`probe` or by
stashing a `TraceHandle`.

**Fit statement** (inference, Proposed): the product workload is batch recompilation per pinned
library release into immutable, validated snapshots; every published relation is re-derived and
validated whole (§6.1, §14.10), and reuse is keyed by exact input identity. DD's value — incremental
maintenance of recursive views under continuous insert/delete with logical time — has no consumer:
there is no long-lived stream of edits, no frontier/recovery contract, and outputs would still have
to be materialised to Arrow, sorted canonically, validated and published through Delta. Adopting it
adds a runtime (workers, progress tracking, arrangements' memory), ~10 crates, a second relational
engine beside DataFusion and a second fixpoint engine beside the S4 candidates, and makes provenance
and five-verdict/unknown semantics harder to preserve (diffs aggregate multiplicities, not witnesses).
The forward plan already lists it under "Avoid" (`docs/plans/behavioral-model-forward-plan_2026-09-24.md:857`)
with a conditional trigger at line 1132. **Recommendation: reject for current scope; trigger =
an actual continuous insert/delete view-maintenance consumer with explicit update/time/frontier and
recovery contracts** (unchanged from §14.11).

---

## 8. Decision-oriented synthesis (inference, Proposed)

(a) **Artifact dependency graph owner.** The cheapest structural improvement is a *declared* finite
DAG: make each stage declare its input relations (the loader's actual scans), parameters/policy and
code identity, derive `Step.dependencies` from those declarations instead of hand-writing them in
`rebuild.rs`, and keep the IPC artifact + recompute-and-compare admission. salsa 0.28.2 is
technically capable (cycles, durability, LRU, cancellation, optional persistence) but: it is frozen
at 0.28.2 by ty/ruff; its persistence is off in the product, has an index-order-dependent format and
unverified interaction with the unified ty build; it needs a new fact→input diff-applier to give
fine-grained reuse; accumulators are unavailable; and PR2 only measured root changes on a small
fixture. Differential dataflow has no consumer. So: declared DAG now; salsa stays conditional on the
§14.11 measurement; DD rejected.

(b) **Recursive closure.** Keep DataFusion `WITH RECURSIVE … UNION` for linear single-relation set
closure inside declared SQL relations (termination via global dedup; add a session memory pool if
recursion size is untrusted). Use a Rust worklist (petgraph SCC + iterate, `arrow_row` keys) for
multi-relation, witness-bearing, cyclic or non-linear closures; ascent remains the S4 comparison
candidate, not a default. Do not rely on UNION ALL + depth caps for cyclic graphs without an explicit
truncation signal.

(c) **Persistence/hydration.** Keep Delta canonical (authority), Arrow IPC for disposable stage
artifacts and generation bundles (consider `FileDecoder` + mmap only if load time is measured),
PostgreSQL 18 serving via pgpq COPY (cast dictionaries before COPY). Reject Delta CDF for snapshot
diffs. moka stays conditional on measured repeated hydration cost; it cannot enforce hard budgets.

## 9. Capability table

| Capability | Library / API (pinned) | Fit | Gaps | Integration burden | Recommendation (Proposed) | Evidence path |
|---|---|---|---|---|---|---|
| Stage dependency ownership (declared DAG) | none (Rust: `Stage` enum + declared scans; optional `petgraph` topo sort already in workspace) | High: finite, closed stage set; matches ADR-0081 | Dependencies hand-written today (`rebuild.rs:184–232`) | Low | **Adapt**: derive `Step.dependencies` from declared stage inputs; keep recompute-compare admission | `crates/cpg-core/src/rebuild.rs:20–51`; ADR-0081 §Options 3 |
| Incremental memoization | salsa 0.28.2 `#[salsa::input/interned/tracked]`, `cycle_fn`/`cycle_initial`/`cycle_result`, `Durability`, `lru`, `CancellationToken` | Medium for fine-grained catalog derivation; low for whole-stage artifacts | Pinned by ty (0.28.3/0.28.4 break ruff 0.0.14); no fact→input diff-applier; cycles ignore local cancellation; entry-order-sensitive if `cycle_fn` non-monotone; accumulators off | Medium–high (separate DB, input diffing, parity harness) | **Conditional**: trigger = §14.11 measured repeated fine-grained derivation exceeding coarse reuse | `R/salsa-0.28.2/src/cycle.rs:1–57,477–500`; `R/salsa-macros-0.28.2/src/tracked_fn.rs:268–305`; `docs/design_review/evidence/2026-09-28_pr2/` |
| Persisted memo cache | salsa `persistence` feature: `<dyn Database>::as_serialize`/`deserialize` | Low now | Off in product; format keyed by ingredient index (layout-dependent); all read inputs must be `persist`; accumulators lost; feature unification with ty untested | Medium (envelope, compatibility key, discard-on-mismatch) | **Conditional** (only with salsa adoption; disposable, version-qualified) | `R/salsa-0.28.2/src/database.rs:170–228`; `…/pr2/persistence-probe/src/main.rs`; `raw/salsa-persistence.json` |
| Continuous view maintenance | differential-dataflow 0.25.1 / timely 0.31.0 | None for batch-per-release | No update stream, frontier or recovery contract; provenance/unknown semantics | High (~10 new crates, runtime, second engine) | **Reject**; trigger = real continuous insert/delete maintenance consumer | `cargo info` 2026-09-29; Context7 `/timelydataflow/differential-dataflow`; forward plan:857,1132 |
| Recursive closure in SQL | DataFusion 55.1 `WITH RECURSIVE` (`enable_recursive_ctes=true`), `LogicalPlanBuilder::to_recursive_query`, `CteWorkTable` | Good for linear set closure | No max-iteration control; one self-reference; partition-0 only; build side rebuilt per iteration; nullable recursive schema; no SEARCH/CYCLE; unbounded memory pool in repo | Low (already used) | **Keep** for linear set closure; add memory pool / explicit truncation signal where inputs are untrusted | `R/datafusion-physical-plan-55.1.0/src/recursive_query.rs:52–500`; `R/datafusion-sql-55.1.0/src/cte.rs:81–205` |
| Recursive closure in Rust | worklist + petgraph SCC; `arrow_row::RowConverter`; ascent 0.8.1 | Good for multi-relation, witness-bearing, cyclic | Ascent adds dashmap 5 / boxcar 0.1 / hashbrown 0.14 raw; timeouts not budgets | Low (worklist) / Medium (ascent) | **Keep** worklist; ascent **conditional** (S4 comparison) | `.claude/skills/rust-reasoning/content/capabilities/reason.ascent-rules.md`; `R/ascent-0.8.1/Cargo.toml` |
| Disposable Arrow artifacts | arrow-ipc 59.3 `FileWriter`/`FileReader`/`FileDecoder`, `IpcWriteOptions` (zstd/lz4) | High | Current read copies whole file; FileWriter forbids dictionary replacement; IPC bytes are representation-dependent | Low | **Keep**; **adapt** to `FileDecoder` + mmap only after measured load cost | `R/arrow-ipc-59.3.0/src/reader.rs:960–1153`, `writer.rs:1330–1380,1593–1726`; `stage_cache.rs:55–127` |
| Canonical sorted adjacency | `arrow_ord::sort::lexsort_to_indices`, `arrow_select::take::take_record_batch`, `arrow_row` | High | lexsort unstable → requires total keys | Low | **Keep** (already `canonical_sort`) | `R/arrow-ord-59.3.0/src/sort.rs:880–1010`; `crates/cpg-schema/src/table.rs:47–67` |
| Snapshot change sets | delta-rs `DeltaTable::scan_cdf()` → `CdfLoadBuilder`, `DeltaCdfTableProvider` | None (append-only, full-relation-per-snapshot) | Only `insert` rows; includes unpublished attempts; needs table-property migration; rejects column mapping | Medium (migration) for no gain | **Reject**; keep id-based two-snapshot diff; trigger = in-place merge/update tables with row-change consumers | `D/crates/core/src/operations/load_cdf.rs:111–378`; `.claude/skills/deltalake/content/capabilities/delta.cdf.md`; storage §6.1–6.2 |
| Serving recursion | PostgreSQL 18 `WITH RECURSIVE … SEARCH/CYCLE`; SQLx 0.9 `query!` | Available if a serving-side closure is ever needed | CTE columns infer as `Option<T>` (need `"col!"`); `path` column likely undecodable; offline metadata refresh | Low | **Keep available**, not needed now | PG18 docs (Context7 `/websites/postgresql_18`); `R/sqlx-postgres-0.9.0/src/connection/describe.rs:60–200`; `R/sqlx-macros-core-0.9.0/src/query/output.rs:59–110` |
| Arrow→PG load | pgpq 0.12 `ArrowToPostgresBinaryEncoder` + SQLx `copy_in_raw` | High | Encode-only; no Dictionary encoder | Low (in place) | **Keep**; cast dictionaries before COPY | `R/pgpq-0.12.0/src/lib.rs:105–289`; `crates/lctx-postgres/src/projection.rs:21–48`, `import.rs:630–655` |
| Hydrated-object cache | moka 0.12.16 `sync::Cache` (`try_get_with`, `weigher`, `max_capacity`, `time_to_idle`, `eviction_listener`) | Unproven (no measurement) | Best-effort bounds; weights are caller-defined not memory; TinyLFU may reject; cannot enforce hard budgets | Low if `sync` (already compiled via hickory-resolver); more for `future` | **Conditional**: trigger = measured repeated immutable-generation selection/hydration cost (§14.11); alternative quick_cache 0.7.0 | `R/moka-0.12.16/src/lib.rs:15–92`, `src/sync/cache.rs:626–1764`, `src/future/value_initializer.rs:55–110`; Cargo.lock reverse deps |
